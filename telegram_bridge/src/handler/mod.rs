pub mod slash;
pub mod prompt;
pub mod callback;

use std::sync::Arc;
use tokio::sync::Mutex;
use teloxide::{Bot, types::Message, utils::command::BotCommands};

use crate::config::Config;
use crate::session::OpenCodeSession;
use crate::AppState;

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum BotCommand {
    #[command(description = "Start bot & show help")]
    Start,
    #[command(description = "Check session status")]
    Check,
    #[command(description = "Show detailed session status")]
    Status,
    #[command(description = "Select CLI backend")]
    Cli,
    #[command(description = "Interrupt current execution")]
    Interrupt,
    #[command(description = "Select working directory")]
    Workdir,
    #[command(description = "Show this help")]
    Help,
}

pub async fn handle_message(
    bot: Bot,
    msg: Message,
    config: Arc<Config>,
    session: Arc<Mutex<OpenCodeSession>>,
    app_state: Arc<Mutex<AppState>>,
) -> HandlerResult {
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
        Ok(cmd) => handle_slash(bot, msg, cmd, config, session).await,
        Err(_) => {
            prompt::handle_prompt(bot, msg, config, session, app_state).await
        }
    }
}

async fn handle_slash(
    bot: Bot,
    msg: Message,
    cmd: BotCommand,
    config: Arc<Config>,
    session: Arc<Mutex<OpenCodeSession>>,
) -> HandlerResult {
    match cmd {
        BotCommand::Start | BotCommand::Help => {
            slash::handle_start(&bot, &msg).await
        }
        BotCommand::Check => {
            slash::handle_check(&bot, &msg, &session).await
        }
        BotCommand::Status => {
            slash::handle_status(&bot, &msg, &session, &config).await
        }
        BotCommand::Cli => {
            slash::handle_cli(&bot, &msg, &config).await
        }
        BotCommand::Interrupt => {
            slash::handle_interrupt(&bot, &msg).await
        }
        BotCommand::Workdir => {
            slash::handle_workdir(&bot, &msg, &config).await
        }
    }
}
