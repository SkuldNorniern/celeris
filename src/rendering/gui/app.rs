#[cfg(feature = "gui")]
use gpui::{px, Application, WindowOptions, AppContext};
#[cfg(feature = "gui")]
use super::BrowserWindow;

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
        use super::address_bar::{Backspace, Delete, Left, Right, SelectLeft, SelectRight, SelectAll, Home, End, Paste, Cut, Copy};
        use super::window::ToggleDevPanel;
        
        Application::new().run(|cx| {
            log::info!(target: "browser", "Browser GUI application started");
            
            // Bind keys - F12 should work globally
            cx.bind_keys([
                gpui::KeyBinding::new("backspace", Backspace, None),
                gpui::KeyBinding::new("delete", Delete, None),
                gpui::KeyBinding::new("left", Left, None),
                gpui::KeyBinding::new("right", Right, None),
                gpui::KeyBinding::new("shift-left", SelectLeft, None),
                gpui::KeyBinding::new("shift-right", SelectRight, None),
                gpui::KeyBinding::new("cmd-a", SelectAll, None),
                gpui::KeyBinding::new("cmd-v", Paste, None),
                gpui::KeyBinding::new("cmd-c", Copy, None),
                gpui::KeyBinding::new("cmd-x", Cut, None),
                gpui::KeyBinding::new("home", Home, None),
                gpui::KeyBinding::new("end", End, None),
                gpui::KeyBinding::new("enter", super::address_bar::Enter, None),
            ]);
            
            // Bind F12 separately and ensure it's available to all windows
            cx.bind_keys([gpui::KeyBinding::new("f12", ToggleDevPanel, None)]);
            cx.bind_keys([gpui::KeyBinding::new("F12", ToggleDevPanel, None)]);
            
            let window_options = WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(
                    gpui::Bounds::centered(None, gpui::size(px(1200.0), px(800.0)), cx)
                )),
                ..Default::default()
            };
            
            let browser = self.browser;
            cx.open_window(window_options, |_window, cx| {
                cx.new(|cx| BrowserWindow::new(browser, cx))
            }).ok();
        });
    }
}
