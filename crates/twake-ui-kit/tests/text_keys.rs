#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    missing_docs,
    dead_code
)]
//! Text field key map: what each macOS key press asks of the field.
//!
//! Need (not the code): `docs/dev/text-input-api-sketch.md` and the macOS text conventions.
//! Arrows: plain = cluster, Alt = word, Cmd = line (Up/Down: Cmd = document edge, Alt = Up/Down);
//! Shift extends. Enter submits, Shift or Alt+Enter is a newline. Ctrl and bare letters go to the
//! system input method.

use twake_gpui::Modifiers;
use twake_ui_kit::text_buffer::{Delete, Move};
use twake_ui_kit::view::text_keys::{Command, command};

const NONE: Modifiers = Modifiers {
    control: false,
    alt: false,
    shift: false,
    platform: false,
    function: false,
};

fn shift() -> Modifiers {
    Modifiers {
        shift: true,
        ..NONE
    }
}
fn alt() -> Modifiers {
    Modifiers { alt: true, ..NONE }
}
fn cmd() -> Modifiers {
    Modifiers {
        platform: true,
        ..NONE
    }
}
fn ctrl() -> Modifiers {
    Modifiers {
        control: true,
        ..NONE
    }
}
fn cmd_shift() -> Modifiers {
    Modifiers {
        platform: true,
        shift: true,
        ..NONE
    }
}
fn cmd_alt() -> Modifiers {
    Modifiers {
        platform: true,
        alt: true,
        ..NONE
    }
}
fn alt_shift() -> Modifiers {
    Modifiers {
        alt: true,
        shift: true,
        ..NONE
    }
}

#[test]
fn left_moves_one_cluster() {
    assert_eq!(
        command("left", &NONE),
        Some(Command::Move(Move::Left, false))
    );
}

#[test]
fn alt_left_moves_one_word() {
    assert_eq!(
        command("left", &alt()),
        Some(Command::Move(Move::WordLeft, false))
    );
}

#[test]
fn cmd_left_moves_to_the_line_edge() {
    assert_eq!(
        command("left", &cmd()),
        Some(Command::Move(Move::LineStart, false))
    );
}

#[test]
fn shift_left_extends_by_one_cluster() {
    assert_eq!(
        command("left", &shift()),
        Some(Command::Move(Move::Left, true))
    );
}

#[test]
fn alt_shift_left_extends_by_one_word() {
    assert_eq!(
        command("left", &alt_shift()),
        Some(Command::Move(Move::WordLeft, true))
    );
}

#[test]
fn cmd_shift_left_extends_to_the_line_edge() {
    assert_eq!(
        command("left", &cmd_shift()),
        Some(Command::Move(Move::LineStart, true))
    );
}

#[test]
fn right_moves_one_cluster() {
    assert_eq!(
        command("right", &NONE),
        Some(Command::Move(Move::Right, false))
    );
}

#[test]
fn alt_right_moves_one_word() {
    assert_eq!(
        command("right", &alt()),
        Some(Command::Move(Move::WordRight, false))
    );
}

#[test]
fn cmd_right_moves_to_the_line_edge() {
    assert_eq!(
        command("right", &cmd()),
        Some(Command::Move(Move::LineEnd, false))
    );
}

#[test]
fn shift_right_extends_by_one_cluster() {
    assert_eq!(
        command("right", &shift()),
        Some(Command::Move(Move::Right, true))
    );
}

#[test]
fn alt_shift_right_extends_by_one_word() {
    assert_eq!(
        command("right", &alt_shift()),
        Some(Command::Move(Move::WordRight, true))
    );
}

#[test]
fn cmd_shift_right_extends_to_the_line_edge() {
    assert_eq!(
        command("right", &cmd_shift()),
        Some(Command::Move(Move::LineEnd, true))
    );
}

#[test]
fn up_moves_one_line() {
    assert_eq!(command("up", &NONE), Some(Command::Move(Move::Up, false)));
}

#[test]
fn alt_up_moves_one_line() {
    assert_eq!(command("up", &alt()), Some(Command::Move(Move::Up, false)));
}

#[test]
fn cmd_up_goes_to_the_document_edge() {
    assert_eq!(
        command("up", &cmd()),
        Some(Command::Move(Move::DocStart, false))
    );
}

#[test]
fn shift_up_extends_by_one_line() {
    assert_eq!(command("up", &shift()), Some(Command::Move(Move::Up, true)));
}

#[test]
fn cmd_shift_up_extends_to_the_document_edge() {
    assert_eq!(
        command("up", &cmd_shift()),
        Some(Command::Move(Move::DocStart, true))
    );
}

#[test]
fn down_moves_one_line() {
    assert_eq!(
        command("down", &NONE),
        Some(Command::Move(Move::Down, false))
    );
}

#[test]
fn alt_down_moves_one_line() {
    assert_eq!(
        command("down", &alt()),
        Some(Command::Move(Move::Down, false))
    );
}

#[test]
fn cmd_down_goes_to_the_document_edge() {
    assert_eq!(
        command("down", &cmd()),
        Some(Command::Move(Move::DocEnd, false))
    );
}

#[test]
fn shift_down_extends_by_one_line() {
    assert_eq!(
        command("down", &shift()),
        Some(Command::Move(Move::Down, true))
    );
}

#[test]
fn cmd_shift_down_extends_to_the_document_edge() {
    assert_eq!(
        command("down", &cmd_shift()),
        Some(Command::Move(Move::DocEnd, true))
    );
}

#[test]
fn home_goes_to_the_document_edge() {
    assert_eq!(
        command("home", &NONE),
        Some(Command::Move(Move::DocStart, false))
    );
}

#[test]
fn shift_home_extends_to_the_document_edge() {
    assert_eq!(
        command("home", &shift()),
        Some(Command::Move(Move::DocStart, true))
    );
}

#[test]
fn end_goes_to_the_document_edge() {
    assert_eq!(
        command("end", &NONE),
        Some(Command::Move(Move::DocEnd, false))
    );
}

#[test]
fn shift_end_extends_to_the_document_edge() {
    assert_eq!(
        command("end", &shift()),
        Some(Command::Move(Move::DocEnd, true))
    );
}

#[test]
fn cmd_left_wins_over_alt() {
    assert_eq!(
        command("left", &cmd_alt()),
        Some(Command::Move(Move::LineStart, false))
    );
}

#[test]
fn cmd_right_wins_over_alt() {
    assert_eq!(
        command("right", &cmd_alt()),
        Some(Command::Move(Move::LineEnd, false))
    );
}

#[test]
fn cmd_a_selects_all() {
    assert_eq!(command("a", &cmd()), Some(Command::SelectAll));
}

#[test]
fn cmd_c_copies() {
    assert_eq!(command("c", &cmd()), Some(Command::Copy));
}

#[test]
fn cmd_x_cuts() {
    assert_eq!(command("x", &cmd()), Some(Command::Cut));
}

#[test]
fn cmd_v_pastes() {
    assert_eq!(command("v", &cmd()), Some(Command::Paste));
}

#[test]
fn cmd_z_undoes() {
    assert_eq!(command("z", &cmd()), Some(Command::Undo));
}

#[test]
fn cmd_shift_z_redoes() {
    assert_eq!(command("z", &cmd_shift()), Some(Command::Redo));
}

#[test]
fn enter_submits() {
    assert_eq!(command("enter", &NONE), Some(Command::Submit));
}

#[test]
fn shift_enter_is_a_newline() {
    assert_eq!(command("enter", &shift()), Some(Command::Newline));
}

#[test]
fn alt_enter_is_a_newline() {
    assert_eq!(command("enter", &alt()), Some(Command::Newline));
}

#[test]
fn backspace_deletes_one_cluster_back() {
    assert_eq!(
        command("backspace", &NONE),
        Some(Command::Delete(Delete::Backward))
    );
}

#[test]
fn alt_backspace_deletes_a_word_back() {
    assert_eq!(
        command("backspace", &alt()),
        Some(Command::Delete(Delete::WordBackward))
    );
}

#[test]
fn cmd_backspace_deletes_to_the_line_start() {
    assert_eq!(
        command("backspace", &cmd()),
        Some(Command::Delete(Delete::ToLineStart))
    );
}

#[test]
fn delete_deletes_one_cluster_forward() {
    assert_eq!(
        command("delete", &NONE),
        Some(Command::Delete(Delete::Forward))
    );
}

#[test]
fn alt_delete_deletes_a_word_forward() {
    assert_eq!(
        command("delete", &alt()),
        Some(Command::Delete(Delete::WordForward))
    );
}

#[test]
fn ctrl_left_is_left_to_the_system() {
    assert_eq!(command("left", &ctrl()), None);
}

#[test]
fn ctrl_a_is_left_to_the_system() {
    assert_eq!(command("a", &ctrl()), None);
}

#[test]
fn ctrl_enter_is_left_to_the_system() {
    assert_eq!(command("enter", &ctrl()), None);
}

#[test]
fn ctrl_backspace_is_left_to_the_system() {
    assert_eq!(command("backspace", &ctrl()), None);
}

#[test]
fn ctrl_z_is_left_to_the_system() {
    assert_eq!(command("z", &ctrl()), None);
}

#[test]
fn ctrl_c_is_left_to_the_system() {
    assert_eq!(command("c", &ctrl()), None);
}

#[test]
fn ctrl_cmd_a_is_left_to_the_system() {
    assert_eq!(
        command(
            "a",
            &Modifiers {
                control: true,
                platform: true,
                ..NONE
            }
        ),
        None
    );
}

#[test]
fn a_letter_without_cmd_is_left_to_the_input_method() {
    assert_eq!(command("a", &NONE), None);
}

#[test]
fn a_shifted_letter_is_left_to_the_input_method() {
    assert_eq!(command("a", &shift()), None);
}

#[test]
fn an_alt_letter_is_left_to_the_input_method() {
    assert_eq!(command("c", &alt()), None);
}

#[test]
fn an_unknown_key_does_nothing() {
    assert_eq!(command("f5", &NONE), None);
}

#[test]
fn tab_does_nothing() {
    assert_eq!(command("tab", &NONE), None);
}

#[test]
fn cmd_with_an_unknown_letter_does_nothing() {
    assert_eq!(command("q", &cmd()), None);
}

#[test]
fn cmd_with_an_unknown_key_does_nothing() {
    assert_eq!(command("f5", &cmd()), None);
}
