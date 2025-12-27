use crate::css::style::StyledNode;
use super::DisplayList;

pub struct RenderTree {
    root: RenderNode,
}

pub struct RenderNode {
    node: StyledNode,
    children: Vec<RenderNode>,
    bounds: Bounds,
}

#[derive(Clone, Copy)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl RenderTree {
    pub fn new(styled_node: StyledNode) -> Self {
        Self {
            root: RenderNode::new(styled_node),
        }
    }
    
    /// Build a RenderTree recursively from a StyledNode
    pub fn build_from_styled_node(styled_node: &StyledNode, x: f32, y: f32, layout_engine: &mut crate::rendering::layout::LayoutEngine) -> Self {
        log::debug!(target: "tree", "Building RenderTree with viewport {}x{}", layout_engine.viewport_width(), layout_engine.viewport_height());
        let mut root = RenderNode::new(styled_node.clone());
        Self::build_render_node_recursive(&mut root, styled_node, x, y, layout_engine);
        Self { root }
    }
    
    fn build_render_node_recursive(
        render_node: &mut RenderNode,
        styled_node: &StyledNode,
        x: f32,
        y: f32,
        layout_engine: &mut crate::rendering::layout::LayoutEngine,
    ) {
        // Calculate bounds using layout engine
        let computed = layout_engine.compute_style(styled_node);
        // Real browsers: Start from y + top margin
        let mut current_y = y + computed.margin.top;
        let left_padding = if x < 20.0 { 20.0 } else { computed.padding.left };
        let right_padding = 20.0; // Viewport right padding
        // Use default line height (can be improved later with font metrics access)
        let line_height = match computed.line_height {
            crate::rendering::layout::LineHeight::Normal => 24.0, // Default line height
            crate::rendering::layout::LineHeight::Number(n) => computed.font_size * n,
            crate::rendering::layout::LineHeight::Length(h) => h,
        };

        // Get viewport dimensions from layout engine
        let viewport_width = layout_engine.viewport_width() as f32;
        let viewport_height = layout_engine.viewport_height() as f32;

        // Calculate width and height based on children
        let mut max_width: f32 = 0.0;
        let mut max_height = 0.0;

        // Real browsers: Calculate block_x including margin
        let block_x = if x < 20.0 {
            // Root element: start at left padding + left margin
            left_padding + computed.margin.left
        } else {
            // Nested element: x already includes parent padding, add margin
            x + computed.margin.left
        };

        // Calculate available width (same as layout_block)
        let available_width = viewport_width - block_x - right_padding;

        // Check if this is a skipped element (html, body, head, etc.) - don't accumulate Y for skipped elements
        let is_skipped = if let crate::dom::NodeType::Element { tag_name, .. } = styled_node.node.node_type() {
            let tag_lower = tag_name.to_lowercase();
            matches!(tag_lower.as_str(), "script" | "style" | "meta" | "link" | "head" | "title" | "#document" | "html" | "body")
        } else {
            false
        };

        // Log first few elements for debugging
        if let crate::dom::NodeType::Element { tag_name, .. } = styled_node.node.node_type() {
            let tag_lower = tag_name.to_lowercase();
            if tag_lower == "div" || tag_lower == "body" || tag_lower == "html" {
                log::debug!(target: "tree", "Building <{}> at y={}, is_skipped={}", tag_lower, y, is_skipped);
            }
        }

        for child in styled_node.node.children() {
            let styled_child = crate::css::style::StyledNode::new(child.clone());
            let mut child_render_node = RenderNode::new(styled_child.clone());

            // For skipped elements, use the same current_y for all children (don't accumulate)
            // For normal elements, use current_y which will be accumulated after
            let child_y = if is_skipped {
                current_y  // Same Y for all children of skipped elements
            } else {
                current_y  // Use current accumulated Y
            };

            // Recursively build child - pass block_x as the new x position
            Self::build_render_node_recursive(&mut child_render_node, &styled_child, block_x, child_y, layout_engine);

            // Get child bounds after recursive build
            let child_bounds = child_render_node.bounds().clone();
            // Use the child's actual height, or a minimum height for layout purposes
            let child_height = if child_bounds.height > 0.0 {
                child_bounds.height
            } else {
                // For elements with no explicit height, use line height as minimum
                line_height
            };

            // Child bounds are already set correctly by recursive call, don't override
            max_width = max_width.max(child_bounds.width);
            
            // Only accumulate Y and height for non-skipped elements
            if !is_skipped {
                // Real browsers: Add child height + bottom margin for next element
                let child_computed = layout_engine.compute_style(&styled_child);
                let child_bottom_margin = child_computed.margin.bottom;
                let child_total_height = child_height + child_bottom_margin;
                current_y += child_total_height;
                max_height += child_total_height;
            } else {
                // For skipped elements, just track max height but don't accumulate Y
                // All children of skipped elements start at the same Y position (current_y)
                max_height = max_height.max(child_height);
            }

            render_node.add_child(child_render_node);
        }

        // Real browsers: Calculate node width including margins
        let node_width = if x < 20.0 {
            // Root element: use full available width minus margins
            (available_width - computed.margin.left - computed.margin.right).max(100.0f32)
        } else {
            // Nested element: use calculated width
            max_width.max(available_width.min(100.0f32))
        };

        // Real browsers: Height includes content + padding + margins
        let content_height = max_height.max(line_height);
        let total_height = content_height + computed.padding.top + computed.padding.bottom + computed.margin.bottom;

        let bounds = Bounds {
            x: block_x,
            y: current_y - computed.margin.top, // Adjust y to account for top margin
            width: node_width,
            height: total_height,
        };
        render_node.set_bounds(bounds);

        // Debug logging
        if let crate::dom::NodeType::Element { tag_name, .. } = styled_node.node.node_type() {
            let tag_lower = tag_name.to_lowercase();
            if tag_lower == "div" || tag_lower == "body" || tag_lower == "html" {
                log::debug!(target: "tree", "Tree node <{}> at x={}, width={}, viewport={}",
                    tag_lower, bounds.x, bounds.width, viewport_width);
            }
        }
    }
    
    pub fn root(&self) -> &RenderNode {
        &self.root
    }
    
    pub fn build_display_list(&self) -> DisplayList {
        let mut display_list = DisplayList::new();
        self.root.build_display_list(&mut display_list);
        display_list
    }
}

impl RenderNode {
    pub fn new(node: StyledNode) -> Self {
        Self {
            node,
            children: Vec::new(),
            bounds: Bounds {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
        }
    }
    
    pub fn add_child(&mut self, child: RenderNode) {
        self.children.push(child);
    }
    
    pub fn node(&self) -> &StyledNode {
        &self.node
    }
    
    pub fn children(&self) -> &[RenderNode] {
        &self.children
    }
    
    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }
    
    pub fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
    }
    
    fn build_display_list(&self, display_list: &mut DisplayList) {
        match self.node.node.node_type() {
            crate::dom::NodeType::Text(text) => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    // Skip JavaScript-like text
                    let looks_like_js = trimmed.contains("function") || 
                                       trimmed.contains("var ") || 
                                       trimmed.contains("const ") ||
                                       trimmed.contains("let ") ||
                                       trimmed.contains("=>");
                    if !looks_like_js {
                        use crate::html::entities;
                        let decoded = entities::decode_html_entities(trimmed);
                        if !decoded.trim().is_empty() {
                            display_list.add_item(super::DisplayItem::Text {
                                content: decoded,
                                x: self.bounds.x,
                                y: self.bounds.y,
                                color: super::Color { r: 0, g: 0, b: 0, a: 255 },
                            });
                        }
                    }
                }
            }
            crate::dom::NodeType::Element { tag_name, .. } => {
                let tag_lower = tag_name.to_lowercase();
                
                // Handle special elements
                match tag_lower.as_str() {
                    "img" => {
                        let img_url = self.node.node.get_attribute("src").unwrap_or("").to_string();
                        let alt_text = self.node.node.get_attribute("alt").unwrap_or("").to_string();
                        let img_width = self.node.node.get_attribute("width")
                            .and_then(|w| w.parse::<f32>().ok())
                            .unwrap_or(200.0);
                        let img_height = self.node.node.get_attribute("height")
                            .and_then(|h| h.parse::<f32>().ok())
                            .unwrap_or(200.0);
                        
                        display_list.add_item(super::DisplayItem::Image {
                            url: img_url,
                            x: self.bounds.x,
                            y: self.bounds.y,
                            width: img_width,
                            height: img_height,
                            alt: alt_text,
                        });
                    }
                    "button" | "input" => {
                        let button_text = if tag_lower == "button" {
                            let mut text = String::new();
                            for child in self.node.node.children() {
                                if let crate::dom::NodeType::Text(t) = child.node_type() {
                                    text.push_str(t.trim());
                                }
                            }
                            if text.is_empty() {
                                self.node.node.get_attribute("value").unwrap_or("Button").to_string()
                            } else {
                                text
                            }
                        } else {
                            self.node.node.get_attribute("value")
                                .or_else(|| self.node.node.get_attribute("placeholder"))
                                .unwrap_or("Input")
                                .to_string()
                        };
                        
                        let button_width = 120.0;
                        let button_height = 32.0;
                        
                        display_list.add_item(super::DisplayItem::Button {
                            text: button_text,
                            x: self.bounds.x,
                            y: self.bounds.y,
                            width: button_width,
                            height: button_height,
                        });
                    }
                    _ => {
                        // For block elements, add rectangle if needed
                        if matches!(tag_lower.as_str(), "div" | "section" | "article" | "header" | "footer" | "main") 
                            && self.bounds.width > 0.0 && self.bounds.height > 0.0 {
                            // Only add non-white rectangles for debugging
                            display_list.add_item(super::DisplayItem::Rectangle {
                                x: self.bounds.x,
                                y: self.bounds.y,
                                width: self.bounds.width,
                                height: self.bounds.height,
                                color: super::Color { r: 255, g: 255, b: 255, a: 255 },
                            });
                        }
                    }
                }
                
                // Process children
                for child in &self.children {
                    child.build_display_list(display_list);
                }
            }
            _ => {
                for child in &self.children {
                    child.build_display_list(display_list);
                }
            }
        }
    }
}

