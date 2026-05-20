use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;
use teloxide::prelude::*;
use teloxide::types::UpdateKind;

use crate::config::Config;
use crate::handler;
use crate::session::OpenCodeSession;
use crate::ui::MessageUiState;

#[derive(Clone, Debug)]
pub struct AppState {
    pub ui_states: HashMap<i32, MessageUiState>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            ui_states: HashMap::new(),
        }
    }
}

pub async fn run(config: Config) -> anyhow::Result<()> {
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(30))
        .build()?;
    let bot = Bot::with_client(&config.bot_token, http_client);

    let config = Arc::new(config);
    let session = Arc::new(Mutex::new(OpenCodeSession::new()));
    let app_state = Arc::new(Mutex::new(AppState::new()));

    tracing::info!("Bot started, waiting for messages...");

    // Test connectivity
    match bot.get_me().await {
        Ok(me) => tracing::info!("Connected to Telegram as @{}", me.username.as_deref().unwrap_or("unknown")),
        Err(e) => tracing::warn!("Initial connectivity check failed: {}", e),
    }

    let mut offset = 0i32;

    loop {
        let updates = match bot.get_updates().offset(offset).timeout(30).await {
            Ok(upds) => upds,
            Err(e) => {
                tracing::error!("getUpdates error: {}", e);
                sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        for update in updates {
            offset = (update.id.0 as i32) + 1;
            tracing::info!("Update: kind={:?}", update.kind);

            match update.kind {
                UpdateKind::Message(msg) => {
                    tracing::info!("Message: {:?}", msg.text());
                    let b = bot.clone();
                    let c = Arc::clone(&config);
                    let s = Arc::clone(&session);
                    let a = Arc::clone(&app_state);
                    tokio::spawn(async move {
                        if let Err(e) = handler::handle_message(b, msg, c, s, a).await {
                            tracing::error!("handle_message error: {}", e);
                        }
                    });
                }
                UpdateKind::CallbackQuery(q) => {
                    let b = bot.clone();
                    let a = Arc::clone(&app_state);
                    tokio::spawn(async move {
                        if let Err(e) = handler::callback::handle_callback(b, q, a).await {
                            tracing::error!("handle_callback error: {}", e);
                        }
                    });
                }
                _ => {}
            }
        }

        sleep(Duration::from_millis(100)).await;
    }
}
