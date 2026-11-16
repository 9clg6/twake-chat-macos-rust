//! Matrix identifiers (ARCHITECTURE.md section 4: ids are newtypes, parsed once at the boundary, P4).
//! Need: Matrix spec, "Common identifier format"
//! (<https://spec.matrix.org/latest/appendices/#common-identifier-format>).
//! Public items used are listed in `docs/dev/api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use std::collections::HashSet;

use proptest::prelude::*;
use twake_domain::{EventId, IdError, RoomAlias, RoomId, UserId};

/// Total length cap of any identifier, sigil and server name included, in bytes.
const MAX_BYTES: usize = 255;

// ---------------------------------------------------------------- RoomId

#[test]
fn a_room_id_keeps_its_opaque_part_and_its_server() {
    let id = RoomId::parse("!636q39766251:example.com").unwrap();
    assert_eq!(id.as_str(), "!636q39766251:example.com");
    assert_eq!(id.server_name(), Some("example.com"));
    assert_eq!(id.to_string(), "!636q39766251:example.com");
}

#[test]
fn a_room_id_opaque_part_is_case_sensitive() {
    let upper = RoomId::parse("!AbC:example.com").unwrap();
    let lower = RoomId::parse("!abc:example.com").unwrap();
    assert_ne!(upper, lower);
    assert_eq!(upper.to_string(), "!AbC:example.com");
}

#[test]
fn a_room_id_rejects_what_is_not_one() {
    assert_eq!(RoomId::parse(""), Err(IdError::Empty));
    // A user id is not a room id.
    assert_eq!(
        RoomId::parse("@alice:example.com"),
        Err(IdError::WrongSigil)
    );
    assert_eq!(RoomId::parse("#room:example.com"), Err(IdError::WrongSigil));
    assert_eq!(RoomId::parse("room:example.com"), Err(IdError::WrongSigil));
    assert!(RoomId::parse("!:example.com").is_err(), "empty opaque part");
    assert!(RoomId::parse("!abc:").is_err(), "empty server");
    assert!(RoomId::parse("!ab c:example.com").is_err(), "space");
    assert!(
        RoomId::parse("!abc:exa mple.com").is_err(),
        "space in server"
    );
    assert!(RoomId::parse("!abc\n:example.com").is_err(), "newline");
}

#[test]
fn a_room_id_over_255_bytes_is_refused_and_255_is_kept() {
    let tail = ":example.com";
    let fits = format!("!{}{tail}", "a".repeat(MAX_BYTES - 1 - tail.len()));
    assert_eq!(fits.len(), MAX_BYTES);
    assert!(RoomId::parse(&fits).is_ok());
    let too_long = format!("!{}{tail}", "a".repeat(MAX_BYTES - tail.len()));
    assert_eq!(RoomId::parse(&too_long), Err(IdError::TooLong));
}

// ---------------------------------------------------------------- EventId

#[test]
fn an_event_id_of_room_version_1_has_a_server() {
    let id = EventId::parse("$Rqnc-F-dvnEYJTyHq:example.org").unwrap();
    assert_eq!(id.server_name(), Some("example.org"));
    assert_eq!(id.to_string(), "$Rqnc-F-dvnEYJTyHq:example.org");
}

#[test]
fn an_event_id_of_a_hash_has_no_server_part() {
    // Room version 3: standard base64 (may hold '/' and '+').
    let v3 = EventId::parse("$acR1l0raoZnm60CBwAVgqbZqoO/mYU81xysh1u6XiP8").unwrap();
    assert_eq!(v3.server_name(), None);
    assert_eq!(
        v3.to_string(),
        "$acR1l0raoZnm60CBwAVgqbZqoO/mYU81xysh1u6XiP8"
    );
    // Room version 4 and later: url-safe base64 (may hold '-' and '_').
    let v4 = EventId::parse("$Rqnc-F-dvnEYJTyHq_iKxU2bZ1CI92-kuZq3a5lr5Zg").unwrap();
    assert_eq!(v4.server_name(), None);
}

#[test]
fn an_event_id_rejects_what_is_not_one() {
    assert_eq!(EventId::parse(""), Err(IdError::Empty));
    assert_eq!(
        EventId::parse("!room:example.com"),
        Err(IdError::WrongSigil)
    );
    assert_eq!(EventId::parse("abc"), Err(IdError::WrongSigil));
    assert!(EventId::parse("$").is_err(), "sigil alone");
    assert!(EventId::parse("$abc:").is_err(), "empty server");
    assert!(EventId::parse("$ab c").is_err(), "space");
    assert!(
        EventId::parse("$abc:exa mple.org").is_err(),
        "space in server"
    );
}

#[test]
fn an_event_id_over_255_bytes_is_refused() {
    let fits = format!("${}", "a".repeat(MAX_BYTES - 1));
    assert!(EventId::parse(&fits).is_ok());
    let too_long = format!("${}", "a".repeat(MAX_BYTES));
    assert_eq!(EventId::parse(&too_long), Err(IdError::TooLong));
}

// ---------------------------------------------------------------- UserId

#[test]
fn a_user_id_splits_into_localpart_and_server() {
    let id = UserId::parse("@alice:example.org").unwrap();
    assert_eq!(id.localpart(), "alice");
    assert_eq!(id.server_name(), "example.org");
    assert_eq!(id.as_str(), "@alice:example.org");
    assert_eq!(id.to_string(), "@alice:example.org");
}

#[test]
fn a_user_id_takes_every_character_the_spec_allows_in_a_localpart() {
    // a-z 0-9 . _ = - / +
    let id = UserId::parse("@a1._=-/+z:example.org").unwrap();
    assert_eq!(id.localpart(), "a1._=-/+z");
}

#[test]
fn a_user_id_server_may_carry_a_port_an_ipv4_or_an_ipv6_literal() {
    assert_eq!(
        UserId::parse("@a:example.org:8448").unwrap().server_name(),
        "example.org:8448"
    );
    assert_eq!(
        UserId::parse("@a:1.2.3.4").unwrap().server_name(),
        "1.2.3.4"
    );
    assert_eq!(
        UserId::parse("@a:1.2.3.4:8448").unwrap().server_name(),
        "1.2.3.4:8448"
    );
    assert_eq!(UserId::parse("@a:[::1]").unwrap().server_name(), "[::1]");
    assert_eq!(
        UserId::parse("@a:[::ffff:1.2.3.4]").unwrap().server_name(),
        "[::ffff:1.2.3.4]"
    );
    assert_eq!(
        UserId::parse("@a:[2001:db8::1]:8448")
            .unwrap()
            .server_name(),
        "[2001:db8::1]:8448"
    );
}

#[test]
fn a_user_id_historical_uppercase_localpart_is_read_and_kept_as_received() {
    // The spec asks for lowercase on new accounts but clients must cope with
    // historical ids from existing servers (open question Q2 in the report).
    let id = UserId::parse("@Alice:example.org").unwrap();
    assert_eq!(id.to_string(), "@Alice:example.org");
    assert_ne!(id, UserId::parse("@alice:example.org").unwrap());
}

#[test]
fn a_user_id_without_the_right_shape_is_refused() {
    assert_eq!(UserId::parse(""), Err(IdError::Empty));
    assert_eq!(UserId::parse("!room:example.org"), Err(IdError::WrongSigil));
    assert_eq!(UserId::parse("alice:example.org"), Err(IdError::WrongSigil));
    assert_eq!(UserId::parse("@alice"), Err(IdError::MissingServer));
    assert!(UserId::parse("@").is_err(), "sigil alone");
}

#[test]
fn a_user_id_with_an_empty_part_is_refused() {
    assert!(UserId::parse("@:example.org").is_err(), "empty localpart");
    assert!(UserId::parse("@alice:").is_err(), "empty server");
}

#[test]
fn a_user_id_with_a_space_in_it_is_refused() {
    for id in [
        "@al ice:example.org",
        "@alice:exam ple.org",
        " @alice:example.org",
        "@alice:example.org ",
    ] {
        assert!(UserId::parse(id).is_err(), "{id:?}");
    }
}

#[test]
fn a_user_id_with_a_control_character_or_a_second_sigil_is_refused() {
    assert!(UserId::parse("@ali\tce:example.org").is_err(), "tab");
    assert!(UserId::parse("@ali\0ce:example.org").is_err(), "NUL");
    assert!(
        UserId::parse("@al@ice:example.org").is_err(),
        "second sigil in localpart"
    );
}

#[test]
fn a_user_id_server_with_a_bad_port_is_refused() {
    assert!(UserId::parse("@a:example.org:").is_err(), "empty port");
    assert!(
        UserId::parse("@a:example.org:abc").is_err(),
        "non-numeric port"
    );
    assert!(
        UserId::parse("@a:example.org:123456").is_err(),
        "more than 5 digits"
    );
    assert!(UserId::parse("@a:[::1").is_err(), "unclosed ipv6 literal");
}

#[test]
fn a_user_id_is_255_bytes_at_most_for_the_whole_id() {
    let tail = ":example.org";
    let fits = format!("@{}{tail}", "a".repeat(MAX_BYTES - 1 - tail.len()));
    assert_eq!(fits.len(), MAX_BYTES);
    assert!(UserId::parse(&fits).is_ok());
    let too_long = format!("@{}{tail}", "a".repeat(MAX_BYTES - tail.len()));
    assert_eq!(too_long.len(), MAX_BYTES + 1);
    assert_eq!(UserId::parse(&too_long), Err(IdError::TooLong));
}

#[test]
fn user_ids_work_as_map_keys() {
    let mut set = HashSet::new();
    set.insert(UserId::parse("@a:example.org").unwrap());
    set.insert(UserId::parse("@a:example.org").unwrap());
    set.insert(UserId::parse("@b:example.org").unwrap());
    assert_eq!(set.len(), 2);
}

#[test]
fn from_str_parses_like_parse() {
    let via_parse = UserId::parse("@alice:example.org").unwrap();
    let via_from_str: UserId = "@alice:example.org".parse().unwrap();
    assert_eq!(via_parse, via_from_str);
    assert!("not an id".parse::<UserId>().is_err());
}

// ---------------------------------------------------------------- RoomAlias

#[test]
fn a_room_alias_splits_into_alias_and_server() {
    let alias = RoomAlias::parse("#twake-dev:example.org").unwrap();
    assert_eq!(alias.localpart(), "twake-dev");
    assert_eq!(alias.server_name(), "example.org");
    assert_eq!(alias.to_string(), "#twake-dev:example.org");
}

#[test]
fn a_room_alias_may_hold_non_ascii_letters() {
    let alias = RoomAlias::parse("#café-équipe:example.org").unwrap();
    assert_eq!(alias.localpart(), "café-équipe");
    assert_eq!(alias.to_string(), "#café-équipe:example.org");
}

#[test]
fn a_room_alias_rejects_what_is_not_one() {
    assert_eq!(RoomAlias::parse(""), Err(IdError::Empty));
    assert_eq!(
        RoomAlias::parse("!room:example.org"),
        Err(IdError::WrongSigil)
    );
    assert_eq!(
        RoomAlias::parse("room:example.org"),
        Err(IdError::WrongSigil)
    );
    assert_eq!(RoomAlias::parse("#room"), Err(IdError::MissingServer));
    assert!(RoomAlias::parse("#:example.org").is_err(), "empty alias");
    assert!(RoomAlias::parse("#room:").is_err(), "empty server");
    assert!(RoomAlias::parse("#my room:example.org").is_err(), "space");
    assert!(RoomAlias::parse("#ro\0om:example.org").is_err(), "NUL");
}

#[test]
fn a_room_alias_limit_counts_bytes_not_characters() {
    // 'é' is 2 bytes: 125 of them + '#' + ":a.b" = 255 bytes, 126 = 257 bytes
    // while still far under 255 characters.
    let fits = format!("#{}:a.b", "é".repeat(125));
    assert_eq!(fits.len(), MAX_BYTES);
    assert!(RoomAlias::parse(&fits).is_ok());
    let too_long = format!("#{}:a.b", "é".repeat(126));
    assert!(too_long.chars().count() < MAX_BYTES);
    assert_eq!(RoomAlias::parse(&too_long), Err(IdError::TooLong));
}

// ---------------------------------------------------------------- properties

fn server_name() -> impl Strategy<Value = String> {
    "[a-z0-9]([a-z0-9-]{0,10}[a-z0-9])?(\\.[a-z]{2,6}){0,2}(:[0-9]{1,5})?"
}

proptest! {
    #[test]
    fn user_id_display_round_trips(local in "[a-z0-9._=/+-]{1,60}", server in server_name()) {
        let text = format!("@{local}:{server}");
        let id = UserId::parse(&text).unwrap();
        prop_assert_eq!(id.to_string(), text.clone());
        prop_assert_eq!(UserId::parse(&id.to_string()).unwrap(), id.clone());
        prop_assert_eq!(id.localpart(), local.as_str());
        prop_assert_eq!(id.server_name(), server.as_str());
    }

    #[test]
    fn room_id_display_round_trips(opaque in "[A-Za-z0-9]{1,60}", server in server_name()) {
        let text = format!("!{opaque}:{server}");
        let id = RoomId::parse(&text).unwrap();
        prop_assert_eq!(id.to_string(), text);
        prop_assert_eq!(RoomId::parse(&id.to_string()).unwrap(), id);
    }

    #[test]
    fn event_id_display_round_trips(hash in "[A-Za-z0-9_-]{1,60}", server in proptest::option::of(server_name())) {
        let text = match &server {
            Some(s) => format!("${hash}:{s}"),
            None => format!("${hash}"),
        };
        let id = EventId::parse(&text).unwrap();
        prop_assert_eq!(id.to_string(), text);
        prop_assert_eq!(id.server_name(), server.as_deref());
    }

    #[test]
    fn room_alias_display_round_trips(local in "[a-z0-9._=/+-]{1,60}", server in server_name()) {
        let text = format!("#{local}:{server}");
        let alias = RoomAlias::parse(&text).unwrap();
        prop_assert_eq!(alias.to_string(), text);
        prop_assert_eq!(alias.localpart(), local.as_str());
    }

    /// Whatever the input, parsing never panics, and what it accepts is
    /// within the length cap, free of whitespace and displays back unchanged.
    #[test]
    fn whatever_is_accepted_is_bounded_clean_and_stable(input in "\\PC{0,300}") {
        if let Ok(id) = UserId::parse(&input) {
            prop_assert!(input.len() <= MAX_BYTES);
            prop_assert!(!input.chars().any(char::is_whitespace));
            prop_assert_eq!(id.to_string(), input.clone());
        }
        if let Ok(id) = RoomId::parse(&input) {
            prop_assert!(input.len() <= MAX_BYTES);
            prop_assert!(!input.chars().any(char::is_whitespace));
            prop_assert_eq!(id.to_string(), input.clone());
        }
        if let Ok(id) = EventId::parse(&input) {
            prop_assert!(input.len() <= MAX_BYTES);
            prop_assert!(!input.chars().any(char::is_whitespace));
            prop_assert_eq!(id.to_string(), input.clone());
        }
    }

    /// An id never parses as another kind: the sigil decides.
    #[test]
    fn the_sigil_decides_the_kind(local in "[a-z0-9]{1,20}", server in server_name()) {
        let user = format!("@{local}:{server}");
        let room = format!("!{local}:{server}");
        let alias = format!("#{local}:{server}");
        prop_assert!(RoomId::parse(&user).is_err() && RoomAlias::parse(&user).is_err());
        prop_assert!(UserId::parse(&room).is_err() && RoomAlias::parse(&room).is_err());
        prop_assert!(UserId::parse(&alias).is_err() && RoomId::parse(&alias).is_err());
    }
}
