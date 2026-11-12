//! Contract suite of `DraftStorePort`: what every store of drafts must do, run on the fake and,
//! later, on the real store (ARCHITECTURE.md section 4).
//!
//! Need: FR-055 (drafts per room, kept across restarts: switch room and back, restart, the draft
//! is restored; Enter empties it), D382 (text only), D487 (the picked mentions are saved with the
//! draft), D512 (last change wins; sending or clearing forgets it). A store that cannot be
//! reached is an error, never a silent loss. Specified by `docs/dev/composer-api-sketch.md`.

use std::sync::Arc;

use futures::executor::block_on;
use twake_app::composer::Draft;
use twake_app::ports::DraftStorePort;
use twake_domain::mention::Mention;
use twake_domain::{RoomId, UserId};

use crate::contract::{Verdict, same};

type Port = Arc<dyn DraftStorePort>;

fn room(n: usize) -> RoomId {
    RoomId::parse(&format!("!draft{n}:example.org")).unwrap_or_else(|_| unreachable!())
}

fn draft(text: &str) -> Draft {
    Draft {
        text: text.into(),
        mentions: vec![],
    }
}

fn with_mention(text: &str) -> Draft {
    let bob = UserId::parse("@bob:example.org").unwrap_or_else(|_| unreachable!());
    Draft {
        text: text.into(),
        mentions: vec![Mention::user(bob, "Bob")],
    }
}

fn save(port: &Port, n: usize, d: Draft) -> Result<(), String> {
    block_on(port.save(room(n), d)).map_err(|e| format!("save failed: {e}"))
}

fn load(port: &Port, n: usize) -> Result<Option<Draft>, String> {
    block_on(port.load(room(n))).map_err(|e| format!("load failed: {e}"))
}

fn clear(port: &Port, n: usize) -> Result<(), String> {
    block_on(port.clear(room(n))).map_err(|e| format!("clear failed: {e}"))
}

/// What every `DraftStorePort` must do. `make` builds an empty store.
pub fn draft_store(make: impl Fn() -> Arc<dyn DraftStorePort>) -> Verdict {
    a_new_store_holds_no_draft(&make())?;
    a_saved_draft_is_read_back_whole(&make())?;
    text_is_kept_exactly(&make())?;
    drafts_are_per_room(&make())?;
    saving_replaces_the_draft(&make())?;
    saving_the_same_draft_twice_changes_nothing(&make())?;
    an_empty_draft_is_no_draft(&make())?;
    clearing_forgets_one_room_only(&make())?;
    clearing_an_unknown_room_is_fine(&make())
}

fn a_new_store_holds_no_draft(port: &Port) -> Verdict {
    same("a new store has no draft", load(port, 1)?, None)
}

fn a_saved_draft_is_read_back_whole(port: &Port) -> Verdict {
    let d = with_mention("hi Bob");
    save(port, 1, d.clone())?;
    same(
        "load gives the saved draft with its mentions",
        load(port, 1)?,
        Some(d),
    )
}

fn text_is_kept_exactly(port: &Port) -> Verdict {
    let d = draft("  line one \n\n\tline é two 😄  \n");
    save(port, 1, d.clone())?;
    same(
        "spaces, newlines and accents are kept",
        load(port, 1)?,
        Some(d),
    )
}

fn drafts_are_per_room(port: &Port) -> Verdict {
    save(port, 1, draft("for room one"))?;
    save(port, 2, draft("for room two"))?;
    same(
        "room one keeps its draft",
        load(port, 1)?,
        Some(draft("for room one")),
    )?;
    same(
        "room two keeps its draft",
        load(port, 2)?,
        Some(draft("for room two")),
    )?;
    same("room three has none", load(port, 3)?, None)
}

fn saving_replaces_the_draft(port: &Port) -> Verdict {
    save(port, 1, with_mention("first"))?;
    save(port, 1, draft("second"))?;
    same(
        "the last draft wins, mentions included",
        load(port, 1)?,
        Some(draft("second")),
    )
}

fn saving_the_same_draft_twice_changes_nothing(port: &Port) -> Verdict {
    let d = with_mention("same");
    save(port, 1, d.clone())?;
    save(port, 1, d.clone())?;
    same("one draft, not two", load(port, 1)?, Some(d))
}

fn an_empty_draft_is_no_draft(port: &Port) -> Verdict {
    save(port, 1, draft("something"))?;
    save(port, 1, draft(""))?;
    same("a draft with no text is a clear", load(port, 1)?, None)
}

fn clearing_forgets_one_room_only(port: &Port) -> Verdict {
    save(port, 1, draft("one"))?;
    save(port, 2, draft("two"))?;
    clear(port, 1)?;
    same("the cleared room has no draft", load(port, 1)?, None)?;
    same(
        "the other room keeps its draft",
        load(port, 2)?,
        Some(draft("two")),
    )
}

fn clearing_an_unknown_room_is_fine(port: &Port) -> Verdict {
    clear(port, 9)?;
    same("still nothing", load(port, 9)?, None)
}
