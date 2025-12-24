use gpui::{
    div, prelude::*, px, Entity, EntityInputHandler, FocusHandle, Focusable, IntoElement,
    SharedString, UTF16Selection, Context, Window, CursorStyle, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Bounds, Point, actions, ClipboardItem,
};
use std::ops::Range;
use std::time::Instant;
use unicode_segmentation::UnicodeSegmentation;

actions!(
    address_bar,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        Paste,
        Cut,
        Copy,
        Enter,
    ]
);

pub struct AddressBar {
    focus_handle: FocusHandle,
    content: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    cursor_blink_start: Option<Instant>,
    is_dragging: bool,
}

impl AddressBar {
    pub fn new(cx: &mut gpui::Context<super::window::BrowserWindow>) -> Entity<Self> {
        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            content: "google.com".into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            cursor_blink_start: None,
            is_dragging: false,
        })
    }
    
    pub fn on_enter(&mut self, _: &Enter, _window: &mut Window, cx: &mut Context<Self>) {
        let url = self.content.to_string().trim().to_string();
        if url.is_empty() {
            log::warn!(target: "address_bar", "Enter pressed but URL is empty");
            return;
        }
        
        log::info!(target: "address_bar", "Enter pressed, loading URL: {}", url);
        
        // Find the parent BrowserWindow and load the URL
        // Note: In GPUI, child entities communicate with parent entities by searching through windows
        // This is a common pattern when actions aren't suitable for the use case
        let mut found = false;
        for window_handle in cx.windows() {
            if let Some(browser_window) = window_handle.downcast::<super::window::BrowserWindow>() {
                match browser_window.update(cx, |bw, _window, cx| {
                    bw.load_url(&url, cx);
                }) {
                    Ok(_) => {
                        found = true;
                        log::debug!(target: "address_bar", "Successfully dispatched load_url to BrowserWindow");
                        break;
                    }
                    Err(e) => {
                        log::error!(target: "address_bar", "Failed to update browser window: {:?}", e);
                    }
                }
            }
        }
        
        if !found {
            log::error!(target: "address_bar", "Could not find BrowserWindow to load URL. This may indicate a window management issue.");
        }
    }

    pub fn set_url(&mut self, url: String) {
        self.content = url.into();
        self.selected_range = self.content.len()..self.content.len();
        self.selection_reversed = false;
    }

    pub fn url(&self) -> &str {
        &self.content
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx)
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let click_pos = self.position_from_point(event.position, window);
        self.is_dragging = true;
        if event.modifiers.shift {
            self.select_to(click_pos, cx);
        } else {
            self.move_to(click_pos, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
        self.is_dragging = false;
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_dragging {
            let pos = self.position_from_point(event.position, window);
            self.select_to(pos, cx);
        }
    }

    // Estimate character position from click point
    // This is approximate - assumes monospace-like behavior
    fn position_from_point(&self, point: Point<gpui::Pixels>, window: &Window) -> usize {
        let bounds = window.bounds();
        let padding = px(12.0);
        let left_edge = bounds.left() + padding;
        
        // Approximate character width (assuming ~8px per character for typical font)
        // This is a rough estimate - in a real implementation you'd measure actual text
        const APPROX_CHAR_WIDTH: f32 = 8.0;
        
        // Simple approximation: estimate position based on click location
        // Since we can't easily convert Pixels, we'll use a simple heuristic
        // Place cursor at a position proportional to click location
        let char_count = self.content.len();
        if char_count == 0 {
            return 0;
        }
        
        // Estimate position: assume the text field spans most of the address bar width
        // This is a rough approximation
        let right_edge = bounds.right() - padding;
        if point.x <= left_edge {
            0
        } else if point.x >= right_edge {
            char_count
        } else {
            // Proportional position: estimate based on where in the field the click occurred
            // This is a rough approximation - in production you'd measure actual text width
            let field_start: f32 = left_edge.into();
            let field_end: f32 = right_edge.into();
            let click_x: f32 = point.x.into();
            let relative_pos = (click_x - field_start) / (field_end - field_start);
            ((relative_pos * char_count as f32) as usize).min(char_count)
        }
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text.replace("\n", " "), window, cx);
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }

    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx)
        }
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        cx.notify()
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify()
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }
}

impl EntityInputHandler for AddressBar {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range.as_ref().map(|range| self.range_to_utf16(range))
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = if let Some(range_utf16) = range_utf16 {
            self.range_from_utf16(&range_utf16)
        } else {
            self.selected_range.clone()
        };

        let start = range.start;
        let end = range.end;
        let new_content = format!(
            "{}{}{}",
            &self.content[..start],
            text,
            &self.content[end..]
        );
        self.content = new_content.into();
        let new_cursor = start + text.len();
        self.selected_range = new_cursor..new_cursor;
        self.selection_reversed = false;
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        _marked_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(marked_range_utf16) = _marked_range_utf16 {
            self.marked_range = Some(self.range_from_utf16(&marked_range_utf16));
        }
        self.replace_text_in_range(range_utf16, text, _window, cx);
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        _bounds: Bounds<gpui::Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<gpui::Pixels>> {
        None
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<gpui::Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(0)
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }
}

impl Focusable for AddressBar {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl gpui::Render for AddressBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_focused = self.focus_handle.is_focused(window);
        
        // Time-based cursor blinking (530ms on, 530ms off)
        const BLINK_PERIOD_MS: u64 = 530;
        let cursor_visible = if is_focused {
            let now = Instant::now();
            if let Some(start) = self.cursor_blink_start {
                let elapsed = now.duration_since(start).as_millis() as u64;
                (elapsed % (BLINK_PERIOD_MS * 2)) < BLINK_PERIOD_MS
            } else {
                self.cursor_blink_start = Some(now);
                true
            }
        } else {
            self.cursor_blink_start = None;
            false
        };
        
        let cursor_pos = self.selected_range.end;
        let display_text = if cursor_visible && self.selected_range.is_empty() && cursor_pos <= self.content.len() {
            let before = &self.content[..cursor_pos];
            let after = &self.content[cursor_pos..];
            format!("{}|{}", before, after)
        } else {
            self.content.to_string()
        };
        
        // Handle input for typing
        let element_bounds = window.bounds();
        let input_bounds = Bounds::from_corners(
            gpui::point(element_bounds.left() + px(12.0), element_bounds.top() + px(4.0)),
            gpui::point(element_bounds.right() - px(12.0), element_bounds.bottom() - px(4.0)),
        );
        
        window.handle_input(
            &self.focus_handle,
            gpui::ElementInputHandler::new(input_bounds, cx.entity()),
            cx,
        );
        
        div()
            .flex_1()
            .h_8()
            .px_3()
            .py_1()
            .bg(gpui::rgb(0xffffff))
            .border(px(1.0))
            .border_color(if is_focused {
                gpui::rgb(0x0066cc)
            } else {
                gpui::rgb(0xcccccc)
            })
            .rounded_md()
            .flex()
            .items_center()
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .shadow_sm()
            .hover(|style| {
                if !is_focused {
                    style.border_color(gpui::rgb(0x999999))
                } else {
                    style
                }
            })
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::on_enter))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .child(display_text)
    }
}
