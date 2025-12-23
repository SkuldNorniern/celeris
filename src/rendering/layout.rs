use super::{DisplayList, DisplayItem};
use crate::css::style::StyledNode;
use crate::html::entities;
use std::collections::HashMap;

pub struct LayoutEngine {
    viewport_width: u32,
    viewport_height: u32,
    computed_styles: HashMap<String, ComputedStyle>,
}

pub struct ComputedStyle {
    pub display: Display,
    pub position: Position,
    pub width: Dimension,
    pub height: Dimension,
    pub margin: Box<Edges>,
    pub padding: Box<Edges>,
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
        }
    }

    pub fn compute_layout(&mut self, styled_node: &StyledNode) -> DisplayList {
        log::info!(target: "layout", "Starting layout computation");
        let mut display_list = DisplayList::new();
        let _height = self.layout_node(styled_node, 0.0, 0.0, &mut display_list);
        log::info!(target: "layout", "Layout complete, created {} display items", display_list.items().len());
        display_list
    }

    fn layout_node(&mut self, node: &StyledNode, x: f32, y: f32, display_list: &mut DisplayList) -> f32 {
        // Handle special elements first (img, button, input) regardless of display type
        if let crate::dom::NodeType::Element { tag_name, .. } = node.node.node_type() {
            match tag_name.as_str() {
                "img" => {
                    let img_url = node.node.get_attribute("src").unwrap_or("").to_string();
                    let alt_text = node.node.get_attribute("alt").unwrap_or("").to_string();
                    let img_width = node.node.get_attribute("width")
                        .and_then(|w| w.parse::<f32>().ok())
                        .unwrap_or(100.0);
                    let img_height = node.node.get_attribute("height")
                        .and_then(|h| h.parse::<f32>().ok())
                        .unwrap_or(100.0);
                    
                    log::debug!(target: "layout", "Found img element: src={}, alt={}, size={}x{}", img_url, alt_text, img_width, img_height);
                    
                    display_list.add_item(DisplayItem::Image {
                        url: img_url,
                        x,
                        y,
                        width: img_width,
                        height: img_height,
                        alt: alt_text,
                    });
                    return img_height; // Return height for parent to track
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
                    
                    log::debug!(target: "layout", "Found {} element: text={}", tag_name, button_text);
                    
                    let button_width = 120.0;
                    let button_height = 32.0;
                    
                    display_list.add_item(DisplayItem::Button {
                        text: button_text,
                        x,
                        y,
                        width: button_width,
                        height: button_height,
                    });
                    return button_height; // Return height for parent to track
                }
                _ => {}
            }
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

    fn compute_style(&self, node: &StyledNode) -> ComputedStyle {
        // Determine display type based on element
        let display = if let crate::dom::NodeType::Element { tag_name, .. } = node.node.node_type() {
            match tag_name.as_str() {
                "div" | "section" | "article" | "header" | "footer" | "main" | "body" | "html" | 
                "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "li" | 
                "blockquote" | "nav" | "aside" | "form" | "table" | "tr" | "td" | "th" => Display::Block,
                "span" | "a" | "strong" | "em" | "b" | "i" | "u" | "code" | "small" | "sub" | "sup" => Display::Inline,
                "img" | "button" | "input" => Display::Inline, // These are handled specially but default to inline
                _ => Display::Block,
            }
        } else {
            Display::Block
        };
        
        ComputedStyle {
            display,
            position: Position::Static,
            width: Dimension::Auto,
            height: Dimension::Auto,
            margin: Box::new(Edges {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            }),
            padding: Box::new(Edges {
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            }),
        }
    }

    fn layout_block(&mut self, node: &StyledNode, x: f32, y: f32, _style: &ComputedStyle, display_list: &mut DisplayList) -> f32 {
        let mut current_y = y;
        let padding = 10.0;
        let line_height = 24.0;
        let margin = 8.0;
        let block_start_y = current_y;
        
        // Skip non-content elements
        if let crate::dom::NodeType::Element { tag_name, .. } = node.node.node_type() {
            if matches!(tag_name.as_str(), "script" | "style" | "meta" | "link" | "head" | "title") {
                return 0.0;
            }
        }
        
        // Layout children first to calculate block dimensions
        let mut has_children = false;
        let mut max_child_height: f32 = 0.0;
        
        for child in node.node.children() {
            let styled_child = crate::css::style::StyledNode::new(child.clone());
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
        match node.node.node_type() {
            crate::dom::NodeType::Text(text) => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    let decoded = entities::decode_html_entities(trimmed);
                    if !decoded.trim().is_empty() {
                        display_list.add_item(DisplayItem::Text {
                            content: decoded,
                            x: x + padding,
                            y: current_y, // Use current_y instead of y for proper positioning
                            color: super::Color { r: 0, g: 0, b: 0, a: 255 },
                        });
                        // Update current_y for text
                        current_y += line_height;
                    }
                }
            }
            crate::dom::NodeType::Element { tag_name, .. } => {
                // Create rectangles for block-level elements
                if matches!(tag_name.as_str(), "div" | "section" | "article" | "header" | "footer" | "main" | "body" | "html" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "li" | "blockquote" | "nav" | "aside") {
                    let block_height = if has_children && current_y > block_start_y {
                        current_y - block_start_y + margin
                    } else {
                        line_height
                    };
                    let block_width = (self.viewport_width as f32) - (x * 2.0);
                    
                    // Only add rectangle if it has meaningful dimensions
                    if block_width > 0.0 && block_height > 0.0 {
                        // Add rectangle for block element with subtle border color
                        // Use a light gray background for visibility
                        display_list.add_item(DisplayItem::Rectangle {
                            x,
                            y: block_start_y,
                            width: block_width.max(50.0),
                            height: block_height.max(10.0),
                            color: super::Color { r: 245, g: 245, b: 245, a: 255 },
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

    fn layout_inline(&mut self, node: &StyledNode, x: f32, y: f32, _style: &ComputedStyle, display_list: &mut DisplayList) -> f32 {
        let line_height = 24.0;
        
        // Extract text content from text nodes
        match node.node.node_type() {
            crate::dom::NodeType::Text(text) => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    let decoded = entities::decode_html_entities(trimmed);
                    if !decoded.trim().is_empty() {
                        display_list.add_item(DisplayItem::Text {
                            content: decoded,
                            x,
                            y,
                            color: super::Color { r: 0, g: 0, b: 0, a: 255 },
                        });
                    }
                }
                return line_height;
            }
            crate::dom::NodeType::Element { tag_name, .. } => {
                // Skip non-content elements
                if matches!(tag_name.as_str(), "script" | "style" | "meta" | "link" | "head" | "title") {
                    return 0.0;
                }
                
                // Create rectangles for inline block elements
                if matches!(tag_name.as_str(), "span" | "a" | "strong" | "em" | "b" | "i" | "code") {
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
                let mut current_x = x;
                let char_width = 8.0;
                let line_height: f32 = 24.0;
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
                                        color: super::Color { r: 0, g: 0, b: 0, a: 255 },
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