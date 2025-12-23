use crate::rendering::{DisplayList, DisplayItem, Color};
use gpui::{div, prelude::*, Entity, IntoElement, Context, px};

pub struct ContentView {
    loaded: bool,
    display_list: Option<DisplayList>,
    loading_text: String,
    page_content: String,
}

impl ContentView {
    pub fn new(cx: &mut Context<super::window::BrowserWindow>) -> Entity<Self> {
        cx.new(|_cx| Self {
            loaded: false,
            display_list: None,
            loading_text: String::from("Loading page..."),
            page_content: String::new(),
        })
    }

    pub fn set_loaded(&mut self, loaded: bool) {
        self.loaded = loaded;
        if loaded {
            self.loading_text = String::from("Page loaded");
        }
    }
    
    pub fn set_loading(&mut self, text: &str) {
        self.loading_text = text.to_string();
        self.loaded = false;
        self.page_content.clear();
        self.display_list = None;
    }

    pub fn set_display_list(&mut self, display_list: DisplayList) {
        self.display_list = Some(display_list);
        self.loaded = true;
    }
    
    pub fn set_page_content(&mut self, content: &str) {
        self.page_content = content.to_string();
        self.loaded = true;
    }
    
    fn color_to_rgb(color: &Color) -> u32 {
        ((color.r as u32) << 16) | ((color.g as u32) << 8) | (color.b as u32)
    }
}

impl gpui::Render for ContentView {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if !self.loaded {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .bg(gpui::rgb(0xffffff))
                .child(self.loading_text.clone());
        }
        
        // Render display list items if available
        if let Some(ref display_list) = self.display_list {
            let items = display_list.items();
            if !items.is_empty() {
                let mut container = div()
                    .size_full()
                    .relative()
                    .bg(gpui::rgb(0xffffff));
                
                // Render rectangles first (background)
                for item in items {
                    if let DisplayItem::Rectangle { x, y, width, height, color } = item {
                        if *width > 0.0 && *height > 0.0 {
                            container = container.child(
                                div()
                                    .absolute()
                                    .left(px(*x))
                                    .top(px(*y))
                                    .w(px(*width))
                                    .h(px(*height))
                                    .bg(gpui::rgb(Self::color_to_rgb(color)))
                                    .border(px(0.5))
                                    .border_color(gpui::rgb(0xe0e0e0))
                            );
                        }
                    }
                }
                
                // Then render text on top
                for item in items {
                    if let DisplayItem::Text { content, x, y, color } = item {
                        container = container.child(
                            div()
                                .absolute()
                                .left(px(*x))
                                .top(px(*y))
                                .text_color(gpui::rgb(Self::color_to_rgb(color)))
                                .child(content.clone())
                        );
                    }
                }
                
                return container;
            }
        }
        
        // Fallback: render text content
        div()
            .size_full()
            .p_4()
            .bg(gpui::rgb(0xffffff))
            .text_color(gpui::rgb(0x000000))
            .text_sm()
            .child(if self.page_content.is_empty() {
                self.loading_text.clone()
            } else {
                self.page_content.clone()
            })
    }
}
