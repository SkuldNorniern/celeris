// Content view component for rendering web pages
// This will be fully implemented once we determine the correct GPUI 0.2.2 API

use crate::rendering::DisplayList;

pub struct ContentView {
    loaded: bool,
    display_list: Option<DisplayList>,
    loading_text: String,
}

impl ContentView {
    pub fn new() -> Self {
        Self {
            loaded: false,
            display_list: None,
            loading_text: String::from("Ready to load page..."),
        }
    }

    pub fn set_loaded(&mut self, loaded: bool) {
        self.loaded = loaded;
        if loaded {
            self.loading_text = String::from("Page loaded");
        }
    }

    pub fn set_display_list(&mut self, display_list: DisplayList) {
        self.display_list = Some(display_list);
        self.loaded = true;
    }
}
