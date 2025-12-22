use celeris::{Browser, BrowserConfig};
use log::info;

#[cfg(feature = "gui")]
use celeris::rendering::gui::BrowserApp;

#[cfg(not(feature = "gui"))]
use std::error::Error;
#[cfg(not(feature = "gui"))]
use std::io::{self, Write};

#[cfg(feature = "gui")]
fn main() {
    // Install rustls crypto provider before any TLS operations
    if let Err(e) = rustls::crypto::ring::default_provider().install_default() {
        eprintln!("Failed to install rustls crypto provider: {:?}", e);
        return;
    }
    
    // Initialize logger with error handling
    if let Err(e) = celeris::logger::init(log::LevelFilter::Info) {
        eprintln!("Failed to initialize logger: {}", e);
        return;
    }

    let browser = match Browser::new(BrowserConfig {
        headless: false,
        debug: true,
        enable_javascript: true,
    }) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to create browser: {}", e);
            return;
        }
    };

    println!("Celeris Browser Engine");
    info!(target: "browser", "Browser engine initialized with GUI");

    // Launch GUI application
    let app = BrowserApp::with_browser(browser);
    app.run();
}

#[cfg(not(feature = "gui"))]
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Install rustls crypto provider before any TLS operations
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| "Failed to install rustls crypto provider")?;
    
    // Initialize logger with error handling
    celeris::logger::init(log::LevelFilter::Info)
        .map_err(|e| format!("Failed to initialize logger: {}", e))?;

    let mut engine = Browser::new(BrowserConfig {
        headless: true,
        debug: true,
        enable_javascript: true,
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
