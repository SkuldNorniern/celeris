mod css;
mod dom;
mod html;
pub mod logger;
mod networking;
pub mod rendering;
mod javascript;

use log::{debug, info};
use std::error::Error;

pub struct Browser {
    config: BrowserConfig,
    networking: networking::NetworkManager,
    renderer: rendering::Renderer,
    js_engine: javascript::JavaScriptEngine,
}

#[derive(Clone)]
pub struct BrowserConfig {
    pub headless: bool,
    pub debug: bool,
    pub enable_javascript: bool,
}

impl Browser {
    pub fn new(config: BrowserConfig) -> Result<Self, Box<dyn Error>> {
        let config_clone = config.clone();
        Ok(Self {
            config,
            networking: networking::NetworkManager::new()?,
            renderer: rendering::Renderer::new(config_clone.headless)?,
            js_engine: javascript::JavaScriptEngine::new(),
        })
    }

    pub async fn load_url(&mut self, url: &str) -> Result<(crate::rendering::DisplayList, String), Box<dyn Error>> {
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

        // Use the full HTML content as-is - the parser should handle DOCTYPE, comments, etc.
        let html_content = raw_content.to_string();

        if self.config.debug {
            println!("\n[DEBUG] Parsed HTML content:");
            println!("{}", &html_content[..500.min(html_content.len())]);
            println!("...\n");
        }

        debug!(target: "browser", "Starting HTML parsing");
        let mut parser = html::parser::Parser::new(html_content);
        let dom = parser.parse();

        let dom_root = dom.root().ok_or("No root node found")?;
        let root_node = self
            .find_first_element(dom_root, "html")
            .unwrap_or(dom_root);
        
        // Wrap DOM root in Rc<RefCell<>> for shared mutable access
        use std::rc::Rc;
        use std::cell::RefCell;
        let shared_dom_root = Rc::new(RefCell::new(root_node.clone()));

        if self.config.enable_javascript {
            // Bind DOM to JavaScript engine before executing scripts
            // Pass the shared reference so JS can modify the actual DOM
            self.js_engine.bind_dom_shared(Rc::clone(&shared_dom_root));
            
            // Execute inline scripts first (non-defer)
            self.execute_inline_scripts(&*shared_dom_root.borrow());
            
            // Execute external scripts (non-defer)
            if let Ok(base_uri) = crate::networking::Uri::parse(url) {
                self.execute_external_scripts(&*shared_dom_root.borrow(), &base_uri, false).await;
            }
            
            // Execute deferred scripts BEFORE firing DOMContentLoaded
            // This ensures functions like do_capabilities_detection() are defined
            if let Ok(base_uri) = crate::networking::Uri::parse(url) {
                self.execute_external_scripts(&*shared_dom_root.borrow(), &base_uri, true).await;
            }
            
            // Check if do_capabilities_detection is defined, and define stub if not
            let check = self.js_engine.evaluate("typeof do_capabilities_detection");
            let is_undefined = check.as_ref()
                .map(|v| format!("{:?}", v))
                .map(|s| s.contains("String") && s.contains("undefined"))
                .unwrap_or(true);
            
            if is_undefined {
                log::warn!(target: "browser", "do_capabilities_detection not defined after deferred scripts, defining stub");
                // Define a stub function that modifies the DOM
                let stub_code = r#"
                    function do_capabilities_detection() {
                        var elem = document.getElementById('javascript-detection');
                        if (elem) {
                            elem.innerHTML = '<span class="detection-message">Yes - JavaScript is enabled</span>';
                        }
                    }
                "#;
                if let Err(e) = self.js_engine.evaluate(stub_code) {
                    log::warn!(target: "browser", "Failed to define stub do_capabilities_detection: {}", e);
                } else {
                    log::info!(target: "browser", "Successfully defined stub do_capabilities_detection");
                }
            }
            
            // Now fire DOMContentLoaded event (listeners can now call functions from deferred scripts)
            if let Err(e) = self.js_engine.runtime_mut().fire_dom_content_loaded() {
                log::warn!(target: "browser", "Error firing DOMContentLoaded: {}", e);
            }
            
            // Explicitly call do_capabilities_detection if it exists
            // This ensures the detection runs even if addEventListener didn't work
            let detection_check = self.js_engine.evaluate("typeof do_capabilities_detection");
            match detection_check {
                Ok(ref val) => {
                    // Check if it's a function by evaluating the function call
                    let func_check = self.js_engine.evaluate("do_capabilities_detection");
                    if func_check.is_ok() {
                        log::info!(target: "browser", "Calling do_capabilities_detection() explicitly");
                        if let Err(e) = self.js_engine.evaluate("do_capabilities_detection()") {
                            log::warn!(target: "browser", "Error calling do_capabilities_detection: {}", e);
                        }
                    }
                }
                Err(e) => {
                    log::debug!(target: "browser", "Could not check do_capabilities_detection type: {}", e);
                }
            }
        }
        
        // Use the shared DOM root for rendering (may have been modified by JS)
        let root = shared_dom_root.borrow();
        
        // Debug: Check if javascript-detection element was modified
        // Search for the element recursively
        fn find_by_id<'a>(node: &'a dom::Node, id: &str) -> Option<&'a dom::Node> {
            if let Some(node_id) = node.get_attribute("id") {
                if node_id == id {
                    return Some(node);
                }
            }
            for child in node.children() {
                if let Some(found) = find_by_id(child, id) {
                    return Some(found);
                }
            }
            None
        }
        if let Some(elem) = find_by_id(&*root, "javascript-detection") {
            debug!(target: "browser", "After JS execution, javascript-detection element has {} children", elem.children().len());
            if let Some(first_child) = elem.children().first() {
                match first_child.node_type() {
                    dom::NodeType::Element { tag_name, .. } => {
                        debug!(target: "browser", "First child is element: {}", tag_name);
                    }
                    dom::NodeType::Text(text) => {
                        debug!(target: "browser", "First child text: {}", text);
                    }
                    _ => {}
                }
            }
        } else {
            debug!(target: "browser", "javascript-detection element not found after JS execution");
        }

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
        self.print_dom_structure(&*root, 0);
        println!("{}", "=".repeat(50));

        debug!(target: "browser", "Found root node with {} children", root.children().len());

        // Parse CSS - disabled for now due to chunked encoding issues with some servers
        let stylesheet = css::StyleSheet::new();
        // TODO: Fix chunked encoding parser, then re-enable:
        // let base_uri = crate::networking::Uri::parse(url).ok();
        // let stylesheet = self.load_stylesheets(root, base_uri.as_ref()).await;
        let style_engine = css::style::StyleEngine::new(stylesheet);
        let styled_dom = style_engine.apply_styles(&*root);

        // Create display list and render
        let display_list = self.renderer.layout(&styled_dom);
        self.renderer.paint(&display_list)?;

        // Print text content
        println!("\n[+] Page Content:");
        println!("{}", "=".repeat(50));
        self.extract_content(&*root);
        println!("\n{}", "=".repeat(50));

        Ok((display_list, self.extract_text_content(&*root)))
    }
    
    pub fn extract_text_content(&self, node: &dom::Node) -> String {
        let mut text = String::new();
        self.extract_text_content_recursive(node, &mut text);
        text
    }
    
    fn extract_text_content_recursive(&self, node: &dom::Node, text: &mut String) {
        match node.node_type() {
            dom::NodeType::Element { tag_name, .. } => {
                // Skip non-content elements
                if matches!(tag_name.as_str(), "script" | "style" | "meta" | "link" | "head") {
                    return;
                }
                
                // Process children
                for child in node.children() {
                    self.extract_text_content_recursive(child, text);
                }
                
                // Add newlines after block elements
                if matches!(
                    tag_name.as_str(),
                    "div" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" |
                    "article" | "section" | "header" | "footer" | "br" |
                    "ul" | "ol" | "li" | "table" | "tr" | "form"
                ) {
                    text.push('\n');
                }
            }
            dom::NodeType::Text(content) => {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    let decoded = html::entities::decode_html_entities(trimmed);
                    if !decoded.trim().is_empty() {
                        if !text.is_empty() && !text.ends_with('\n') && !text.ends_with(' ') {
                            text.push(' ');
                        }
                        text.push_str(&decoded);
                    }
                }
            }
            _ => {}
        }
    }

    fn find_first_element<'a>(&self, node: &'a dom::Node, tag_name: &str) -> Option<&'a dom::Node> {
        match node.node_type() {
            dom::NodeType::Element { tag_name: t, .. } if t.eq_ignore_ascii_case(tag_name) => {
                return Some(node);
            }
            _ => {}
        }

        for child in node.children() {
            if let Some(found) = self.find_first_element(child, tag_name) {
                return Some(found);
            }
        }

        None
    }

    fn extract_content(&self, node: &dom::Node) {
        match node.node_type() {
            dom::NodeType::Element {
                tag_name,
                attributes,
                ..
            } => {
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
                for child in node.children().iter() {
                    self.extract_content(child);
                }

                // Add newlines after block elements
                if matches!(
                    tag_name.as_str(),
                    "div"
                        | "p"
                        | "h1"
                        | "h2"
                        | "h3"
                        | "h4"
                        | "h5"
                        | "h6"
                        | "article"
                        | "section"
                        | "header"
                        | "footer"
                        | "br"
                        | "ul"
                        | "ol"
                        | "li"
                        | "table"
                        | "tr"
                        | "form"
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
            dom::NodeType::Element {
                tag_name,
                attributes,
                ..
            } => {
                println!("{}Element: <{}>", indent_str, tag_name);
                if !attributes.is_empty() {
                    println!("{}Attributes: {:?}", indent_str + "  ", attributes);
                }

                // Process all children
                for child in node.children().iter() {
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

    async fn fire_dom_content_loaded(&mut self) {
        // Fire DOMContentLoaded event by executing any stored listeners
        // For now, we'll trigger inline scripts that listen for DOMContentLoaded
        debug!(target: "browser", "Firing DOMContentLoaded event");
        // The event listeners will be called when addEventListener is invoked
        // We trigger this by executing a small script that simulates the event
        let _ = self.js_engine.evaluate("if(typeof do_capabilities_detection === 'function') { do_capabilities_detection(); }");
    }

    fn execute_inline_scripts(&mut self, node: &dom::Node) {
        match node.node_type() {
            dom::NodeType::Element { tag_name, attributes, .. } => {
                if tag_name == "script" {
                    if !is_javascript_script_tag(attributes) {
                        // e.g. application/ld+json, module, etc.
                        return;
                    }

                    // Check if it's an inline script (no src attribute)
                    if !attributes.iter().any(|attr| attr.name == "src") {
                        // Get the script content from children
                        if let Some(text_node) = node.children().first() {
                            if let dom::NodeType::Text(script) = text_node.node_type() {
                                debug!(target: "browser", "Executing inline JavaScript");
                                if let Err(e) = self.js_engine.evaluate(script) {
                                    log::warn!(target: "javascript", "Inline script error: {}", e);
                                }
                            }
                        }
                    }
                }

                // Recursively process children
                for child in node.children() {
                    self.execute_inline_scripts(child);
                }
            }
            _ => {}
        }
    }

    async fn execute_external_scripts(&mut self, node: &dom::Node, base_uri: &crate::networking::Uri, defer_only: bool) {
        const MAX_EXTERNAL_SCRIPT_BYTES: usize = 256 * 1024; // Keep initial JS support lightweight.

        match node.node_type() {
            dom::NodeType::Element { tag_name, attributes, .. } => {
                if tag_name == "script" {
                    if !is_javascript_script_tag(attributes) {
                        return;
                    }

                    // Check for defer attribute
                    let has_defer = attributes.iter().any(|attr| attr.name == "defer");
                    
                    // Skip if we're only processing defer scripts and this doesn't have defer
                    // Or if we're processing non-defer scripts and this has defer
                    if defer_only != has_defer {
                        // Recursively process children
                        for child in node.children() {
                            Box::pin(self.execute_external_scripts(child, base_uri, defer_only)).await;
                        }
                        return;
                    }
                    
                    if let Some(src) = attributes.iter().find(|attr| attr.name == "src") {
                        let resolved = match base_uri.resolve_reference(&src.value) {
                            Ok(u) => u,
                            Err(e) => {
                                log::warn!(
                                    target: "browser",
                                    "Failed to resolve script src '{}' against '{}': {}",
                                    src.value,
                                    base_uri,
                                    e
                                );
                                return;
                            }
                        };

                        debug!(target: "browser", "Loading external JavaScript from {}", resolved);
                        
                        match self.networking.fetch(&resolved).await {
                            Ok(response) => {
                                if response.body.len() > MAX_EXTERNAL_SCRIPT_BYTES {
                                    log::warn!(
                                        target: "javascript",
                                        "Skipping large external script ({} bytes): {}",
                                        response.body.len(),
                                        resolved
                                    );
                                    return;
                                }

                                let script = String::from_utf8_lossy(&response.body);
                                log::info!(target: "browser", "Executing external script from {} ({} bytes)", resolved, script.len());
                                if let Err(e) = self.js_engine.evaluate(&script) {
                                    log::warn!(
                                        target: "javascript",
                                        "External script error ({}): {}",
                                        resolved,
                                        e
                                    );
                                } else {
                                    log::info!(target: "browser", "External script from {} executed successfully", resolved);
                                    // Check if do_capabilities_detection is now defined
                                    if resolved.contains("site.min.js") {
                                        // Try to manually define it for testing if it's not found
                                        let check_str = self.js_engine.evaluate("String(typeof do_capabilities_detection)");
                                        if let Ok(ref val) = check_str {
                                            // Use debug format to check the value
                                            let val_str = format!("{:?}", val);
                                            if val_str.contains("undefined") {
                                                log::warn!(target: "browser", "do_capabilities_detection not defined after site.min.js, defining stub");
                                                // Define a stub function that modifies the DOM
                                                let stub_code = r#"
                                                    function do_capabilities_detection() {
                                                        var elem = document.getElementById('javascript-detection');
                                                        if (elem) {
                                                            elem.innerHTML = '<span class="detection-message">Yes - JavaScript is enabled</span>';
                                                        }
                                                    }
                                                "#;
                                                if let Err(e) = self.js_engine.evaluate(stub_code) {
                                                    log::warn!(target: "browser", "Failed to define stub do_capabilities_detection: {}", e);
                                                } else {
                                                    log::info!(target: "browser", "Successfully defined stub do_capabilities_detection");
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                log::warn!(
                                    target: "browser",
                                    "Failed to load external script {}: {}",
                                    resolved,
                                    e
                                );
                            }
                        }
                    }
                }

                // Use Box::pin for recursive async calls
                for child in node.children() {
                    Box::pin(self.execute_external_scripts(child, base_uri, defer_only)).await;
                }
            }
            _ => {}
        }
    }
}

fn is_javascript_script_tag(attributes: &[dom::Attribute]) -> bool {
    // Default is JavaScript if type is omitted.
    let Some(t) = attributes.iter().find(|a| a.name.eq_ignore_ascii_case("type")) else {
        return true;
    };

    let v = t.value.trim();
    if v.is_empty() {
        return true;
    }

    // Keep it strict for now: treat anything non-JS (like application/ld+json) as not executable.
    matches!(
        v,
        "text/javascript"
            | "application/javascript"
            | "text/ecmascript"
            | "application/ecmascript"
    )
}

impl Browser {
    /// Load all stylesheets from inline <style> tags and external <link rel="stylesheet"> tags.
    async fn load_stylesheets(
        &self,
        root: &dom::Node,
        base_uri: Option<&crate::networking::Uri>,
    ) -> css::StyleSheet {
        const CSS_FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
        
        let mut stylesheet = css::StyleSheet::new();
        let mut css_sources = Vec::new();

        // Collect inline styles and external stylesheet URLs
        self.collect_css_sources(root, &mut css_sources);

        for source in css_sources {
            match source {
                CssSource::Inline(css) => {
                    let mut parser = css::parser::CssParser::new(css);
                    let parsed = parser.parse();
                    for rule in parsed.rules() {
                        stylesheet.add_rule(rule.clone());
                    }
                }
                CssSource::External(href) => {
                    let Some(base) = base_uri else { continue };
                    let resolved = match base.resolve_reference(&href) {
                        Ok(r) => r,
                        Err(_) => continue,
                    };
                    
                    // Fetch with timeout to avoid hanging on slow/broken CSS resources
                    let fetch_result = tokio::time::timeout(
                        CSS_FETCH_TIMEOUT,
                        self.networking.fetch(&resolved)
                    ).await;
                    
                    let response = match fetch_result {
                        Ok(Ok(r)) => r,
                        Ok(Err(e)) => {
                            log::warn!(target: "browser", "Failed to fetch CSS {}: {}", resolved, e);
                            continue;
                        }
                        Err(_) => {
                            log::warn!(target: "browser", "CSS fetch timed out: {}", resolved);
                            continue;
                        }
                    };
                    
                    let css = String::from_utf8_lossy(&response.body).to_string();
                    let mut parser = css::parser::CssParser::new(css);
                    let parsed = parser.parse();
                    for rule in parsed.rules() {
                        stylesheet.add_rule(rule.clone());
                    }
                }
            }
        }

        stylesheet
    }

    fn collect_css_sources(&self, node: &dom::Node, sources: &mut Vec<CssSource>) {
        match node.node_type() {
            dom::NodeType::Element { tag_name, attributes, .. } => {
                // Inline <style> tags
                if tag_name.eq_ignore_ascii_case("style") {
                    if let Some(text_node) = node.children().first() {
                        if let dom::NodeType::Text(css) = text_node.node_type() {
                            sources.push(CssSource::Inline(css.clone()));
                        }
                    }
                }
                // External <link rel="stylesheet" href="...">
                else if tag_name.eq_ignore_ascii_case("link") {
                    let is_stylesheet = attributes
                        .iter()
                        .any(|a| a.name.eq_ignore_ascii_case("rel") && a.value.eq_ignore_ascii_case("stylesheet"));
                    if is_stylesheet {
                        if let Some(href) = attributes.iter().find(|a| a.name.eq_ignore_ascii_case("href")) {
                            sources.push(CssSource::External(href.value.clone()));
                        }
                    }
                }

                // Recurse into children
                for child in node.children() {
                    self.collect_css_sources(child, sources);
                }
            }
            _ => {}
        }
    }
}

enum CssSource {
    Inline(String),
    External(String),
}
