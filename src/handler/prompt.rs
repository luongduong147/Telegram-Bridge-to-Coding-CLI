use teloxide::{Bot, prelude::Requester, types::Message};
use crate::config::Config;
use crate::session::TmuxSession;
use crate::executor;

pub async fn handle_prompt(
    bot: &Bot,
    msg: &Message,
    text: &str,
    config: &Config,
    session: &TmuxSession,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let sent = bot.send_message(chat_id, "Dang gui prompt toi OpenCode...").await?;

    let session_name = session.name.clone();
    let opencode_bin = config.opencode_bin.clone();
    let prompt_text = text.to_string();
    let result = tokio::task::spawn_blocking(move || {
        executor::send_prompt(&session_name, &opencode_bin, &prompt_text)
    })
    .await
    .map_err(|e| anyhow::anyhow!("Task join error: {}", e))?;

    bot.delete_message(chat_id, sent.id).await.ok();

    match result {
        Ok(output) => {
            let chunks = split_message(&output, 4096);
            for chunk in chunks {
                bot.send_message(chat_id, chunk).await?;
            }
        }
        Err(e) => {
            bot.send_message(chat_id, format!("Loi: {}", e)).await?;
        }
    }

    Ok(())
}

fn split_message(text: &str, max_len: usize) -> Vec<String> {
    if text.len() <= max_len {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let end = (start + max_len).min(text.len());
        if end < text.len() {
            if let Some(break_pos) = text[start..end].rfind('\n') {
                chunks.push(text[start..start + break_pos].to_string());
                start += break_pos + 1;
            } else {
                chunks.push(text[start..end].to_string());
                start = end;
            }
        } else {
            chunks.push(text[start..].to_string());
            break;
        }
    }
    chunks
}
