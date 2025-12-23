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
        log::info!(target: "content_view", "Setting display list with {} items", display_list.items().len());
        let item_counts = display_list.items().iter().fold((0, 0, 0, 0), |(text, rect, img, btn), item| {
            match item {
                DisplayItem::Text { .. } => (text + 1, rect, img, btn),
                DisplayItem::Rectangle { .. } => (text, rect + 1, img, btn),
                DisplayItem::Image { .. } => (text, rect, img + 1, btn),
                DisplayItem::Button { .. } => (text, rect, img, btn + 1),
            }
        });
        log::info!(target: "content_view", "Display list breakdown: {} text, {} rects, {} images, {} buttons", 
            item_counts.0, item_counts.1, item_counts.2, item_counts.3);
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
            log::debug!(target: "content_view", "Rendering loading state: {}", self.loading_text);
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
            log::debug!(target: "content_view", "Rendering display list with {} items", items.len());
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
                
                // Render images (as placeholder boxes with alt text)
                for item in items {
                    if let DisplayItem::Image { url: _, x, y, width, height, alt } = item {
                        container = container.child(
                            div()
                                .absolute()
                                .left(px(*x))
                                .top(px(*y))
                                .w(px(*width))
                                .h(px(*height))
                                .bg(gpui::rgb(0xf0f0f0))
                                .border(px(1.0))
                                .border_color(gpui::rgb(0xcccccc))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(gpui::rgb(0x666666))
                                .text_xs()
                                .child(if alt.is_empty() {
                                    "[Image]".to_string()
                                } else {
                                    format!("[{}]", alt)
                                })
                        );
                    }
                }
                
                // Render buttons
                for item in items {
                    if let DisplayItem::Button { text, x, y, width, height } = item {
                        container = container.child(
                            div()
                                .absolute()
                                .left(px(*x))
                                .top(px(*y))
                                .w(px(*width))
                                .h(px(*height))
                                .bg(gpui::rgb(0x4285f4))
                                .border(px(1.0))
                                .border_color(gpui::rgb(0x357ae8))
                                .rounded_md()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(gpui::rgb(0xffffff))
                                .text_sm()
                                .cursor_pointer()
                                .hover(|style| {
                                    style.bg(gpui::rgb(0x357ae8))
                                })
                                .child(text.clone())
                        );
                    }
                }
                
                // Then render text on top
                let mut text_count = 0;
                for item in items {
                    if let DisplayItem::Text { content, x, y, color } = item {
                        text_count += 1;
                        if text_count <= 5 {
                            log::debug!(target: "content_view", "Rendering text #{}: '{}' at ({}, {})", text_count, 
                                content.chars().take(30).collect::<String>(), x, y);
                        }
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
                log::debug!(target: "content_view", "Rendered {} text items total", text_count);
                
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
