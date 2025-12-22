use celeris::{Browser, BrowserConfig};
use log::info;
use std::error::Error;
use std::io::{self, Write}; // Assuming your crate is named celeris_browser

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logger with error handling
    celeris::logger::init(log::LevelFilter::Info)
        .map_err(|e| format!("Failed to initialize logger: {}", e))?;

    let mut engine = Browser::new(BrowserConfig {
        headless: true,
        debug: true,
        enable_javascript: false,
    })?;

    println!("Celeris Browser Engine");
    info!(target: "browser", "Browser engine initialized");

    // Get URL from user input
    print!("Enter URL (default: https://example.com): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let url = input.trim();
    let url = if url.is_empty() {
        String::from("https://example.com")
    } else if !url.starts_with("http://") && !url.starts_with("https://") {
        // Add https:// prefix if not present
        format!("https://{}", url)
    } else {
        url.to_string()
    };

    let start = std::time::Instant::now();
    engine.load_url(&url).await?;
    let duration = start.elapsed();

    println!("\n[+] Page loaded in: {:.2?}", duration);
    info!(target: "browser", "Page load completed in {:?}", duration);

    Ok(())
}
