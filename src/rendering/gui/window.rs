// Browser window component
// This will be implemented once we determine the correct GPUI 0.2.2 API structure

pub struct BrowserWindow {
    browser: Option<crate::Browser>,
}

impl BrowserWindow {
    pub fn new(browser: Option<crate::Browser>) -> Self {
        Self { browser }
    }

    pub fn load_url(&mut self, url: &str) {
        if let Some(ref mut browser) = self.browser {
            // URL loading will be handled asynchronously
            log::info!(target: "browser", "Loading URL: {}", url);
        }
    }
}
