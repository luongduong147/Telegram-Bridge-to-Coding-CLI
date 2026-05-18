use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use teloxide::payloads::{EditMessageTextSetters, SendMessageSetters};
use teloxide::prelude::Requester;
use teloxide::types::{Message, ParseMode};
use teloxide::Bot;

use crate::AppState;
use crate::config::Config;
use crate::executor::{self, is_interrupted};
use crate::session::OpenCodeSession;
use crate::cli::create_backend;
use crate::stream::InteractiveStream;
use crate::ui::MessageUiState;
use crate::filter::filter_sensitive;
use crate::markdown::markdown_to_html;

const DEBOUNCE: Duration = Duration::from_millis(2000);
const RATE_LIMIT: Duration = Duration::from_millis(1000);
const PROCESS_TIMEOUT: Duration = Duration::from_secs(120);

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub async fn handle_prompt(
    bot: Bot,
    msg: Message,
    config: Arc<Config>,
    session: Arc<Mutex<OpenCodeSession>>,
    app_state: Arc<Mutex<AppState>>,
) -> HandlerResult {
    let text = match msg.text() {
        Some(t) => t,
        None => return Ok(()),
    };
    let chat_id = msg.chat.id;

    let mut sess = session.lock().await;
    let cli_config = config.default_cli_config();
    let mut backend = create_backend(cli_config);
    let workdir = config.current_workdir().to_string_lossy().to_string();
    let continue_session = !sess.is_expired();

    let sent = bot
        .send_message(chat_id, "\u{1f680} <b>Dang khoi tao...</b>")
        .parse_mode(ParseMode::Html)
        .await?;
    let message_id = sent.id;

    executor::clear_interrupt();

    let mut stream = match InteractiveStream::spawn(
        backend.as_ref(), &workdir, &text, continue_session,
    ) {
        Ok(s) => s,
        Err(e) => {
            bot.edit_message_text(chat_id, message_id, format!("\u{274c} Loi: {}", e))
                .await?;
            return Ok(());
        }
    };

    let mut ui_state = MessageUiState::new();
    let mut last_edit = Instant::now();
    let mut has_pending = false;
    let start_time = Instant::now();

    loop {
        if is_interrupted() {
            stream.kill().await;
            break;
        }

        if start_time.elapsed() >= PROCESS_TIMEOUT {
            stream.kill().await;
            let _ = bot.edit_message_text(
                chat_id, message_id,
                format!("{}\n\n\u{23f0} <b>Qua thoi gian cho</b>", ui_state.render_html()),
            )
            .parse_mode(ParseMode::Html)
            .await;
            break;
        }

        tokio::select! {
            line = stream.read_line() => {
                match line {
                    Ok(Some(line)) => {
                        let trimmed = line.trim().to_string();
                        if let Some((bt, content)) = backend.process_line(&trimmed) {
                            let filtered = filter_sensitive(&content);
                            let html = markdown_to_html(&filtered);
                            for line in html.lines() {
                                let should_start_new = ui_state.blocks.last()
                                    .map(|b| b.block_type != bt)
                                    .unwrap_or(true);
                                if should_start_new {
                                    ui_state.start_new_block(bt.clone(), line);
                                } else {
                                    ui_state.push_line(line);
                                }
                            }
                            has_pending = true;
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        tracing::error!("Read error: {}", e);
                        break;
                    }
                }
            }
            _ = sleep(Duration::from_millis(100)) => {
                if has_pending && last_edit.elapsed() >= DEBOUNCE {
                    let md = ui_state.render_html();
                    let kb = ui_state.build_keyboard();
                    let wait = RATE_LIMIT.saturating_sub(last_edit.elapsed());
                    if !wait.is_zero() {
                        sleep(wait).await;
                    }
                    bot.edit_message_text(chat_id, message_id, &md)
                        .parse_mode(ParseMode::Html)
                        .reply_markup(kb)
                        .await
                        .ok();
                    last_edit = Instant::now();
                    has_pending = false;
                }
            }
        }
    }

    ui_state.has_finished = true;
    let final_text = ui_state.render_html();
    let kb = ui_state.build_keyboard();
    bot.edit_message_text(
        chat_id,
        message_id,
        format!("{}\n\n\u{2705} <b>Hoan thanh</b>", final_text),
    )
    .parse_mode(ParseMode::Html)
    .reply_markup(kb)
    .await?;

    sess.touch();

    let mut app = app_state.lock().await;
    app.ui_states.retain(|_, s| !s.has_finished);
    drop(app);

    Ok(())
}
