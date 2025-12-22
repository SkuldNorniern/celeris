#[cfg(feature = "gui")]
use gpui::{Application, WindowOptions};

pub struct BrowserApp {
    browser: Option<crate::Browser>,
}

impl BrowserApp {
    pub fn new() -> Self {
        Self {
            browser: None,
        }
    }

    pub fn with_browser(browser: crate::Browser) -> Self {
        Self {
            browser: Some(browser),
        }
    }

    #[cfg(feature = "gui")]
    pub fn run(self) {
        Application::new().run(|_cx| {
            // Window creation will be implemented once we determine the correct GPUI API
            log::info!(target: "browser", "Browser GUI application started");
        });
    }
}
