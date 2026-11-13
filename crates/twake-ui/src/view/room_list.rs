//! The room list: one fixed-height row per room, only the rows in the window are built.

use twake_app::room_list::RoomListState;
use twake_domain::{RoomId, Timestamp};
use twake_gpui::{
    AnyElement, Context, EventEmitter, FontWeight, InteractiveElement, IntoElement, ParentElement,
    Render, ScrollEvent, StatefulInteractiveElement, Styled, Window, div, px,
};
use twake_i18n::Locale;
use twake_ui_kit::theme::Theme;
use twake_ui_kit::view::hsla;

use super::{HEADER_HEIGHT, OVERSCAN, avatar, clamp, scrolled, viewport};
use crate::room_row::{BadgeView, RoomRowView};
use crate::room_row_style::{BadgeSpec, LIST_WIDTH_PX, ROW_HEIGHT_PX, RowState, RowStyle};
use crate::virtual_list::HeightIndex;

/// Rows keep one height (so the window needs no measure).
const ROW_HEIGHT: f32 = ROW_HEIGHT_PX;

/// A row was clicked.
pub struct RoomSelected(pub RoomId);

/// The room list column.
pub struct RoomListView {
    rows: Vec<(RoomId, RoomRowView)>,
    index: HeightIndex,
    scroll_top: f32,
    selected: Option<RoomId>,
    theme: Theme,
}

impl EventEmitter<RoomSelected> for RoomListView {}

impl RoomListView {
    /// The rows of what the state shows, with their texts made once for `now` and `locale`.
    pub fn new(state: &RoomListState, now: Timestamp, locale: &Locale, theme: Theme) -> Self {
        let rows: Vec<(RoomId, RoomRowView)> = state
            .view()
            .into_iter()
            .map(|room| (room.id.clone(), RoomRowView::from(&room, now, locale)))
            .collect();
        let index = HeightIndex::new(&vec![ROW_HEIGHT; rows.len()]);
        Self {
            rows,
            index,
            scroll_top: 0.0,
            selected: None,
            theme,
        }
    }

    /// Marks a room as the open one.
    pub fn select(&mut self, id: RoomId, cx: &mut Context<Self>) {
        self.selected = Some(id.clone());
        cx.emit(RoomSelected(id));
        cx.notify();
    }

    fn row(&self, position: usize, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (id, row) = self.rows.get(position)?;
        let selected = self.selected.as_ref() == Some(id);
        let rest = RowStyle::resolve(
            RowState {
                selected,
                hovered: false,
            },
            &self.theme,
        );
        let hover = RowStyle::resolve(
            RowState {
                selected,
                hovered: true,
            },
            &self.theme,
        )
        .background;
        let target = id.clone();
        let content = row_content(row, &rest, self.theme);
        Some(
            div()
                .id(("room", position))
                .absolute()
                .top(px(self.index.offset(position)))
                .left(px(0.0))
                .w_full()
                .h(px(ROW_HEIGHT))
                .bg(hsla(rest.background))
                .hover(move |style| style.bg(hsla(hover)))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| this.select(target.clone(), cx)))
                .child(content)
                .into_any_element(),
        )
    }
}

fn row_content(row: &RoomRowView, style: &RowStyle, theme: Theme) -> impl IntoElement {
    let colors = theme.colors();
    let preview = row.preview.clone().unwrap_or_default();
    let title = div()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_ellipsis()
        .text_size(px(style.name_font.font_size))
        .font_weight(FontWeight(f32::from(style.name_font.font_weight)))
        .child(row.title.clone());
    let tail = div()
        .flex()
        .flex_col()
        .flex_none()
        .items_end()
        .gap_1()
        .text_size(px(12.0))
        .text_color(hsla(colors.text.secondary))
        .child(row.time.clone().unwrap_or_default())
        .children(row.badge.as_ref().map(|badge| badge_view(badge, theme)));
    div()
        .flex()
        .items_center()
        .size_full()
        .px(px(16.0))
        .gap(px(style.gap_px))
        .text_color(hsla(colors.text.primary))
        .child(avatar(&row.avatar, style.avatar_px))
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(title)
                .child(
                    div()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .text_size(px(13.0))
                        .text_color(hsla(colors.text.secondary))
                        .child(preview),
                ),
        )
        .child(tail)
}

fn badge_view(badge: &BadgeView, theme: Theme) -> impl IntoElement {
    let spec = BadgeSpec::resolve(badge.style, badge.text.is_empty(), &theme);
    div()
        .flex()
        .items_center()
        .justify_center()
        .min_w(px(spec.min_width_px))
        .h(px(spec.height_px))
        .px(px(spec.padding_x_px))
        .rounded(px(spec.radius_px))
        .bg(hsla(spec.background))
        .text_color(hsla(spec.text))
        .text_size(px(spec.font_size_px))
        .font_weight(FontWeight(f32::from(spec.font_weight)))
        .child(badge.text.clone())
}

impl Render for RoomListView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = self.theme.colors();
        let height = viewport(window);
        self.scroll_top = clamp(self.scroll_top, self.index.total_height(), height);
        let window_rows = self.index.visible_range(self.scroll_top, height, OVERSCAN);
        let rows: Vec<AnyElement> = window_rows
            .filter_map(|position| self.row(position, cx))
            .collect();
        div()
            .flex()
            .flex_col()
            .flex_none()
            .w(px(LIST_WIDTH_PX))
            .h_full()
            .bg(hsla(colors.background.paper))
            .border_r_1()
            .border_color(hsla(colors.divider))
            .text_color(hsla(colors.text.primary))
            .child(
                div()
                    .flex()
                    .items_center()
                    .flex_none()
                    .h(px(HEADER_HEIGHT))
                    .px(px(16.0))
                    .text_size(px(20.0))
                    .child("Chats"),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .overflow_hidden()
                    .on_scroll(
                        cx.listener(|this: &mut Self, event: &ScrollEvent, window, cx| {
                            let total = this.index.total_height();
                            this.scroll_top =
                                scrolled(this.scroll_top, event, total, viewport(window));
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(-self.scroll_top))
                            .left(px(0.0))
                            .w_full()
                            .h(px(self.index.total_height()))
                            .children(rows),
                    ),
            )
    }
}
