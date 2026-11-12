//! `ComposerState`: the text, the cursor, the banner, the picked mentions and the layers on top.

use twake_domain::commands::prepare;
use twake_domain::mention::Mention;
use twake_domain::{RoomId, UserId};

use super::outgoing::{OutgoingMessage, SubmitError, build, escape_names};
use super::suggest::{Sources, Suggestion, suggestions, written};
use super::trigger::{Trigger, find_trigger};
use super::{Access, ComposerMode, Draft, EditTarget, PrimaryAction, ReplyTarget, Selection};

/// What Escape did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeOutcome {
    /// The suggestions are closed; the text and the banner are as they were.
    ClosedSuggestions,
    /// The reply or the edit is cancelled.
    CancelledMode,
    /// Nothing was open.
    Nothing,
}

/// The compose text kept aside while a message is edited.
#[derive(Debug, Clone)]
struct Aside {
    text: String,
    picked: Vec<Mention>,
}

/// The composer of one room.
#[derive(Debug)]
pub struct ComposerState {
    me: UserId,
    text: String,
    selection: Selection,
    picked: Vec<Mention>,
    mode: ComposerMode,
    aside: Option<Aside>,
    access: Access,
    sources: Sources,
    dismissed: Option<Trigger>,
}

impl ComposerState {
    /// An empty composer: cursor 0, no banner, enabled, no sources. It does not hold the room: the
    /// screen pairs it with the room whose draft it keeps.
    pub fn new(_room: RoomId, me: UserId) -> Self {
        Self {
            me,
            text: String::new(),
            selection: Selection { anchor: 0, head: 0 },
            picked: Vec::new(),
            mode: ComposerMode::Compose,
            aside: None,
            access: Access::Enabled,
            sources: Sources::default(),
            dismissed: None,
        }
    }

    /// What is typed.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The selection; `head` is the cursor.
    pub fn selection(&self) -> Selection {
        self.selection
    }

    /// The banner.
    pub fn mode(&self) -> &ComposerMode {
        &self.mode
    }

    /// Whether the user can write.
    pub fn access(&self) -> Access {
        self.access
    }

    /// The microphone while the field holds nothing but spaces (FR-050), else Send.
    pub fn action(&self) -> PrimaryAction {
        if self.text.trim().is_empty() {
            PrimaryAction::Mic
        } else {
            PrimaryAction::Send
        }
    }

    fn enabled(&self) -> bool {
        self.access == Access::Enabled
    }

    fn at_end(&self) -> Selection {
        let end = self.text.len();
        Selection {
            anchor: end,
            head: end,
        }
    }

    /// A change of the text or of the cursor: a list closed by Escape opens again once the
    /// trigger is another one.
    fn changed(&mut self) {
        if self.dismissed != find_trigger(&self.text, self.selection.head) {
            self.dismissed = None;
        }
    }

    /// The user typed: the text and the cursor (selection collapsed) are replaced.
    pub fn set_text(&mut self, text: &str, cursor: usize) {
        if !self.enabled() {
            return;
        }
        self.text = text.to_string();
        if text.is_empty() {
            self.picked.clear();
        }
        let at = self.text.floor_char_boundary(cursor);
        self.selection = Selection {
            anchor: at,
            head: at,
        };
        self.changed();
    }

    /// Moves the selection, clamped to the text.
    pub fn select(&mut self, anchor: usize, head: usize) {
        self.selection = Selection {
            anchor: self.text.floor_char_boundary(anchor),
            head: self.text.floor_char_boundary(head),
        };
        self.changed();
    }

    /// Replaces the selection by `s`, the cursor after it.
    pub fn insert(&mut self, s: &str) {
        if !self.enabled() {
            return;
        }
        let from = self.selection.anchor.min(self.selection.head);
        let to = self.selection.anchor.max(self.selection.head);
        self.text.replace_range(from..to, s);
        let at = from + s.len();
        self.selection = Selection {
            anchor: at,
            head: at,
        };
        self.changed();
    }

    /// A disabled composer ignores `set_text`, `insert`, `accept` and the banners.
    pub fn set_access(&mut self, access: Access) {
        self.access = access;
    }

    /// What the suggestions draw from.
    pub fn set_sources(&mut self, sources: Sources) {
        self.sources = sources;
    }

    /// Answers a message; replaces a previous reply, keeps the text.
    pub fn start_reply(&mut self, to: ReplyTarget) {
        if !self.enabled() {
            return;
        }
        self.leave_edit();
        self.mode = ComposerMode::Reply(to);
    }

    /// Loads a message to edit, the cursor at its end; the text typed before is kept aside.
    pub fn start_edit(&mut self, target: EditTarget) {
        if !self.enabled() {
            return;
        }
        if self.aside.is_none() {
            self.aside = Some(Aside {
                text: std::mem::take(&mut self.text),
                picked: std::mem::take(&mut self.picked),
            });
        }
        self.text = escape_names(&target.body, &target.mentions);
        self.picked = target.mentions.clone();
        self.selection = self.at_end();
        self.mode = ComposerMode::Edit(target);
        self.changed();
    }

    /// Gives back the text typed before an edit, if one is on.
    fn leave_edit(&mut self) {
        if let Some(aside) = self.aside.take() {
            self.text = aside.text;
            self.picked = aside.picked;
            self.selection = self.at_end();
            self.changed();
        }
    }

    /// A reply: back to compose, the text kept. An edit: back to compose, the text typed before
    /// comes back.
    pub fn cancel_mode(&mut self) {
        self.leave_edit();
        self.mode = ComposerMode::Compose;
    }

    /// The draft to keep: the compose text, never the edited one.
    pub fn draft(&self) -> Draft {
        let (text, mentions) = match &self.aside {
            Some(aside) => (&aside.text, &aside.picked),
            None => (&self.text, &self.picked),
        };
        Draft {
            text: text.clone(),
            mentions: mentions.clone(),
        }
    }

    /// Puts a draft back, the cursor at its end.
    pub fn restore(&mut self, draft: Draft) {
        self.text = draft.text;
        self.picked = draft.mentions;
        self.selection = self.at_end();
        self.changed();
    }

    /// The layer asked by the text before the cursor; `None` while Escape holds it closed.
    pub fn trigger(&self) -> Option<Trigger> {
        find_trigger(&self.text, self.selection.head).filter(|t| self.dismissed.as_ref() != Some(t))
    }

    /// The lines of the layer; empty without a trigger, while closed, or with no match.
    pub fn suggestions(&self) -> Vec<Suggestion> {
        self.trigger()
            .map(|t| suggestions(&self.sources, &self.me, &t))
            .unwrap_or_default()
    }

    /// Writes the suggestion in place of the trigger, a space after it, and remembers the
    /// mention. Nothing happens without a trigger.
    pub fn accept(&mut self, suggestion: Suggestion) {
        let Some(trigger) = self.trigger().filter(|_| self.enabled()) else {
            return;
        };
        let (words, mention) = written(&suggestion);
        let head = self.selection.head;
        self.text.replace_range(trigger.start..head, &words);
        let at = trigger.start + words.len();
        self.selection = Selection {
            anchor: at,
            head: at,
        };
        if let Some(mention) = mention.filter(|m| !self.picked.contains(m)) {
            self.picked.push(mention);
        }
        self.changed();
    }

    /// Closes the open suggestions, else cancels the reply or the edit (D621).
    pub fn escape(&mut self) -> EscapeOutcome {
        if !self.suggestions().is_empty() {
            self.dismissed = self.trigger();
            EscapeOutcome::ClosedSuggestions
        } else if self.mode != ComposerMode::Compose {
            self.cancel_mode();
            EscapeOutcome::CancelledMode
        } else {
            EscapeOutcome::Nothing
        }
    }

    /// The field emptied, the banner gone, the text kept aside by an edit back.
    fn finish(&mut self) {
        self.text.clear();
        self.picked.clear();
        self.selection = self.at_end();
        self.dismissed = None;
        self.cancel_mode();
    }

    /// Enter: the message to send, or why none. The field is emptied after a message and after a
    /// command of the device; an unknown command, an empty text and a disabled composer change
    /// nothing.
    pub fn submit(&mut self) -> Result<OutgoingMessage, SubmitError> {
        if let Access::Disabled(reason) = self.access {
            return Err(SubmitError::Disabled(reason));
        }
        let text = self.text.trim();
        if text.is_empty() {
            return Err(SubmitError::Empty);
        }
        let prepared = prepare(text, &self.sources.commands);
        let result = build(&self.mode, &self.picked, &self.me, prepared);
        if matches!(result, Ok(_) | Err(SubmitError::LocalCommand(_))) {
            self.finish();
        }
        result
    }
}
