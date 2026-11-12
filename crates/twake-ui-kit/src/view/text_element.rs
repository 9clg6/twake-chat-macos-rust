//! The element that shapes, measures and paints the text of a `TextInput`, and registers the
//! input handler of the window so that the system input method talks to the field.

use twake_gpui::{
    App, AvailableSpace, Bounds, ContentMask, Element, ElementId, ElementInputHandler, Entity,
    GlobalElementId, InspectorElementId, IntoElement, LayoutId, Pixels, Style, TextAlign, Window,
    fill, point, px, relative, size,
};

use super::hsla;
use super::text_input::{FONT_SIZE, LINE_HEIGHT, Painted, TextInput};
use super::text_layout::{Face, Laid, scroll_to_caret, shape};
use crate::text_buffer::Selection;

const MAX_ROWS: f32 = 8.0;
const CARET_WIDTH: f32 = 1.5;
/// Width added to a selection that covers a line break, so that it shows.
const BREAK_WIDTH: f32 = 4.0;
const SELECTION_ALPHA: f32 = 0.3;

/// The element of the text of a field.
pub(super) struct TextElement {
    input: Entity<TextInput>,
}

impl TextElement {
    pub fn new(input: Entity<TextInput>) -> Self {
        Self { input }
    }
}

/// What `prepaint` hands to `paint`.
pub(super) struct Frame {
    laid: Laid,
    placeholder: Vec<twake_gpui::WrappedLine>,
    scroll: f32,
    selection: Selection,
    caret: bool,
}

fn face(window: &Window) -> Face {
    let style = window.text_style();
    Face {
        font: style.font(),
        size: px(FONT_SIZE),
        color: style.color,
        line_height: px(LINE_HEIGHT),
    }
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = Frame;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let input = self.input.read(cx);
        let (text, marked) = (input.shared.clone(), input.buffer.marked_range());
        let face = face(window);
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        let layout = window.request_measured_layout(style, move |known, available, window, _| {
            let width = known.width.or(match available.width {
                AvailableSpace::Definite(width) => Some(width),
                _ => None,
            });
            let laid = Laid::new(
                shape(window, &text, marked.as_ref(), &face, width),
                face.line_height,
            );
            let height = laid.height().min(px(LINE_HEIGHT * MAX_ROWS));
            size(width.unwrap_or_default(), height)
        });
        (layout, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Frame {
        let input = self.input.read(cx);
        let face = face(window);
        let marked = input.buffer.marked_range();
        let lines = shape(
            window,
            &input.shared,
            marked.as_ref(),
            &face,
            Some(bounds.size.width),
        );
        let laid = Laid::new(lines, face.line_height);
        let selection = input.buffer.selection();
        let caret = laid.point_for_offset(selection.head);
        let scroll = scroll_to_caret(
            input.painted.as_ref().map_or(0.0, |p| p.scroll),
            f32::from(caret.y),
            LINE_HEIGHT,
            f32::from(bounds.size.height),
            f32::from(laid.height()),
        );
        let hint = Face {
            color: hsla(input.theme.colors().text.secondary),
            ..face
        };
        let placeholder = if input.shared.is_empty() {
            shape(window, &input.placeholder, None, &hint, None)
        } else {
            Vec::new()
        };
        Frame {
            laid,
            placeholder,
            scroll,
            selection,
            caret: input.caret_visible,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        frame: &mut Frame,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus = self.input.read(cx).focus_handle_clone();
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        let accent = hsla(self.input.read(cx).theme.colors().primary.main);
        let ink = hsla(self.input.read(cx).theme.colors().text.primary);
        let origin = point(bounds.left(), bounds.top() - px(frame.scroll));
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            paint_selection(frame, origin, accent, window);
            paint_text(frame, origin, window, cx);
            if focus.is_focused(window) && frame.caret && frame.selection.is_collapsed() {
                let at = frame.laid.point_for_offset(frame.selection.head);
                let caret = Bounds::new(
                    point(origin.x + at.x, origin.y + at.y),
                    size(px(CARET_WIDTH), px(LINE_HEIGHT)),
                );
                window.paint_quad(fill(caret, ink));
            }
        });
        let laid = std::mem::replace(&mut frame.laid, Laid::new(Vec::new(), px(LINE_HEIGHT)));
        let scroll = frame.scroll;
        self.input.update(cx, |input, _| {
            input.painted = Some(Painted {
                laid,
                bounds,
                scroll,
            });
        });
    }
}

fn paint_text(frame: &Frame, origin: twake_gpui::Point<Pixels>, window: &mut Window, cx: &mut App) {
    let lines = if frame.placeholder.is_empty() {
        frame.laid.lines()
    } else {
        &frame.placeholder
    };
    for (n, line) in lines.iter().enumerate() {
        let top = frame.laid.line_top(n);
        let at = point(origin.x, origin.y + top);
        let _ = line.paint(at, px(LINE_HEIGHT), TextAlign::Left, None, window, cx);
    }
}

fn paint_selection(
    frame: &Frame,
    origin: twake_gpui::Point<Pixels>,
    accent: twake_gpui::Hsla,
    window: &mut Window,
) {
    let range = frame.selection.range();
    for row in &frame.laid.rows {
        let (from, to) = (range.start.max(row.start), range.end.min(row.end));
        let breaks = row.last && range.start <= row.end && range.end > row.end;
        if from > to || (from == to && !breaks) {
            continue;
        }
        let x0 = frame.laid.x_of(row, from);
        let x1 = frame.laid.x_of(row, to)
            + if breaks {
                px(BREAK_WIDTH)
            } else {
                Pixels::ZERO
            };
        let quad = Bounds::new(
            point(origin.x + x0, origin.y + row.top),
            size(x1 - x0, px(LINE_HEIGHT)),
        );
        window.paint_quad(fill(quad, accent.opacity(SELECTION_ALPHA)));
    }
}
