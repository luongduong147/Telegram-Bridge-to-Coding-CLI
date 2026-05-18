use std::sync::Arc;
use tokio::sync::Mutex;
use teloxide::payloads::EditMessageTextSetters;
use teloxide::prelude::Requester;
use teloxide::{Bot, types::{CallbackQuery, MaybeInaccessibleMessage, ChatId, MessageId, ParseMode}};

use crate::executor;
use crate::AppState;

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

fn get_msg_info(msg: &MaybeInaccessibleMessage) -> (ChatId, MessageId) {
    (msg.chat().id, msg.id())
}

pub async fn handle_callback(
    bot: Bot,
    q: CallbackQuery,
    app_state: Arc<Mutex<AppState>>,
) -> HandlerResult {
    let data = q.data.as_deref().unwrap_or("");
    let msg = match &q.message {
        Some(m) => m,
        None => return Ok(()),
    };
    let (chat_id, message_id) = get_msg_info(msg);

    bot.answer_callback_query(q.id).await?;

    let mut state = app_state.lock().await;

    if data == "interrupt" {
        executor::set_interrupt();
        if let Some(ui_state) = state.ui_states.get_mut(&message_id.0) {
            ui_state.has_finished = true;
            let text = format!("{}\n\n\u{23f9} <b>Da dung theo yeu cau</b>", ui_state.render_html());
            bot.edit_message_text(chat_id, message_id, text)
                .parse_mode(ParseMode::Html)
                .await?;
        }
        return Ok(());
    }

    if data == "hide" {
        if let Some(ui_state) = state.ui_states.get_mut(&message_id.0) {
            ui_state.is_hidden = true;
            let text = ui_state.render_html();
            let keyboard = ui_state.build_keyboard();
            bot.edit_message_text(chat_id, message_id, text)
                .parse_mode(ParseMode::Html)
                .reply_markup(keyboard)
                .await?;
        }
        return Ok(());
    }

    if data == "unhide" {
        if let Some(ui_state) = state.ui_states.get_mut(&message_id.0) {
            ui_state.is_hidden = false;
            let text = ui_state.render_html();
            let keyboard = ui_state.build_keyboard();
            bot.edit_message_text(chat_id, message_id, text)
                .parse_mode(ParseMode::Html)
                .reply_markup(keyboard)
                .await?;
        }
        return Ok(());
    }

    let parts: Vec<&str> = data.split(':').collect();
    if parts.len() != 2 {
        return Ok(());
    }
    let action = parts[0];
    let index: usize = parts[1].parse().unwrap_or(0);

    if let Some(ui_state) = state.ui_states.get_mut(&message_id.0) {
        if let Some(block) = ui_state.blocks.get_mut(index) {
            match action {
                "expand" => block.is_expanded = true,
                "collapse" => block.is_expanded = false,
                _ => {}
            }
        }

        let text = ui_state.render_html();
        let keyboard = ui_state.build_keyboard();
        bot.edit_message_text(chat_id, message_id, text)
            .parse_mode(ParseMode::Html)
            .reply_markup(keyboard)
            .await?;
    }

    Ok(())
}
