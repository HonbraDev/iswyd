use chrono::{
    serde::{ts_milliseconds, ts_milliseconds_option},
    DateTime, NaiveDateTime, Utc,
};
use serde::{Deserialize, Serialize};
use serenity::model::{
    application::{component::ActionRow, interaction::MessageInteraction},
    channel::{Attachment, Embed, Message, MessageType},
    event::MessageUpdateEvent,
    id::*,
    prelude::MessageReference,
    sticker::StickerItem,
    timestamp::Timestamp as SerenityTimestamp,
};
use thiserror::Error;
use uuid::Uuid;

pub type Timestamp = DateTime<Utc>;

pub fn convert_ts(ts: SerenityTimestamp) -> Timestamp {
    Timestamp::from_utc(
        NaiveDateTime::from_timestamp_millis((ts.unix_timestamp_nanos() / 1_000_000) as i64)
            .expect("already checked"),
        Utc,
    )
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "archive_type")]
pub enum ArchivedMessage {
    Full(ArchivedMessageFull),
    FullDeleted(ArchivedMessageFullDeleted),
    Incomplete(ArchivedMessageIncomplete),
    IncompleteDeleted(ArchivedMessageIncompleteDeleted),
    UnknownDeleted(ArchivedMessageUnknownDeleted),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchivedMessageFull {
    // Assumed to be static
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub guild_id: Option<GuildId>,
    pub author_id: UserId,
    #[serde(with = "ts_milliseconds")]
    pub timestamp: Timestamp,
    #[serde(rename = "type")]
    pub kind: ArchivedMessageType,
    pub message_reference: Option<MessageReference>,
    pub webhook_id: Option<WebhookId>,
    pub application_id: Option<ApplicationId>,
    pub interaction: Option<MessageInteraction>,

    // Tracked when editing
    /// The original body and subsequent modifications, may or may not contain
    /// the full history
    pub iterations: Vec<ArchivedMessageIteration>,
    pub marked_as_edited: bool,
}

impl ArchivedMessageFull {
    pub fn from_gateway(message: Message, session_id: Uuid) -> Self {
        Self {
            id: message.id,
            channel_id: message.channel_id,
            guild_id: message.guild_id,
            author_id: message.author.id,
            timestamp: convert_ts(message.timestamp),
            kind: message.kind.into(),
            message_reference: message.message_reference,
            webhook_id: message.webhook_id,
            application_id: message.application_id,
            interaction: message.interaction,
            iterations: vec![ArchivedMessageIteration {
                timestamp: convert_ts(message.timestamp),
                may_contain_gap: false,
                session_id,

                content: message.content,
                attachments: message.attachments,
                embeds: message.embeds,
                components: message.components,
                sticker_items: message.sticker_items,
            }],
            marked_as_edited: false,
        }
    }

    pub fn into_deleted(self, timestamp: Option<Timestamp>) -> ArchivedMessageFullDeleted {
        ArchivedMessageFullDeleted {
            id: self.id,
            channel_id: self.channel_id,
            guild_id: self.guild_id,
            author_id: self.author_id,
            timestamp: self.timestamp,
            kind: self.kind.into(),
            message_reference: self.message_reference,
            webhook_id: self.webhook_id,
            application_id: self.application_id,
            interaction: self.interaction,
            iterations: self.iterations,
            marked_as_edited: self.marked_as_edited,
            deleted_timestamp: timestamp,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchivedMessageFullDeleted {
    // Assumed to be static
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub guild_id: Option<GuildId>,
    pub author_id: UserId,
    #[serde(with = "ts_milliseconds")]
    pub timestamp: Timestamp,
    #[serde(rename = "type")]
    pub kind: ArchivedMessageType,
    pub message_reference: Option<MessageReference>,
    pub webhook_id: Option<WebhookId>,
    pub application_id: Option<ApplicationId>,
    pub interaction: Option<MessageInteraction>,

    // Tracked when editing
    /// The original body and subsequent modifications, may or may not contain
    /// the full history
    pub iterations: Vec<ArchivedMessageIteration>,
    pub marked_as_edited: bool,
    #[serde(with = "ts_milliseconds_option")]
    pub deleted_timestamp: Option<Timestamp>,
}

impl ArchivedMessageFullDeleted {
    #[allow(dead_code)]
    pub fn from_undeleted(message: ArchivedMessageFull, timestamp: Option<Timestamp>) -> Self {
        message.into_deleted(timestamp)
    }
}

/// We have first heard of this message when it was updated
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchivedMessageIncomplete {
    // Assumed to be static
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub guild_id: Option<GuildId>,
    pub author_id: UserId,
    #[serde(with = "ts_milliseconds")]
    pub timestamp: Timestamp,

    // Tracked when editing or deleting
    /// The original body and subsequent modifications, may or may not contain
    /// the full history
    pub iterations: Vec<ArchivedMessageIteration>,
    pub marked_as_edited: bool,
}

#[derive(Debug, Error)]
pub enum ArchivedMessageIncompleteFromSerenityError {
    #[error("no author in message update event")]
    NoAuthor,

    #[error("no timestamp in message update event")]
    NoTimestamp,
}

impl ArchivedMessageIncomplete {
    pub fn from_gateway(
        update: MessageUpdateEvent,
        timestamp: Timestamp,
        session_id: Uuid,
    ) -> Result<Self, ArchivedMessageIncompleteFromSerenityError> {
        let update2 = update.clone();
        Ok(Self {
            id: update.id,
            channel_id: update.channel_id,
            guild_id: update.guild_id,
            author_id: update
                .author
                .ok_or(ArchivedMessageIncompleteFromSerenityError::NoAuthor)?
                .id,
            timestamp: convert_ts(
                update
                    .timestamp
                    .ok_or(ArchivedMessageIncompleteFromSerenityError::NoTimestamp)?,
            ),
            iterations: vec![ArchivedMessageIteration::from_gateway(
                update2, timestamp, session_id,
            )],
            marked_as_edited: update.edited_timestamp.is_some(),
        })
    }
}

impl ArchivedMessageIncomplete {
    pub fn into_deleted(self, timestamp: Option<Timestamp>) -> ArchivedMessageIncompleteDeleted {
        ArchivedMessageIncompleteDeleted {
            id: self.id,
            channel_id: self.channel_id,
            guild_id: self.guild_id,
            author_id: self.author_id,
            timestamp: self.timestamp,
            iterations: self.iterations,
            marked_as_edited: self.marked_as_edited,
            deleted_timestamp: timestamp,
        }
    }
}

/// We have first heard of this message when it was updated
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchivedMessageIncompleteDeleted {
    // Assumed to be static
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub guild_id: Option<GuildId>,
    pub author_id: UserId,
    #[serde(with = "ts_milliseconds")]
    pub timestamp: Timestamp,

    // Tracked when editing or deleting
    /// The original body and subsequent modifications, may or may not contain
    /// the full history
    pub iterations: Vec<ArchivedMessageIteration>,
    pub marked_as_edited: bool,
    #[serde(with = "ts_milliseconds_option")]
    pub deleted_timestamp: Option<Timestamp>,
}

impl ArchivedMessageIncompleteDeleted {
    #[allow(dead_code)]
    pub fn from_undeleted(
        undeleted: ArchivedMessageIncomplete,
        timestamp: Option<Timestamp>,
    ) -> Self {
        undeleted.into_deleted(timestamp)
    }
}

/// We have very little data on this message and it has been deleted
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchivedMessageUnknownDeleted {
    pub id: MessageId,
    pub channel_id: ChannelId,
    pub guild_id: Option<GuildId>,
    pub deleted_timestamp: Option<Timestamp>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ArchivedMessageIteration {
    /// When the event was received / Discord says the iteration was created
    #[serde(with = "ts_milliseconds")]
    pub timestamp: Timestamp,
    /// Did we listen for this event ourselves or did we backfill it using the
    /// latest available version
    pub may_contain_gap: bool,
    /// Which session originally saved this iteration, used for determining if
    /// we *might* be missing some history
    pub session_id: Uuid,

    // The things that changed
    pub content: String,
    pub attachments: Vec<Attachment>,
    pub embeds: Vec<Embed>,
    pub components: Vec<ActionRow>,
    pub sticker_items: Vec<StickerItem>,
}

impl ArchivedMessageIteration {
    pub fn from_gateway(
        update: MessageUpdateEvent,
        timestamp: Timestamp,
        session_id: Uuid,
    ) -> Self {
        Self {
            timestamp,
            may_contain_gap: false,
            session_id,

            content: update.content.unwrap_or_default(),
            attachments: update.attachments.unwrap_or_default(),
            embeds: update.embeds.unwrap_or_default(),
            components: update.components.unwrap_or_default(),
            sticker_items: update.sticker_items.unwrap_or_default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum ArchivedMessageType {
    Regular = 0,
    GroupRecipientAddition = 1,
    GroupRecipientRemoval = 2,
    GroupCallCreation = 3,
    GroupNameUpdate = 4,
    GroupIconUpdate = 5,
    PinsAdd = 6,
    MemberJoin = 7,
    NitroBoost = 8,
    NitroTier1 = 9,
    NitroTier2 = 10,
    NitroTier3 = 11,
    ChannelFollowAdd = 12,
    GuildDiscoveryDisqualified = 14,
    GuildDiscoveryRequalified = 15,
    GuildDiscoveryGracePeriodInitialWarning = 16,
    GuildDiscoveryGracePeriodFinalWarning = 17,
    ThreadCreated = 18,
    InlineReply = 19,
    ChatInputCommand = 20,
    ThreadStarterMessage = 21,
    GuildInviteReminder = 22,
    ContextMenuCommand = 23,
    AutoModerationAction = 24,
    Unknown = !0,
}

impl From<MessageType> for ArchivedMessageType {
    fn from(value: MessageType) -> Self {
        match value {
            MessageType::Regular => Self::Regular,
            MessageType::GroupRecipientAddition => Self::GroupRecipientAddition,
            MessageType::GroupRecipientRemoval => Self::GroupRecipientRemoval,
            MessageType::GroupCallCreation => Self::GroupCallCreation,
            MessageType::GroupNameUpdate => Self::GroupNameUpdate,
            MessageType::PinsAdd => Self::PinsAdd,
            MessageType::MemberJoin => Self::MemberJoin,
            MessageType::NitroBoost => Self::NitroBoost,
            MessageType::NitroTier1 => Self::NitroTier1,
            MessageType::NitroTier2 => Self::NitroTier2,
            MessageType::NitroTier3 => Self::NitroTier3,
            MessageType::ChannelFollowAdd => Self::ChannelFollowAdd,
            MessageType::GuildDiscoveryDisqualified => Self::GuildDiscoveryDisqualified,
            MessageType::GuildDiscoveryRequalified => Self::GuildDiscoveryRequalified,
            MessageType::GuildDiscoveryGracePeriodInitialWarning => {
                Self::GuildDiscoveryGracePeriodInitialWarning
            }
            MessageType::GuildDiscoveryGracePeriodFinalWarning => {
                Self::GuildDiscoveryGracePeriodFinalWarning
            }
            MessageType::ThreadCreated => Self::ThreadCreated,
            MessageType::InlineReply => Self::InlineReply,
            MessageType::ChatInputCommand => Self::ChatInputCommand,
            MessageType::ThreadStarterMessage => Self::ThreadStarterMessage,
            MessageType::GuildInviteReminder => Self::GuildInviteReminder,
            MessageType::ContextMenuCommand => Self::ContextMenuCommand,
            MessageType::AutoModerationAction => Self::AutoModerationAction,
            _ => Self::Unknown,
        }
    }
}

/* #[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CachedUser {
    pub id: UserId,
    #[serde(rename = "username")]
    pub name: String,
    #[serde(with = "discriminator")]
    pub discriminator: u16,
    pub avatar: Option<String>,
    #[serde(default)]
    pub bot: bool,
}

impl From<User> for CachedUser {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            name: value.name,
            discriminator: value.discriminator,
            avatar: value.avatar,
            bot: value.bot,
        }
    }
} */
