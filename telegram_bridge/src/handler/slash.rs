use tokio::sync::Mutex;
use teloxide::{Bot, prelude::Requester, types::Message};

use crate::config::Config;
use crate::session::OpenCodeSession;
use crate::executor;

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub async fn handle_start(bot: &Bot, msg: &Message) -> HandlerResult {
    let text = "\
Telegram Bridge v2 — Coding CLI

Toi la cau noi Telegram <-> Coding CLI.

Slash Commands:
/check — Kiem tra session
/status — Trang thai chi tiet
/cli — Chon CLI (OpenCode / Codex / Claude)
/workdir — Chon thu muc lam viec
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
) -> HandlerResult {
    let cli_list: String = config
        .clis
        .iter()
        .map(|c| format!("\u{1f527} {}", c.name))
        .collect::<Vec<_>>()
        .join("\n");
    let text = format!("Chon CLI:\n{}\n\nDung hien tai: {}", cli_list, config.default_cli);
    bot.send_message(msg.chat.id, text).await?;
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
) -> HandlerResult {
    let wd_list: String = config
        .workdirs
        .iter()
        .enumerate()
        .map(|(i, p)| format!("{}. \u{1f4c1} {}", i + 1, p.display()))
        .collect::<Vec<_>>()
        .join("\n");
    let current = config.current_workdir().display();
    let text = format!("Danh sach workdir:\n{}\n\nDung hien tai: {}", wd_list, current);
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

