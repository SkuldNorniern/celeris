use crate::css::style::StyledNode;
use std::error::Error;

#[cfg(feature = "gui")]
pub mod gui;

pub mod layout;
pub mod painter;
pub mod tree;

pub use tree::{RenderTree, RenderNode, Bounds};

pub struct Renderer {
    headless: bool,
    layout_engine: layout::LayoutEngine,
    painter: Painter,
}

struct Painter {
    headless: bool,
    // Add rendering backend specific fields
}

#[derive(Debug, Clone)]
pub struct DisplayList {
    items: Vec<DisplayItem>,
}

#[derive(Debug, Clone)]
pub enum DisplayItem {
    Text {
        content: String,
        x: f32,
        y: f32,
        color: Color,
    },
    Rectangle {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    },
    Image {
        url: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        alt: String,
    },
    Button {
        text: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
}

#[derive(Debug, Clone)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Renderer {
    pub fn new(headless: bool) -> Result<Self, Box<dyn Error>> {
        // Default viewport size - needed for layout calculations even in headless mode
        // In headless mode, this is a reasonable default for text extraction and layout
        // The viewport can be changed via set_viewport_size() if needed
        const DEFAULT_VIEWPORT_WIDTH: u32 = 800;
        const DEFAULT_VIEWPORT_HEIGHT: u32 = 600;
        
        Ok(Self {
            headless,
            layout_engine: layout::LayoutEngine::new(DEFAULT_VIEWPORT_WIDTH, DEFAULT_VIEWPORT_HEIGHT),
            painter: Painter::new(headless)?,
        })
    }
    
    pub fn set_viewport_size(&mut self, width: u32, height: u32) {
        self.layout_engine.set_viewport_size(width, height);
    }

    pub fn layout(&mut self, styled_node: &StyledNode) -> DisplayList {
        self.layout_engine.compute_layout(styled_node)
    }
    
    /// Build a RenderTree from a StyledNode (alternative to direct DisplayList)
    pub fn build_render_tree(&mut self, styled_node: &StyledNode) -> tree::RenderTree {
        tree::RenderTree::build_from_styled_node(styled_node, 0.0, 0.0, &mut self.layout_engine)
    }

    pub fn paint(&mut self, display_list: &DisplayList) -> Result<(), Box<dyn Error>> {
        self.painter.paint(display_list)
    }
}

impl Painter {
    fn new(headless: bool) -> Result<Self, Box<dyn Error>> {
        Ok(Self { headless })
    }

    fn paint(&mut self, display_list: &DisplayList) -> Result<(), Box<dyn Error>> {
        // Implement painting logic based on headless mode
        Ok(())
    }
}

impl DisplayList {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn items(&self) -> &[DisplayItem] {
        &self.items
    }
    
    pub fn add_item(&mut self, item: DisplayItem) {
        self.items.push(item);
    }
}
