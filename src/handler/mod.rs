pub mod slash;
pub mod prompt;

use teloxide::{Bot, types::Message, utils::command::BotCommands};
use crate::config::Config;
use crate::session::TmuxSession;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum BotCommand {
    #[command(description = "Start bot & show help")]
    Start,
    #[command(description = "Check tmux session status")]
    Check,
    #[command(description = "Show detailed session status")]
    Status,
    #[command(description = "Kill the tmux session")]
    Kill,
    #[command(description = "Restart the tmux session")]
    Restart,
    #[command(description = "Clear tmux screen")]
    Clear,
    #[command(description = "Execute a shell command directly")]
    Exec(String),
    #[command(description = "Show this help")]
    Help,
}

pub async fn handle_message(
    bot: Bot,
    msg: Message,
    config: &Config,
    session: &TmuxSession,
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
            session.ensure_running()?;
            prompt::handle_prompt(&bot, &msg, text, config, session).await
        }
    }
}

async fn handle_slash(
    bot: Bot,
    msg: Message,
    cmd: BotCommand,
    session: &TmuxSession,
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
        BotCommand::Kill => {
            slash::handle_kill(&bot, &msg, session).await
        }
        BotCommand::Restart => {
            slash::handle_restart(&bot, &msg, session).await
        }
        BotCommand::Clear => {
            slash::handle_clear(&bot, &msg, session).await
        }
        BotCommand::Exec(cmd) => {
            slash::handle_exec(&bot, &msg, session, &cmd).await
        }
    }
}
