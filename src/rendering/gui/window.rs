use gpui::{div, prelude::*, Context, Render, Window, Entity, actions, px};
use super::{address_bar::AddressBar, toolbar::Toolbar, content_view::ContentView, dev_panel::DevPanel};
use std::sync::mpsc;

actions!(window, [ToggleDevPanel]);

pub enum LoadResult {
    Success { content: String, display_list: crate::rendering::DisplayList, console_logs: Vec<(String, String)> },
    Error(String),
}

pub struct BrowserWindow {
    browser: Option<crate::Browser>,
    address_bar: Entity<AddressBar>,
    toolbar: Toolbar,
    content_view: Entity<ContentView>,
    dev_panel: Entity<DevPanel>,
    load_result_rx: Option<mpsc::Receiver<LoadResult>>,
}

impl BrowserWindow {
    pub fn new(browser: Option<crate::Browser>, cx: &mut Context<Self>) -> Self {
        let mut window = Self {
            browser,
            address_bar: AddressBar::new(cx),
            toolbar: Toolbar::new(),
            content_view: ContentView::new(cx),
            dev_panel: DevPanel::new(cx),
            load_result_rx: None,
        };
        
        // Load default page on launch
        window.load_url("https://google.com", cx);
        
        window
    }

    pub fn toggle_dev_panel(&mut self, _: &ToggleDevPanel, _window: &mut Window, cx: &mut Context<Self>) {
        log::info!(target: "browser", "F12 pressed - toggling dev panel");
        self.dev_panel.update(cx, |panel, _cx| {
            panel.toggle();
        });
        // Force a re-render
        cx.notify();
    }

    pub fn current_url(&self, cx: &Context<Self>) -> String {
        self.address_bar.read(cx).url().to_string()
    }

    pub fn load_url(&mut self, url: &str, cx: &mut Context<Self>) {
        // Get viewport size from window if available
        // We'll need to get this from the window in render, but for now use default
        let url = if !url.starts_with("http://") && !url.starts_with("https://") {
            format!("https://{}", url)
        } else {
            url.to_string()
        };
        
        self.address_bar.update(cx, |address_bar, _cx| {
            address_bar.set_url(url.clone());
        });
        
        self.content_view.update(cx, |content_view, _cx| {
            content_view.set_loading(&format!("Loading {}...", url));
            content_view.set_loading_progress(0.1);
        });
        
            log::info!(target: "browser", "Loading URL: {}", url);
        log::debug!(target: "browser", "Starting URL load process for: {}", url);
        self.dev_panel.update(cx, |panel, _cx| {
            panel.add_log_from_string(super::dev_panel::LogLevel::Info, format!("Loading URL: {}", url));
            panel.add_log_from_string(super::dev_panel::LogLevel::Debug, format!("Starting URL load process for: {}", url));
        });
        
        // Load page in a separate thread with local async runtime
        // Note: Viewport size will be set in render method when we have window access
        // For now use default size - this will be updated when we can pass viewport size properly
        let url_clone = url.clone();
        let (tx, rx) = mpsc::channel();
        let (console_tx, console_rx) = mpsc::channel();
        self.load_result_rx = Some(rx);
        let content_view = self.content_view.clone();
        // Default viewport size - will be improved when we can pass actual size
        let vw = 1200u32;
        let vh = 800u32;
        
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut browser = match crate::Browser::new(crate::BrowserConfig {
                    headless: false,
                    debug: true,
                    enable_javascript: true,
                }) {
                    Ok(b) => b,
                    Err(e) => {
                        log::error!(target: "browser", "Failed to create browser: {}", e);
                        let _ = tx.send(LoadResult::Error(format!("Error creating browser: {}", e)));
                        return;
                    }
                };
                
                // Set up console log capture
                browser.js_engine.set_console_log_sender(console_tx);
                
                // Set viewport size before loading (using default for now)
                browser.set_viewport_size(vw, vh);
                
                match browser.load_url(&url_clone).await {
                    Ok((display_list, content)) => {
                        log::info!(target: "browser", "Successfully loaded URL: {}", url_clone);
                        log::info!(target: "browser", "Display list has {} items, content length: {}", 
                            display_list.items().len(), content.len());
                        
                        // Collect console logs
                        let mut console_logs = Vec::new();
                        while let Ok((level, message)) = console_rx.try_recv() {
                            console_logs.push((level, message));
                        }
                        
                        match tx.send(LoadResult::Success { content, display_list, console_logs }) {
                            Ok(_) => log::info!(target: "browser", "Sent success result to UI thread"),
                            Err(e) => log::error!(target: "browser", "Failed to send result: {}", e),
                        }
                    }
                    Err(e) => {
                        log::error!(target: "browser", "Failed to load URL {}: {}", url_clone, e);
                        match tx.send(LoadResult::Error(format!("Error loading page: {}", e))) {
                            Ok(_) => log::info!(target: "browser", "Sent error result to UI thread"),
                            Err(send_err) => log::error!(target: "browser", "Failed to send error: {}", send_err),
                        }
                    }
                }
            });
        });
    }
}

impl Render for BrowserWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Get viewport size for future use
        // Note: We can't easily pass this to the background thread right now
        // This is a limitation we'll need to address with a better architecture
        let _bounds = window.bounds();
        // let viewport_width = (bounds.right() - bounds.left()).into_px() as u32;
        // let viewport_height = (bounds.bottom() - bounds.top()).into_px() as u32;
        
        // Update browser viewport size if we have a browser instance
        // Note: This is a bit tricky since browser is created in a separate thread
        // We'll need to pass viewport size when creating the browser in load_url
        // Check for load results from background thread
        if let Some(ref rx) = self.load_result_rx {
            match rx.try_recv() {
                Ok(result) => {
                    log::info!(target: "browser", "Received load result in render");
                    match result {
                        LoadResult::Success { content, display_list, console_logs } => {
                            let item_count = display_list.items().len();
                            let content_len = content.len();
                            log::info!(target: "browser", "Load success: content len={}, display_list items={}", 
                                content_len, item_count);
                            log::debug!(target: "browser", "Page rendering complete with {} display items", item_count);
                            self.content_view.update(cx, |cv, _cx| {
                                cv.set_display_list(display_list);
                                cv.set_page_content(&content);
                                cv.set_loading_progress(1.0);
                                // Set layout viewport size (default 1200x800 used in browser)
                                cv.set_layout_viewport_size(1200.0, 800.0);
                            });
                            let has_console_logs = !console_logs.is_empty();
                            self.dev_panel.update(cx, |panel, _cx| {
                                panel.add_log_from_string(super::dev_panel::LogLevel::Info, 
                                    format!("Page loaded: {} items, {} bytes", item_count, content_len));
                                panel.add_log_from_string(super::dev_panel::LogLevel::Debug, 
                                    format!("Page rendering complete with {} display items", item_count));
                                
                                // Add console logs to the console tab
                                for (level, message) in console_logs {
                                    let log_level = match level.as_str() {
                                        "error" => super::dev_panel::LogLevel::Error,
                                        "warn" => super::dev_panel::LogLevel::Warn,
                                        "debug" => super::dev_panel::LogLevel::Debug,
                                        _ => super::dev_panel::LogLevel::Info,
                                    };
                                    panel.add_console_log(log_level, message);
                                }
                                
                            });
                        }
                        LoadResult::Error(err) => {
                            log::error!(target: "browser", "Load error: {}", err);
                            self.content_view.update(cx, |cv, _cx| {
                                cv.set_loading(&err);
                            });
                            self.dev_panel.update(cx, |panel, _cx| {
                                panel.add_log_from_string(super::dev_panel::LogLevel::Error, err.clone());
                            });
                        }
                    }
                    self.load_result_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // No message yet, this is normal
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    log::warn!(target: "browser", "Load result channel disconnected");
                    self.load_result_rx = None;
                }
            }
        }
        
        div()
            .flex()
            .flex_col()
            .size_full()
            .relative()
            .on_action(cx.listener(Self::toggle_dev_panel))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .p_2()
                    .gap_2()
                    .bg(gpui::rgb(0xf8f8f8))
                    .border_b(gpui::px(1.0))
                    .border_color(gpui::rgb(0xe0e0e0))
                    .child(self.toolbar.render(cx))
                    .child(self.address_bar.clone())
            )
            .child(
                div()
                    .flex_1()
                    .bg(gpui::rgb(0xffffff))
                    .relative()
                    .child(self.content_view.clone())
                    .child(
                        div()
                            .absolute()
                            .bottom(px(0.0))
                            .left(px(0.0))
                            .right(px(0.0))
                            .child(self.dev_panel.clone())
                    )
            )
    }
}
