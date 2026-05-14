use teloxide::{Bot, prelude::Requester, types::Message};
use crate::session::OpenCodeSession;

pub async fn handle_start(bot: &Bot, msg: &Message) -> anyhow::Result<()> {
    let text = "\
Telegram Bridge — OpenCode

Toi la cau noi Telegram <-> OpenCode CLI.

Slash Commands:
/check — Kiem tra session
/status — Trang thai chi tiet
/help — Danh sach commands

Cach dung:
Gui tin nhan bat ky, toi se gui no lam prompt cho OpenCode
va tra ket qua ve day.";
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

pub async fn handle_check(bot: &Bot, msg: &Message, session: &OpenCodeSession) -> anyhow::Result<()> {
    let expired = session.is_expired();
    let remaining = session.remaining().as_secs();
    let text = if expired {
        format!(
            "[SESSION] Session da het han (qua 10 phut). Gui tin nhan de tao session moi."
        )
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

pub async fn handle_status(bot: &Bot, msg: &Message, session: &OpenCodeSession) -> anyhow::Result<()> {
    let expired = session.is_expired();
    let remaining = session.remaining().as_secs();
    let text = format!(
        "[Session Status]\n\nWorkdir: {}\nTrang thai: {}\nThoi gian con lai: {} phut {} giay\n",
        session.workdir,
        if expired { "Het han" } else { "Hoat dong" },
        remaining / 60,
        remaining % 60,
    );
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

pub async fn handle_help(bot: &Bot, msg: &Message) -> anyhow::Result<()> {
    handle_start(bot, msg).await
}
