# Text input model sketch: a pure multi-line text buffer, no GPUI

Written by the test author from the need, before the production code exists. This is the exact
public surface the tests use. The implementer may add more, but every item here must exist with
these names, types and semantics. A disagreement is argued against the need (the cited source),
not against the code.

Need: the composer field of the macOS app (`docs/spec/features/composer.md`: D181 one pill field,
D264 Enter sends and Shift+Enter makes a newline, D332 paste is plain text only, D602 on US
International the dead-key backtick commits with the next character), the accessibility rules
(`docs/spec/features/accessibility.md` B3 keyboard only, B16 no keyboard trap), and ARCHITECTURE.md
(section 3: `twake-ui-kit` holds the `TextInput`; section 8: nothing allocated per frame on a hot
path, undo must not copy a long text per key; P4/P5: pure model, state in an entity). Not a
user-visible spec by itself: GPUI will draw it and forward `NSTextInputClient` calls to it.

Tests: `crates/twake-ui-kit/tests/text_buffer_{edit,move,select,ime,history,utf16,props}.rs`,
helpers in `tests/tb/mod.rs`.

## Where it lives: `twake_ui_kit::text_buffer` (decision, to confirm)

Not `twake-domain`. The domain holds chat rules (ids, read state, Markdown to Matrix HTML); its
allow list (ARCHITECTURE section 3) has no `unicode-segmentation`, and a text field's cursor,
word movement, UTF-16 conversion and IME marking are input-device concerns that no other layer
reads. `ComposerState` (twake-app) keeps its own `text` + `Selection` (byte offsets on a char
boundary, `set_text(text, cursor)`); the field copies `text()` and `cursor()` into it after each
edit, so the use-case layer never learns graphemes or IME. Not `twake-app` either (no UI concept
there). `twake-ui-kit` is the crate the section 3 table names for `TextInput`; the module is
GPUI-free (like `theme` and `color`), so its tests run without a window. Cost: `cargo test -p
twake-ui-kit` still links GPUI (already the case for the other kit tests). When a second app needs
it, it moves with the kit (second-app rule).

Dependency: `unicode-segmentation` (pure, MIT/Apache, already in `Cargo.lock` through GPUI,
version 1.13.3). One line in `docs/decisions.md` (P8). `proptest` as dev-dependency.

## Conventions

- Offsets are **byte offsets into `text()`**. Cursor and selection ends are always on **extended
  grapheme cluster** boundaries (UAX 29) of the whole text, whatever was done, except while a
  composition is open (below). An offset that is past the end is clamped to the end; one inside a
  cluster is rounded **down** to its start (`select`, `select_word_at`, `select_line_at`).
- After an edit (insert, paste, delete, commit), a cursor that lands inside a cluster (typed text, or
  a deletion, that glues a letter to a following combining mark) is moved **forward** to the end of that cluster.
- Word rules and grapheme rules can disagree (a flag split by an accent). A word boundary that
  falls inside a cluster is snapped to a cluster edge (which one is not specified); the result of
  a word move, a word deletion or a double click is always on grapheme boundaries.
- "Character" in `max_len` is a Unicode scalar value (`str::chars`), not a cluster and not a byte.
- Line = text between `'\n'`. There is no soft wrap in the model: the view maps visual lines.
- `'\r'` never enters the buffer (see Sanitising), so a `\r\n` cluster cannot exist: a line break
  is one `'\n'`, one byte, one cursor step.
- Programming errors do not panic: every offset argument is clamped, every range is clamped.

## Sanitising (D332 paste is plain; control characters refused)

Applied to every text that enters (`insert`, `paste`, `set_text`, `set_marked_text`):

1. `"\r\n"` becomes `"\n"`, then a lone `'\r'` becomes `'\n'`.
2. Every other control character (`char::is_control`: NUL, BEL, ESC, DEL, C1...) is dropped,
   except `'\n'` and `'\t'`.
3. Format characters are kept: the zero width joiner (U+200D) and variation selectors make emoji.
4. If what remains is empty, **nothing happens** (the selection is not deleted, no undo step).
   (`insert("")` is therefore a no-op; deleting is `delete`.)
5. Then `max_len` cuts the text (see `with_max_len`).

## API

```rust
/// Anchor is where the selection started, head is the cursor (it can be before the anchor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection { pub anchor: usize, pub head: usize }
impl Selection {
    pub fn range(&self) -> std::ops::Range<usize>;   // min..max
    pub fn is_collapsed(&self) -> bool;              // anchor == head
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    Left, Right,            // one grapheme
    WordLeft, WordRight,    // Alt+Left / Alt+Right
    LineStart, LineEnd,     // Cmd+Left / Cmd+Right (the logical line)
    Up, Down,               // one line, same column
    DocStart, DocEnd,       // Cmd+Up / Cmd+Down
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delete {
    Backward, Forward,          // Backspace / Fn+Backspace, one grapheme
    WordBackward, WordForward,  // Alt+Backspace / Alt+Fn+Backspace
    ToLineStart,                // Cmd+Backspace
}

#[derive(Debug, Default)]
pub struct TextBuffer { /* private */ }

impl TextBuffer {
    pub fn new() -> Self;                         // empty, no limit
    pub fn with_max_len(max_chars: usize) -> Self;

    // --- read
    pub fn text(&self) -> &str;
    pub fn selection(&self) -> Selection;        // collapsed = a cursor
    pub fn selected_text(&self) -> &str;         // "" when collapsed (copy, cut)
    pub fn marked_range(&self) -> Option<std::ops::Range<usize>>;  // the composition, if any
    pub fn can_undo(&self) -> bool;
    pub fn can_redo(&self) -> bool;

    // --- replace the whole content (load a draft, clear after send)
    /// Sanitised, cut to max_len, cursor at the end, history and composition dropped.
    pub fn set_text(&mut self, text: &str);

    // --- type, paste
    /// Typed text: replaces the selection (or the composition, see below). Consecutive typing
    /// is one undo step (History).
    pub fn insert(&mut self, text: &str);
    /// Pasted text: same as `insert`, but always its own undo step and it closes the typing run.
    pub fn paste(&mut self, text: &str);

    // --- delete
    /// A non-empty selection is removed whatever the kind. Else: the grapheme / word / line part
    /// before or after the cursor. At an edge: nothing, no undo step. Each call is one undo step.
    pub fn delete(&mut self, kind: Delete);

    // --- move and select
    /// `extend` = Shift held: the anchor stays, the head moves. Else the selection collapses on the head.
    /// Left / Right with a non-empty selection and no `extend` collapse to its start / end
    /// without moving further (macOS). Other moves start from the head.
    pub fn move_by(&mut self, to: Move, extend: bool);
    pub fn select(&mut self, anchor: usize, head: usize);          // click, drag; clamped and snapped
    pub fn select_all(&mut self);                                  // Cmd+A
    pub fn select_word_at(&mut self, offset: usize);               // double click
    pub fn select_line_at(&mut self, offset: usize);               // triple click

    // --- history
    pub fn undo(&mut self) -> bool;               // false: nothing to undo (or composing)
    pub fn redo(&mut self) -> bool;

    // --- IME (NSTextInputClient: setMarkedText, insertText, unmarkText)
    pub fn set_marked_text(&mut self, replace: Option<std::ops::Range<usize>>, text: &str);
    pub fn commit_marked(&mut self);              // unmarkText: the marked text stays as plain text
    pub fn cancel_marked(&mut self);              // the composition is dropped

    // --- UTF-16 <-> UTF-8 (NSRange / NSTextInputClient speak UTF-16 code units)
    pub fn utf8_to_utf16(&self, offset: usize) -> usize;
    pub fn utf16_to_utf8(&self, offset: usize) -> usize;
    pub fn utf8_range_to_utf16(&self, r: std::ops::Range<usize>) -> std::ops::Range<usize>;
    pub fn utf16_range_to_utf8(&self, r: std::ops::Range<usize>) -> std::ops::Range<usize>;
}
```

Enter (send) and Shift+Enter are not here: the field decides; Shift+Enter is `insert("\n")`.
Copy is `selected_text()`, cut is that then `delete(Delete::Backward)`.

## Rules in detail

### Words (UAX 29 word boundaries)
A word is a UAX 29 word segment that contains a letter or a digit (`"don't"`, `"foo_bar"`,
`"cafe\u{301}"` are one word each); every other segment (spaces, newlines, punctuation) is a
separator. `WordLeft` skips separators back, then the word. `WordRight` skips separators forward,
then the word (lands at its end). Lines do not stop it. At the edge: stays. Same for
`WordBackward` / `WordForward` deletions. Emoji as words: not specified (open question).

### Lines and columns
`LineStart` / `LineEnd` stop at the logical line (`'\n'`), never at the document edge when there
is a line break next to the cursor. `Up` / `Down` keep a goal column counted in **graphemes**
from the line start (a model with no font cannot do better); the goal is set by the first
vertical move and kept by the following ones, and every other operation forgets it. A shorter
line puts the cursor at its end; the goal comes back on a longer one. `Up` on the first line goes
to the start of the text, `Down` on the last to its end (macOS). `ToLineStart` at the start of a
line removes the line break before it (so the key never does nothing mid-document).

### Double and triple click
`select_word_at(o)`: the whole UAX 29 segment that contains `o` (a word, a run of spaces, a
punctuation mark), anchor at its start; at the end of the text, the last segment; empty text: `(0,0)`.
`select_line_at(o)`: the line, **with its trailing `'\n'` if any** (as `NSTextView` selects a
paragraph); `o` on the line break belongs to the line it ends.

### History
- A **typing run**: an `insert` extends the open run when the selection is empty, a run is open and the cursor
  is where the run ended; otherwise it opens a new step (and a run). An `insert` that replaces a non-empty selection is its own
  step, and typing continues in it.
- Everything else closes the run: `paste`, `delete`, `move_by`, `select*`, `undo`, `redo`, `set_text`.
- Every `delete` and every `paste` is one step. Moves and selections are not steps.
- `undo` restores text **and selection** as before the step; `redo` the selection after it.
- A new edit empties the redo stack. `set_text` empties both.
- Composition is not in the history while open; its commit (by `insert` or `commit_marked`) is
  **one step of its own** from before the composition to after, and closes the typing run (a
  committed phrase is undone as a unit, never merged with the typing before it). An empty commit
  is none. `undo` and `redo` return `false` while a composition is open.
- Implementation hint (not tested): store edits (range, removed, inserted), not snapshots, so a long
  text costs nothing per key.

### Composition (IME)
- `set_marked_text(replace, text)`: `replace` is a byte range (clamped, floored to a char boundary);
  `None` means the current composition if there is one, else the current selection. The sanitised
  and cut text replaces it and becomes the marked range; the cursor goes to its end.
  An empty text ends the composition (the IME deleted its last character): text replaced, no mark.
- `insert` while composing replaces the marked range (this is `insertText:` with a final string:
  dead key `´` then `e` gives `é`, `´` then space gives `´`), unmarks, and is the end of the composition.
- `commit_marked`: the mark is dropped, text and cursor stay, the cursor and selection are then snapped to
  grapheme boundaries (forward). `select*`, `move_by`, `delete`, `paste` and `set_text` while composing
  commit first (`set_text` just drops it).
- `cancel_marked`: text and selection return to what they were **before the composition started**
  (a selection the composition replaced comes back).
- While a composition is open the boundary rule is suspended: the marked text is the IME's.

### UTF-16
Offsets past the end clamp to the end. A UTF-8 offset inside a char, or a UTF-16 offset in the
middle of a surrogate pair, rounds down to the start of that char. No grapheme snapping here (the
OS may ask for any range); `select` snaps.

## Open questions (none blocks the tests)

1. `max_len` counts scalar values. A user-perceived character count (clusters) is the other
   reading of "caracteres". Changing it is a one-line change in the sanitiser; tests pin scalars.
2. `set_marked_text` ignores the `selectedRange` argument of macOS: the cursor goes to the end of
   the marked text. A Japanese IME that moves the caret inside the clause needs a third parameter.
3. Emoji and other symbols as "words" for word movement and word deletion: unspecified, untested.
4. Triple click includes the trailing newline (see above); the other choice is to exclude it.
5. Are `Alt+Delete` forward and `Ctrl+K` / `Ctrl+A` / `Ctrl+E` (Emacs bindings of macOS text fields)
   wanted? Only the first is covered (`WordForward`); `LineStart` / `LineEnd` already give Ctrl+A / E.
