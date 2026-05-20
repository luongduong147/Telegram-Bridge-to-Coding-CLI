use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let token = std::env::var("TELEGRAM_BOT_TOKEN")
        .or_else(|_| std::env::var("BOT_TOKEN"))
        .unwrap_or_else(|_| "MISSING_TOKEN".into());

    // Test 1: with resolve (direct to IP)
    println!("=== Test 1: Direct with .resolve() ===");
    {
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(17))
            .connect_timeout(Duration::from_secs(5))
            .tcp_nodelay(true);
        let ips = ["149.154.167.99", "149.154.167.220", "91.108.56.100"];
        for ip in &ips {
            let addr: SocketAddr = format!("{}:443", ip).parse()?;
            builder = builder.resolve("api.telegram.org", addr);
        }
        let client = builder.build()?;
        test_get_me(&client, &token).await;
    }

    // Test 2: without resolve
    println!("\n=== Test 2: No resolve ===");
    {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()?;
        test_get_me(&client, &token).await;
    }

    Ok(())
}

async fn test_get_me(client: &reqwest::Client, token: &str) {
    let url = format!("https://api.telegram.org/bot{}/getMe", token);
    println!("Request: GET https://api.telegram.org/botTOKEN/getMe");
    match client.get(&url).send().await {
        Ok(resp) => {
            println!("  Status: {}", resp.status());
            let text = resp.text().await.unwrap_or_default();
            println!("  Body: {}", &text[..200.min(text.len())]);
        }
        Err(e) => {
            println!("  Error: {:?}", e);
            let mut source = e.source();
            while let Some(s) = source {
                println!("    caused by: {}", s);
                source = s.source();
            }
        }
    }
}
