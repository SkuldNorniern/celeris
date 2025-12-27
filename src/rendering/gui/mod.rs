#[cfg(feature = "gui")]
mod app;
#[cfg(feature = "gui")]
mod window;
#[cfg(feature = "gui")]
mod address_bar;
#[cfg(feature = "gui")]
mod toolbar;
#[cfg(feature = "gui")]
mod content_view;
#[cfg(feature = "gui")]
mod dev_panel;
#[cfg(feature = "gui")]
mod skia;

#[cfg(feature = "gui")]
pub use app::BrowserApp;
#[cfg(feature = "gui")]
pub use window::BrowserWindow;

#[cfg(not(feature = "gui"))]
pub struct BrowserApp;

#[cfg(not(feature = "gui"))]
impl BrowserApp {
    pub fn new() -> Self {
        Self
    }
}

