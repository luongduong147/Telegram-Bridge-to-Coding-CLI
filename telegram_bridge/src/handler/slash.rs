use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::AsyncBufReadExt;
use teloxide::payloads::{EditMessageTextSetters, SendMessageSetters};
use teloxide::{Bot, prelude::Requester, types::{Message, ParseMode}};

use crate::config::Config;
use crate::session::OpenCodeSession;
use crate::executor;
use crate::cli::create_backend;
use crate::stream::InteractiveStream;
use crate::markdownv2;
use crate::json_parser;
use crate::AppState;

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub async fn handle_start(bot: &Bot, msg: &Message) -> HandlerResult {
    let text = "\
Telegram Bridge v2 — Coding CLI

Toi la cau noi Telegram <-> Coding CLI.

Slash Commands:
/check — Kiem tra session
/status — Trang thai chi tiet
/cli <name> — Chon CLI (opencode/codex/claude)
/workdir <index> — Chon thu muc lam viec
/quick <prompt> — Mode nhanh, output gon
/showthinking <prompt> — Chi tiet JSON, reasoning, tokens
/interrupt — Dung lenh dang chay
/help — Danh sach commands

Cach dung:
Gui tin nhan bat ky, toi se gui no lam prompt cho CLI
va tra ket qua ve day.";
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

pub async fn handle_check(
    bot: &Bot,
    msg: &Message,
    session: &Mutex<OpenCodeSession>,
) -> HandlerResult {
    let sess = session.lock().await;
    let expired = sess.is_expired();
    let remaining = sess.remaining().as_secs();
    let text = if expired {
        "[SESSION] Session da het han (qua 10 phut). Gui tin nhan de tao session moi.".to_string()
    } else {
        format!(
            "[SESSION] Con hieu luc trong {} phut {} giay",
            remaining / 60,
            remaining % 60
        )
    };
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

pub async fn handle_status(
    bot: &Bot,
    msg: &Message,
    session: &Mutex<OpenCodeSession>,
    config: &Config,
) -> HandlerResult {
    let sess = session.lock().await;
    let expired = sess.is_expired();
    let remaining = sess.remaining().as_secs();
    let cli_name = if sess.active_cli_name.is_empty() {
        &config.default_cli
    } else {
        &sess.active_cli_name
    };
    let wd = &config.workdirs[sess.active_workdir_index];
    let text = format!(
        "[Session Status]\n\n\
         CLI: {}\n\
         Workdir: {}\n\
         Trang thai: {}\n\
         Thoi gian con lai: {} phut {} giay\n",
        cli_name,
        wd.display(),
        if expired { "Het han" } else { "Hoat dong" },
        remaining / 60,
        remaining % 60,
    );
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

pub async fn handle_cli(
    bot: &Bot,
    msg: &Message,
    config: &Config,
    session: &Mutex<OpenCodeSession>,
) -> HandlerResult {
    let text = msg.text().unwrap_or("");
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 2 {
        let list: String = config.clis.iter()
            .map(|c| format!("\u{1f527} {}", c.name))
            .collect::<Vec<_>>()
            .join("\n");
        let current = &config.default_cli;
        bot.send_message(msg.chat.id,
            format!("Chon CLI:\n{}\n\nDung: /cli <ten>\nHien tai: {}", list, current)
        ).await?;
        return Ok(());
    }
    let name = parts[1];
    if config.get_cli(name).is_some() {
        let mut sess = session.lock().await;
        sess.active_cli_name = name.to_string();
        bot.send_message(msg.chat.id, format!("\u{2705} Da chuyen sang CLI: {}", name)).await?;
    } else {
        let available: Vec<&str> = config.clis.iter().map(|c| c.name.as_str()).collect();
        bot.send_message(msg.chat.id,
            format!("\u{274c} Khong tim thay CLI '{}'.\nCo: {:?}", name, available)
        ).await?;
    }
    Ok(())
}

pub async fn handle_interrupt(
    bot: &Bot,
    msg: &Message,
) -> HandlerResult {
    executor::set_interrupt();
    bot.send_message(msg.chat.id, "\u{23f9} Da gui tin hieu dung. Vui long cho...").await?;
    Ok(())
}

pub async fn handle_workdir(
    bot: &Bot,
    msg: &Message,
    config: &Config,
    session: &Mutex<OpenCodeSession>,
) -> HandlerResult {
    let text = msg.text().unwrap_or("");
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 2 {
        let list: String = config.workdirs.iter()
            .enumerate()
            .map(|(i, p)| format!("{}. \u{1f4c1} {}", i + 1, p.display()))
            .collect::<Vec<_>>()
            .join("\n");
        let current = config.current_workdir().display();
        bot.send_message(msg.chat.id,
            format!("Workdir:\n{}\n\nDung: /workdir <so>\nHien tai: {}", list, current)
        ).await?;
        return Ok(());
    }
    if let Ok(idx) = parts[1].parse::<usize>() {
        if idx >= 1 && idx <= config.workdirs.len() {
            let mut sess = session.lock().await;
            sess.active_workdir_index = idx - 1;
            bot.send_message(msg.chat.id,
                format!("\u{2705} Da chuyen sang workdir: {}", config.workdirs[idx - 1].display())
            ).await?;
            return Ok(());
        }
    }
    bot.send_message(msg.chat.id, "\u{274c} So thu tu khong hop le.").await?;
    Ok(())
}

pub async fn handle_quick(
    bot: Bot,
    msg: Message,
    config: Arc<Config>,
    session: Arc<Mutex<OpenCodeSession>>,
    _app_state: Arc<Mutex<AppState>>,
) -> HandlerResult {
    let text = msg.text().unwrap_or("");
    let prompt = text.strip_prefix("/quick").unwrap_or("").trim();
    if prompt.is_empty() {
        bot.send_message(msg.chat.id, "\u{274c} Nhap them prompt. VD: /quick viet file hello.py").await?;
        return Ok(());
    }

    let sent = bot.send_message(msg.chat.id, "\u{1f680} *Dang chay quick mode\\.\\.\\.*")
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    let mut sess = session.lock().await;
    let cli_config = if sess.active_cli_name.is_empty() {
        config.default_cli_config()
    } else {
        match config.get_cli(&sess.active_cli_name) {
            Some(c) => c,
            None => config.default_cli_config(),
        }
    };
    let mut backend = create_backend(cli_config);
    let workdir = config.workdirs[sess.active_workdir_index].to_string_lossy().to_string();
    let continue_session = !sess.is_expired();
    drop(sess);

    executor::clear_interrupt();

    let mut stream = match InteractiveStream::spawn(backend.as_ref(), &workdir, prompt, continue_session) {
        Ok(s) => s,
        Err(e) => {
            bot.edit_message_text(msg.chat.id, sent.id,
                format!("\u{274c} Loi: {}", markdownv2::escape(&e.to_string())))
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
            return Ok(());
        }
    };

    let mut all_blocks: Vec<(crate::ui::BlockType, String)> = Vec::new();
    loop {
        tokio::select! {
            line = stream.read_line() => {
                match line {
                    Ok(Some(line)) => {
                        let trimmed = line.trim().to_string();
                        if let Some((bt, content)) = backend.process_line(&trimmed) {
                            let filtered = crate::filter::filter_sensitive(&content);
                            all_blocks.push((bt, filtered));
                        }
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                if executor::is_interrupted() {
                    stream.kill().await;
                    break;
                }
            }
        }
    }

    let mut output = String::new();
    let mut current_bt: Option<crate::ui::BlockType> = None;
    for (bt, line) in &all_blocks {
        match current_bt {
            Some(ref cur) if cur == bt => {}
            _ => {
                if current_bt.is_some() {
                    output.push('\n');
                }
                current_bt = Some(bt.clone());
                match bt {
                    crate::ui::BlockType::CommandExec => output.push_str("\u{1f4bb} *Command:*\n"),
                    crate::ui::BlockType::Thinking => output.push_str("\u{1f9e0} *Response:*\n"),
                }
            }
        }
        output.push_str(&markdownv2::escape(line));
        output.push('\n');
    }

    {
        let mut sess = session.lock().await;
        sess.touch();
    }

    bot.edit_message_text(msg.chat.id, sent.id,
        if output.is_empty() { "\u{274c} Khong co output".to_string() } else { output }
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;

    Ok(())
}

pub async fn handle_showthinking(
    bot: Bot,
    msg: Message,
    config: Arc<Config>,
    session: Arc<Mutex<OpenCodeSession>>,
    _app_state: Arc<Mutex<AppState>>,
) -> HandlerResult {
    let text = msg.text().unwrap_or("");
    let prompt = text.strip_prefix("/showthinking").unwrap_or("").trim();
    if prompt.is_empty() {
        bot.send_message(msg.chat.id, "\u{274c} Nhap them prompt. VD: /showthinking viet file hello.py").await?;
        return Ok(());
    }

    let sent = bot.send_message(msg.chat.id, "\u{1f9e0} *Dang chay JSON mode\\.\\.\\.*")
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    let mut sess = session.lock().await;
    let cli_config = if sess.active_cli_name.is_empty() {
        config.default_cli_config()
    } else {
        match config.get_cli(&sess.active_cli_name) {
            Some(c) => c,
            None => config.default_cli_config(),
        }
    };
    let mut backend = create_backend(cli_config);
    let workdir = config.workdirs[sess.active_workdir_index].to_string_lossy().to_string();
    drop(sess);

    executor::clear_interrupt();

    let json_cmd = backend.build_json_command(&workdir, prompt);
    let mut std_cmd: std::process::Command = json_cmd;
    for var in crate::stream::CLEAR_ENV_VARS {
        std_cmd.env_remove(var);
    }
    std_cmd.stdout(std::process::Stdio::piped());
    std_cmd.stderr(std::process::Stdio::null());
    std_cmd.stdin(std::process::Stdio::null());

    let mut tokio_cmd: tokio::process::Command = std_cmd.into();
    let mut child = match tokio_cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            bot.edit_message_text(msg.chat.id, sent.id,
                format!("\u{274c} Loi spawn: {}", markdownv2::escape(&e.to_string())))
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
            return Ok(());
        }
    };

    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => {
            bot.edit_message_text(msg.chat.id, sent.id, "\u{274c} Khong the doc stdout".to_string()).await?;
            return Ok(());
        }
    };

    let reader = tokio::io::BufReader::new(stdout);
    let mut lines = reader.lines();
    let mut events: Vec<json_parser::NdjsonEvent> = Vec::new();

    loop {
        tokio::select! {
            line = lines.next_line() => {
                match line {
                    Ok(Some(l)) => {
                        if let Some(event) = json_parser::parse_ndjson(&l) {
                            events.push(event);
                        }
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                if executor::is_interrupted() {
                    let _ = child.kill().await;
                    break;
                }
            }
        }
    }

    let _ = child.wait().await;

    let parsed = json_parser::accumulate(&events);
    let mut out = String::new();

    if !parsed.tool_calls.is_empty() {
        out.push_str("\u{1f4bb} *Tool Calls:*\n");
        for tc in &parsed.tool_calls {
            out.push_str(&markdownv2::escape(&format!("  \u{2022} {}\n", tc)));
        }
        out.push('\n');
    }

    if !parsed.text_content.is_empty() {
        out.push_str("\u{1f9e0} *Response:*\n");
        for t in &parsed.text_content {
            out.push_str(&markdownv2::escape(t));
            out.push('\n');
        }
        out.push('\n');
    }

    out.push_str("*Tokens:*\n");
    if let Some(t) = parsed.total_tokens {
        out.push_str(&format!("  \u{2022} Total: {}\n", t));
    }
    if let Some(r) = parsed.reasoning_tokens {
        out.push_str(&format!("  \u{2022} Reasoning: {}\n", r));
    }
    if let Some(c) = parsed.cache_read {
        out.push_str(&format!("  \u{2022} Cache read: {}\n", c));
    }
    if let Some(c) = parsed.cache_write {
        out.push_str(&format!("  \u{2022} Cache write: {}\n", c));
    }

    {
        let mut sess = session.lock().await;
        sess.touch();
    }

    bot.edit_message_text(msg.chat.id, sent.id,
        if out.is_empty() { "\u{274c} Khong co JSON output".to_string() } else { out }
    )
    .parse_mode(ParseMode::MarkdownV2)
    .await?;

    Ok(())
}
