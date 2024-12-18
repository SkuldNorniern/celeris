mod css;
mod dom;
mod html;
mod networking;
mod rendering;
mod logger;

use std::error::Error;
use log::{debug, info, warn};
use std::io::{self, Write};

pub struct Browser {
    config: BrowserConfig,
    networking: networking::NetworkManager,
    renderer: rendering::Renderer,
}

#[derive(Clone)]
pub struct BrowserConfig {
    pub headless: bool,
    pub debug: bool,
}

impl Browser {
    pub fn new(config: BrowserConfig) -> Result<Self, Box<dyn Error>> {
        let config_clone = config.clone();
        Ok(Self {
            config,
            networking: networking::NetworkManager::new()?,
            renderer: rendering::Renderer::new(config_clone.headless)?,
        })
    }

    pub async fn load_url(&mut self, url: &str) -> Result<(), Box<dyn Error>> {
        println!("\n[*] Loading: {}", url);
        info!(target: "browser", "Starting request for URL: {}", url);
        
        let response = self.networking.fetch(url).await?;
        println!("[+] Status: {}", response.status.code);
        
        let raw_content = String::from_utf8_lossy(&response.body);
        
        // Add debug print for raw content
        println!("\n[+] Raw Page Content:");
        println!("{}", "=".repeat(50));
        println!("{}", raw_content);
        println!("{}", "=".repeat(50));
        
        let html_content = if let Some(body_start) = raw_content.find("<!doctype") {
            raw_content[body_start..].to_string()
        } else if let Some(body_start) = raw_content.find("<html") {
            raw_content[body_start..].to_string()
        } else if let Some(body_start) = raw_content.find("<?xml") {
            raw_content[body_start..].to_string()
        } else if let Some(body_start) = raw_content.find("<body") {
            raw_content[body_start..].to_string()
        } else {
            raw_content.to_string()
        };

        if self.config.debug {
            println!("\n[DEBUG] Parsed HTML content:");
            println!("{}", &html_content[..500.min(html_content.len())]);
            println!("...\n");
        }

        debug!(target: "browser", "Starting HTML parsing");
        let mut parser = html::parser::Parser::new(html_content);
        let dom = parser.parse();

        let root = dom.root().ok_or("No root node found")?;
        
        // Add debug information about the root node
        println!("\n[+] DOM Root Node Info:");
        println!("{}", "=".repeat(50));
        println!("Tag: {:?}", root.node_type());
        println!("Children count: {}", root.children().len());
        
        // Print first level children
        println!("\nFirst level children:");
        for (i, child) in root.children().iter().enumerate() {
            println!("Child {}: {:?}", i, child.node_type());
        }
        println!("{}", "=".repeat(50));

        // Continue with DOM structure printing
        println!("\n[+] Parsed DOM Structure:");
        println!("{}", "=".repeat(50));
        self.print_dom_structure(root, 0);
        println!("{}", "=".repeat(50));

        debug!(target: "browser", "Found root node with {} children", root.children().len());

        // Parse CSS and apply styles
        let stylesheet = css::StyleSheet::new();
        let style_engine = css::style::StyleEngine::new(stylesheet);
        let styled_dom = style_engine.apply_styles(root);

        // Create display list and render
        let display_list = self.renderer.layout(&styled_dom);
        self.renderer.paint(&display_list)?;

        // Print text content
        println!("\n[+] Page Content:");
        println!("{}", "=".repeat(50));
        self.extract_content(root);
        println!("\n{}", "=".repeat(50));

        Ok(())
    }

    fn extract_content(&self, node: &dom::Node) {
        match node.node_type() {
            dom::NodeType::Element { tag_name, attributes } => {
                debug!(target: "browser", "Processing element: {} with {} children", 
                    tag_name, node.children().len());

                // Skip non-content elements
                if matches!(tag_name.as_str(), "script" | "style" | "meta" | "link") {
                    debug!(target: "browser", "Skipping non-content element: {}", tag_name);
                    return;
                }

                // Handle title specially
                if tag_name == "title" {
                    if let Some(first_child) = node.children().first() {
                        if let dom::NodeType::Text(text) = first_child.node_type() {
                            println!("\nTitle: {}\n", text.trim());
                        }
                    }
                }

                // Process children
                for child in node.children() {
                    self.extract_content(child);
                }

                // Add newlines after block elements
                if matches!(tag_name.as_str(), 
                    "div" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" |
                    "article" | "section" | "header" | "footer" | "br" |
                    "ul" | "ol" | "li" | "table" | "tr" | "form"
                ) {
                    println!();
                }
            }
            dom::NodeType::Text(text) => {
                let text = text.trim();
                if !text.is_empty() {
                    debug!(target: "browser", "Processing text node: {}", text);
                    print!("{} ", text);
                }
            }
            dom::NodeType::Comment(_) => {
                debug!(target: "browser", "Skipping comment node");
            }
        }
    }

    fn print_dom_structure(&self, node: &dom::Node, indent: usize) {
        let indent_str = " ".repeat(indent);
        
        match node.node_type() {
            dom::NodeType::Element { tag_name, attributes } => {
                println!("{}Element: <{}>", indent_str, tag_name);
                if !attributes.is_empty() {
                    println!("{}Attributes: {:?}", indent_str + "  ", attributes);
                }
                
                // Process all children
                for child in node.children() {
                    self.print_dom_structure(child, indent + 2);
                }
            }
            dom::NodeType::Text(text) => {
                let text = text.trim();
                if !text.is_empty() {
                    println!("{}Text: \"{}\"", indent_str, text);
                }
            }
            dom::NodeType::Comment(comment) => {
                println!("{}Comment: \"{}\"", indent_str, comment);
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logger with error handling
    logger::init(log::LevelFilter::Info).map_err(|e| format!("Failed to initialize logger: {}", e))?;
    
    let mut engine = Browser::new(BrowserConfig {
        headless: true,
        debug: true,
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
