//! The model of a multi-line text field: text, cursor and selection on grapheme clusters, word
//! and line moves, undo, IME composition and UTF-16 offsets. No GPUI, no window: the view draws
//! it and forwards `NSTextInputClient` calls to it (`docs/dev/text-input-api-sketch.md`).

mod text;

use std::ops::Range;
use text::{
    ceil_boundary, column, floor_boundary, from_utf16, line_at, line_down, line_end, line_start,
    line_up, next_boundary, prev_boundary, sanitize, take_chars, to_utf16, word_at, word_left,
    word_right,
};

/// Anchor is where the selection started, head is the cursor (it can be before the anchor).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    /// Where the selection started.
    pub anchor: usize,
    /// The cursor.
    pub head: usize,
}

impl Selection {
    fn at(offset: usize) -> Self {
        Self {
            anchor: offset,
            head: offset,
        }
    }

    /// The covered bytes, `min..max`.
    pub fn range(&self) -> Range<usize> {
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }

    /// `true` for a cursor.
    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.head
    }
}

/// A cursor move (Shift extends the selection).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    /// One cluster left.
    Left,
    /// One cluster right.
    Right,
    /// Alt+Left.
    WordLeft,
    /// Alt+Right.
    WordRight,
    /// Cmd+Left (the logical line).
    LineStart,
    /// Cmd+Right (the logical line).
    LineEnd,
    /// One line up, same column.
    Up,
    /// One line down, same column.
    Down,
    /// Cmd+Up.
    DocStart,
    /// Cmd+Down.
    DocEnd,
}

/// A deletion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delete {
    /// Backspace: one cluster.
    Backward,
    /// Fn+Backspace: one cluster.
    Forward,
    /// Alt+Backspace.
    WordBackward,
    /// Alt+Fn+Backspace.
    WordForward,
    /// Cmd+Backspace.
    ToLineStart,
}

/// One undo step: `removed` was replaced by `inserted` at byte `at`.
#[derive(Debug)]
struct Edit {
    at: usize,
    removed: String,
    inserted: String,
    before: Selection,
    after: Selection,
}

/// An open IME composition: the marked bytes, and what to put back on cancel.
#[derive(Debug)]
struct Composition {
    range: Range<usize>,
    base: Selection,
    removed: String,
}

/// The text of a field with its selection, history and composition.
#[derive(Debug, Default)]
pub struct TextBuffer {
    text: String,
    sel: Selection,
    max: Option<usize>,
    goal: Option<usize>,
    marked: Option<Composition>,
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    typing: bool,
}

impl TextBuffer {
    /// An empty buffer with no limit.
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty buffer that holds at most `max_chars` Unicode scalar values.
    pub fn with_max_len(max_chars: usize) -> Self {
        Self {
            max: Some(max_chars),
            ..Self::default()
        }
    }

    /// The whole text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The cursor or selection.
    pub fn selection(&self) -> Selection {
        self.sel
    }

    /// What the selection covers; empty for a cursor.
    pub fn selected_text(&self) -> &str {
        &self.text[self.sel.range()]
    }

    /// The composition, if any.
    pub fn marked_range(&self) -> Option<Range<usize>> {
        self.marked.as_ref().map(|c| c.range.clone())
    }

    /// `true` when `undo` would do something.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// `true` when `redo` would do something.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Replaces the whole content: cleaned, cut, cursor at the end, history and composition dropped.
    pub fn set_text(&mut self, text: &str) {
        let clean = sanitize(text);
        let kept = take_chars(&clean, self.max.unwrap_or(usize::MAX));
        *self = Self {
            sel: Selection::at(kept.len()),
            text: kept.to_string(),
            max: self.max,
            ..Self::default()
        };
    }

    /// Typed text: replaces the selection (or the composition); consecutive typing is one step.
    pub fn insert(&mut self, text: &str) {
        self.goal = None;
        self.put(text, true);
    }

    /// Pasted text: like `insert`, always its own undo step.
    pub fn paste(&mut self, text: &str) {
        self.settle();
        self.put(text, false);
    }

    /// Removes the selection, or the part before or after the cursor; one undo step.
    pub fn delete(&mut self, kind: Delete) {
        self.settle();
        let range = if self.sel.is_collapsed() {
            self.delete_range(kind)
        } else {
            self.sel.range()
        };
        if !range.is_empty() {
            self.edit(range, "", false);
        }
    }

    /// Moves the head; `extend` (Shift) keeps the anchor.
    pub fn move_by(&mut self, to: Move, extend: bool) {
        let goal = self.goal;
        self.settle();
        self.goal = goal;
        let head = match to {
            Move::Left | Move::Right => self.step(to == Move::Right, extend),
            Move::Up | Move::Down => self.vertical(to == Move::Down),
            _ => self.jump(to),
        };
        if !matches!(to, Move::Up | Move::Down) {
            self.goal = None;
        }
        let anchor = if extend { self.sel.anchor } else { head };
        self.sel = Selection { anchor, head };
    }

    /// Click or drag: clamped and snapped to cluster starts.
    pub fn select(&mut self, anchor: usize, head: usize) {
        self.settle();
        self.sel = Selection {
            anchor: floor_boundary(&self.text, anchor),
            head: floor_boundary(&self.text, head),
        };
    }

    /// Cmd+A.
    pub fn select_all(&mut self) {
        self.settle();
        self.sel = Selection {
            anchor: 0,
            head: self.text.len(),
        };
    }

    /// Double click.
    pub fn select_word_at(&mut self, offset: usize) {
        self.settle();
        let r = word_at(&self.text, offset);
        self.sel = Selection {
            anchor: r.start,
            head: r.end,
        };
    }

    /// Triple click: the line with its trailing `'\n'`.
    pub fn select_line_at(&mut self, offset: usize) {
        self.settle();
        let r = line_at(&self.text, offset);
        self.sel = Selection {
            anchor: r.start,
            head: r.end,
        };
    }

    /// Undoes one step; `false` when there is none or a composition is open.
    pub fn undo(&mut self) -> bool {
        self.step_history(true)
    }

    /// Redoes one step; `false` when there is none or a composition is open.
    pub fn redo(&mut self) -> bool {
        self.step_history(false)
    }

    /// `setMarkedText`: `replace` (default: the composition, else the selection) becomes the
    /// marked text, the cursor goes to its end. An empty text ends the composition.
    pub fn set_marked_text(&mut self, replace: Option<Range<usize>>, text: &str) {
        self.goal = None;
        let clean = sanitize(text);
        if clean.is_empty() && !(text.is_empty() && self.marked.is_some()) {
            return;
        }
        let range = self.marked_target(replace);
        let clean = self.cut(&clean, &range);
        if clean.is_empty() && self.marked.is_none() {
            return;
        }
        self.splice_marked(range, &clean);
    }

    /// `unmarkText`: the marked text stays, as one undo step.
    pub fn commit_marked(&mut self) {
        self.goal = None;
        self.end_composition();
    }

    /// The composition is dropped, text and selection come back.
    pub fn cancel_marked(&mut self) {
        self.goal = None;
        if let Some(c) = self.marked.take() {
            self.text.replace_range(c.range, &c.removed);
            self.sel = c.base;
        }
    }

    /// UTF-16 units before a byte offset.
    pub fn utf8_to_utf16(&self, offset: usize) -> usize {
        to_utf16(&self.text, offset)
    }

    /// The byte offset of a UTF-16 offset.
    pub fn utf16_to_utf8(&self, offset: usize) -> usize {
        from_utf16(&self.text, offset)
    }

    /// A byte range in UTF-16 units.
    pub fn utf8_range_to_utf16(&self, r: Range<usize>) -> Range<usize> {
        self.utf8_to_utf16(r.start)..self.utf8_to_utf16(r.end)
    }

    /// A UTF-16 range in bytes.
    pub fn utf16_range_to_utf8(&self, r: Range<usize>) -> Range<usize> {
        self.utf16_to_utf8(r.start)..self.utf16_to_utf8(r.end)
    }

    /// Ends the composition and closes the typing run, forgetting the goal column.
    fn settle(&mut self) {
        self.end_composition();
        self.goal = None;
        self.typing = false;
    }

    fn clamp(&self, r: Range<usize>) -> Range<usize> {
        let (a, b) = (
            self.text.floor_char_boundary(r.start),
            self.text.floor_char_boundary(r.end),
        );
        a.min(b)..a.max(b)
    }

    /// `text` cut so that it fits once `replaced` is gone.
    fn cut(&self, text: &str, replaced: &Range<usize>) -> String {
        let Some(max) = self.max else {
            return text.to_string();
        };
        let used = self.text.chars().count() - self.text[replaced.clone()].chars().count();
        take_chars(text, max.saturating_sub(used)).to_string()
    }

    fn put(&mut self, text: &str, typing: bool) {
        let clean = sanitize(text);
        let range = self.marked_range().unwrap_or_else(|| self.sel.range());
        let clean = self.cut(&clean, &range);
        if clean.is_empty() {
            return;
        }
        if self.marked.is_some() {
            self.splice_marked(range, &clean);
            self.end_composition();
        } else {
            self.edit(range, &clean, typing);
        }
    }

    /// Replaces `range` and records the step; consecutive typing extends the last one.
    fn edit(&mut self, range: Range<usize>, inserted: &str, typing: bool) {
        let merge = typing
            && self.typing
            && range.is_empty()
            && self
                .undo
                .last()
                .is_some_and(|e| e.at + e.inserted.len() == range.start);
        let end = range.start + inserted.len();
        if merge {
            if let Some(e) = self.undo.last_mut() {
                e.inserted.push_str(inserted);
            }
        } else {
            self.undo.push(Edit {
                at: range.start,
                removed: self.text[range.clone()].to_string(),
                inserted: inserted.to_string(),
                before: self.sel,
                after: self.sel,
            });
        }
        self.text.replace_range(range, inserted);
        self.redo.clear();
        self.sel = Selection::at(ceil_boundary(&self.text, end));
        if let Some(e) = self.undo.last_mut() {
            e.after = self.sel;
        }
        self.typing = typing;
    }

    fn step_history(&mut self, undo: bool) -> bool {
        if self.marked.is_some() {
            return false;
        }
        self.goal = None;
        self.typing = false;
        let Some(e) = (if undo {
            self.undo.pop()
        } else {
            self.redo.pop()
        }) else {
            return false;
        };
        let (gone, back, sel) = if undo {
            (&e.inserted, &e.removed, e.before)
        } else {
            (&e.removed, &e.inserted, e.after)
        };
        self.text.replace_range(e.at..e.at + gone.len(), back);
        self.sel = sel;
        if undo { &mut self.redo } else { &mut self.undo }.push(e);
        true
    }

    fn delete_range(&self, kind: Delete) -> Range<usize> {
        let (t, h) = (&self.text, self.sel.head);
        match kind {
            Delete::Backward => prev_boundary(t, h)..h,
            Delete::Forward => h..next_boundary(t, h),
            Delete::WordBackward => word_left(t, h)..h,
            Delete::WordForward => h..word_right(t, h),
            Delete::ToLineStart => match line_start(t, h) {
                start if start == h => h.saturating_sub(1)..h,
                start => start..h,
            },
        }
    }

    /// Left or Right; a selection without Shift collapses on its edge.
    fn step(&self, forward: bool, extend: bool) -> usize {
        let s = self.sel;
        match (forward, extend || s.is_collapsed()) {
            (false, false) => s.range().start,
            (true, false) => s.range().end,
            (false, true) => prev_boundary(&self.text, s.head),
            (true, true) => next_boundary(&self.text, s.head),
        }
    }

    fn vertical(&mut self, down: bool) -> usize {
        let head = self.sel.head;
        let col = *self.goal.get_or_insert_with(|| column(&self.text, head));
        if down {
            line_down(&self.text, head, col)
        } else {
            line_up(&self.text, head, col)
        }
    }

    fn jump(&self, to: Move) -> usize {
        let (t, h) = (&self.text, self.sel.head);
        match to {
            Move::WordLeft => word_left(t, h),
            Move::WordRight => word_right(t, h),
            Move::LineStart => line_start(t, h),
            Move::LineEnd => line_end(t, h),
            Move::DocStart => 0,
            _ => t.len(),
        }
    }

    /// Where the next `set_marked_text` writes.
    fn marked_target(&mut self, replace: Option<Range<usize>>) -> Range<usize> {
        let asked = replace.map(|r| self.clamp(r));
        match (self.marked_range(), asked) {
            (Some(m), None) => m,
            (Some(m), Some(r)) if m.start <= r.start && r.end <= m.end => r,
            (Some(_), Some(r)) => {
                self.end_composition();
                r
            }
            (None, Some(r)) => r,
            (None, None) => self.sel.range(),
        }
    }

    /// Writes `clean` over `range`, which becomes (part of) the marked text.
    fn splice_marked(&mut self, range: Range<usize>, clean: &str) {
        let (base, removed, start, end) = match self.marked.take() {
            Some(c) => {
                let end = c.range.end - range.len() + clean.len();
                (c.base, c.removed, c.range.start, end)
            }
            None => {
                self.typing = false;
                let removed = self.text[range.clone()].to_string();
                (self.sel, removed, range.start, range.start + clean.len())
            }
        };
        self.text.replace_range(range, clean);
        self.sel = Selection::at(end);
        self.marked = Some(Composition {
            range: start..end,
            base,
            removed,
        });
        if start == end {
            self.end_composition();
        }
    }

    /// Drops the mark, snaps the selection forward and records the composition as one step.
    fn end_composition(&mut self) {
        let Some(c) = self.marked.take() else {
            return;
        };
        let inserted = self.text[c.range.clone()].to_string();
        self.sel = Selection {
            anchor: ceil_boundary(&self.text, self.sel.anchor),
            head: ceil_boundary(&self.text, self.sel.head),
        };
        self.typing = false;
        if c.removed.is_empty() && inserted.is_empty() {
            return;
        }
        self.redo.clear();
        self.undo.push(Edit {
            at: c.range.start,
            removed: c.removed,
            inserted,
            before: c.base,
            after: self.sel,
        });
    }
}
