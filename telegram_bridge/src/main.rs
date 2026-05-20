mod config;
mod bot;
mod session;
mod executor;
mod handler;
mod ui;
mod stream;
mod filter;
mod markdown;
mod markdownv2;
mod cli;
mod json_parser;

pub use bot::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "telegram_bridge=info".into())
        )
        .init();

    dotenvy::dotenv().ok();

    let config = config::Config::from_env()?;

    tracing::info!(
        workdirs = ?config.workdirs,
        "Starting Telegram Bridge"
    );

    bot::run(config).await
}
