//! The macOS keyboard of the text field: which key does what.

use twake_gpui::Modifiers;

use crate::text_buffer::{Delete, Move, TextBuffer};

/// What a key press asks of the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Move the caret; `true` extends the selection.
    Move(Move, bool),
    /// Delete around the caret or the selection.
    Delete(Delete),
    /// Insert a line break.
    Newline,
    /// Send the text.
    Submit,
    /// Select the whole text.
    SelectAll,
    /// Copy the selection to the clipboard.
    Copy,
    /// Copy the selection, then delete it.
    Cut,
    /// Insert the clipboard text.
    Paste,
    /// Undo the last edit.
    Undo,
    /// Redo the last undone edit.
    Redo,
}

/// Plain, Alt (word) and Cmd (line) targets of each navigation key.
const ARROWS: [(&str, [Move; 3]); 6] = [
    ("left", [Move::Left, Move::WordLeft, Move::LineStart]),
    ("right", [Move::Right, Move::WordRight, Move::LineEnd]),
    ("up", [Move::Up, Move::Up, Move::DocStart]),
    ("down", [Move::Down, Move::Down, Move::DocEnd]),
    ("home", [Move::DocStart; 3]),
    ("end", [Move::DocEnd; 3]),
];

fn arrow(key: &str, m: &Modifiers) -> Option<Move> {
    let (_, [plain, word, line]) = ARROWS.iter().find(|(name, _)| *name == key)?;
    Some(if m.platform {
        *line
    } else if m.alt {
        *word
    } else {
        *plain
    })
}

fn shortcut(key: &str, m: &Modifiers) -> Option<Command> {
    match key {
        "a" => Some(Command::SelectAll),
        "c" => Some(Command::Copy),
        "x" => Some(Command::Cut),
        "v" => Some(Command::Paste),
        "z" if m.shift => Some(Command::Redo),
        "z" => Some(Command::Undo),
        _ => None,
    }
}

fn enter(m: &Modifiers) -> Command {
    if m.shift || m.alt {
        Command::Newline
    } else {
        Command::Submit
    }
}

fn backspace(m: &Modifiers) -> Delete {
    if m.platform {
        Delete::ToLineStart
    } else if m.alt {
        Delete::WordBackward
    } else {
        Delete::Backward
    }
}

fn forward_delete(m: &Modifiers) -> Delete {
    if m.alt {
        Delete::WordForward
    } else {
        Delete::Forward
    }
}

fn editing(key: &str, m: &Modifiers) -> Option<Command> {
    match key {
        "enter" => Some(enter(m)),
        "backspace" => Some(Command::Delete(backspace(m))),
        "delete" => Some(Command::Delete(forward_delete(m))),
        _ => None,
    }
}

/// The command of a key press, `None` for a key the system input method reads (letters, dead keys).
pub fn command(key: &str, m: &Modifiers) -> Option<Command> {
    if m.control {
        return None;
    }
    if m.platform
        && let Some(command) = shortcut(key, m)
    {
        return Some(command);
    }
    arrow(key, m)
        .map(|to| Command::Move(to, m.shift))
        .or_else(|| editing(key, m))
}

/// Applies a command that needs only the text: the others (caret rows, clipboard, submit) need the screen.
pub fn edit(buffer: &mut TextBuffer, command: Command) {
    match command {
        Command::Delete(kind) => buffer.delete(kind),
        Command::Newline => buffer.insert("\n"),
        Command::SelectAll => buffer.select_all(),
        Command::Undo => {
            buffer.undo();
        }
        Command::Redo => {
            buffer.redo();
        }
        _ => {}
    }
}
