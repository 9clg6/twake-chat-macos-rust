//! The conversation: bubbles, day dividers and the marker, virtualised with estimated heights.

use std::time::Instant;

use twake_gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Render, ScrollWheelEvent,
    Styled, Window, div, px,
};
use twake_ui_kit::theme::Theme;
use twake_ui_kit::view::hsla;

use super::{HEADER_HEIGHT, OVERSCAN, avatar, clamp, scrolled, viewport};
use crate::timeline_rows::{BubbleView, QuoteView, RowView, Side, Ticks};
use crate::virtual_list::HeightIndex;

const BUBBLE_MAX_WIDTH: f32 = 440.0;
const BUBBLE_PADDING: f32 = 12.0;
const TEXT_SIZE: f32 = 14.0;
const LINE_HEIGHT: f32 = 20.0;
const AVERAGE_CHAR_WIDTH: f32 = 7.4;
const SLOT_GAP: f32 = 4.0;
const FOOTER_HEIGHT: f32 = 16.0;
const NAME_HEIGHT: f32 = 18.0;
const QUOTE_HEIGHT: f32 = 40.0;
const DIVIDER_HEIGHT: f32 = 40.0;
const AVATAR_SIZE: f32 = 32.0;
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

/// The height a row is given: an estimate, since the text is not measured before it is drawn.
fn estimated_height(row: &RowView) -> f32 {
    let RowView::Bubble(bubble) = row else {
        return DIVIDER_HEIGHT;
    };
    let width = BUBBLE_MAX_WIDTH - 2.0 * BUBBLE_PADDING;
    let per_line = (width / AVERAGE_CHAR_WIDTH) as usize;
    let lines = wrap_lines(&bubble.body, per_line) as f32;
    let name = if bubble.shows_name { NAME_HEIGHT } else { 0.0 };
    let quote = if bubble.quote.is_some() {
        QUOTE_HEIGHT
    } else {
        0.0
    };
    SLOT_GAP + 2.0 * BUBBLE_PADDING * 0.67 + lines * LINE_HEIGHT + FOOTER_HEIGHT + name + quote
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
        }
    }

    /// Shows another room, scrolled to its newest message.
    pub fn set_conversation(&mut self, title: String, rows: Vec<RowView>, cx: &mut Context<Self>) {
        let heights: Vec<f32> = rows.iter().map(estimated_height).collect();
        self.index = HeightIndex::new(&heights);
        self.title = title;
        self.rows = rows;
        self.stick_to_bottom = true;
        cx.notify();
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

    fn slot(&self, position: usize) -> Option<AnyElement> {
        let row = self.rows.get(position)?;
        let height = self.index.offset(position + 1) - self.index.offset(position);
        let content = match row {
            RowView::Bubble(bubble) => bubble_slot(bubble, self.theme).into_any_element(),
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
                .px(px(12.0))
                .py(px(2.0))
                .rounded_full()
                .bg(hsla(colors.background.contrast))
                .text_size(px(12.0))
                .text_color(hsla(ink))
                .child(label.to_owned()),
        )
}

fn bubble_slot(bubble: &BubbleView, theme: Theme) -> impl IntoElement {
    let line = div()
        .flex()
        .size_full()
        .px(px(16.0))
        .pt(px(SLOT_GAP))
        .gap_2();
    match bubble.side {
        Side::Own => line.justify_end().child(bubble_body(bubble, theme)),
        Side::Other => {
            let side = if bubble.shows_avatar {
                avatar(&bubble.avatar, AVATAR_SIZE)
            } else {
                div().flex_none().w(px(AVATAR_SIZE))
            };
            line.child(side).child(bubble_body(bubble, theme))
        }
    }
}

fn bubble_body(bubble: &BubbleView, theme: Theme) -> impl IntoElement {
    let colors = theme.colors();
    let (background, ink) = match bubble.side {
        Side::Own => (colors.primary.main, colors.primary.contrast_text),
        Side::Other => (colors.background.default, colors.text.primary),
    };
    let name = bubble.shows_name.then(|| {
        div()
            .text_size(px(12.0))
            .text_color(hsla(colors.primary.dark))
            .child(bubble.author.clone())
    });
    div()
        .flex()
        .flex_col()
        .flex_initial()
        .max_w(px(BUBBLE_MAX_WIDTH))
        .px(px(BUBBLE_PADDING))
        .py(px(BUBBLE_PADDING * 0.5))
        .rounded(px(12.0))
        .bg(hsla(background))
        .text_color(hsla(ink))
        .text_size(px(TEXT_SIZE))
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
        .text_size(px(12.0))
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

impl Render for TimelineView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let started = Instant::now();
        let colors = self.theme.colors();
        let height = viewport(window);
        let total = self.index.total_height();
        if self.stick_to_bottom {
            self.scroll_top = total;
            self.stick_to_bottom = false;
        }
        self.scroll_top = clamp(self.scroll_top, total, height);
        let range = self.index.visible_range(self.scroll_top, height, OVERSCAN);
        let slots: Vec<AnyElement> = range.filter_map(|position| self.slot(position)).collect();
        self.follow_bench(window, height, started);
        div()
            .flex()
            .flex_col()
            .flex_1()
            .h_full()
            .min_w_0()
            .bg(hsla(colors.background.paper))
            .text_color(hsla(colors.text.primary))
            .child(
                div()
                    .flex()
                    .items_center()
                    .flex_none()
                    .h(px(HEADER_HEIGHT))
                    .px(px(16.0))
                    .border_b_1()
                    .border_color(hsla(colors.divider))
                    .text_size(px(18.0))
                    .child(self.title.clone()),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .overflow_hidden()
                    .on_scroll_wheel(cx.listener(
                        |this: &mut Self, event: &ScrollWheelEvent, window, cx| {
                            let total = this.index.total_height();
                            this.scroll_top =
                                scrolled(this.scroll_top, event, total, viewport(window));
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
    }
}
