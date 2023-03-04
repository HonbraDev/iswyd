use async_trait::async_trait;
use bson::doc;
use chrono::Utc;
use mongodb::options::{InsertOneOptions, UpdateOptions};
use serenity::{
    client::{Context, EventHandler},
    model::{
        channel::Message,
        event::MessageUpdateEvent,
        id::{ChannelId, GuildId, MessageId},
    },
};
use uuid::Uuid;

use crate::archived_message::{
    convert_ts, ArchivedMessage, ArchivedMessageFull, ArchivedMessageIncomplete,
    ArchivedMessageIteration, ArchivedMessageUnknownDeleted,
};

pub struct Archiver {
    pub ignored_guilds: Vec<GuildId>,
    pub ignored_channels: Vec<ChannelId>,
    pub mong: mongodb::Client,
    pub session_id: Uuid,
}

impl Archiver {
    pub fn mong_messages(&self) -> mongodb::Collection<ArchivedMessage> {
        self.mong.database("discor").collection("messages")
    }
}

#[async_trait]
impl EventHandler for Archiver {
    async fn message(&self, _ctx: Context, msg: Message) {
        if self.is_event_ignored(&msg.channel_id, &msg.guild_id) {
            return;
        }
        let message_id = msg.id;
        let archived = ArchivedMessageFull::from_gateway(msg, self.session_id);
        if let Err(err) = self
            .mong_messages()
            .insert_one(
                &ArchivedMessage::Full(archived),
                InsertOneOptions::default(),
            )
            .await
        {
            println!("Failed to insert new message into mong: {err}");
            return;
        }
        println!("Stored message {}", message_id);
    }

    async fn message_update(&self, _ctx: Context, update: MessageUpdateEvent) {
        if self.is_event_ignored(&update.channel_id, &update.guild_id) {
            return;
        }
        let message_id = update.id;
        let timestamp = update
            .edited_timestamp
            .and_then(|ts| Some(convert_ts(ts)))
            .unwrap_or_else(Utc::now);
        let marked_as_edited = update.edited_timestamp.is_some();

        let filter = doc! {
            "id": message_id.to_string(),
        };
        let db_message = match self.mong_messages().find_one(filter.clone(), None).await {
            Ok(m) => m,
            Err(err) => {
                println!("Couldn't fetch message {message_id} from mong: {err}");
                return;
            }
        };

        let new_message = match db_message {
            Some(db_message) => match db_message {
                ArchivedMessage::Full(mut db_message) => {
                    db_message
                        .iterations
                        .push(ArchivedMessageIteration::from_gateway(
                            update,
                            timestamp,
                            self.session_id,
                        ));
                    db_message.marked_as_edited = marked_as_edited;
                    ArchivedMessage::Full(db_message)
                }
                ArchivedMessage::Incomplete(mut db_message) => {
                    db_message
                        .iterations
                        .push(ArchivedMessageIteration::from_gateway(
                            update,
                            timestamp,
                            self.session_id,
                        ));
                    db_message.marked_as_edited = marked_as_edited;
                    ArchivedMessage::Incomplete(db_message)
                }
                _ => {
                    println!("Discor sent update for deleted message {message_id}??? wtf???");
                    return;
                }
            },
            None => ArchivedMessage::Incomplete(
                match ArchivedMessageIncomplete::from_gateway(update, timestamp, self.session_id) {
                    Ok(m) => m,
                    Err(err) => {
                        println!("Failed to create incomplete message from update event: {err}");
                        return;
                    }
                },
            ),
        };

        let encoded = match bson::to_bson(&new_message) {
            Ok(e) => e,
            Err(err) => {
                println!("Failed to serialize database message: {err}");
                return;
            }
        };
        let update = doc! {
            "$set": encoded,
        };
        let options = UpdateOptions::builder().upsert(true).build();
        match self
            .mong_messages()
            .update_one(filter, update, options)
            .await
        {
            Ok(_) => println!("Stored update for message {message_id}"),
            Err(err) => println!("Failed to store update for message {message_id}: {err}"),
        }
    }

    async fn message_delete(
        &self,
        _: Context,
        channel_id: ChannelId,
        id: MessageId,
        guild_id: Option<GuildId>,
    ) {
        if self.is_event_ignored(&channel_id, &guild_id) {
            return;
        }

        println!("Message {id} deleted");

        let timestamp = Utc::now();
        let filter = doc! {
            "id": id.to_string(),
        };
        let db_message = match self.mong_messages().find_one(filter.clone(), None).await {
            Ok(m) => m,
            Err(err) => {
                println!("Couldn't fetch message {id} from mong: {err}");
                return;
            }
        };

        let new_message = match db_message {
            Some(db_message) => match db_message {
                ArchivedMessage::Full(db_message) => {
                    ArchivedMessage::FullDeleted(db_message.into_deleted(Some(timestamp)))
                }
                ArchivedMessage::Incomplete(db_message) => {
                    ArchivedMessage::IncompleteDeleted(db_message.into_deleted(Some(timestamp)))
                }
                _ => {
                    println!("Discor sent delete event for deleted message {id}??? wtf???");
                    return;
                }
            },
            None => ArchivedMessage::UnknownDeleted(ArchivedMessageUnknownDeleted {
                id,
                channel_id,
                guild_id,
                deleted_timestamp: Some(timestamp),
            }),
        };

        let encoded = match bson::to_bson(&new_message) {
            Ok(e) => e,
            Err(err) => {
                println!("Failed to serialize database message: {err}");
                return;
            }
        };
        let update = doc! {
            "$set": encoded,
        };
        let options = UpdateOptions::builder().upsert(true).build();
        match self
            .mong_messages()
            .update_one(filter, update, options)
            .await
        {
            Ok(_) => println!("Stored update for message {id}"),
            Err(err) => println!("Failed to store update for message {id}: {err}"),
        }

        println!("Stored deletion timestamp of message {}", id);
    }

    async fn message_delete_bulk(
        &self,
        _: Context,
        _: ChannelId,
        message_ids: Vec<MessageId>,
        _: Option<GuildId>,
    ) {
        println!("bruh moment {message_ids:?}");
    }
}

impl Archiver {
    fn is_event_ignored(&self, channel_id: &ChannelId, guild_id: &Option<GuildId>) -> bool {
        match guild_id.as_ref() {
            Some(guild_id) => {
                self.ignored_channels.contains(channel_id) || self.ignored_guilds.contains(guild_id)
            }
            None => self.ignored_channels.contains(channel_id),
        }
    }
}
