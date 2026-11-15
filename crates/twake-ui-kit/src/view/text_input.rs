//! `TextInput`: the multi-line text field of the composer, drawn with GPUI on a `TextBuffer`.
//! Wrapped text, blinking caret, mouse selection, macOS keys, and the system input method
//! (dead keys, IME) through `EntityInputHandler`. No test draws it: the model is tested alone.

use std::ops::Range;
use std::time::{Duration, Instant};

use twake_gpui::{
    App, Bounds, BoxShadow, ClipboardItem, Context, CursorStyle, EntityInputHandler, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render,
    SharedString, Styled, UTF16Selection, Window, div, point, px,
};

use super::hsla;
use super::text_element::TextElement;
use super::text_keys::{Command, command, edit};
use super::text_layout::Laid;
use crate::button::{ButtonColor, ButtonSize, ButtonState, ButtonStyle, ButtonVariant};
use crate::text_buffer::{Delete, Move, TextBuffer};
use crate::theme::Theme;

/// Font size and line height of the text, in pixels.
pub(super) const FONT_SIZE: f32 = 14.0;
pub(super) const LINE_HEIGHT: f32 = 20.0;
/// Rows shown before the field scrolls.
const MAX_ROWS: usize = 8;
const PAD_X: f32 = 12.0;
const PAD_Y: f32 = 8.0;
const BORDER: f32 = 1.0;
const RADIUS: f32 = 12.0;
const BLINK: Duration = Duration::from_millis(530);

/// Emitted when the user presses Enter (Shift+Enter makes a new line): read `text()` then.
pub struct Submit;

/// What was last painted: the rows, where, and how far the text is scrolled.
pub(super) struct Painted {
    pub laid: Laid,
    pub bounds: Bounds<Pixels>,
    pub scroll: f32,
}

/// The multi-line text field.
pub struct TextInput {
    focus: FocusHandle,
    pub(super) buffer: TextBuffer,
    pub(super) shared: SharedString,
    pub(super) placeholder: SharedString,
    pub(super) theme: Theme,
    pub(super) painted: Option<Painted>,
    pub(super) caret_visible: bool,
    last_input: Instant,
    selecting: bool,
}

impl EventEmitter<Submit> for TextInput {}

impl Focusable for TextInput {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TextInput {
    /// An empty field. The caret blinks while the field has the focus.
    pub fn new(
        theme: Theme,
        placeholder: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(BLINK).await;
                if this.update_in(cx, Self::blink).is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            focus: cx.focus_handle(),
            buffer: TextBuffer::new(),
            shared: SharedString::default(),
            placeholder: placeholder.into(),
            theme,
            painted: None,
            caret_visible: true,
            last_input: Instant::now(),
            selecting: false,
        }
    }

    /// The text typed.
    pub fn text(&self) -> &str {
        self.buffer.text()
    }

    /// Empties the field (after a send).
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.buffer.set_text("");
        self.changed(cx);
    }

    pub(super) fn focus_handle_clone(&self) -> FocusHandle {
        self.focus.clone()
    }

    /// Gives the field the keyboard focus.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus, cx);
    }

    /// Height of the field with its padding and border, in pixels: grows with the rows, up to
    /// the scroll limit.
    pub fn outer_height(&self) -> f32 {
        let rows = self
            .painted
            .as_ref()
            .map_or(1, |p| p.laid.rows.len().max(1));
        rows.min(MAX_ROWS) as f32 * LINE_HEIGHT + 2.0 * (PAD_Y + BORDER)
    }

    fn blink(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.focus.is_focused(window) && self.last_input.elapsed() >= BLINK {
            self.caret_visible = !self.caret_visible;
            cx.notify();
        }
    }

    /// The text or the selection changed: show the caret, restart the blink, redraw.
    fn changed(&mut self, cx: &mut Context<Self>) {
        self.shared = self.buffer.text().to_string().into();
        self.caret_visible = true;
        self.last_input = Instant::now();
        cx.notify();
    }

    fn key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(command) = command(&event.keystroke.key, &event.keystroke.modifiers) else {
            return;
        };
        cx.stop_propagation();
        match command {
            Command::Move(to, extend) => self.move_caret(to, extend),
            Command::Submit => cx.emit(Submit),
            Command::Copy => self.copy(cx),
            Command::Cut => self.cut(cx),
            Command::Paste => self.paste(cx),
            other => edit(&mut self.buffer, other),
        }
        self.changed(cx);
    }

    fn copy(&self, cx: &mut Context<Self>) {
        let selected = self.buffer.selected_text();
        if !selected.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(selected.to_string()));
        }
    }

    fn cut(&mut self, cx: &mut Context<Self>) {
        self.copy(cx);
        self.buffer.delete(Delete::Backward);
    }

    fn paste(&mut self, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.buffer.paste(&text);
        }
    }

    /// Up and Down go by visual rows; the model goes by logical lines and handles the text edges.
    fn move_caret(&mut self, to: Move, extend: bool) {
        let down = to == Move::Down;
        if matches!(to, Move::Up | Move::Down)
            && let Some(offset) = self.row_offset(down)
        {
            let anchor = if extend {
                self.buffer.selection().anchor
            } else {
                offset
            };
            self.buffer.select(anchor, offset);
        } else {
            self.buffer.move_by(to, extend);
        }
    }

    /// The offset one visual row above or below the caret, `None` at the first or last row.
    fn row_offset(&self, down: bool) -> Option<usize> {
        let painted = self.painted.as_ref()?;
        let caret = painted.laid.point_for_offset(self.buffer.selection().head);
        let row = px(LINE_HEIGHT);
        let y = if down {
            caret.y + row * 1.5
        } else {
            caret.y - row * 0.5
        };
        let inside = y >= Pixels::ZERO && y < painted.laid.height();
        inside.then(|| painted.laid.offset_for_point(point(caret.x, y)))
    }

    /// The text offset under a point of the window.
    fn offset_at(&self, position: Point<Pixels>) -> Option<usize> {
        let painted = self.painted.as_ref()?;
        let local = point(
            position.x - painted.bounds.left(),
            position.y - painted.bounds.top() + px(painted.scroll),
        );
        Some(painted.laid.offset_for_point(local))
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, cx: &mut Context<Self>) {
        let Some(offset) = self.offset_at(event.position) else {
            return;
        };
        self.selecting = true;
        match event.click_count {
            1 if event.modifiers.shift => {
                self.buffer.select(self.buffer.selection().anchor, offset);
            }
            1 => self.buffer.select(offset, offset),
            2 => self.buffer.select_word_at(offset),
            _ => self.buffer.select_line_at(offset),
        }
        self.changed(cx);
    }

    fn mouse_move(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        if !self.selecting {
            return;
        }
        if let Some(offset) = self.offset_at(event.position) {
            self.buffer.select(self.buffer.selection().anchor, offset);
            self.changed(cx);
        }
    }

    fn mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.selecting = false;
    }

    fn utf8(&self, range: &Range<usize>) -> Range<usize> {
        self.buffer.utf16_range_to_utf8(range.clone())
    }

    fn utf16(&self, range: Range<usize>) -> Range<usize> {
        self.buffer.utf8_range_to_utf16(range)
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let bytes = self.utf8(&range);
        *adjusted = Some(self.utf16(bytes.clone()));
        self.buffer.text().get(bytes).map(str::to_string)
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let sel = self.buffer.selection();
        Some(UTF16Selection {
            range: self.utf16(sel.range()),
            reversed: sel.head < sel.anchor,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.buffer.marked_range().map(|r| self.utf16(r))
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.buffer.commit_marked();
        self.changed(cx);
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(bytes) = range.as_ref().map(|r| self.utf8(r))
            && Some(&bytes) != self.buffer.marked_range().as_ref()
        {
            self.buffer.select(bytes.start, bytes.end);
        }
        if !text.is_empty() {
            self.buffer.insert(text);
        } else if self.buffer.marked_range().is_some() {
            self.buffer.set_marked_text(None, "");
        } else if !self.buffer.selection().is_collapsed() {
            self.buffer.delete(Delete::Backward);
        }
        self.changed(cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _selected: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // The caret goes to the end of the marked text: the selection the IME asks for is ignored (A021).
        let bytes = range.as_ref().map(|r| self.utf8(r));
        self.buffer.set_marked_text(bytes, text);
        self.changed(cx);
    }

    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let painted = self.painted.as_ref()?;
        let bytes = self.utf8(&range);
        let start = painted.laid.point_for_offset(bytes.start);
        let end = painted.laid.point_for_offset(bytes.end);
        let width = if start.y == end.y {
            end.x - start.x
        } else {
            px(1.0)
        };
        let origin = point(
            element_bounds.left() + start.x,
            element_bounds.top() + start.y - px(painted.scroll),
        );
        Some(Bounds::new(
            origin,
            twake_gpui::size(width.max(px(1.0)), px(LINE_HEIGHT)),
        ))
    }

    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let offset = self.offset_at(position)?;
        Some(self.buffer.utf8_to_utf16(offset))
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = self.theme.colors();
        let focused = self.focus.is_focused(window);
        let ring = ButtonStyle::resolve(
            ButtonVariant::Contained,
            ButtonColor::Primary,
            ButtonSize::Medium,
            ButtonState::FocusVisible,
            &self.theme,
        )
        .focus_ring;
        let border = if focused {
            colors.primary.main
        } else {
            colors.border.main
        };
        let mut field = div()
            .id("text-input")
            .key_context("TextInput")
            .track_focus(&self.focus)
            .cursor(CursorStyle::IBeam)
            .w_full()
            .px(px(PAD_X))
            .py(px(PAD_Y))
            .rounded(px(RADIUS))
            .border(px(BORDER))
            .border_color(hsla(border))
            .bg(hsla(colors.background.paper))
            .text_color(hsla(colors.text.primary))
            .text_size(px(FONT_SIZE))
            .line_height(px(LINE_HEIGHT))
            .on_key_down(cx.listener(|this, event, _window, cx| this.key_down(event, cx)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event, _window, cx| this.mouse_down(event, cx)),
            )
            .on_mouse_move(cx.listener(|this, event, _window, cx| this.mouse_move(event, cx)))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::mouse_up))
            .child(TextElement::new(cx.entity()));
        if let (true, Some(ring)) = (focused, ring) {
            field = field.shadow(vec![BoxShadow {
                color: hsla(ring.color),
                offset: point(px(0.0), px(0.0)),
                blur_radius: px(0.0),
                spread_radius: px(ring.width_px),
                inset: false,
            }]);
        }
        field
    }
}
