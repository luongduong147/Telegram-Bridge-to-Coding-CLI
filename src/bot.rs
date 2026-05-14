use std::sync::Arc;
use teloxide::prelude::*;
use crate::config::Config;
use crate::session::TmuxSession;
use crate::handler;

pub async fn run(config: Config) -> anyhow::Result<()> {
    let bot = Bot::new(&config.bot_token);
    let session = TmuxSession::new(&config.tmux_session_name, &config.opencode_workdir.to_string_lossy());

    let config = Arc::new(config);
    let session = Arc::new(session);

    tracing::info!("Bot started, waiting for messages...");

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let config = Arc::clone(&config);
        let session = Arc::clone(&session);
        async move {
            if let Err(e) = handler::handle_message(bot, msg, &config, &session).await {
                tracing::error!("Handler error: {:#}", e);
            }
            Ok(())
        }
    })
    .await;

    Ok(())
}
