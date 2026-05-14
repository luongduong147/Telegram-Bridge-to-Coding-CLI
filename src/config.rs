use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub bot_token: String,
    pub authorized_chat_id: Option<i64>,
    pub opencode_workdir: PathBuf,
    pub tmux_session_name: String,
    pub opencode_bin: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let bot_token = std::env::var("BOT_TOKEN")
            .or_else(|_| std::env::var("TELEGRAM_BOT_TOKEN"))
            .map_err(|_| anyhow::anyhow!("BOT_TOKEN or TELEGRAM_BOT_TOKEN is required"))?;

        let authorized_chat_id = match std::env::var("AUTHORIZED_CHAT_ID") {
            Ok(v) => Some(v.parse().map_err(|_| {
                anyhow::anyhow!("AUTHORIZED_CHAT_ID must be a numeric user ID")
            })?),
            Err(_) => None,
        };

        let opencode_workdir = std::env::var("OPENCODE_WORKDIR")
            .unwrap_or_else(|_| "/workspace".to_string())
            .into();

        let tmux_session_name = std::env::var("TMUX_SESSION_NAME")
            .unwrap_or_else(|_| "opencode-bridge".to_string());

        let opencode_bin = std::env::var("OPENCODE_BIN")
            .unwrap_or_else(|_| "opencode".to_string());

        Ok(Self {
            bot_token,
            authorized_chat_id,
            opencode_workdir,
            tmux_session_name,
            opencode_bin,
        })
    }

    pub fn is_authorized(&self, chat_id: i64) -> bool {
        match self.authorized_chat_id {
            Some(id) => chat_id == id,
            None => true,
        }
    }
}
