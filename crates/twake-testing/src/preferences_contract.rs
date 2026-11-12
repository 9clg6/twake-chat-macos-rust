//! Contract suite of `PreferencesPort`: what every store of the preferences must do, run on the
//! fake and, later, on the real store (ARCHITECTURE.md section 4).
//!
//! Need: FR-128/FR-129 (choices kept across restarts and shown by every window: a change made
//! in one is seen by the others), settings.md D349. The document is saved whole and atomically;
//! last write wins; a store that cannot be reached is an error, never a silent loss. Specified by
//! `docs/dev/settings-api-sketch.md` (section 3).

use std::sync::Arc;

use futures::executor::block_on;
use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use twake_app::ports::PreferencesPort;
use twake_domain::preferences::{
    Density, GlobalNotificationLevel, Language, Preferences, TextSize, ThemeChoice,
};

use crate::contract::{Verdict, holds, same};

type Port = Arc<dyn PreferencesPort>;

fn dark() -> Preferences {
    Preferences {
        theme: ThemeChoice::Dark,
        ..Preferences::default()
    }
}

fn compact() -> Preferences {
    Preferences {
        density: Density::Compact,
        ..Preferences::default()
    }
}

/// Differs from `Preferences::default()` and from `dark()` in several fields.
fn busy() -> Preferences {
    Preferences {
        theme: ThemeChoice::Light,
        density: Density::Compact,
        language: Language::Code("fr"),
        text_size: TextSize::Large,
        high_contrast: Some(true),
        notifications: GlobalNotificationLevel::Mentions,
        message_sound: false,
        ..Preferences::default()
    }
}

fn load(port: &Port) -> Result<Preferences, String> {
    block_on(port.load()).map_err(|e| format!("load failed: {e}"))
}

fn save(port: &Port, p: Preferences) -> Result<(), String> {
    block_on(port.save(p)).map_err(|e| format!("save failed: {e}"))
}

fn subscribe(port: &Port) -> Result<BoxStream<'static, Preferences>, String> {
    block_on(port.subscribe()).map_err(|e| format!("subscribe failed: {e}"))
}

/// The next item if one is ready, `None` if the stream has nothing to say now.
fn ready(stream: &mut BoxStream<'static, Preferences>) -> Option<Preferences> {
    stream.next().now_or_never().flatten()
}

/// What every `PreferencesPort` must do. `make` builds an empty store.
pub fn preferences(make: impl Fn() -> Arc<dyn PreferencesPort>) -> Verdict {
    an_empty_store_holds_the_defaults(&make())?;
    a_saved_document_is_read_back_whole(&make())?;
    saving_replaces_the_whole_document(&make())?;
    the_last_save_wins(&make())?;
    concurrent_saves_never_mix(&make())?;
    saving_the_same_document_twice_changes_nothing(&make())?;
    a_subscription_starts_with_the_current_document(&make())?;
    a_subscription_hears_each_real_change_once(&make())?;
    a_save_of_the_same_value_notifies_nothing(&make())?;
    every_subscriber_hears_the_change(&make())?;
    a_dropped_subscriber_does_not_disturb_the_others(&make())?;
    a_late_subscriber_starts_from_the_current_document(&make())
}

fn an_empty_store_holds_the_defaults(port: &Port) -> Verdict {
    same(
        "a new store loads the defaults",
        load(port)?,
        Preferences::default(),
    )
}

fn a_saved_document_is_read_back_whole(port: &Port) -> Verdict {
    save(port, busy())?;
    same("load gives the saved document", load(port)?, busy())
}

fn saving_replaces_the_whole_document(port: &Port) -> Verdict {
    save(port, busy())?;
    save(port, Preferences::default())?;
    same(
        "nothing of the first document stays",
        load(port)?,
        Preferences::default(),
    )
}

fn the_last_save_wins(port: &Port) -> Verdict {
    save(port, dark())?;
    save(port, compact())?;
    same("the last document wins", load(port)?, compact())
}

fn concurrent_saves_never_mix(port: &Port) -> Verdict {
    let (a, b) = (busy(), dark());
    let (first, second) = block_on(futures::future::join(
        port.save(a.clone()),
        port.save(b.clone()),
    ));
    first.map_err(|e| format!("first save failed: {e}"))?;
    second.map_err(|e| format!("second save failed: {e}"))?;
    let got = load(port)?;
    holds(
        &format!("after two concurrent saves the store holds one of them, got {got:?}"),
        got == a || got == b,
    )
}

fn saving_the_same_document_twice_changes_nothing(port: &Port) -> Verdict {
    save(port, busy())?;
    save(port, busy())?;
    same("still that document", load(port)?, busy())
}

fn a_subscription_starts_with_the_current_document(port: &Port) -> Verdict {
    let mut empty = subscribe(port)?;
    same(
        "an empty store starts with the defaults",
        ready(&mut empty),
        Some(Preferences::default()),
    )?;
    save(port, busy())?;
    let mut later = subscribe(port)?;
    same(
        "a stored document starts the stream",
        ready(&mut later),
        Some(busy()),
    )
}

fn a_subscription_hears_each_real_change_once(port: &Port) -> Verdict {
    let mut stream = subscribe(port)?;
    ready(&mut stream);
    save(port, dark())?;
    same(
        "one item for the first change",
        ready(&mut stream),
        Some(dark()),
    )?;
    same("and only one", ready(&mut stream), None)?;
    save(port, compact())?;
    same(
        "one item for the next change",
        ready(&mut stream),
        Some(compact()),
    )?;
    same("and only one again", ready(&mut stream), None)
}

fn a_save_of_the_same_value_notifies_nothing(port: &Port) -> Verdict {
    save(port, dark())?;
    let mut stream = subscribe(port)?;
    ready(&mut stream);
    save(port, dark())?;
    same("an identical save is silent", ready(&mut stream), None)?;
    save(port, Preferences::default())?;
    save(port, Preferences::default())?;
    same(
        "only the real change is heard",
        ready(&mut stream),
        Some(Preferences::default()),
    )?;
    same("nothing more", ready(&mut stream), None)
}

fn every_subscriber_hears_the_change(port: &Port) -> Verdict {
    let mut one = subscribe(port)?;
    let mut two = subscribe(port)?;
    ready(&mut one);
    ready(&mut two);
    save(port, busy())?;
    same("the first window hears it", ready(&mut one), Some(busy()))?;
    same("the second window hears it", ready(&mut two), Some(busy()))
}

fn a_dropped_subscriber_does_not_disturb_the_others(port: &Port) -> Verdict {
    let gone = subscribe(port)?;
    let mut kept = subscribe(port)?;
    ready(&mut kept);
    drop(gone);
    save(port, dark())?;
    same("the other keeps hearing", ready(&mut kept), Some(dark()))
}

fn a_late_subscriber_starts_from_the_current_document(port: &Port) -> Verdict {
    let mut early = subscribe(port)?;
    ready(&mut early);
    save(port, dark())?;
    save(port, compact())?;
    let mut late = subscribe(port)?;
    same(
        "the late one sees the last document, not the history",
        ready(&mut late),
        Some(compact()),
    )?;
    same("and nothing else", ready(&mut late), None)
}
