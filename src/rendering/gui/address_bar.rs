// Address bar component for entering URLs
// This will be fully implemented once we determine the correct GPUI 0.2.2 API

pub struct AddressBar {
    url: String,
}

impl AddressBar {
    pub fn new() -> Self {
        Self {
            url: String::new(),
        }
    }

    pub fn set_url(&mut self, url: String) {
        self.url = url;
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}
