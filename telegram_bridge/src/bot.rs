use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use teloxide::dispatching::{Dispatcher, UpdateFilterExt};
use teloxide::prelude::*;
use teloxide::types::Update;

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
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(30))
        .build()?;
    let bot = Bot::with_client(&config.bot_token, http_client);

    let config = Arc::new(config);
    let session = Arc::new(Mutex::new(OpenCodeSession::new()));
    let app_state = Arc::new(Mutex::new(AppState::new()));

    tracing::info!("Bot started, waiting for messages...");

    let handler = teloxide::dptree::entry()
        .branch(
            Update::filter_message()
                .endpoint(handler::handle_message),
        )
        .branch(
            Update::filter_callback_query()
                .endpoint(handler::callback::handle_callback),
        );

    Dispatcher::builder(bot, handler)
        .dependencies(teloxide::dptree::deps![
            Arc::clone(&config),
            Arc::clone(&session),
            Arc::clone(&app_state)
        ])
        .build()
        .dispatch()
        .await;

    Ok(())
}
