use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use teloxide::prelude::*;
use crate::config::Config;
use crate::session::OpenCodeSession;
use crate::handler;

pub async fn run(config: Config) -> anyhow::Result<()> {
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(30))
        .build()?;
    let bot = Bot::with_client(&config.bot_token, http_client);

    let session = Arc::new(Mutex::new(
        OpenCodeSession::new(
            &config.opencode_workdir.to_string_lossy(),
            &config.opencode_workdir.to_string_lossy(),
        )
    ));

    let config = Arc::new(config);

    tracing::info!("Bot started, waiting for messages...");

    teloxide::repl(bot.clone(), move |bot: Bot, msg: Message| {
        let config = Arc::clone(&config);
        let session = Arc::clone(&session);
        async move {
            let mut session = session.lock().await;
            if let Err(e) = handler::handle_message(bot, msg, &config, &mut session).await {
                tracing::error!("Handler error: {:#}", e);
            }
            Ok(())
        }
    })
    .await;

    Ok(())
}
