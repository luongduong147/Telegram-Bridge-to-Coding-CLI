pub mod slash;
pub mod prompt;

use teloxide::{Bot, types::Message, utils::command::BotCommands};
use crate::config::Config;
use crate::session::OpenCodeSession;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum BotCommand {
    #[command(description = "Start bot & show help")]
    Start,
    #[command(description = "Check session status")]
    Check,
    #[command(description = "Show detailed session status")]
    Status,
    #[command(description = "Show this help")]
    Help,
}

pub async fn handle_message(
    bot: Bot,
    msg: Message,
    config: &Config,
    session: &mut OpenCodeSession,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id.0;
    if !config.is_authorized(chat_id) {
        tracing::warn!(chat_id, "Unauthorized access attempt");
        return Ok(());
    }

    let text = match msg.text() {
        Some(t) => t,
        None => return Ok(()),
    };

    match BotCommand::parse(text, "") {
        Ok(cmd) => handle_slash(bot, msg, cmd, session).await,
        Err(_) => {
            prompt::handle_prompt(&bot, &msg, text, config, session).await
        }
    }
}

async fn handle_slash(
    bot: Bot,
    msg: Message,
    cmd: BotCommand,
    session: &mut OpenCodeSession,
) -> anyhow::Result<()> {
    match cmd {
        BotCommand::Start => {
            slash::handle_start(&bot, &msg).await
        }
        BotCommand::Help => {
            slash::handle_help(&bot, &msg).await
        }
        BotCommand::Check => {
            slash::handle_check(&bot, &msg, session).await
        }
        BotCommand::Status => {
            slash::handle_status(&bot, &msg, session).await
        }
    }
}
