use uuid::Uuid;

use crate::{archiver::archiver::Archiver, config::Config, mong::get_mong, MainError};

mod archiver;

pub async fn run(config: Config) -> Result<(), MainError> {
    let mong = get_mong(&config.mong_connstring).await?;

    let handler = Archiver {
        mong,
        guild_whitelist: config.guild_whitelist,
        session_id: Uuid::new_v4(),
    };

    let mut client = serenity::Client::builder(&config.discor_token)
        .event_handler(handler)
        .await?;

    println!("Starting client");

    if let Err(why) = client.start().await {
        eprintln!("Client error: {why:?}");
    }

    Ok(())
}
