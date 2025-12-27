use super::{DisplayList, DisplayItem, Color};
use crate::css::style::StyledNode;
use crate::css::{Value, Unit};
use crate::html::entities;
use std::collections::HashMap;

pub struct LayoutEngine {
    viewport_width: u32,
    viewport_height: u32,
    computed_styles: HashMap<String, ComputedStyle>,
    font_manager: FontManager,
}

#[derive(Clone, Debug)]
pub struct FontMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    pub line_height: f32,
}

pub struct FontManager {
    // Font cache and metrics
    font_cache: HashMap<String, FontMetrics>,
}

impl FontManager {
    pub fn new() -> Self {
        let mut font_cache = HashMap::new();

        // Default font metrics (approximations)
        font_cache.insert("default".to_string(), FontMetrics {
            ascent: 12.0,
            descent: 3.0,
            line_gap: 2.0,
            line_height: 17.0,
        });

        font_cache.insert("serif".to_string(), FontMetrics {
            ascent: 13.0,
            descent: 4.0,
            line_gap: 2.0,
            line_height: 19.0,
        });

        font_cache.insert("sans-serif".to_string(), FontMetrics {
            ascent: 12.0,
            descent: 3.0,
            line_gap: 1.0,
            line_height: 16.0,
        });

        font_cache.insert("monospace".to_string(), FontMetrics {
            ascent: 11.0,
            descent: 3.0,
            line_gap: 1.0,
            line_height: 15.0,
        });

        Self { font_cache }
    }

    pub fn get_metrics(&self, font_family: &[String], font_size: f32) -> &FontMetrics {
        // Find the first available font family
        for family in font_family {
            if let Some(metrics) = self.font_cache.get(family) {
                return metrics;
            }
        }

        // Fallback to default
        &self.font_cache["default"]
    }

    pub fn measure_text(&self, text: &str, font_family: &[String], font_size: f32) -> TextMetrics {
        let metrics = self.get_metrics(font_family, font_size);

        // Simple text measurement - assumes monospace for now
        // In a real browser, this would use actual font rendering
        let char_width = font_size * 0.6; // Approximate character width
        let width = text.chars().count() as f32 * char_width;

        TextMetrics {
            width,
            height: metrics.line_height,
            ascent: metrics.ascent,
            descent: metrics.descent,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    pub ascent: f32,
    pub descent: f32,
}

pub struct ComputedStyle {
    pub display: Display,
    pub position: Position,
    pub width: Dimension,
    pub height: Dimension,
    pub margin: Box<Edges>,
    pub padding: Box<Edges>,
    pub font_family: Vec<String>,
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub line_height: LineHeight,
    pub color: Color,
    pub text_align: TextAlign,
    pub vertical_align: VerticalAlign,
}

#[derive(Clone, Debug)]
pub enum FontWeight {
    Normal,
    Bold,
    Bolder,
    Lighter,
    Number(u16), // 100, 200, ..., 900
}

#[derive(Clone, Debug)]
pub enum LineHeight {
    Normal,
    Number(f32),
    Length(f32),
}

#[derive(Clone, Debug)]
pub enum TextAlign {
    Left,
    Right,
    Center,
    Justify,
}

#[derive(Clone, Debug)]
pub enum VerticalAlign {
    Baseline,
    Top,
    Middle,
    Bottom,
    Sub,
    Super,
    TextTop,
    TextBottom,
    Length(f32),
}

pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

pub enum Display {
    Block,
    Inline,
    None,
}

pub enum Position {
    Static,
    Relative,
    Absolute,
    Fixed,
}

pub enum Dimension {
    Auto,
    Length(f32),
    Percentage(f32),
}

impl LayoutEngine {
    pub fn new(viewport_width: u32, viewport_height: u32) -> Self {
        Self {
            viewport_width,
            viewport_height,
            computed_styles: HashMap::new(),
            font_manager: FontManager::new(),
        }
    }
    
    pub fn set_viewport_size(&mut self, width: u32, height: u32) {
        self.viewport_width = width;
        self.viewport_height = height;
    }

    pub fn viewport_width(&self) -> u32 {
        self.viewport_width
    }

    pub fn viewport_height(&self) -> u32 {
        self.viewport_height
    }

    pub fn compute_layout(&mut self, styled_node: &StyledNode) -> DisplayList {
        log::info!(target: "layout", "Starting layout computation with viewport: {}x{}", 
            self.viewport_width, self.viewport_height);
        
        // Log root node info
        match styled_node.node.node_type() {
            crate::dom::NodeType::Element { tag_name, .. } => {
                log::info!(target: "layout", "Root node: <{}> with {} children", tag_name, styled_node.node.children().len());
            }
            _ => {
                log::info!(target: "layout", "Root node type: {:?}", styled_node.node.node_type());
            }
        }
        
        let mut display_list = DisplayList::new();
        // Start layout at top of viewport (y=0)
        let height = self.layout_node(styled_node, 0.0, 0.0, &mut display_list);
        log::info!(target: "layout", "Layout complete, created {} display items, root height: {}", 
            display_list.items().len(), height);
        
        // Log first 10 items' positions to debug
        for (idx, item) in display_list.items().iter().take(10).enumerate() {
            match item {
                super::DisplayItem::Text { x, y, .. } => {
                    log::info!(target: "layout", "Item #{}: Text at ({}, {})", idx, x, y);
                }
                super::DisplayItem::Rectangle { x, y, width, height, .. } => {
                    log::info!(target: "layout", "Item #{}: Rectangle at ({}, {}), size {}x{}", idx, x, y, width, height);
                }
                super::DisplayItem::Image { x, y, width, height, .. } => {
                    log::info!(target: "layout", "Item #{}: Image at ({}, {}), size {}x{}", idx, x, y, width, height);
                }
                super::DisplayItem::Button { x, y, width, height, .. } => {
                    log::info!(target: "layout", "Item #{}: Button at ({}, {}), size {}x{}", idx, x, y, width, height);
                }
            }
        }
        
        // Log breakdown of items and sample x positions
        let (text, rect, img, btn) = display_list.items().iter().fold((0, 0, 0, 0), |(t, r, i, b), item| {
            match item {
                super::DisplayItem::Text { .. } => (t + 1, r, i, b),
                super::DisplayItem::Rectangle { .. } => (t, r + 1, i, b),
                super::DisplayItem::Image { .. } => (t, r, i + 1, b),
                super::DisplayItem::Button { .. } => (t, r, i, b + 1),
            }
        });
        log::info!(target: "layout", "Items breakdown: {} text, {} rects, {} images, {} buttons", text, rect, img, btn);
        
        // Log first 10 items' x positions to debug positioning
        for (idx, item) in display_list.items().iter().take(10).enumerate() {
            match item {
                super::DisplayItem::Text { x, .. } => {
                    log::info!(target: "layout", "Item #{}: Text at x={}", idx, x);
                }
                super::DisplayItem::Rectangle { x, width, .. } => {
                    log::info!(target: "layout", "Item #{}: Rectangle at x={}, width={}", idx, x, width);
                }
                super::DisplayItem::Image { x, width, .. } => {
                    log::info!(target: "layout", "Item #{}: Image at x={}, width={}", idx, x, width);
                }
                super::DisplayItem::Button { x, width, .. } => {
                    log::info!(target: "layout", "Item #{}: Button at x={}, width={}", idx, x, width);
                }
            }
        }
        
        display_list
    }

    fn layout_node(&mut self, node: &StyledNode, x: f32, y: f32, display_list: &mut DisplayList) -> f32 {
        // Log what node we're processing
        match node.node.node_type() {
            crate::dom::NodeType::Element { tag_name, .. } => {
                log::debug!(target: "layout", "layout_node: processing <{}> at ({}, {})", tag_name, x, y);
            }
            crate::dom::NodeType::Text(text) => {
                log::debug!(target: "layout", "layout_node: processing text '{}' at ({}, {})", 
                    text.chars().take(20).collect::<String>(), x, y);
            }
            _ => {}
        }
        
        // Handle special elements first (img, button, input) regardless of display type
        // This must happen BEFORE checking for script/style tags
        if let crate::dom::NodeType::Element { tag_name, .. } = node.node.node_type() {
            let tag_lower = tag_name.to_lowercase();
            match tag_lower.as_str() {
                "img" => {
                    let img_url = node.node.get_attribute("src").unwrap_or("").to_string();
                    let alt_text = node.node.get_attribute("alt").unwrap_or("").to_string();
                    // Try to get dimensions from attributes, with better defaults
                    let img_width = node.node.get_attribute("width")
                        .and_then(|w| w.parse::<f32>().ok())
                        .unwrap_or(200.0); // Default to 200px instead of 100px
                    let img_height = node.node.get_attribute("height")
                        .and_then(|h| h.parse::<f32>().ok())
                        .unwrap_or(200.0); // Default to 200px instead of 100px
                    
                    // Calculate proper x position: if x < padding, use padding, else use x
                    let padding = 20.0;
                    let img_x = if x < padding {
                        padding
                    } else {
                        x
                    };
                    
                    log::info!(target: "layout", "Found img element: src='{}', alt='{}', size={}x{} at ({}, {}) -> img_x={}", 
                        img_url, alt_text, img_width, img_height, x, y, img_x);
                    
                    display_list.add_item(DisplayItem::Image {
                        url: img_url,
                        x: img_x,
                        y,
                        width: img_width,
                        height: img_height,
                        alt: alt_text,
                    });
                    return img_height + 8.0; // Return height with margin for parent to track
                }
                "button" | "input" => {
                    let button_text = if tag_name == "button" {
                        // Extract text from button children
                        let mut text = String::new();
                        for child in node.node.children() {
                            if let crate::dom::NodeType::Text(t) = child.node_type() {
                                text.push_str(t.trim());
                            }
                        }
                        if text.is_empty() {
                            node.node.get_attribute("value").unwrap_or("Button").to_string()
                        } else {
                            text
                        }
                    } else {
                        // input element
                        node.node.get_attribute("value")
                            .or_else(|| node.node.get_attribute("placeholder"))
                            .unwrap_or("Input")
                            .to_string()
                    };
                    
                    // Calculate proper x position: if x < padding, use padding, else use x
                    let padding = 20.0;
                    let button_x = if x < padding {
                        padding
                    } else {
                        x
                    };
                    
                    log::debug!(target: "layout", "Found {} element: text={} at ({}, {}) -> button_x={}", tag_name, button_text, x, y, button_x);
                    
                    let button_width = 120.0;
                    let button_height = 32.0;
                    
                    display_list.add_item(DisplayItem::Button {
                        text: button_text,
                        x: button_x,
                        y,
                        width: button_width,
                        height: button_height,
                    });
                    return button_height; // Return height for parent to track
                }
                _ => {}
            }
        }
        
        // Skip script and style content - they should not be rendered
        if let crate::dom::NodeType::Element { tag_name, .. } = node.node.node_type() {
            let tag_lower = tag_name.to_lowercase();
            if matches!(tag_lower.as_str(), "script" | "style" | "noscript") {
                log::debug!(target: "layout", "Skipping {} element (not renderable)", tag_lower);
                // Don't process children of script/style tags - they should not be rendered
                return 0.0;
            }
        }
        
        // Also skip text nodes that are direct children of script/style tags
        // (This is a safety check - script tags should already be filtered above)
        if let crate::dom::NodeType::Text(_) = node.node.node_type() {
            // We can't easily check parent here, but script tags should be filtered above
        }
        
        // Basic layout algorithm - expand as needed
        let computed = self.compute_style(node);
        
        match computed.display {
            Display::Block => {
                // Handle block layout
                self.layout_block(node, x, y, &computed, display_list)
            },
            Display::Inline => {
                // Handle inline layout
                self.layout_inline(node, x, y, &computed, display_list)
            },
            Display::None => 0.0,
        }
    }

    pub fn compute_style(&self, node: &StyledNode) -> ComputedStyle {
        // Start with defaults based on element type
        let mut display = if let crate::dom::NodeType::Element { tag_name, .. } = node.node.node_type() {
            let tag_lower = tag_name.to_lowercase();
            match tag_lower.as_str() {
                "div" | "section" | "article" | "header" | "footer" | "main" | "body" | "html" | 
                "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "li" | 
                "blockquote" | "nav" | "aside" | "form" | "table" | "tr" | "td" | "th" => Display::Block,
                "span" | "a" | "strong" | "em" | "b" | "i" | "u" | "code" | "small" | "sub" | "sup" => Display::Inline,
                "img" | "button" | "input" => Display::Inline,
                _ => Display::Block,
            }
        } else {
            Display::Block
        };
        
        let mut position = Position::Static;
        let mut width = Dimension::Auto;
        let mut height = Dimension::Auto;
        let mut margin = Edges { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 };
        let mut padding = Edges { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 };
        let mut font_family = vec!["sans-serif".to_string()];
        let mut font_size = 16.0;
        let mut font_weight = FontWeight::Normal;
        let mut line_height = LineHeight::Normal;
        let mut color = Color { r: 0, g: 0, b: 0, a: 255 };
        let mut text_align = TextAlign::Left;
        let mut vertical_align = VerticalAlign::Baseline;
        
        // Apply CSS declarations from stylesheet
        for decl in &node.styles {
            match decl.property.to_lowercase().as_str() {
                "display" => {
                    if let Value::Keyword(kw) = &decl.value {
                        match kw.to_lowercase().as_str() {
                            "none" => display = Display::None,
                            "block" => display = Display::Block,
                            "inline" => display = Display::Inline,
                            _ => {}
                        }
                    }
                }
                "color" => {
                    if let Value::Color(c) = &decl.value {
                        color = Color { r: c.r, g: c.g, b: c.b, a: c.a };
                    } else if let Value::Keyword(kw) = &decl.value {
                        if let Some(c) = crate::css::Color::from_named(kw) {
                            color = Color { r: c.r, g: c.g, b: c.b, a: c.a };
                        }
                    }
                }
                "font-size" => {
                    if let Value::Length(val, unit) = &decl.value {
                        match unit {
                            Unit::Px => font_size = *val,
                            Unit::Em => font_size = *val * 16.0,
                            Unit::Rem => font_size = *val * 16.0,
                            Unit::Percent => font_size = *val * 16.0 / 100.0,
                            _ => font_size = *val,
                        }
                    }
                }
                "font-family" => {
                    if let Value::Multiple(values) = &decl.value {
                        font_family = values.iter().filter_map(|v| {
                            if let Value::Keyword(kw) = v {
                                Some(kw.clone())
                            } else if let Value::String(s) = v {
                                Some(s.clone())
                            } else {
                                None
                            }
                        }).collect();
                    } else if let Value::Keyword(kw) = &decl.value {
                        font_family = vec![kw.clone()];
                    }
                }
                "font-weight" => {
                    if let Value::Keyword(kw) = &decl.value {
                        match kw.to_lowercase().as_str() {
                            "bold" | "bolder" => font_weight = FontWeight::Bold,
                            "normal" => font_weight = FontWeight::Normal,
                            "lighter" => font_weight = FontWeight::Lighter,
                            _ => {
                                if let Ok(num) = kw.parse::<u16>() {
                                    font_weight = FontWeight::Number(num);
                                }
                            }
                        }
                    }
                }
                "line-height" => {
                    if let Value::Length(val, _) = &decl.value {
                        line_height = LineHeight::Length(*val);
                    } else if let Value::Keyword(kw) = &decl.value {
                        if kw.to_lowercase() == "normal" {
                            line_height = LineHeight::Normal;
                        }
                    }
                }
                "text-align" => {
                    if let Value::Keyword(kw) = &decl.value {
                        match kw.to_lowercase().as_str() {
                            "left" => text_align = TextAlign::Left,
                            "right" => text_align = TextAlign::Right,
                            "center" => text_align = TextAlign::Center,
                            "justify" => text_align = TextAlign::Justify,
                            _ => {}
                        }
                    }
                }
                "margin" | "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => {
                    if let Value::Length(val, unit) = &decl.value {
                        let px_val = match unit {
                            Unit::Px => *val,
                            _ => *val,
                        };
                        match decl.property.to_lowercase().as_str() {
                            "margin-top" => margin.top = px_val,
                            "margin-right" => margin.right = px_val,
                            "margin-bottom" => margin.bottom = px_val,
                            "margin-left" => margin.left = px_val,
                            "margin" => {
                                margin.top = px_val;
                                margin.right = px_val;
                                margin.bottom = px_val;
                                margin.left = px_val;
                            }
                            _ => {}
                        }
                    }
                }
                "padding" | "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => {
                    if let Value::Length(val, unit) = &decl.value {
                        let px_val = match unit {
                            Unit::Px => *val,
                            _ => *val,
                        };
                        match decl.property.to_lowercase().as_str() {
                            "padding-top" => padding.top = px_val,
                            "padding-right" => padding.right = px_val,
                            "padding-bottom" => padding.bottom = px_val,
                            "padding-left" => padding.left = px_val,
                            "padding" => {
                                padding.top = px_val;
                                padding.right = px_val;
                                padding.bottom = px_val;
                                padding.left = px_val;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        
        ComputedStyle {
            display,
            position,
            width,
            height,
            margin: Box::new(margin),
            padding: Box::new(padding),
            font_family,
            font_size,
            font_weight,
            line_height,
            color,
            text_align,
            vertical_align,
        }
    }

    fn layout_block(&mut self, node: &StyledNode, x: f32, y: f32, style: &ComputedStyle, display_list: &mut DisplayList) -> f32 {
        let mut current_y = y;
        // Use viewport-based padding and margins for better layout
        // Increased padding for better left/right margins
        let padding = 20.0;
        let font_metrics = self.font_manager.get_metrics(&style.font_family, style.font_size);
        let line_height = match style.line_height {
            LineHeight::Normal => font_metrics.line_height,
            LineHeight::Number(n) => style.font_size * n,
            LineHeight::Length(h) => h,
        };
        let margin = 12.0;
        let block_start_y = current_y;
        
        // Skip non-content elements, but still process their children
        // CRITICAL: For skipped elements like #document, head, html, body, etc., we should NOT accumulate Y
        // because they don't take up visual space. Their children should start at the same Y position.
        if let crate::dom::NodeType::Element { tag_name, .. } = node.node.node_type() {
            let tag_lower = tag_name.to_lowercase();
            if matches!(tag_lower.as_str(), "script" | "style" | "meta" | "link" | "head" | "title" | "#document" | "html" | "body") {
                // For skipped elements, process children at the SAME Y position (don't accumulate)
                // This prevents invisible elements from pushing content down
                // For html/body, we want their children to start at y=0 (or the passed y)
                let mut max_child_height: f32 = 0.0;
                for child in node.node.children() {
                    let styled_child = crate::css::style::StyledNode::new(child.clone());
                    // Use the same current_y for all children of skipped elements
                    // For html/body, this ensures content starts at the top
                    let child_height: f32 = self.layout_node(&styled_child, x, current_y, display_list);
                    if child_height > 0.0 {
                        max_child_height = max_child_height.max(child_height);
                        // Don't accumulate Y for skipped elements - their children should start at the same Y
                    }
                }
                return max_child_height;
            }
        }
        
        // Layout children first to calculate block dimensions
        let mut has_children = false;
        let mut max_child_height: f32 = 0.0;
        
        for (idx, child) in node.node.children().iter().enumerate() {
            let styled_child = crate::css::style::StyledNode::new(child.clone());
            if let crate::dom::NodeType::Element { tag_name, .. } = child.node_type() {
                log::debug!(target: "layout", "Processing child #{}: <{}> at y={}", idx, tag_name, current_y);
            }
            let child_height: f32 = self.layout_node(&styled_child, x + padding, current_y, display_list);
            
            if child_height > 0.0 {
                has_children = true;
                max_child_height = max_child_height.max(child_height);
                current_y += child_height + margin;
            } else {
                // Fallback for elements that don't return height
                let child_computed = self.compute_style(&styled_child);
                match child_computed.display {
                    Display::Block => {
                        has_children = true;
                        let h: f32 = self.layout_block(&styled_child, x + padding, current_y, &child_computed, display_list);
                        current_y += h.max(line_height) + margin;
                        max_child_height = max_child_height.max(h);
                    }
                    Display::Inline => {
                        has_children = true;
                        let h: f32 = self.layout_inline(&styled_child, x + padding, current_y, &child_computed, display_list);
                        current_y += h.max(line_height) + margin;
                        max_child_height = max_child_height.max(h);
                    }
                    Display::None => {}
                }
            }
        }
        
        // Handle text nodes directly in block elements
        // Skip text that looks like JavaScript code (heuristic: contains common JS patterns)
        match node.node.node_type() {
            crate::dom::NodeType::Text(text) => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    // Heuristic: skip text that looks like JavaScript or CSS code
                    // This catches script/style content that might have slipped through
                    let looks_like_js = trimmed.contains("function") || 
                                       trimmed.contains("var ") || 
                                       trimmed.contains("const ") ||
                                       trimmed.contains("let ") ||
                                       trimmed.contains("=>");
                    let looks_like_css = (trimmed.contains("{") && trimmed.contains("}")) ||
                                       (trimmed.contains(":") && (trimmed.contains("px") || trimmed.contains("em") || trimmed.contains("rgb") || trimmed.contains("#"))) ||
                                       trimmed.starts_with("@") ||
                                       (trimmed.contains(";") && trimmed.contains(":") && trimmed.len() > 20);
                    let looks_like_code = looks_like_js || looks_like_css || (trimmed.contains("{") && trimmed.contains("}") && trimmed.len() > 50);
                    
                    if !looks_like_code {
                        let decoded = entities::decode_html_entities(trimmed);
                        if !decoded.trim().is_empty() {
                            // Calculate proper x position: if x < padding, use padding, else use x
                            let text_x = if x < padding {
                                padding
                            } else {
                                x
                            };
                            display_list.add_item(DisplayItem::Text {
                                content: decoded,
                                x: text_x,
                                y: current_y, // Use current_y instead of y for proper positioning
                                color: style.color.clone(),
                            });
                            // Update current_y for text
                            current_y += line_height;
                        }
                    } else {
                        log::debug!(target: "layout", "Skipping text that looks like code (JS/CSS)");
                    }
                }
            }
            crate::dom::NodeType::Element { tag_name, .. } => {
                let tag_lower = tag_name.to_lowercase();
                // Create rectangles for block-level elements
                if matches!(tag_lower.as_str(), "div" | "section" | "article" | "header" | "footer" | "main" | "body" | "html" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "li" | "blockquote" | "nav" | "aside") {
                    let block_height = if has_children && current_y > block_start_y {
                        current_y - block_start_y + margin
                    } else {
                        line_height
                    };
                    // Calculate block width - use full viewport width minus padding on both sides
                    // The x position already includes left padding for nested elements
                    // For root elements (x=0), we need to add left padding
                    // For nested elements, x already includes padding, so we use it as-is
                    let left_padding = 20.0;
                    let right_padding = 20.0;
                    
                    // Determine the actual left edge of this block
                    let block_x = if x < left_padding {
                        // Root element: start at left padding
                        left_padding
                    } else {
                        // Nested element: x already includes padding
                        x
                    };
                    
                    // Calculate available width from block_x to viewport edge
                    let available_width = (self.viewport_width as f32) - block_x - right_padding;
                    let block_width = available_width.max(50.0);
                    
                    // Log for debugging - log ALL block elements to see what's happening
                    log::info!(target: "layout", "Block <{}> x={}, block_x={}, viewport={}, available={}, width={}", 
                        tag_lower, x, block_x, self.viewport_width, available_width, block_width);
                    
                    // Only add rectangle if it has meaningful dimensions and is not the root html/body
                    // Use white background for layout structure (will be filtered in rendering)
                    // Use block_x instead of x for proper positioning
                    if block_width > 0.0 && block_height > 0.0 && !matches!(tag_lower.as_str(), "html" | "body") {
                        display_list.add_item(DisplayItem::Rectangle {
                            x: block_x,
                            y: block_start_y,
                            width: block_width,
                            height: block_height.max(10.0),
                            color: super::Color { r: 255, g: 255, b: 255, a: 255 }, // White background
                        });
                    }
                }
            }
            _ => {}
        }
        
        // Return the height of this block
        let block_height = if has_children && current_y > block_start_y {
            current_y - block_start_y
        } else {
            line_height
        };
        block_height
    }

    fn layout_inline(&mut self, node: &StyledNode, x: f32, y: f32, style: &ComputedStyle, display_list: &mut DisplayList) -> f32 {
        let font_metrics = self.font_manager.get_metrics(&style.font_family, style.font_size);
        let line_height = match style.line_height {
            LineHeight::Normal => font_metrics.line_height,
            LineHeight::Number(n) => style.font_size * n,
            LineHeight::Length(h) => h,
        };
        
        // Handle img tags in inline context
        if let crate::dom::NodeType::Element { tag_name, .. } = node.node.node_type() {
            let tag_lower = tag_name.to_lowercase();
            if tag_lower == "img" {
                let img_url = node.node.get_attribute("src").unwrap_or("").to_string();
                let alt_text = node.node.get_attribute("alt").unwrap_or("").to_string();
                let img_width = node.node.get_attribute("width")
                    .and_then(|w| w.parse::<f32>().ok())
                    .unwrap_or(200.0);
                let img_height = node.node.get_attribute("height")
                    .and_then(|h| h.parse::<f32>().ok())
                    .unwrap_or(200.0);
                
                log::info!(target: "layout", "Found inline img element: src='{}', alt='{}', size={}x{} at ({}, {})", 
                    img_url, alt_text, img_width, img_height, x, y);
                
                display_list.add_item(DisplayItem::Image {
                    url: img_url,
                    x,
                    y,
                    width: img_width,
                    height: img_height,
                    alt: alt_text,
                });
                return img_height;
            }
        }
        
        // Extract text content from text nodes
        match node.node.node_type() {
            crate::dom::NodeType::Text(text) => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    // Skip JavaScript/CSS-like text
                    let looks_like_js = trimmed.contains("function") || 
                                       trimmed.contains("var ") || 
                                       trimmed.contains("const ") ||
                                       trimmed.contains("let ") ||
                                       trimmed.contains("=>");
                    let looks_like_css = (trimmed.contains("{") && trimmed.contains("}")) ||
                                       trimmed.contains(":") && (trimmed.contains("px") || trimmed.contains("em") || trimmed.contains("rgb") || trimmed.contains("#")) ||
                                       trimmed.starts_with("@") ||
                                       (trimmed.contains(";") && trimmed.contains(":") && trimmed.len() > 20);
                    let looks_like_code = looks_like_js || looks_like_css || (trimmed.contains("{") && trimmed.contains("}") && trimmed.len() > 50);
                    if !looks_like_code {
                        let decoded = entities::decode_html_entities(trimmed);
                        if !decoded.trim().is_empty() {
                            display_list.add_item(DisplayItem::Text {
                                content: decoded,
                                x,
                                y,
                                color: Color { r: 0, g: 0, b: 0, a: 255 },
                            });
                        }
                    }
                }
                return line_height;
            }
            crate::dom::NodeType::Element { tag_name, .. } => {
                let tag_lower = tag_name.to_lowercase();
                // Skip non-content elements
                if matches!(tag_lower.as_str(), "script" | "style" | "meta" | "link" | "head" | "title") {
                    return 0.0;
                }
                
                // Create rectangles for inline block elements
                if matches!(tag_lower.as_str(), "span" | "a" | "strong" | "em" | "b" | "i" | "code") {
                    let char_width = 8.0;
                    let mut text_width = 0.0;
                    
                    // Calculate text width first
                    for child in node.node.children() {
                        match child.node_type() {
                            crate::dom::NodeType::Text(text) => {
                                text_width += text.trim().len() as f32 * char_width;
                            }
                            _ => {}
                        }
                    }
                    
                    if text_width > 0.0 {
                        display_list.add_item(DisplayItem::Rectangle {
                            x,
                            y: y - 2.0,
                            width: text_width,
                            height: 20.0,
                            color: super::Color { r: 255, g: 255, b: 255, a: 0 },
                        });
                    }
                }
                
                // Layout children inline with proper spacing
                // Calculate proper starting x position: if x < padding, use padding, else use x
                let padding = 20.0;
                let start_x = if x < padding {
                    padding
                } else {
                    x
                };
                let mut current_x = start_x;
                let char_width = style.font_size * 0.6; // Approximate character width based on font size
                let line_height = match style.line_height {
                    LineHeight::Normal => font_metrics.line_height,
                    LineHeight::Number(n) => style.font_size * n,
                    LineHeight::Length(h) => h,
                };
                let mut max_height: f32 = line_height;
                
                for child in node.node.children() {
                    let styled_child = crate::css::style::StyledNode::new(child.clone());
                    let child_computed = self.compute_style(&styled_child);
                    
                    match child.node_type() {
                        crate::dom::NodeType::Text(text) => {
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                let decoded = entities::decode_html_entities(trimmed);
                                if !decoded.trim().is_empty() {
                                    display_list.add_item(DisplayItem::Text {
                                        content: decoded.clone(),
                                        x: current_x,
                                        y,
                                        color: Color { r: 0, g: 0, b: 0, a: 255 },
                                    });
                                    current_x += decoded.len() as f32 * char_width;
                                }
                            }
                        }
                        _ => {
                            let child_height: f32 = self.layout_inline(&styled_child, current_x, y, &child_computed, display_list);
                            max_height = max_height.max(child_height);
                            // Estimate width for inline elements
                            current_x += 50.0; // Space for inline elements
                        }
                    }
                }
                
                return max_height;
            }
            _ => {}
        }
        
        // Return the height of inline content (typically line height)
        24.0
    }
} 