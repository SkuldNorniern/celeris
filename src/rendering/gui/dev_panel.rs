use gpui::{div, prelude::*, px, Context, Render, Window, Entity, IntoElement, MouseButton};
use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::SystemTime;

#[derive(Clone, Debug)]
pub struct LogEntry {
    level: LogLevel,
    message: String,
    timestamp: SystemTime,
    source: LogSource,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum LogSource {
    Browser,
    Console,
}

impl LogEntry {
    pub fn new(level: LogLevel, message: String, source: LogSource) -> Self {
        Self {
            level,
            message,
            timestamp: SystemTime::now(),
            source,
        }
    }
}

pub struct DevPanel {
    browser_logs: VecDeque<LogEntry>,
    console_logs: VecDeque<LogEntry>,
    max_logs: usize,
    is_visible: bool,
    active_tab: Tab,
    log_receiver: Option<mpsc::Receiver<LogEntry>>,
    visible_log_count: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Browser,
    Console,
}

impl DevPanel {
    pub fn new(cx: &mut Context<super::window::BrowserWindow>) -> Entity<Self> {
        let (_tx, rx) = mpsc::channel();
        
        cx.new(|_cx| Self {
            browser_logs: VecDeque::new(),
            console_logs: VecDeque::new(),
            max_logs: 1000,
            is_visible: false,
            active_tab: Tab::Browser,
            log_receiver: Some(rx),
            visible_log_count: 200,
        })
    }

    pub fn toggle(&mut self) {
        self.is_visible = !self.is_visible;
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    fn add_log(&mut self, entry: LogEntry) {
        match entry.source {
            LogSource::Browser => {
                self.browser_logs.push_back(entry);
                if self.browser_logs.len() > self.max_logs {
                    self.browser_logs.pop_front();
                }
            }
            LogSource::Console => {
                self.console_logs.push_back(entry);
                if self.console_logs.len() > self.max_logs {
                    self.console_logs.pop_front();
                }
            }
        }
    }

    pub fn add_log_from_string(&mut self, level: LogLevel, message: String) {
        self.add_log(LogEntry::new(level, message, LogSource::Browser));
    }

    pub fn add_console_log(&mut self, level: LogLevel, message: String) {
        self.add_log(LogEntry::new(level, message, LogSource::Console));
    }

    fn current_logs(&self) -> &VecDeque<LogEntry> {
        match self.active_tab {
            Tab::Browser => &self.browser_logs,
            Tab::Console => &self.console_logs,
        }
    }

    fn switch_tab(&mut self, tab: Tab) {
        self.active_tab = tab;
    }
}

impl Render for DevPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Check for new logs
        let mut new_logs = Vec::new();
        if let Some(ref rx) = self.log_receiver {
            while let Ok(entry) = rx.try_recv() {
                new_logs.push(entry);
            }
        }
        for entry in new_logs {
            self.add_log(entry);
        }

        if !self.is_visible {
            return div();
        }

        let current_logs = self.current_logs();
        let log_count = current_logs.len();
        let visible_logs: Vec<_> = current_logs.iter().rev().take(self.visible_log_count).collect();

        div()
            .h(px(300.0))
            .bg(gpui::rgb(0x1e1e1e))
            .border_t(px(1.0))
            .border_color(gpui::rgb(0x3e3e3e))
            .flex()
            .flex_col()
            .w_full()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2()
                    .bg(gpui::rgb(0x252526))
                    .border_b(px(1.0))
                    .border_color(gpui::rgb(0x3e3e3e))
                    .w_full()
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_2()
                            .flex_1()
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_1()
                                    .child(
                                        div()
                                            .px_3()
                                            .py_1()
                                            .rounded_t_md()
                                            .cursor_pointer()
                                            .bg(if self.active_tab == Tab::Browser {
                                                gpui::rgb(0x1e1e1e)
                                            } else {
                                                gpui::rgb(0x252526)
                                            })
                                            .text_color(gpui::rgb(0xcccccc))
                                            .text_sm()
                                            .hover(|style| {
                                                if self.active_tab != Tab::Browser {
                                                    style.bg(gpui::rgb(0x2d2d30))
                                                } else {
                                                    style
                                                }
                                            })
                                            .on_mouse_up(MouseButton::Left, cx.listener(move |this, _, _, _| {
                                                this.switch_tab(Tab::Browser);
                                            }))
                                            .child(format!("Browser ({})", self.browser_logs.len()))
                                    )
                                    .child(
                                        div()
                                            .px_3()
                                            .py_1()
                                            .rounded_t_md()
                                            .cursor_pointer()
                                            .bg(if self.active_tab == Tab::Console {
                                                gpui::rgb(0x1e1e1e)
                                            } else {
                                                gpui::rgb(0x252526)
                                            })
                                            .text_color(gpui::rgb(0xcccccc))
                                            .text_sm()
                                            .hover(|style| {
                                                if self.active_tab != Tab::Console {
                                                    style.bg(gpui::rgb(0x2d2d30))
                                                } else {
                                                    style
                                                }
                                            })
                                            .on_mouse_up(MouseButton::Left, cx.listener(move |this, _, _, _| {
                                                this.switch_tab(Tab::Console);
                                            }))
                                            .child(format!("Console ({})", self.console_logs.len()))
                                    )
                            )
                            .child(
                                div()
                                    .text_color(gpui::rgb(0x999999))
                                    .text_xs()
                                    .child(if log_count > self.visible_log_count {
                                        format!("Showing {} of {} logs", self.visible_log_count, log_count)
                                    } else {
                                        format!("{} logs", log_count)
                                    })
                            )
                            .child(
                                div()
                                    .w_6()
                                    .h_6()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .rounded_md()
                                    .text_color(gpui::rgb(0xcccccc))
                                    .text_sm()
                                    .hover(|style| {
                                        style.bg(gpui::rgb(0x3e3e3e)).text_color(gpui::rgb(0xffffff))
                                    })
                                    .on_mouse_up(MouseButton::Left, cx.listener(move |this, _, _, _| {
                                        this.toggle();
                                    }))
                                    .child("×")
                            )
                    )
            )
            .child(
                div()
                    .flex_1()
                    .p_2()
                    .overflow_hidden()
                    .min_w_0()
                    .child({
                        let mut container = div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .w_full();
                        for log in visible_logs {
                            let (color, prefix) = match log.level {
                                LogLevel::Info => (gpui::rgb(0x4ec9b0), "INFO"),
                                LogLevel::Warn => (gpui::rgb(0xce9178), "WARN"),
                                LogLevel::Error => (gpui::rgb(0xf48771), "ERROR"),
                                LogLevel::Debug => (gpui::rgb(0x9cdcfe), "DEBUG"),
                            };
                            container = container.child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap_2()
                                    .text_xs()
                                    .py_1()
                                    .w_full()
                                    .min_w_0()
                                    .hover(|style| {
                                        style.bg(gpui::rgb(0x2a2d2e))
                                    })
                                    .child(
                                        div()
                                            .text_color(color)
                                            .w(px(50.0))
                                            .flex_shrink_0()
                                            .child(prefix)
                                    )
                                    .child(
                                        div()
                                            .text_color(gpui::rgb(0xcccccc))
                                            .flex_1()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .child(log.message.clone())
                                    )
                            );
                        }
                        container
                    })
            )
    }
}

