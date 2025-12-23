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
                    display_list.add_item(super::DisplayItem::Text {
                        content: trimmed.to_string(),
                        x: self.bounds.x,
                        y: self.bounds.y,
                        color: super::Color { r: 0, g: 0, b: 0, a: 255 },
                    });
                }
            }
            crate::dom::NodeType::Element { tag_name, .. } => {
                if matches!(tag_name.as_str(), "div" | "section" | "article" | "header" | "footer" | "main") {
                    display_list.add_item(super::DisplayItem::Rectangle {
                        x: self.bounds.x,
                        y: self.bounds.y,
                        width: self.bounds.width,
                        height: self.bounds.height,
                        color: super::Color { r: 255, g: 255, b: 255, a: 255 },
                    });
                }
                
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

