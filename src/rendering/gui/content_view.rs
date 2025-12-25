use crate::rendering::{DisplayList, DisplayItem, Color};
use gpui::{div, prelude::*, Entity, IntoElement, Context, px};
use std::collections::HashMap;
use std::path::PathBuf;

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
                // Create container that adapts to viewport size
                let mut container = div()
                    .w_full()
                    .h_full()
                    .relative()
                    .bg(gpui::rgb(0xffffff));
                
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
                        // Resolve relative URLs to absolute URLs
                        let image_url = if url.is_empty() {
                            // Fallback: skip if no URL
                            continue;
                        } else {
                            self.resolve_url(url.as_str())
                        };
                        
                        log::debug!(target: "content_view", "Resolved image URL: '{}' -> '{}'", url, image_url);
                        
                        let scaled_width = (*width * scale).max(1.0);
                        let scaled_height = (*height * scale).max(1.0);
                        let alt_text = alt.clone();
                        
                        // Check if we have cached image data and render using GPUI img()
                        log::info!(target: "content_view", "Checking cache for image URL: '{}'", image_url);
                        log::info!(target: "content_view", "Cache has {} images. All keys: {:?}", 
                            self.image_cache.len(),
                            self.image_cache.keys().collect::<Vec<_>>());
                        
                        // Try to find image in cache - check exact match first, then try all cache keys
                        // This handles URL resolution mismatches between fetching and lookup
                        let cached_data = self.get_image_data(&image_url)
                            .or_else(|| {
                                // Try with trailing slash removed
                                let url_no_slash = image_url.trim_end_matches('/');
                                if url_no_slash != image_url {
                                    self.get_image_data(url_no_slash)
                                } else {
                                    None
                                }
                            })
                            .or_else(|| {
                                // Try with trailing slash added
                                if !image_url.ends_with('/') {
                                    self.get_image_data(&format!("{}/", image_url))
                                } else {
                                    None
                                }
                            })
                            .or_else(|| {
                                // Last resort: try to find any cache entry that ends with the same path
                                // This handles cases where the domain might differ but path is the same
                                let url_path = if let Some(path_start) = image_url.rfind('/') {
                                    &image_url[path_start..]
                                } else {
                                    &image_url
                                };
                                
                                self.image_cache.iter()
                                    .find(|(key, _)| key.ends_with(url_path))
                                    .map(|(_, data)| data)
                            });
                        
                        if let Some(image_data) = cached_data {
                            log::info!(target: "content_view", "Found cached image data for '{}' ({} bytes)", image_url, image_data.len());
                            // Decode image using image crate
                            // Note: image crate is available when gui feature is enabled
                            {
                                // image 0.7 API - use open() directly
                                log::info!(target: "content_view", "Attempting to decode image data ({} bytes)", image_data.len());
                                // image 0.7 API - use load_from_memory
                                let decode_result = image::load_from_memory(image_data)
                                    .map_err(|e| {
                                        log::error!(target: "content_view", "Failed to decode image: {}", e);
                                        format!("Failed to decode image: {}", e)
                                    });
                                
                                match decode_result {
                                    Ok(decoded_image) => {
                                        // Convert to rgba8 buffer for consistent handling (image 0.7 API)
                                        // DynamicImage needs to be converted - use as_rgba8() or match on variants
                                        let rgba_img = match decoded_image {
                                            image::DynamicImage::ImageRgba8(img) => img,
                                            _ => decoded_image.to_rgba(),
                                        };
                                        let img_width = rgba_img.width();
                                        let img_height = rgba_img.height();
                                        log::info!(target: "content_view", "Successfully decoded image: {}x{}", 
                                            img_width, img_height);
                                        // Save image to temporary file and use GPUI's ImageSource
                                        use std::fs;
                                        
                                        // Create temp directory if it doesn't exist
                                        let temp_dir = std::env::temp_dir().join("celeris_images");
                                        if let Err(e) = fs::create_dir_all(&temp_dir) {
                                            log::warn!(target: "content_view", "Failed to create temp dir: {}", e);
                                        }
                                        
                                        // Generate unique filename from URL hash
                                        use std::collections::hash_map::DefaultHasher;
                                        use std::hash::{Hash, Hasher};
                                        let mut hasher = DefaultHasher::new();
                                        image_url.hash(&mut hasher);
                                        let hash = hasher.finish();
                                        let temp_file = temp_dir.join(format!("img_{:x}.png", hash));
                                        
                                        // Save image as PNG to file (image 0.7 API)
                                        match rgba_img.save(&temp_file) {
                                            Ok(_) => {
                                                // Get file size for logging
                                                if let Ok(metadata) = fs::metadata(&temp_file) {
                                                    let file_size = metadata.len();
                                                    log::info!(target: "content_view", "Saved image to temp file: {:?} ({} bytes)", temp_file, file_size);
                                                } else {
                                                    log::info!(target: "content_view", "Saved image to temp file: {:?}", temp_file);
                                                }
                                                
                                                // Use GPUI's img() with file path string
                                                let file_path_str = temp_file.to_string_lossy().to_string();
                                                
                                                log::info!(target: "content_view", "Rendering image using GPUI img() at x={}, y={}, size {}x{}", 
                                                    *x * scale, *y * scale, scaled_width, scaled_height);
                                                
                                                container = container.child(
                                                    div()
                                                        .absolute()
                                                        .left(px(*x * scale))
                                                        .top(px(*y * scale))
                                                        .w(px(scaled_width))
                                                        .h(px(scaled_height))
                                                        .child(
                                                            gpui::img(file_path_str)
                                                                .w(px(scaled_width))
                                                                .h(px(scaled_height))
                                                                .object_fit(gpui::ObjectFit::Contain)
                                                        )
                                                );
                                            }
                                            Err(e) => {
                                                log::warn!(target: "content_view", "Failed to save image to temp file: {}", e);
                                                // Fallback to placeholder
                                                let placeholder_text = format!("File Error\n{}", alt_text);
                                                container = container.child(
                                                    div()
                                                        .absolute()
                                                        .left(px(*x * scale))
                                                        .top(px(*y * scale))
                                                        .w(px(scaled_width))
                                                        .h(px(scaled_height))
                                                        .bg(gpui::rgb(0xf0f0f0))
                                                        .border(px(1.0))
                                                        .border_color(gpui::rgb(0xcccccc))
                                                        .flex()
                                                        .flex_col()
                                                        .items_center()
                                                        .justify_center()
                                                        .text_color(gpui::rgb(0x666666))
                                                        .text_xs()
                                                        .p_2()
                                                        .child(placeholder_text)
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!(target: "content_view", "Failed to decode image: {}", e);
                                        // Fallback to placeholder
                                        let placeholder_text = format!("Decode Error\n{}", alt_text);
                                        container = container.child(
                                            div()
                                                .absolute()
                                                .left(px(*x * scale))
                                                .top(px(*y * scale))
                                                .w(px(scaled_width))
                                                .h(px(scaled_height))
                                                .bg(gpui::rgb(0xf0f0f0))
                                                .border(px(1.0))
                                                .border_color(gpui::rgb(0xcccccc))
                                                .flex()
                                                .flex_col()
                                                .items_center()
                                                .justify_center()
                                                .text_color(gpui::rgb(0x666666))
                                                .text_xs()
                                                .p_2()
                                                .child(placeholder_text)
                                        );
                                    }
                                }
                            }
                            // Note: image crate is always available when gui feature is enabled
                            // No fallback needed - if decoding fails, error handling above will show placeholder
                        } else {
                            // No cached image data - show loading placeholder
                            let placeholder_text: String = if alt_text.is_empty() { 
                                if image_url.len() > 40 {
                                    format!("Loading...\n{}...", &image_url[..40])
                                } else {
                                    format!("Loading...\n{}", image_url)
                                }
                            } else { 
                                format!("Loading...\n{}", alt_text)
                            };
                            
                            container = container.child(
                                div()
                                    .absolute()
                                    .left(px(*x * scale))
                                    .top(px(*y * scale))
                                    .w(px(scaled_width))
                                    .h(px(scaled_height))
                                    .bg(gpui::rgb(0xf0f0f0))
                                    .border(px(1.0))
                                    .border_color(gpui::rgb(0xcccccc))
                                    .flex()
                                    .flex_col()
                                    .items_center()
                                    .justify_center()
                                    .text_color(gpui::rgb(0x666666))
                                    .text_xs()
                                    .p_2()
                                    .child(
                                        div()
                                            .text_center()
                                            .child(placeholder_text)
                                    )
                            );
                        }
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
