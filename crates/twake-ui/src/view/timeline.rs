//! The conversation: bubbles, day dividers and the marker, virtualised with estimated heights.

use std::time::Instant;

use twake_gpui::{
    AnyElement, AnyView, Context, Div, InteractiveElement, IntoElement, ParentElement, Render,
    ScrollWheelEvent, Styled, Window, div, px,
};
use twake_ui_kit::theme::Theme;
use twake_ui_kit::view::hsla;

use super::{HEADER_HEIGHT, OVERSCAN, avatar, clamp, scrolled, viewport};
use crate::bubble_style::{BubbleStyle, wallpaper};
use crate::room_row_style::LIST_WIDTH_PX;
use crate::timeline_rows::{BubbleView, QuoteView, RowView, Side, Ticks};
use crate::virtual_list::HeightIndex;

/// Width of the column before the first draw, until the window says its own.
const ESTIMATE_COLUMN: f32 = 800.0;
/// Average width of a character of the 15 px body text.
const AVERAGE_CHAR_WIDTH: f32 = 7.0;
const CAPTION_PX: f32 = 12.0;
const DIVIDER_PADDING_X: f32 = 12.0;
/// Padding of a line of the conversation, and the gap between its avatar and its bubble.
const LINE_PADDING: f32 = 16.0;
const LINE_GAP: f32 = 8.0;
const SLOT_GAP: f32 = 4.0;
const FOOTER_HEIGHT: f32 = 16.0;
const NAME_HEIGHT: f32 = 18.0;
const QUOTE_HEIGHT: f32 = 40.0;
const DIVIDER_HEIGHT: f32 = 40.0;
const AVATAR_SIZE: f32 = 32.0;
/// Vertical padding of the bar that holds the footer, and its top border.
const BAR_PADDING: f32 = 8.0;
const BAR_BORDER: f32 = 1.0;
const BENCH_FRAMES: usize = 600;
const BENCH_STEP: f32 = 48.0;

/// Lines a text takes in a bubble, filling each line with words greedily.
fn wrap_lines(text: &str, per_line: usize) -> usize {
    let mut lines = 0;
    for paragraph in text.split('\n') {
        let mut used = 0;
        lines += 1;
        for length in paragraph.split_whitespace().map(|w| w.chars().count()) {
            if used > 0 && used + 1 + length > per_line {
                lines += 1;
                used = 0;
            }
            used += length + usize::from(used > 0);
            // A word longer than a line breaks over several.
            lines += length.saturating_sub(1) / per_line;
        }
    }
    lines.max(1)
}

/// Width a bubble may fill in a column `column_px` wide: the line without its padding and, for
/// others, without the avatar and its gap.
fn container_px(side: Side, column_px: f32) -> f32 {
    let line = column_px - 2.0 * LINE_PADDING;
    match side {
        Side::Own => line,
        Side::Other => line - AVATAR_SIZE - LINE_GAP,
    }
}

/// The height a row is given: an estimate, since the text is not measured before it is drawn.
fn estimated_height(row: &RowView, theme: Theme, column_px: f32) -> f32 {
    let RowView::Bubble(bubble) = row else {
        return DIVIDER_HEIGHT;
    };
    let style = BubbleStyle::resolve(bubble.side, &theme, container_px(bubble.side, column_px));
    let width = style.max_width_px - 2.0 * style.padding_x_px;
    let per_line = (width / AVERAGE_CHAR_WIDTH) as usize;
    let lines = wrap_lines(&bubble.body, per_line) as f32;
    let name = if bubble.shows_name { NAME_HEIGHT } else { 0.0 };
    let quote = if bubble.quote.is_some() {
        QUOTE_HEIGHT
    } else {
        0.0
    };
    SLOT_GAP
        + 2.0 * style.padding_y_px
        + lines * style.line_height_px
        + FOOTER_HEIGHT
        + name
        + quote
}

/// Frame times of an automatic scroll, printed on stderr when it ends.
struct Bench {
    last: Instant,
    frames: Vec<f32>,
    builds: Vec<f32>,
    step: f32,
}

fn percentile(sorted: &[f32], fraction: f32) -> f32 {
    let at = ((sorted.len() as f32 - 1.0) * fraction).round() as usize;
    sorted.get(at).copied().unwrap_or(0.0)
}

impl Bench {
    fn report(&mut self, rows: usize) {
        self.frames.sort_by(f32::total_cmp);
        self.builds.sort_by(f32::total_cmp);
        let slow = self.frames.iter().filter(|ms| **ms > 25.0).count();
        eprintln!(
            "bench: {} frames over {rows} rows; frame ms p50 {:.1} p95 {:.1} p99 {:.1} max {:.1}; \
             {slow} over 25 ms; build ms p50 {:.2} p95 {:.2} max {:.2}",
            self.frames.len(),
            percentile(&self.frames, 0.5),
            percentile(&self.frames, 0.95),
            percentile(&self.frames, 0.99),
            percentile(&self.frames, 1.0),
            percentile(&self.builds, 0.5),
            percentile(&self.builds, 0.95),
            percentile(&self.builds, 1.0),
        );
    }
}

/// The conversation column.
pub struct TimelineView {
    title: String,
    rows: Vec<RowView>,
    index: HeightIndex,
    scroll_top: f32,
    stick_to_bottom: bool,
    theme: Theme,
    bench: Option<Bench>,
    footer: Option<AnyView>,
    footer_height: f32,
    at_bottom: bool,
    column_px: f32,
}

impl TimelineView {
    /// An empty conversation.
    pub fn new(theme: Theme) -> Self {
        Self {
            title: String::new(),
            rows: Vec::new(),
            index: HeightIndex::new(&[]),
            scroll_top: 0.0,
            stick_to_bottom: true,
            theme,
            bench: None,
            footer: None,
            footer_height: 0.0,
            at_bottom: true,
            column_px: ESTIMATE_COLUMN,
        }
    }

    /// Shows `footer` (the composer, `height` pixels tall) in a bar under the list: a list at
    /// its end stays at its end when the footer grows.
    pub fn set_footer(&mut self, footer: AnyView, height: f32, cx: &mut Context<Self>) {
        let height = height + 2.0 * BAR_PADDING + BAR_BORDER;
        if self.footer.is_some() && height == self.footer_height {
            return;
        }
        self.stick_to_bottom |= self.at_bottom;
        self.footer = Some(footer);
        self.footer_height = height;
        cx.notify();
    }

    /// Shows another room, scrolled to its newest message.
    pub fn set_conversation(&mut self, title: String, rows: Vec<RowView>, cx: &mut Context<Self>) {
        let heights: Vec<f32> = rows
            .iter()
            .map(|row| estimated_height(row, self.theme, self.column_px))
            .collect();
        self.index = HeightIndex::new(&heights);
        self.title = title;
        self.rows = rows;
        self.stick_to_bottom = true;
        cx.notify();
    }

    /// Estimates the heights again for a column of another width.
    fn resize_column(&mut self, column_px: f32) {
        self.column_px = column_px;
        let heights: Vec<f32> = self
            .rows
            .iter()
            .map(|row| estimated_height(row, self.theme, column_px))
            .collect();
        self.index = HeightIndex::new(&heights);
    }

    /// Scrolls up and down by itself for a few hundred frames, then prints the frame times.
    pub fn start_bench(&mut self, cx: &mut Context<Self>) {
        self.bench = Some(Bench {
            last: Instant::now(),
            frames: Vec::new(),
            builds: Vec::new(),
            step: -BENCH_STEP,
        });
        cx.notify();
    }

    fn follow_bench(&mut self, window: &Window, height: f32, started: Instant) {
        let total = self.index.total_height();
        let Some(bench) = self.bench.as_mut() else {
            return;
        };
        bench
            .frames
            .push(bench.last.elapsed().as_secs_f32() * 1000.0);
        bench.builds.push(started.elapsed().as_secs_f32() * 1000.0);
        bench.last = Instant::now();
        let next = self.scroll_top + bench.step;
        if next < 0.0 || next > total - height {
            bench.step = -bench.step;
        }
        self.scroll_top = clamp(next, total, height);
        if bench.frames.len() >= BENCH_FRAMES {
            bench.report(self.rows.len());
            self.bench = None;
        } else {
            window.request_animation_frame();
        }
    }

    /// The bar under the list that holds the composer.
    fn footer_bar(&self) -> Option<Div> {
        let footer = self.footer.clone()?;
        Some(
            div()
                .flex_none()
                .px(px(16.0))
                .py(px(BAR_PADDING))
                .border_t_1()
                .border_color(hsla(self.theme.colors().divider))
                .child(footer),
        )
    }

    fn slot(&self, position: usize) -> Option<AnyElement> {
        let row = self.rows.get(position)?;
        let height = self.index.offset(position + 1) - self.index.offset(position);
        let content = match row {
            RowView::Bubble(bubble) => {
                bubble_slot(bubble, self.column_px, self.theme).into_any_element()
            }
            RowView::Day(day) => divider(&day.label, false, self.theme).into_any_element(),
            RowView::NewMessages(marker) => {
                divider(&marker.label, true, self.theme).into_any_element()
            }
        };
        Some(
            div()
                .absolute()
                .top(px(self.index.offset(position)))
                .left(px(0.0))
                .w_full()
                .h(px(height))
                .overflow_hidden()
                .child(content)
                .into_any_element(),
        )
    }
}

fn divider(label: &str, highlighted: bool, theme: Theme) -> impl IntoElement {
    let colors = theme.colors();
    let ink = if highlighted {
        colors.primary.main
    } else {
        colors.text.secondary
    };
    div()
        .flex()
        .justify_center()
        .items_center()
        .size_full()
        .child(
            div()
                .px(px(DIVIDER_PADDING_X))
                .py(px(2.0))
                .rounded_full()
                .bg(hsla(colors.background.contrast))
                .text_size(px(CAPTION_PX))
                .text_color(hsla(ink))
                .child(label.to_owned()),
        )
}

fn bubble_slot(bubble: &BubbleView, column_px: f32, theme: Theme) -> impl IntoElement {
    let line = div()
        .flex()
        .size_full()
        .px(px(LINE_PADDING))
        .pt(px(SLOT_GAP))
        .gap(px(LINE_GAP));
    let container = container_px(bubble.side, column_px);
    match bubble.side {
        Side::Own => line
            .justify_end()
            .child(bubble_body(bubble, container, theme)),
        Side::Other => {
            let side = if bubble.shows_avatar {
                avatar(&bubble.avatar, AVATAR_SIZE)
            } else {
                div().flex_none().w(px(AVATAR_SIZE))
            };
            line.child(side)
                .child(bubble_body(bubble, container, theme))
        }
    }
}

fn bubble_body(bubble: &BubbleView, container_px: f32, theme: Theme) -> impl IntoElement {
    let colors = theme.colors();
    let style = BubbleStyle::resolve(bubble.side, &theme, container_px);
    let name = bubble.shows_name.then(|| {
        div()
            .h(px(NAME_HEIGHT))
            .text_size(px(CAPTION_PX))
            .text_color(hsla(colors.primary.main))
            .child(bubble.author.clone())
    });
    div()
        .flex()
        .flex_col()
        .flex_initial()
        .max_w(px(style.max_width_px))
        .px(px(style.padding_x_px))
        .py(px(style.padding_y_px))
        .rounded(px(style.radius_px))
        .bg(hsla(style.background))
        .text_color(hsla(style.text))
        .text_size(px(style.font_size_px))
        .line_height(px(style.line_height_px))
        .children(name)
        .children(bubble.quote.as_ref().map(|quote| quote_view(quote, theme)))
        .child(bubble.body.clone())
        .child(footer(bubble, theme))
}

fn quote_view(quote: &QuoteView, theme: Theme) -> impl IntoElement {
    let colors = theme.colors();
    div()
        .flex()
        .flex_col()
        .mb_1()
        .pl_2()
        .border_l_2()
        .border_color(hsla(colors.primary.light))
        .text_size(px(CAPTION_PX))
        .overflow_hidden()
        .max_h(px(QUOTE_HEIGHT - 4.0))
        .child(quote.author.clone())
        .child(quote.text.clone())
}

fn tick_text(ticks: Ticks) -> &'static str {
    match ticks {
        Ticks::Clock => "...",
        Ticks::Sent => "\u{2713}",
        Ticks::Read => "\u{2713}\u{2713}",
        Ticks::Failed => "!",
    }
}

fn footer(bubble: &BubbleView, _theme: Theme) -> impl IntoElement {
    let mut text = bubble.time.clone();
    if let Some(edited) = &bubble.edited {
        text = format!("{edited} {text}");
    }
    if let Some(ticks) = bubble.ticks {
        text = format!("{text} {}", tick_text(ticks));
    }
    div()
        .flex()
        .justify_end()
        .h(px(FOOTER_HEIGHT))
        .text_size(px(11.0))
        .opacity(0.75)
        .child(text)
}

fn header(title: &str, theme: Theme) -> impl IntoElement {
    let colors = theme.colors();
    div()
        .flex()
        .items_center()
        .flex_none()
        .h(px(HEADER_HEIGHT))
        .px(px(16.0))
        .bg(hsla(colors.background.paper))
        .border_b_1()
        .border_color(hsla(colors.divider))
        .text_size(px(18.0))
        .child(title.to_owned())
}

impl Render for TimelineView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let started = Instant::now();
        let colors = self.theme.colors();
        let height = viewport(window) - self.footer_height;
        let column_px = f32::from(window.viewport_size().width) - LIST_WIDTH_PX;
        if (column_px - self.column_px).abs() >= 1.0 {
            self.resize_column(column_px);
        }
        let total = self.index.total_height();
        if self.stick_to_bottom {
            self.scroll_top = total;
            self.stick_to_bottom = false;
        }
        self.scroll_top = clamp(self.scroll_top, total, height);
        self.at_bottom = self.scroll_top >= total - height - 1.0;
        let range = self.index.visible_range(self.scroll_top, height, OVERSCAN);
        let slots: Vec<AnyElement> = range.filter_map(|position| self.slot(position)).collect();
        self.follow_bench(window, height, started);
        div()
            .flex()
            .flex_col()
            .flex_1()
            .h_full()
            .min_w_0()
            .bg(hsla(wallpaper(&self.theme)))
            .text_color(hsla(colors.text.primary))
            .child(header(&self.title, self.theme))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .overflow_hidden()
                    .on_scroll_wheel(cx.listener(
                        |this: &mut Self, event: &ScrollWheelEvent, window, cx| {
                            let total = this.index.total_height();
                            let height = viewport(window) - this.footer_height;
                            this.scroll_top = scrolled(this.scroll_top, event, total, height);
                            cx.notify();
                        },
                    ))
                    .child(
                        div()
                            .absolute()
                            .top(px(-self.scroll_top))
                            .left(px(0.0))
                            .w_full()
                            .h(px(total))
                            .children(slots),
                    ),
            )
            .children(self.footer_bar())
    }
}
