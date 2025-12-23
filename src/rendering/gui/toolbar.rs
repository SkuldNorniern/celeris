use gpui::{div, prelude::*, MouseButton, IntoElement};

pub struct Toolbar {
    can_go_back: bool,
    can_go_forward: bool,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            can_go_back: false,
            can_go_forward: false,
        }
    }

    pub fn set_navigation_state(&mut self, can_go_back: bool, can_go_forward: bool) {
        self.can_go_back = can_go_back;
        self.can_go_forward = can_go_forward;
    }

    pub fn render(&self, cx: &gpui::Context<super::window::BrowserWindow>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .gap_1()
            .child(
                div()
                    .w_8()
                    .h_8()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .text_color(if self.can_go_back {
                        gpui::rgb(0x333333)
                    } else {
                        gpui::rgb(0x999999)
                    })
                    .hover(|style| {
                        style.bg(gpui::rgb(0xf0f0f0)).rounded_md()
                    })
                    .child("←")
            )
            .child(
                div()
                    .w_8()
                    .h_8()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .text_color(if self.can_go_forward {
                        gpui::rgb(0x333333)
                    } else {
                        gpui::rgb(0x999999)
                    })
                    .hover(|style| {
                        style.bg(gpui::rgb(0xf0f0f0)).rounded_md()
                    })
                    .child("→")
            )
            .child(
                div()
                    .w_8()
                    .h_8()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .text_color(gpui::rgb(0x333333))
                    .hover(|style| {
                        style.bg(gpui::rgb(0xf0f0f0)).rounded_md()
                    })
                    .on_mouse_up(MouseButton::Left, cx.listener(|_this, _event, _window, cx| {
                        for window_handle in cx.windows() {
                            if let Some(browser_window) = window_handle.downcast::<super::window::BrowserWindow>() {
                                let _ = browser_window.update(cx, |browser_window, _window, cx| {
                                    let url = browser_window.current_url(cx);
                                    browser_window.load_url(&url, cx);
                                });
                                break;
                            }
                        }
                    }))
                    .child("↻")
            )
    }
}
