//! Contract suite of `SessionStorePort`: what every store of sessions must do, run on the fake
//! and, later, on the keychain adapter (ARCHITECTURE.md section 4, NFR-026).
//!
//! Need: FR-014 (a session is kept and read back), D373 (one session per user id, up to
//! `MAX_ACCOUNTS` accounts, the first signed in is the default one), A005 (a failing keychain
//! is an error, never a silent loss). Specified by `docs/dev/signin-api-sketch.md`.

use std::sync::Arc;

use futures::executor::block_on;
use twake_app::ports::{MAX_ACCOUNTS, SessionStoreError, SessionStorePort};
use twake_domain::UserId;
use twake_domain::session::{DeviceId, HomeserverUrl, Secret, Session};

use crate::contract::{Verdict, holds, same};

type Port = Arc<dyn SessionStorePort>;

fn user(n: usize) -> UserId {
    UserId::parse(&format!("@contract{n}:example.org")).unwrap_or_else(|_| unreachable!())
}

fn session(n: usize, token: &str) -> Session {
    Session {
        user_id: user(n),
        device_id: DeviceId::parse(&format!("DEVICE{n}")).unwrap_or_else(|_| unreachable!()),
        access_token: Secret::new(token),
        homeserver: HomeserverUrl::parse("https://matrix.example.org")
            .unwrap_or_else(|_| unreachable!()),
    }
}

fn save(port: &Port, s: Session) -> Result<(), String> {
    block_on(port.save(s)).map_err(|e| format!("save failed: {e}"))
}

fn list(port: &Port) -> Result<Vec<Session>, String> {
    block_on(port.list()).map_err(|e| format!("list failed: {e}"))
}

fn load(port: &Port, n: usize) -> Result<Option<Session>, String> {
    block_on(port.load(user(n))).map_err(|e| format!("load failed: {e}"))
}

fn clear(port: &Port, n: usize) -> Result<(), String> {
    block_on(port.clear(user(n))).map_err(|e| format!("clear failed: {e}"))
}

fn users(port: &Port) -> Result<Vec<UserId>, String> {
    Ok(list(port)?.into_iter().map(|s| s.user_id).collect())
}

/// What every `SessionStorePort` must do. `make` builds an empty store.
pub fn session_store(make: impl Fn() -> Arc<dyn SessionStorePort>) -> Verdict {
    a_new_store_is_empty(&make())?;
    a_saved_session_is_read_back_whole(&make())?;
    sessions_come_back_in_the_order_they_were_first_saved(&make())?;
    one_session_per_user_id(&make())?;
    saving_the_same_session_twice_changes_nothing(&make())?;
    the_sixth_account_is_refused_and_nothing_changes(&make())?;
    an_existing_account_can_be_replaced_when_the_store_is_full(&make())?;
    clearing_forgets_one_account_only(&make())?;
    clearing_frees_a_place(&make())
}

fn a_new_store_is_empty(port: &Port) -> Verdict {
    same("a new store lists nothing", list(port)?, vec![])?;
    same("a new store loads nothing", load(port, 1)?, None)
}

fn a_saved_session_is_read_back_whole(port: &Port) -> Verdict {
    let s = session(1, "token-1");
    save(port, s.clone())?;
    same(
        "load gives the saved session",
        load(port, 1)?,
        Some(s.clone()),
    )?;
    same("list gives the saved session", list(port)?, vec![s])?;
    same("another user is not found", load(port, 2)?, None)
}

fn sessions_come_back_in_the_order_they_were_first_saved(port: &Port) -> Verdict {
    for n in [3, 1, 2] {
        save(port, session(n, "t"))?;
    }
    same(
        "first saved first",
        users(port)?,
        vec![user(3), user(1), user(2)],
    )
}

fn one_session_per_user_id(port: &Port) -> Verdict {
    save(port, session(1, "old"))?;
    save(port, session(2, "other"))?;
    save(port, session(1, "new"))?;
    same(
        "the newer session replaces",
        load(port, 1)?,
        Some(session(1, "new")),
    )?;
    same(
        "the replaced one keeps its place",
        users(port)?,
        vec![user(1), user(2)],
    )?;
    same(
        "the other account is untouched",
        load(port, 2)?,
        Some(session(2, "other")),
    )
}

fn saving_the_same_session_twice_changes_nothing(port: &Port) -> Verdict {
    save(port, session(1, "same"))?;
    let once = list(port)?;
    save(port, session(1, "same"))?;
    same("a second identical save", list(port)?, once)
}

fn fill(port: &Port) -> Result<(), String> {
    (1..=MAX_ACCOUNTS).try_for_each(|n| save(port, session(n, "t")))
}

fn the_sixth_account_is_refused_and_nothing_changes(port: &Port) -> Verdict {
    fill(port)?;
    let before = list(port)?;
    let refused = block_on(port.save(session(MAX_ACCOUNTS + 1, "t")));
    same(
        "the sixth account",
        refused,
        Err(SessionStoreError::TooManyAccounts),
    )?;
    same("a refused save changes nothing", list(port)?, before)?;
    same(
        "the refused account is not found",
        load(port, MAX_ACCOUNTS + 1)?,
        None,
    )
}

fn an_existing_account_can_be_replaced_when_the_store_is_full(port: &Port) -> Verdict {
    fill(port)?;
    save(port, session(2, "fresh"))?;
    same("the replacement", load(port, 2)?, Some(session(2, "fresh")))?;
    holds("still five accounts", list(port)?.len() == MAX_ACCOUNTS)
}

fn clearing_forgets_one_account_only(port: &Port) -> Verdict {
    save(port, session(1, "t"))?;
    save(port, session(2, "t"))?;
    clear(port, 1)?;
    same("the cleared account is gone", load(port, 1)?, None)?;
    same("the other stays", users(port)?, vec![user(2)])?;
    clear(port, 1)?;
    clear(port, 9)?;
    same(
        "clearing an unknown account is fine",
        users(port)?,
        vec![user(2)],
    )?;
    clear(port, 2)?;
    same("the last account cleared", list(port)?, vec![])
}

fn clearing_frees_a_place(port: &Port) -> Verdict {
    fill(port)?;
    clear(port, 1)?;
    save(port, session(MAX_ACCOUNTS + 1, "t"))?;
    let expected: Vec<UserId> = (2..=MAX_ACCOUNTS + 1).map(user).collect();
    same("the new account goes last", users(port)?, expected)
}
