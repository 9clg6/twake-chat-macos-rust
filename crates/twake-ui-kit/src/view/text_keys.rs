//! The macOS keyboard of the text field: which key does what.

use twake_gpui::Modifiers;

use crate::text_buffer::{Delete, Move};

/// What a key press asks of the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Command {
    Move(Move, bool),
    Delete(Delete),
    Newline,
    Submit,
    SelectAll,
    Copy,
    Cut,
    Paste,
    Undo,
    Redo,
}

fn arrow(key: &str, m: &Modifiers) -> Option<Move> {
    let (plain, word, line) = match key {
        "left" => (Move::Left, Move::WordLeft, Move::LineStart),
        "right" => (Move::Right, Move::WordRight, Move::LineEnd),
        "up" => (Move::Up, Move::Up, Move::DocStart),
        "down" => (Move::Down, Move::Down, Move::DocEnd),
        "home" => (Move::DocStart, Move::DocStart, Move::DocStart),
        "end" => (Move::DocEnd, Move::DocEnd, Move::DocEnd),
        _ => return None,
    };
    Some(if m.platform {
        line
    } else if m.alt {
        word
    } else {
        plain
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

fn editing(key: &str, m: &Modifiers) -> Option<Command> {
    match key {
        "enter" if m.shift || m.alt => Some(Command::Newline),
        "enter" => Some(Command::Submit),
        "backspace" if m.platform => Some(Command::Delete(Delete::ToLineStart)),
        "backspace" if m.alt => Some(Command::Delete(Delete::WordBackward)),
        "backspace" => Some(Command::Delete(Delete::Backward)),
        "delete" if m.alt => Some(Command::Delete(Delete::WordForward)),
        "delete" => Some(Command::Delete(Delete::Forward)),
        _ => None,
    }
}

/// The command of a key press, `None` for a key the system input method reads (letters, dead keys).
pub(super) fn command(key: &str, m: &Modifiers) -> Option<Command> {
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
