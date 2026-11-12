//! Contract suite of `MessageActionsPort`: what every implementation must do, run on the fake
//! and, later, on the real adapter (ARCHITECTURE.md section 4).
//!
//! Need: FR-075 and FR-077 (react, take a reaction back), FR-079 and D229 (pins: several per
//! room, unpinning one keeps the others, none lost), D255 (report with an optional reason),
//! Matrix (`m.reaction`, redactions, `m.room.pinned_events`). A room the account is not in is
//! an error, never a silent success. Specified by `docs/dev/message-actions-api-sketch.md`.

use std::sync::Arc;

use futures::executor::block_on;
use twake_app::ports::{ActionError, MessageActionsPort};
use twake_domain::{EventId, RoomId};

use crate::contract::{Verdict, holds, same};

type Port = Arc<dyn MessageActionsPort>;

fn known() -> RoomId {
    RoomId::parse("!actions1:example.org").unwrap_or_else(|_| unreachable!())
}

fn unknown() -> RoomId {
    RoomId::parse("!actions99:example.org").unwrap_or_else(|_| unreachable!())
}

fn ev(n: usize) -> EventId {
    EventId::parse(&format!("$action{n}")).unwrap_or_else(|_| unreachable!())
}

fn react(port: &Port, n: usize, key: &str) -> Result<EventId, String> {
    block_on(port.send_reaction(known(), ev(n), key.into()))
        .map_err(|e| format!("send_reaction failed: {e}"))
}

fn pin(port: &Port, n: usize) -> Result<(), String> {
    block_on(port.pin(known(), ev(n))).map_err(|e| format!("pin failed: {e}"))
}

fn unpin(port: &Port, n: usize) -> Result<(), String> {
    block_on(port.unpin(known(), ev(n))).map_err(|e| format!("unpin failed: {e}"))
}

fn pinned(port: &Port) -> Result<Vec<EventId>, String> {
    block_on(port.pinned(known())).map_err(|e| format!("pinned failed: {e}"))
}

/// What every `MessageActionsPort` must do. `make` builds a port that knows the room
/// `!actions1:example.org`, with no pin and no reaction.
pub fn message_actions(make: impl Fn() -> Arc<dyn MessageActionsPort>) -> Verdict {
    a_new_room_has_no_pin(&make())?;
    a_reaction_gets_an_event_id(&make())?;
    two_reactions_have_two_ids(&make())?;
    a_reaction_can_be_taken_back_twice(&make())?;
    redacting_an_unknown_event_is_fine(&make())?;
    pins_are_kept_in_the_order_they_were_made(&make())?;
    pinning_twice_pins_once(&make())?;
    unpinning_keeps_the_others(&make())?;
    unpinning_an_unpinned_message_is_fine(&make())?;
    a_report_goes_with_or_without_a_reason(&make())?;
    an_unknown_room_is_refused_everywhere(&make())
}

fn a_new_room_has_no_pin(port: &Port) -> Verdict {
    same("no pin at first", pinned(port)?, vec![])
}

fn a_reaction_gets_an_event_id(port: &Port) -> Verdict {
    let id = react(port, 1, "🎉")?;
    holds("the reaction event has an id", !id.as_str().is_empty())
}

fn two_reactions_have_two_ids(port: &Port) -> Verdict {
    let first = react(port, 1, "🎉")?;
    let second = react(port, 1, "👀")?;
    holds("each reaction is its own event", first != second)
}

fn a_reaction_can_be_taken_back_twice(port: &Port) -> Verdict {
    let id = react(port, 1, "🎉")?;
    block_on(port.redact(known(), id.clone())).map_err(|e| format!("redact failed: {e}"))?;
    block_on(port.redact(known(), id)).map_err(|e| format!("second redact failed: {e}"))
}

fn redacting_an_unknown_event_is_fine(port: &Port) -> Verdict {
    block_on(port.redact(known(), ev(77))).map_err(|e| format!("redact failed: {e}"))
}

fn pins_are_kept_in_the_order_they_were_made(port: &Port) -> Verdict {
    pin(port, 3)?;
    pin(port, 1)?;
    same("oldest pin first", pinned(port)?, vec![ev(3), ev(1)])
}

fn pinning_twice_pins_once(port: &Port) -> Verdict {
    pin(port, 1)?;
    pin(port, 2)?;
    pin(port, 1)?;
    same("one pin, place kept", pinned(port)?, vec![ev(1), ev(2)])
}

fn unpinning_keeps_the_others(port: &Port) -> Verdict {
    pin(port, 1)?;
    pin(port, 2)?;
    pin(port, 3)?;
    unpin(port, 2)?;
    same("the others stay", pinned(port)?, vec![ev(1), ev(3)])
}

fn unpinning_an_unpinned_message_is_fine(port: &Port) -> Verdict {
    pin(port, 1)?;
    unpin(port, 9)?;
    same("nothing changed", pinned(port)?, vec![ev(1)])
}

fn a_report_goes_with_or_without_a_reason(port: &Port) -> Verdict {
    block_on(port.report(known(), ev(1), Some("spam".into())))
        .map_err(|e| format!("report with reason failed: {e}"))?;
    block_on(port.report(known(), ev(1), None))
        .map_err(|e| format!("report without reason failed: {e}"))
}

fn no_room<T>() -> Result<T, ActionError> {
    Err(ActionError::RoomNotFound)
}

fn an_unknown_room_is_refused_everywhere(port: &Port) -> Verdict {
    let room = unknown();
    same(
        "send_reaction",
        block_on(port.send_reaction(room.clone(), ev(1), "🎉".into())),
        no_room(),
    )?;
    same(
        "redact",
        block_on(port.redact(room.clone(), ev(1))),
        no_room(),
    )?;
    same("pin", block_on(port.pin(room.clone(), ev(1))), no_room())?;
    same(
        "unpin",
        block_on(port.unpin(room.clone(), ev(1))),
        no_room(),
    )?;
    same("pinned", block_on(port.pinned(room.clone())), no_room())?;
    same(
        "report",
        block_on(port.report(room, ev(1), None)),
        no_room(),
    )
}
