//! `FakePreferences`: a deterministic stand-in of the preferences file, held as the port.
//!
//! Need: FR-128/FR-129 (choices are kept across restarts), ARCHITECTURE.md section 4 (every port
//! has a fake and a contract suite run on both the fake and the real adapter), NFR-015 style: a
//! store that cannot be reached is an error, never a silent loss. The behaviour is
//! `twake_testing::contract::preferences`; this file runs it and adds what only the fake has.
//!
//! Public items used are listed in `docs/dev/settings-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::sync::Arc;

use futures::executor::block_on;
use twake_app::ports::{PreferencesError, PreferencesPort};
use twake_domain::preferences::{Preferences, ThemeChoice};
use twake_testing::FakePreferences;
use twake_testing::contract;

fn dark() -> Preferences {
    Preferences {
        theme: ThemeChoice::Dark,
        ..Preferences::default()
    }
}

#[test]
fn the_fake_passes_the_contract_suite() {
    contract::preferences(|| Arc::new(FakePreferences::new())).expect("contract");
}

#[test]
fn each_fake_is_its_own_store() {
    let one = FakePreferences::new();
    let two = FakePreferences::new();
    block_on(one.save(dark())).unwrap();
    assert_eq!(block_on(two.load()), Ok(Preferences::default()));
}

#[test]
fn the_default_fake_holds_the_defaults() {
    let store = FakePreferences::default();
    assert_eq!(block_on(store.load()), Ok(Preferences::default()));
}

#[test]
fn a_seeded_fake_holds_the_seed() {
    let store = FakePreferences::with(dark());
    assert_eq!(block_on(store.load()), Ok(dark()));
}

#[test]
fn an_unreachable_store_fails_every_call_and_keeps_nothing() {
    let store = FakePreferences::new().unavailable();
    assert_eq!(block_on(store.load()), Err(PreferencesError::Unavailable));
    assert_eq!(
        block_on(store.save(dark())),
        Err(PreferencesError::Unavailable)
    );
    assert!(matches!(
        block_on(store.subscribe()),
        Err(PreferencesError::Unavailable)
    ));
}

#[test]
fn the_error_says_something() {
    assert!(!PreferencesError::Unavailable.to_string().is_empty());
}
