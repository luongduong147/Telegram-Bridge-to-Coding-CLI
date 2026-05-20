use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let token = std::env::var("TELEGRAM_BOT_TOKEN")
        .or_else(|_| std::env::var("BOT_TOKEN"))
        .unwrap_or_else(|_| "MISSING_TOKEN".into());

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(17))
        .connect_timeout(Duration::from_secs(5))
        .build()?;

    // Test getMe
    let url = format!("https://api.telegram.org/bot{}/getMe", token);
    println!("=== getMe ===");
    match client.get(&url).send().await {
        Ok(resp) => println!("  OK: {}", resp.status()),
        Err(e) => println!("  FAIL: {:?}", e),
    }

    // Test sendMessage
    let url2 = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let body = serde_json::json!({
        "chat_id": 12345,
        "text": "test",
    });
    println!("=== sendMessage (expects 400) ===");
    match client.post(&url2).json(&body).send().await {
        Ok(resp) => println!("  OK: {}", resp.status()),
        Err(e) => println!("  FAIL: {:?}", e),
    }

    Ok(())
}
