use crate::rendering::{DisplayList, DisplayItem, Color};
use gpui::{div, prelude::*, Entity, IntoElement, Context, px};
use std::collections::HashMap;
use super::skia::SkiaImage;

pub struct ContentView {
    loaded: bool,
    display_list: Option<DisplayList>,
    loading_text: String,
    page_content: String,
    loading_progress: f32,
    layout_viewport_width: f32,  // Viewport width used for layout
    layout_viewport_height: f32,  // Viewport height used for layout
    current_url: String,  // Current page URL for resolving relative image URLs
    image_cache: HashMap<String, Vec<u8>>,  // Cache of fetched image data (URL -> raw bytes)
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
            current_url: String::new(),
            image_cache: HashMap::new(),
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
    
    pub fn set_current_url(&mut self, url: &str) {
        self.current_url = url.to_string();
    }
    
    pub fn set_image_data(&mut self, url: String, data: Vec<u8>) {
        log::debug!(target: "content_view", "Caching image data for URL: {} ({} bytes)", url, data.len());
        self.image_cache.insert(url, data);
    }
    
    pub fn get_image_data(&self, url: &str) -> Option<&Vec<u8>> {
        self.image_cache.get(url)
    }
    
    // Resolve relative URL to absolute URL based on current page URL
    fn resolve_url(&self, url: &str) -> String {
        if url.is_empty() {
            return String::new();
        }
        
        // If URL is already absolute (starts with http:// or https://), return as-is
        if url.starts_with("http://") || url.starts_with("https://") {
            return url.to_string();
        }
        
        // If current_url is empty, can't resolve
        if self.current_url.is_empty() {
            log::warn!(target: "content_view", "Cannot resolve relative URL '{}': no base URL", url);
            return url.to_string();
        }
        
        // Parse base URL
        let base_url = self.current_url.trim_end_matches('/');
        
        if url.starts_with("//") {
            // Protocol-relative URL (//example.com/image.png)
            if let Some(scheme_end) = base_url.find("://") {
                let scheme = &base_url[..scheme_end];
                return format!("{}:{}", scheme, url);
            }
        } else if url.starts_with('/') {
            // Absolute path relative to domain root (/images/logo.png)
            if let Some(scheme_end) = base_url.find("://") {
                let after_scheme = &base_url[scheme_end + 3..];
                if let Some(path_start) = after_scheme.find('/') {
                    let domain = &after_scheme[..path_start];
                    return format!("{}://{}{}", &base_url[..scheme_end], domain, url);
                } else {
                    // No path in base URL, just domain
                    return format!("{}{}", base_url, url);
                }
            }
        } else {
            // Relative path (images/logo.png)
            // Find the last '/' in base_url to get the directory
            if let Some(scheme_end) = base_url.find("://") {
                let after_scheme = &base_url[scheme_end + 3..];
                if let Some(last_slash) = after_scheme.rfind('/') {
                    // Get the base path up to the last directory
                    let base_path = &base_url[..scheme_end + 3 + last_slash + 1];
                    return format!("{}{}", base_path, url);
                } else {
                    // No path, just domain - append with /
                    return format!("{}/{}", base_url, url);
                }
            }
        }
        
        // Fallback: return original URL
        url.to_string()
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
                // Calculate maximum y coordinate to determine content height for scrolling
                let max_y = items.iter().fold(0.0f32, |max, item| {
                    let item_max = match item {
                        DisplayItem::Text { y, .. } => *y + 20.0, // Approximate text height
                        DisplayItem::Rectangle { y, height, .. } => *y + *height,
                        DisplayItem::Image { y, height, .. } => *y + *height,
                        DisplayItem::Button { y, height, .. } => *y + *height,
                    };
                    max.max(item_max)
                });
                let content_height = (max_y * scale).max(800.0); // At least viewport height
                
                log::debug!(target: "content_view", "Content height: {} (max_y: {}, scale: {})", content_height, max_y, scale);
                
                // Following GPUI scrollable example pattern:
                // Outer container with size_full (scrolling handled by parent)
                // Inner container with full content height
                // Create inner container with full content height
                // Use min_h to ensure container is at least content_height tall
                // Create container - we'll build it by chaining all children at once
                // GPUI's .child() should accumulate children, but let's verify by building the chain
                let mut container_builder = div()
                    .w_full()
                    .min_h(px(content_height))
                    .h(px(content_height))
                    .relative()
                    .bg(gpui::rgb(0xffffff));
                
                // Use fold to accumulate all children - track container and counts in a tuple
                // Add test elements at visible position to verify rendering
                let (container, rect_count, button_count, text_count, image_count) = items.iter().fold(
                    (
                        container_builder
                            .child(
                                div()
                                    .absolute()
                                    .left(px(10.0))
                                    .top(px(10.0))
                                    .w(px(200.0))
                                    .h(px(50.0))
                                    .bg(gpui::rgb(0xff0000))
                                    .text_color(gpui::rgb(0xffffff))
                                    .child("TEST ELEMENT")
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left(px(220.0))
                                    .top(px(10.0))
                                    .w(px(120.0))
                                    .h(px(32.0))
                                    .bg(gpui::rgb(0x00ff00))
                                    .border(px(2.0))
                                    .border_color(gpui::rgb(0x0000ff))
                                    .text_color(gpui::rgb(0x000000))
                                    .text_sm()
                                    .child("TEST BUTTON")
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left(px(350.0))
                                    .top(px(10.0))
                                    .w(px(272.0))
                                    .h(px(92.0))
                                    .bg(gpui::rgb(0xff8888))
                                    .border(px(2.0))
                                    .border_color(gpui::rgb(0xff0000))
                                    .text_color(gpui::rgb(0x000000))
                                    .text_xs()
                                    .child("TEST IMAGE")
                            ),
                        0, 0, 0, 0
                    ),
                    |(acc, rc, bc, tc, ic), item| {
                        match item {
                            DisplayItem::Rectangle { x, y, width, height, color } => {
                                if *width > 0.0 && *height > 0.0 {
                                    let rgb = Self::color_to_rgb(color);
                                    if rgb != 0xffffff {
                                        (acc.child(
                                            div()
                                                .absolute()
                                                .left(px(*x * scale))
                                                .top(px(*y * scale))
                                                .w(px((*width * scale).max(1.0)))
                                                .h(px((*height * scale).max(1.0)))
                                                .bg(gpui::rgb(rgb))
                                                .border(px(0.5))
                                                .border_color(gpui::rgb(0xe0e0e0))
                                        ), rc + 1, bc, tc, ic)
                                    } else {
                                        (acc, rc, bc, tc, ic)
                                    }
                                } else {
                                    (acc, rc, bc, tc, ic)
                                }
                            }
                            DisplayItem::Button { text, x, y, width, height } => {
                                let new_bc = bc + 1;
                                if new_bc <= 3 {
                                    log::info!(target: "content_view", "Rendering button #{}: '{}' at x={}, y={}, size {}x{}", 
                                        new_bc, text, x, y, width, height);
                                }
                                let btn_w = (*width * scale).max(1.0);
                                let btn_h = (*height * scale).max(1.0);
                                (acc.child(
                                    div()
                                        .absolute()
                                        .left(px(*x * scale))
                                        .top(px(*y * scale))
                                        .w(px(btn_w))
                                        .h(px(btn_h))
                                        .bg(gpui::rgb(0x00ff00))
                                        .border(px(2.0))
                                        .border_color(gpui::rgb(0x0000ff))
                                        .text_color(gpui::rgb(0x000000))
                                        .text_sm()
                                        .cursor_pointer()
                                        .hover(|style: gpui::StyleRefinement| {
                                            style.bg(gpui::rgb(0x00cc00))
                                        })
                                        .child(text.clone())
                                ), rc, new_bc, tc, ic)
                            }
                            DisplayItem::Text { content, x, y, color } => {
                                let trimmed = content.trim();
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
                                    (acc.child(
                                        div()
                                            .absolute()
                                            .left(px(*x * scale))
                                            .top(px(*y * scale))
                                            .text_color(gpui::rgb(Self::color_to_rgb(color)))
                                            .child(content.clone())
                                    ), rc, bc, tc + 1, ic)
                                } else {
                                    (acc, rc, bc, tc, ic)
                                }
                            }
                            DisplayItem::Image { url, x, y, width, height, alt } => {
                                let new_ic = ic + 1;
                                log::info!(target: "content_view", "Rendering image #{}: '{}' at x={}, y={}, size {}x{}", 
                                    new_ic, alt, x, y, width, height);
                                let image_url = if url.is_empty() {
                                    return (acc, rc, bc, tc, ic);
                                } else {
                                    self.resolve_url(url.as_str())
                                };
                                
                                let scaled_width = (*width * scale).max(1.0);
                                let scaled_height = (*height * scale).max(1.0);
                                let alt_text = alt.clone();
                                
                                // Try to get cached image data
                                if let Some(_image_data) = self.get_image_data(&image_url) {
                                    (acc.child(
                                        div()
                                            .absolute()
                                            .left(px(*x * scale))
                                            .top(px(*y * scale))
                                            .w(px(scaled_width))
                                            .h(px(scaled_height))
                                            .bg(gpui::rgb(0xff8888))
                                            .border(px(2.0))
                                            .border_color(gpui::rgb(0xff0000))
                                            .text_color(gpui::rgb(0x000000))
                                            .text_xs()
                                            .child(format!("Image\n{}", alt_text))
                                    ), rc, bc, tc, new_ic)
                                } else {
                                    (acc.child(
                                        div()
                                            .absolute()
                                            .left(px(*x * scale))
                                            .top(px(*y * scale))
                                            .w(px(scaled_width))
                                            .h(px(scaled_height))
                                            .bg(gpui::rgb(0xffff88))
                                            .border(px(2.0))
                                            .border_color(gpui::rgb(0xff8800))
                                            .text_color(gpui::rgb(0x000000))
                                            .text_xs()
                                            .child(format!("Loading...\n{}", alt_text))
                                    ), rc, bc, tc, new_ic)
                                }
                            }
                        }
                    }
                );
                
                log::info!(target: "content_view", "Built container: {} rects, {} buttons, {} text, {} images (total: {})", 
                    rect_count, button_count, text_count, image_count, 
                    rect_count + button_count + text_count + image_count);
                
                // Debug: Log if we're actually adding elements
                if rect_count == 0 && button_count == 0 && text_count == 0 && image_count == 0 {
                    log::warn!(target: "content_view", "WARNING: No elements were added to container! Only test element will be visible.");
                } else {
                    log::info!(target: "content_view", "Added {} elements to container (test element + {} content elements)", 
                        1 + rect_count + button_count + text_count + image_count,
                        rect_count + button_count + text_count + image_count);
                }
                
                // Return the container - all elements are already added
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
