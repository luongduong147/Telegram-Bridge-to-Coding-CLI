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
use crate::markdownv2;

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
    tracing::info!("handle_prompt: text={:?} chat={}", text, chat_id);

    let cli_config;
    let workdir;
    let continue_session;
    {
        let mut sess = session.lock().await;
        cli_config = if sess.active_cli_name.is_empty() {
            config.default_cli_config().clone()
        } else {
            match config.get_cli(&sess.active_cli_name) {
                Some(c) => c.clone(),
                None => config.default_cli_config().clone(),
            }
        };
        workdir = config.workdirs[sess.active_workdir_index].to_string_lossy().to_string();
        continue_session = !sess.is_expired();
    }

    let mut backend = create_backend(&cli_config);

    let sent = bot
        .send_message(chat_id, "\u{1f680} *Dang khoi tao\\.\\.\\.*")
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
    let message_id = sent.id;

    executor::clear_interrupt();

    tracing::info!("Spawning subprocess: workdir={}", workdir);
    let mut stream = match InteractiveStream::spawn(
        backend.as_ref(), &workdir, &text, continue_session,
    ) {
        Ok(s) => {
            tracing::info!("Subprocess spawned successfully");
            s
        }
        Err(e) => {
            tracing::error!("Failed to spawn subprocess: {}", e);
            let err_text = format!("\u{274c} Loi: {}", markdownv2::escape(&e.to_string()));
            bot.edit_message_text(chat_id, message_id, err_text)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
            return Ok(());
        }
    };

    let mut ui_state = MessageUiState::new();

    if let Some(ctx) = backend.get_user_context() {
        let escaped = markdownv2::escape(&ctx);
        ui_state.start_new_block(crate::ui::BlockType::UserContext, &escaped);
        let ctx_md = ui_state.render_markdown();
        let _ = bot.edit_message_text(chat_id, message_id, &ctx_md)
            .parse_mode(ParseMode::MarkdownV2)
            .await;
    }

    {
        let mut app = app_state.lock().await;
        app.ui_states.insert(message_id.0, ui_state.clone());
    }
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
                format!("{}\n\n\u{23f0} *Qua thoi gian cho*", ui_state.render_markdown()),
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await;
            break;
        }

        tokio::select! {
            line = stream.read_line() => {
                match line {
                    Ok(Some(line)) => {
                        tracing::debug!("line from subprocess: {:?}", &line[..line.len().min(80)]);
                        let trimmed = line.trim().to_string();
                        if let Some((bt, content)) = backend.process_line(&trimmed) {
                            let filtered = filter_sensitive(&content);
                            let display = if bt == crate::ui::BlockType::CommandExec {
                                filtered
                            } else {
                                markdownv2::escape(&filtered)
                            };
                            let should_start_new = ui_state.blocks.last()
                                .map(|b| b.block_type != bt)
                                .unwrap_or(true);
                            if should_start_new {
                                ui_state.start_new_block(bt.clone(), &display);
                            } else {
                                ui_state.push_line(&display);
                            }
                            has_pending = true;
                        }
                    }
                    Ok(None) => {
                        tracing::info!("Subprocess stdout closed (EOF)");
                        break;
                    }
                    Err(e) => {
                        tracing::error!("Read error: {}", e);
                        break;
                    }
                }
            }
            _ = sleep(Duration::from_millis(100)) => {
                if has_pending && last_edit.elapsed() >= DEBOUNCE {
                    let md = ui_state.render_markdown();
                    let kb = ui_state.build_keyboard();
                    let wait = RATE_LIMIT.saturating_sub(last_edit.elapsed());
                    if !wait.is_zero() {
                        sleep(wait).await;
                    }
                    bot.edit_message_text(chat_id, message_id, &md)
                        .parse_mode(ParseMode::MarkdownV2)
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
    tracing::info!("Process finished, sending final message");

    let parts = ui_state.split_into_messages();
    let kb = ui_state.build_keyboard();
    if let Some(first) = parts.first() {
        bot.edit_message_text(chat_id, message_id, first)
            .parse_mode(ParseMode::MarkdownV2)
            .reply_markup(kb)
            .await?;
    }
    for part in parts.iter().skip(1) {
        bot.send_message(chat_id, part)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
    }

    {
        let mut sess = session.lock().await;
        sess.touch();
    }

    let mut app = app_state.lock().await;
    app.ui_states.retain(|_, s| !s.has_finished);
    drop(app);

    Ok(())
}
