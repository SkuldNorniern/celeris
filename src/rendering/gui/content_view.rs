use crate::rendering::{DisplayList, DisplayItem, Color};
use gpui::{div, img, prelude::*, Entity, IntoElement, Context, px};

pub struct ContentView {
    loaded: bool,
    display_list: Option<DisplayList>,
    loading_text: String,
    page_content: String,
    loading_progress: f32,
    layout_viewport_width: f32,  // Viewport width used for layout
    layout_viewport_height: f32,  // Viewport height used for layout
}

impl ContentView {
    pub fn new(cx: &mut Context<super::window::BrowserWindow>) -> Entity<Self> {
        cx.new(|_cx| Self {
            loaded: false,
            display_list: None,
            loading_text: String::from("Loading page..."),
            page_content: String::new(),
            loading_progress: 0.0,
            layout_viewport_width: 1200.0,  // Default layout viewport
            layout_viewport_height: 800.0,
        })
    }

    pub fn set_loaded(&mut self, loaded: bool) {
        self.loaded = loaded;
        if loaded {
            self.loading_text = String::from("Page loaded");
            self.loading_progress = 1.0;
        }
    }
    
    pub fn set_loading(&mut self, text: &str) {
        self.loading_text = text.to_string();
        self.loaded = false;
        self.loading_progress = 0.0;
        self.page_content.clear();
        self.display_list = None;
    }

    pub fn set_loading_progress(&mut self, progress: f32) {
        self.loading_progress = progress.clamp(0.0, 1.0);
    }

    pub fn set_display_list(&mut self, display_list: DisplayList) {
        log::info!(target: "content_view", "Setting display list with {} items", display_list.items().len());
        log::debug!(target: "content_view", "Processing display list for rendering");
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
        log::debug!(target: "content_view", "Display list processed: {} total items", display_list.items().len());
        self.display_list = Some(display_list);
        self.loaded = true;
    }
    
    pub fn set_layout_viewport_size(&mut self, width: f32, height: f32) {
        self.layout_viewport_width = width;
        self.layout_viewport_height = height;
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
    fn render(&mut self, window: &mut gpui::Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Get current viewport size for responsive scaling
        // Since we can't easily convert Pixels to f32, we'll use a simpler approach:
        // Use the layout viewport size as-is and let GPUI handle the scaling through
        // the container's size_full() which will make it fill the available space.
        // For now, use scale 1.0 and rely on container sizing.
        // TODO: Implement proper viewport size tracking when GPUI API allows
        let scale = 1.0;
        if !self.loaded {
            log::debug!(target: "content_view", "Rendering loading state: {}", self.loading_text);
            return div()
                .size_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .bg(gpui::rgb(0xffffff))
                .gap_4()
                .child(
                    div()
                        .text_color(gpui::rgb(0x666666))
                        .text_sm()
                        .child(self.loading_text.clone())
                )
                .child(
                    div()
                        .w_64()
                        .h_1()
                        .bg(gpui::rgb(0xe0e0e0))
                        .rounded_full()
                        .relative()
                        .overflow_hidden()
                        .shadow_sm()
                        .child(
                            div()
                                .absolute()
                                .left(px(0.0))
                                .top(px(0.0))
                                .h_full()
                                .w(px(self.loading_progress * 256.0))
                                .bg(gpui::rgb(0x4285f4))
                                .rounded_full()
                        )
                );
        }
        
        // Render display list items if available
        if let Some(ref display_list) = self.display_list {
            let items = display_list.items();
            log::debug!(target: "content_view", "Rendering display list with {} items", items.len());
            if !items.is_empty() {
                // Create container that adapts to viewport size
                let mut container = div()
                    .w_full()
                    .h_full()
                    .relative()
                    .bg(gpui::rgb(0xffffff))
                    // Test box to verify absolute positioning works
                    .child(
                        div()
                            .absolute()
                            .left(px(10.0))
                            .top(px(10.0))
                            .w(px(100.0))
                            .h(px(100.0))
                            .bg(gpui::rgb(0xff00ff)) // Magenta test box
                            .border(px(5.0))
                            .border_color(gpui::rgb(0x000000))
                            .child("TEST")
                    );
                
                // Render rectangles first (background) - but skip white rectangles to avoid visual clutter
                for item in items {
                    if let DisplayItem::Rectangle { x, y, width, height, color } = item {
                        if *width > 0.0 && *height > 0.0 {
                            let rgb = Self::color_to_rgb(color);
                            if rgb != 0xffffff {
                                container = container.child(
                                    div()
                                        .absolute()
                                        .left(px(*x * scale))
                                        .top(px(*y * scale))
                                        .w(px(*width * scale))
                                        .h(px(*height * scale))
                                        .bg(gpui::rgb(rgb))
                                        .border(px(0.5))
                                        .border_color(gpui::rgb(0xe0e0e0))
                                );
                            }
                        }
                    }
                }
                
                // Render images (as placeholder boxes with alt text)
                let mut image_count = 0;
                for item in items {
                    if let DisplayItem::Image { url, x, y, width, height, alt } = item {
                        image_count += 1;
                        log::info!(target: "content_view", "Rendering image #{}: '{}' at x={}, y={}, size {}x{}", 
                            image_count, alt, x, y, width, height);
                        
                        // Render actual image using GPUI's img() function
                        // If URL is relative, we might need to resolve it, but for now use as-is
                        let image_url = if url.is_empty() {
                            // Fallback: show placeholder if no URL
                            continue;
                        } else {
                            url.as_str()
                        };
                        
                        let scaled_width = (*width * scale).max(1.0);
                        let scaled_height = (*height * scale).max(1.0);
                        
                        container = container.child(
                            div()
                                .absolute()
                                .left(px(*x * scale))
                                .top(px(*y * scale))
                                .w(px(scaled_width))
                                .h(px(scaled_height))
                                .child(
                                    img(image_url)
                                        .w(px(scaled_width))
                                        .h(px(scaled_height))
                                )
                        );
                    }
                }
                
                // Render buttons
                let mut button_count = 0;
                for item in items {
                    if let DisplayItem::Button { text, x, y, width, height } = item {
                        button_count += 1;
                        log::info!(target: "content_view", "Rendering button #{}: '{}' at x={}, y={}, size {}x{}", 
                            button_count, text, x, y, width, height);
                        
                        // Scale button position and size
                        container = container.child(
                            div()
                                .absolute()
                                .left(px(*x * scale))
                                .top(px(*y * scale))
                                .w(px(*width * scale))
                                .h(px(*height * scale))
                                .bg(gpui::rgb(0x00ff00)) // Bright green
                                .border(px(3.0 * scale))
                                .border_color(gpui::rgb(0x0000ff)) // Blue border
                                .text_color(gpui::rgb(0x000000))
                                .text_sm()
                                .p_2()
                                .cursor_pointer()
                                .hover(|style| {
                                    style.bg(gpui::rgb(0x00cc00))
                                })
                                .child(text.clone())
                        );
                    }
                }
                
                // Then render text on top (scaled)
                let mut text_count = 0;
                for item in items {
                    if let DisplayItem::Text { content, x, y, color } = item {
                        text_count += 1;
                        container = container.child(
                            div()
                                .absolute()
                                .left(px(*x * scale))
                                .top(px(*y * scale))
                                .text_color(gpui::rgb(Self::color_to_rgb(color)))
                                .child(content.clone())
                        );
                    }
                }
                log::info!(target: "content_view", "Rendered {} children: {} images, {} buttons, {} text", 
                    image_count + button_count + text_count, image_count, button_count, text_count);
                
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
