use teloxide::{Bot, prelude::Requester, types::Message};
use crate::session::TmuxSession;
use crate::executor;
use std::thread;

pub async fn handle_start(bot: &Bot, msg: &Message) -> anyhow::Result<()> {
    let text = "\
🤖 Telegram Bridge — OpenCode

Toi la cau noi Telegram <-> OpenCode CLI.

Slash Commands:
/check — Kiem tra tmux session
/status — Trang thai chi tiet
/kill — Kill tmux session
/restart — Restart tmux session
/clear — Clear tmux screen
/exec <cmd> — Chay lenh shell (debug)
/help — Danh sach commands

Cach dung:
Gui tin nhan bat ky, toi se gui no lam prompt cho OpenCode
va tra ket qua ve day.";
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

pub async fn handle_check(bot: &Bot, msg: &Message, session: &TmuxSession) -> anyhow::Result<()> {
    let exists = session.exists().unwrap_or(false);
    let text = if exists {
        format!("[OK] Session {} dang chay", session.name)
    } else {
        format!("[FAIL] Session {} khong ton tai. Dung /restart de tao moi.", session.name)
    };
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

pub async fn handle_status(bot: &Bot, msg: &Message, session: &TmuxSession) -> anyhow::Result<()> {
    let exists = session.exists().unwrap_or(false);
    let output = if exists {
        executor::capture_pane(&session.name).unwrap_or_default()
    } else {
        String::new()
    };

    let last_lines: Vec<&str> = output.lines().rev().take(10).collect::<Vec<_>>().into_iter().rev().collect();
    let preview = if last_lines.is_empty() { "(empty)" } else { &last_lines.join("\n") };

    let text = format!(
        "[Session Status]\n\nSession: {}\nStatus: {}\nWorkdir: {}\n\n[Last output]\n{}\n",
        session.name,
        if exists { "Running" } else { "Not found" },
        session.workdir,
        preview,
    );
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

pub async fn handle_kill(bot: &Bot, msg: &Message, session: &TmuxSession) -> anyhow::Result<()> {
    match session.kill() {
        Ok(_) => {
            bot.send_message(msg.chat.id, format!("[OK] Session {} da duoc kill", session.name)).await?;
        }
        Err(e) => {
            bot.send_message(msg.chat.id, format!("[ERR] Loi kill session: {}", e)).await?;
        }
    }
    Ok(())
}

pub async fn handle_restart(bot: &Bot, msg: &Message, session: &TmuxSession) -> anyhow::Result<()> {
    let _ = session.kill();
    thread::sleep(std::time::Duration::from_millis(500));
    match session.create() {
        Ok(_) => {
            bot.send_message(msg.chat.id, format!("[OK] Session {} da duoc tao lai", session.name)).await?;
        }
        Err(e) => {
            bot.send_message(msg.chat.id, format!("[ERR] Loi tao session: {}", e)).await?;
        }
    }
    Ok(())
}

pub async fn handle_clear(bot: &Bot, msg: &Message, session: &TmuxSession) -> anyhow::Result<()> {
    executor::clear_scrollback(&session.name).ok();
    bot.send_message(msg.chat.id, "[OK] Da clear tmux screen").await?;
    Ok(())
}

pub async fn handle_exec(bot: &Bot, msg: &Message, session: &TmuxSession, cmd: &str) -> anyhow::Result<()> {
    let full_cmd = format!("{} ; echo __EXEC_DONE__", cmd);
    executor::send_keys(&session.name, &full_cmd)?;
    executor::send_enter(&session.name)?;

    thread::sleep(std::time::Duration::from_secs(3));
    let output = executor::capture_pane(&session.name).unwrap_or_default();
    let lines: Vec<&str> = output.lines().rev().take(20).collect::<Vec<_>>().into_iter().rev().collect();
    let result = lines.join("\n");

    let text = if result.is_empty() { "(no output)" } else { &result };
    bot.send_message(msg.chat.id, text.to_string()).await?;
    Ok(())
}

pub async fn handle_help(bot: &Bot, msg: &Message) -> anyhow::Result<()> {
    handle_start(bot, msg).await
}
