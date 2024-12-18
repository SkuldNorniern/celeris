mod css;
mod dom;
mod html;
mod networking;
mod rendering;

use std::error::Error;

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

        // Fetch the document
        let response = self.networking.fetch(url).await?;
        println!("[+] Status: {}", response.status.code);

        // Parse HTML
        let html_content = String::from_utf8_lossy(&response.body);
        let mut parser = html::parser::Parser::new(html_content.to_string());
        let dom = parser.parse();

        // Parse CSS (simplified for now)
        let stylesheet = css::StyleSheet::new();
        let style_engine = css::style::StyleEngine::new(stylesheet);

        // Apply styles and layout
        let root = dom.root().ok_or("No root node")?;
        let styled_dom = style_engine.apply_styles(root);

        if self.config.debug {
            self.print_dom_tree(root, 0)?;
        }

        // Create display list and render
        let display_list = self.renderer.layout(&styled_dom);
        self.renderer.paint(&display_list)?;

        // Print text content
        println!("\n[+] Page Content:");
        println!("{}", "=".repeat(50));
        self.print_content(root, 0);
        println!("{}", "=".repeat(50));

        Ok(())
    }

    fn print_dom_tree(&self, node: &dom::Node, depth: usize) -> Result<(), Box<dyn Error>> {
        let indent = "  ".repeat(depth);
        match node.node_type() {
            dom::NodeType::Element {
                tag_name,
                attributes,
            } => {
                let attrs: Vec<String> = attributes
                    .iter()
                    .map(|attr| format!("{}=\"{}\"", attr.name, attr.value))
                    .collect();
                let attrs_str = if attrs.is_empty() {
                    String::new()
                } else {
                    format!(" {}", attrs.join(" "))
                };
                println!("{}<{}{}>", indent, tag_name, attrs_str);
            }
            dom::NodeType::Text(text) => {
                let text = text.trim();
                if !text.is_empty() {
                    println!("{}{}", indent, text);
                }
            }
            dom::NodeType::Comment(comment) => {
                println!("{}<!-- {} -->", indent, comment);
            }
        }

        for child in node.children() {
            self.print_dom_tree(child, depth + 1)?;
        }

        if let dom::NodeType::Element { tag_name, .. } = node.node_type() {
            println!("{}</{}>\n", indent, tag_name);
        }

        Ok(())
    }

    fn print_content(&self, node: &dom::Node, depth: usize) {
        match node.node_type() {
            dom::NodeType::Text(text) => {
                let text = text.trim();
                if !text.is_empty() {
                    println!("{}{}", "  ".repeat(depth), text);
                }
            }
            dom::NodeType::Element { tag_name, .. } => match tag_name.as_str() {
                "br" => println!(),
                "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    if depth > 0 {
                        println!();
                    }
                    for child in node.children() {
                        self.print_content(child, depth + 1);
                    }
                    println!();
                }
                _ => {
                    for child in node.children() {
                        self.print_content(child, depth);
                    }
                }
            },
            _ => {
                for child in node.children() {
                    self.print_content(child, depth);
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut engine = Browser::new(BrowserConfig {
        headless: true,
        debug: false, // Set to true to see DOM tree
    })?;

    println!("Celeris Browser Engine");

    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://www.google.com".to_string());

    let start = std::time::Instant::now();
    engine.load_url(&url).await?;
    let duration = start.elapsed();

    println!("\n[+] Page loaded in: {:.2?}", duration);

    Ok(())
}
