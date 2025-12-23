use gpui::{div, prelude::*, Context, Render, Window, Entity};
use super::{address_bar::AddressBar, toolbar::Toolbar, content_view::ContentView};
use std::sync::mpsc;

pub enum LoadResult {
    Success { content: String, display_list: crate::rendering::DisplayList },
    Error(String),
}

pub struct BrowserWindow {
    browser: Option<crate::Browser>,
    address_bar: Entity<AddressBar>,
    toolbar: Toolbar,
    content_view: Entity<ContentView>,
    load_result_rx: Option<mpsc::Receiver<LoadResult>>,
}

impl BrowserWindow {
    pub fn new(browser: Option<crate::Browser>, cx: &mut Context<Self>) -> Self {
        let mut window = Self {
            browser,
            address_bar: AddressBar::new(cx),
            toolbar: Toolbar::new(),
            content_view: ContentView::new(cx),
            load_result_rx: None,
        };
        
        // Load default page on launch
        window.load_url("https://google.com", cx);
        
        window
    }

    pub fn current_url(&self, cx: &Context<Self>) -> String {
        self.address_bar.read(cx).url().to_string()
    }

    pub fn load_url(&mut self, url: &str, cx: &mut Context<Self>) {
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
        });
        
        log::info!(target: "browser", "Loading URL: {}", url);
        
        // Load page in a separate thread with local async runtime
        let url_clone = url.clone();
        let (tx, rx) = mpsc::channel();
        self.load_result_rx = Some(rx);
        let content_view = self.content_view.clone();
        
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
                
                match browser.load_url(&url_clone).await {
                    Ok((display_list, content)) => {
                        log::info!(target: "browser", "Successfully loaded URL: {}", url_clone);
                        log::info!(target: "browser", "Display list has {} items, content length: {}", 
                            display_list.items().len(), content.len());
                        match tx.send(LoadResult::Success { content, display_list }) {
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Check for load results from background thread
        if let Some(ref rx) = self.load_result_rx {
            match rx.try_recv() {
                Ok(result) => {
                    log::info!(target: "browser", "Received load result in render");
                    match result {
                        LoadResult::Success { content, display_list } => {
                            log::info!(target: "browser", "Load success: content len={}, display_list items={}", 
                                content.len(), display_list.items().len());
                            self.content_view.update(cx, |cv, _cx| {
                                cv.set_display_list(display_list);
                                cv.set_page_content(&content);
                            });
                        }
                        LoadResult::Error(err) => {
                            log::error!(target: "browser", "Load error: {}", err);
                            self.content_view.update(cx, |cv, _cx| {
                                cv.set_loading(&err);
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
            .child(
                div()
                    .flex()
                    .flex_row()
                    .p_2()
                    .gap_2()
                    .bg(gpui::rgb(0xf8f8f8))
                    .child(self.toolbar.render(cx))
                    .child(self.address_bar.clone())
            )
            .child(
                div()
                    .flex_1()
                    .bg(gpui::rgb(0xffffff))
                    .child(self.content_view.clone())
            )
    }
}
