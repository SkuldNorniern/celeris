use super::{DisplayList, DisplayItem};
use crate::css::style::StyledNode;
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
        let mut display_list = DisplayList::new();
        self.layout_node(styled_node, 0.0, 0.0, &mut display_list);
        display_list
    }

    fn layout_node(&mut self, node: &StyledNode, x: f32, y: f32, display_list: &mut DisplayList) {
        // Basic layout algorithm - expand as needed
        let computed = self.compute_style(node);
        
        match computed.display {
            Display::Block => {
                // Handle block layout
                self.layout_block(node, x, y, &computed, display_list);
            },
            Display::Inline => {
                // Handle inline layout
                self.layout_inline(node, x, y, &computed, display_list);
            },
            Display::None => {},
        }
    }

    fn compute_style(&self, node: &StyledNode) -> ComputedStyle {
        // Basic style computation - expand as needed
        ComputedStyle {
            display: Display::Block,
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

    fn layout_block(&mut self, node: &StyledNode, x: f32, y: f32, _style: &ComputedStyle, display_list: &mut DisplayList) {
        let mut current_y = y;
        let padding = 10.0;
        let line_height = 24.0;
        let margin = 8.0;
        let block_start_y = current_y;
        
        // Skip non-content elements
        if let crate::dom::NodeType::Element { tag_name, .. } = node.node.node_type() {
            if matches!(tag_name.as_str(), "script" | "style" | "meta" | "link" | "head" | "title") {
                return;
            }
        }
        
        // Layout children first to calculate block dimensions
        let mut has_children = false;
        for child in node.node.children() {
            let styled_child = crate::css::style::StyledNode::new(child.clone());
            let child_computed = self.compute_style(&styled_child);
            
            match child_computed.display {
                Display::Block => {
                    has_children = true;
                    self.layout_block(&styled_child, x + padding, current_y + margin, &child_computed, display_list);
                    current_y += line_height + margin;
                }
                Display::Inline => {
                    has_children = true;
                    let text_y = current_y;
                    self.layout_inline(&styled_child, x + padding, text_y, &child_computed, display_list);
                    current_y += line_height;
                }
                Display::None => {}
            }
        }
        
        // Handle text nodes directly in block elements
        match node.node.node_type() {
            crate::dom::NodeType::Text(text) => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    display_list.add_item(DisplayItem::Text {
                        content: trimmed.to_string(),
                        x: x + padding,
                        y,
                        color: super::Color { r: 0, g: 0, b: 0, a: 255 },
                    });
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
    }

    fn layout_inline(&mut self, node: &StyledNode, x: f32, y: f32, _style: &ComputedStyle, display_list: &mut DisplayList) {
        // Extract text content from text nodes
        match node.node.node_type() {
            crate::dom::NodeType::Text(text) => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    display_list.add_item(DisplayItem::Text {
                        content: trimmed.to_string(),
                        x,
                        y,
                        color: super::Color { r: 0, g: 0, b: 0, a: 255 },
                    });
                }
            }
            crate::dom::NodeType::Element { tag_name, .. } => {
                // Skip non-content elements
                if matches!(tag_name.as_str(), "script" | "style" | "meta" | "link" | "head" | "title") {
                    return;
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
                
                for child in node.node.children() {
                    let styled_child = crate::css::style::StyledNode::new(child.clone());
                    let child_computed = self.compute_style(&styled_child);
                    
                    match child.node_type() {
                        crate::dom::NodeType::Text(text) => {
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                display_list.add_item(DisplayItem::Text {
                                    content: trimmed.to_string(),
                                    x: current_x,
                                    y,
                                    color: super::Color { r: 0, g: 0, b: 0, a: 255 },
                                });
                                current_x += trimmed.len() as f32 * char_width;
                            }
                        }
                        _ => {
                            self.layout_inline(&styled_child, current_x, y, &child_computed, display_list);
                            current_x += 10.0;
                        }
                    }
                }
            }
            _ => {}
        }
    }
} 