//! `--preview`: the room list and a conversation on fixtures, no account and no server.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use twake_app::composer::ComposerState;
use twake_app::room_list::RoomListState;
use twake_app::timeline::TimelineState;
use twake_domain::room_list::{RoomKind, RoomListDiff, RoomSummary};
use twake_domain::timeline::{Message, TimelineDiff};
use twake_domain::{RoomId, Timestamp, TxnId};
use twake_gpui::{
    AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div,
};
use twake_i18n::Locale;
use twake_ui::timeline_rows::{TimelineContext, timeline_rows};
use twake_ui::view::{RoomListView, RoomSelected, TimelineView};
use twake_ui_kit::theme::Theme;
use twake_ui_kit::view::{Submit, TextInput, hsla};

use crate::preview_data::{PreviewData, preview_data};

/// How many messages before the end the "New messages" marker stands.
const UNREAD_TAIL: usize = 20;
/// The others have read everything but this many of the newest messages.
const UNREAD_BY_OTHERS: usize = 5;

/// The two columns.
pub struct Preview {
    list: Entity<RoomListView>,
    timeline: Entity<TimelineView>,
    data: PreviewData,
    rooms: HashMap<RoomId, RoomSummary>,
    locale: Arc<Locale>,
    now: Timestamp,
    theme: Theme,
    composer: Entity<TextInput>,
    composer_state: ComposerState,
    open: Option<RoomId>,
    state: TimelineState,
    sent: u32,
}

fn now() -> Timestamp {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    Timestamp::from_millis(u64::try_from(elapsed.as_millis()).unwrap_or(0))
}

impl Preview {
    /// Builds the fixtures, the room list state and the first conversation (the 10 000 messages).
    pub fn new(dark: bool, bench: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let now = now();
        let theme = Theme::system(dark);
        let data = preview_data(now);
        let lang = std::env::var("LANG").unwrap_or_default();
        let locale = Arc::new(Locale::bundled(&lang, 0));
        let mut state = RoomListState::new();
        state.apply(vec![RoomListDiff::Reset(data.rooms.clone())]);
        let list = cx.new(|_| RoomListView::new(&state, now, &locale, theme));
        let timeline = cx.new(|_| TimelineView::new(theme));
        let composer = cx.new(|cx| TextInput::new(theme, "Write a message", window, cx));
        cx.subscribe(&composer, |this, _, _: &Submit, cx| this.send(cx))
            .detach();
        cx.observe(&composer, |this, _, cx| this.fit_composer(cx))
            .detach();
        cx.subscribe(&list, |this, _, event: &RoomSelected, cx| {
            this.open(&event.0, cx);
        })
        .detach();
        let composer_state = ComposerState::new(data.big_room.clone(), data.me.clone());
        let rooms = data
            .rooms
            .iter()
            .map(|r| (r.id.clone(), r.clone()))
            .collect();
        let preview = Self {
            list,
            timeline,
            data,
            rooms,
            locale,
            now,
            theme,
            composer,
            composer_state,
            open: None,
            state: TimelineState::new(0),
            sent: 0,
        };
        let big = preview.data.big_room.clone();
        preview.list.update(cx, |list, cx| list.select(big, cx));
        if bench {
            preview
                .timeline
                .update(cx, |timeline, cx| timeline.start_bench(cx));
        }
        preview
    }

    fn open(&mut self, id: &RoomId, cx: &mut Context<Self>) {
        let (Some(room), Some(history)) = (self.rooms.get(id), self.data.histories.get(id)) else {
            return;
        };
        let mut state = TimelineState::new(self.locale.utc_offset_secs());
        state.apply(TimelineDiff::Reset(history.clone()));
        if let Some(event) = history
            .len()
            .checked_sub(UNREAD_TAIL)
            .and_then(|n| history.get(n))
            .and_then(|m| m.event_id())
        {
            state.mark_new_messages_from(event.clone());
        }
        self.composer_state = ComposerState::new(room.id.clone(), self.data.me.clone());
        self.composer.update(cx, |input, cx| input.clear(cx));
        self.state = state;
        self.open = Some(id.clone());
        self.show(cx);
    }

    /// Draws the open conversation from its state.
    fn show(&mut self, cx: &mut Context<Self>) {
        let (Some(room), Some(history)) = (
            self.open.as_ref().and_then(|id| self.rooms.get(id)),
            self.open
                .as_ref()
                .and_then(|id| self.data.histories.get(id)),
        ) else {
            return;
        };
        let read_up_to = history
            .get(history.len().saturating_sub(UNREAD_BY_OTHERS))
            .and_then(|m| m.event_id().cloned());
        let ctx = TimelineContext {
            me: &self.data.me,
            is_direct: room.kind == RoomKind::Direct,
            names: &self.data.names,
            read_by_others_up_to: read_up_to.as_ref(),
        };
        let rows = timeline_rows(&self.state.items(), &ctx, self.now, &self.locale);
        let title = room.name.clone();
        self.timeline.update(cx, |timeline, cx| {
            timeline.set_conversation(title, rows, cx)
        });
        self.fit_composer(cx);
    }

    /// Enter in the composer: the message goes to the thread as a local echo.
    fn send(&mut self, cx: &mut Context<Self>) {
        let text = self.composer.read(cx).text().to_string();
        self.composer_state.set_text(&text, text.len());
        let Ok(outgoing) = self.composer_state.submit() else {
            return;
        };
        self.sent += 1;
        if let Ok(txn) = TxnId::parse(&format!("preview-{}", self.sent)) {
            let echo = Message::local(txn, self.data.me.clone(), now(), outgoing.body);
            self.state.apply(TimelineDiff::Append(vec![echo]));
        }
        self.composer.update(cx, |input, cx| input.clear(cx));
        self.show(cx);
    }

    /// The conversation leaves room for the composer, which grows with its text.
    fn fit_composer(&mut self, cx: &mut Context<Self>) {
        let height = self.composer.read(cx).outer_height();
        let footer = self.composer.clone().into();
        self.timeline
            .update(cx, |timeline, cx| timeline.set_footer(footer, height, cx));
    }
}

impl Render for Preview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .size_full()
            .bg(hsla(self.theme.colors().background.default))
            .child(self.list.clone())
            .child(self.timeline.clone())
    }
}
