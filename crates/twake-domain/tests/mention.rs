//! Mentions: who is notified (`m.mentions`), the names linked in the HTML, and the lists offered
//! after "@" and "#".
//!
//! Need (not the code): `docs/spec/features/composer.md` and FR-054
//! - e2e "Composer": "@bo" lists bob; Enter writes "bob " and the sent message renders "bob" as a
//!   link to `https://matrix.to/#/%40bob%3Alocalhost`; "@@bob:localhost" (a full Matrix id) lists bob.
//! - FR-054: up to 25 mentions. Commit "Mention list offers up to 25 people".
//! - D401/D431: "@room" is `m.mentions: { room: true }`, the text keeps "@room" with no link;
//!   last in a bare "@" list (only when someone is listed), first once "room", "all" or
//!   "everyone" is started; only for who may notify the room.
//! - D487: a name erased from the text notifies nobody.
//! - D502 + e2e "location-rooms-tombstone": "#target" links the room by `https://matrix.to/#/%23..`
//!   and `m.mentions` stays `{}`.
//! - D302: the Matrix name is written, never the private name given to a contact.
//! - Matrix spec (D418): `m.mentions` (`user_ids`, `room`), matrix.to permalinks.
//!
//! Public items used are listed in `docs/dev/composer-api-sketch.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use proptest::prelude::*;
use twake_domain::markdown::{OutgoingBody, to_message};
use twake_domain::mention::{
    LinkableRoom, MAX_MENTIONS, Member, Mention, MentionCandidate, Mentions, extract, link_names,
    match_members, match_rooms,
};
use twake_domain::search::fold;
use twake_domain::{RoomAlias, UserId};

fn user(local: &str) -> UserId {
    UserId::parse(&format!("@{local}:localhost")).unwrap()
}

fn alias(local: &str) -> RoomAlias {
    RoomAlias::parse(&format!("#{local}:localhost")).unwrap()
}

fn bob() -> Mention {
    Mention::user(user("bob"), "bob")
}

fn member(local: &str, name: &str) -> Member {
    Member {
        user_id: user(local),
        name: name.into(),
        matrix_name: None,
    }
}

fn plain(text: &str) -> OutgoingBody {
    to_message(text)
}

fn people(found: &[MentionCandidate]) -> Vec<String> {
    found
        .iter()
        .map(|c| match c {
            MentionCandidate::Person(m) => m.name.clone(),
            MentionCandidate::Everyone => "@room".to_string(),
        })
        .collect()
}

fn find(members: &[Member], query: &str, room: bool) -> Vec<String> {
    people(&match_members(members, query, &user("me"), room))
}

fn crowd(n: usize) -> Vec<Member> {
    (0..n)
        .map(|i| member(&format!("user{i:02}"), &format!("Person {i:02}")))
        .collect()
}

// ------------------------------------------------------------ fold

#[test]
fn fold_drops_accents_and_case() {
    assert_eq!(fold("Élodie"), "elodie");
    assert_eq!(fold("ÀÉÎÕÜ çà"), "aeiou ca");
}

#[test]
fn fold_keeps_letters_that_do_not_decompose() {
    assert_eq!(fold("Ørsted"), "ørsted");
}

// ------------------------------------------------------------ extract: who is notified

#[test]
fn a_picked_person_still_in_the_text_is_mentioned() {
    assert_eq!(
        extract("hi bob", &[bob()]),
        Mentions {
            user_ids: vec![user("bob")],
            room: false
        }
    );
}

#[test]
fn a_name_erased_from_the_text_notifies_nobody() {
    assert_eq!(extract("hi there", &[bob()]), Mentions::default());
}

#[test]
fn each_person_once_in_the_order_picked() {
    let picked = [
        Mention::user(user("carol"), "carol"),
        bob(),
        Mention::user(user("carol"), "carol"),
    ];
    assert_eq!(
        extract("bob and carol", &picked).user_ids,
        vec![user("carol"), user("bob")]
    );
}

#[test]
fn everyone_sets_room_and_adds_no_user() {
    assert_eq!(
        extract("hey @room", &[Mention::everyone()]),
        Mentions {
            user_ids: vec![],
            room: true
        }
    );
}

#[test]
fn everyone_erased_from_the_text_does_not_set_room() {
    assert!(!extract("hey", &[Mention::everyone()]).room);
}

#[test]
fn a_linked_room_notifies_nobody() {
    let picked = [Mention::room(alias("target"), "#target:localhost")];
    assert_eq!(
        extract("see #target:localhost", &picked),
        Mentions::default()
    );
}

#[test]
fn at_most_25_people_are_mentioned_the_first_picked() {
    let picked: Vec<Mention> = (0..30)
        .map(|i| Mention::user(user(&format!("u{i:02}")), &format!("n{i:02}x")))
        .collect();
    let text: String = picked.iter().map(|m| format!("{} ", m.name)).collect();
    let got = extract(&text, &picked);
    assert_eq!(MAX_MENTIONS, 25);
    assert_eq!(got.user_ids.len(), 25);
    assert_eq!(got.user_ids[24], user("u24"));
    assert_eq!(got.user_ids[0], user("u00"));
}

#[test]
fn people_beyond_25_do_not_hide_everyone() {
    let mut picked: Vec<Mention> = (0..26)
        .map(|i| Mention::user(user(&format!("u{i:02}")), &format!("n{i:02}x")))
        .collect();
    picked.push(Mention::everyone());
    let text: String = picked.iter().map(|m| format!("{} ", m.name)).collect();
    assert!(extract(&text, &picked).room);
}

// ------------------------------------------------------------ link_names

#[test]
fn a_name_becomes_a_matrix_to_link() {
    let out = link_names(plain("hi bob"), &[bob()]);
    assert_eq!(out.body, "hi bob");
    assert_eq!(
        out.formatted_body.as_deref(),
        Some("hi <a href=\"https://matrix.to/#/%40bob%3Alocalhost\">bob</a>")
    );
}

#[test]
fn no_mention_leaves_the_body_as_it_was() {
    assert_eq!(link_names(plain("hi bob"), &[]), plain("hi bob"));
    assert_eq!(link_names(plain("hi **bob**"), &[]), plain("hi **bob**"));
}

#[test]
fn a_picked_name_erased_links_nothing() {
    assert_eq!(link_names(plain("hi all"), &[bob()]), plain("hi all"));
}

#[test]
fn every_occurrence_is_linked() {
    let out = link_names(plain("bob and bob"), &[bob()]);
    assert_eq!(out.formatted_body.unwrap().matches("<a href").count(), 2);
}

#[test]
fn a_name_glued_to_a_word_is_not_linked() {
    let out = link_names(plain("bobby and sbob"), &[bob()]);
    assert_eq!(out.formatted_body, None);
}

#[test]
fn a_name_is_linked_inside_markdown() {
    let out = link_names(plain("**bob**"), &[bob()]);
    assert_eq!(
        out.formatted_body.as_deref(),
        Some("<strong><a href=\"https://matrix.to/#/%40bob%3Alocalhost\">bob</a></strong>")
    );
}

#[test]
fn a_name_in_code_is_not_linked() {
    let out = link_names(plain("`bob` ok"), &[bob()]);
    assert_eq!(out.formatted_body.as_deref(), Some("<code>bob</code> ok"));
}

#[test]
fn a_name_in_a_link_is_not_linked_again() {
    let out = link_names(plain("[bob](https://a.org)"), &[bob()]);
    assert_eq!(
        out.formatted_body.as_deref(),
        Some("<a href=\"https://a.org\">bob</a>")
    );
}

#[test]
fn the_longest_name_wins() {
    let jean = Mention::user(user("jean"), "Jean");
    let lepierre = Mention::user(user("jl"), "Jean Lepierre");
    let out = link_names(plain("Jean Lepierre!"), &[jean, lepierre]);
    assert_eq!(
        out.formatted_body.as_deref(),
        Some("<a href=\"https://matrix.to/#/%40jl%3Alocalhost\">Jean Lepierre</a>!")
    );
}

#[test]
fn a_newline_stays_a_break_around_a_link() {
    let out = link_names(plain("hi\nbob"), &[bob()]);
    assert_eq!(
        out.formatted_body.as_deref(),
        Some("hi<br><a href=\"https://matrix.to/#/%40bob%3Alocalhost\">bob</a>")
    );
}

#[test]
fn text_around_a_name_is_escaped() {
    let out = link_names(plain("<b>x</b> bob"), &[bob()]);
    assert_eq!(
        out.formatted_body.as_deref(),
        Some("&lt;b&gt;x&lt;/b&gt; <a href=\"https://matrix.to/#/%40bob%3Alocalhost\">bob</a>")
    );
}

#[test]
fn a_room_is_linked_by_its_alias() {
    let room = Mention::room(alias("general"), "#general:localhost");
    let out = link_names(plain("go #general:localhost"), &[room]);
    assert_eq!(
        out.formatted_body.as_deref(),
        Some("go <a href=\"https://matrix.to/#/%23general%3Alocalhost\">#general:localhost</a>")
    );
}

#[test]
fn everyone_stays_text() {
    let out = link_names(plain("hey @room"), &[Mention::everyone()]);
    assert_eq!(out, plain("hey @room"));
}

// ------------------------------------------------------------ people after "@"

#[test]
fn a_prefix_of_the_name_finds_the_person() {
    let members = [member("bob", "Bob Stone"), member("amy", "Amy Fox")];
    assert_eq!(find(&members, "bo", false), ["Bob Stone"]);
}

#[test]
fn accents_and_case_are_ignored_on_both_sides() {
    let members = [member("elo", "Élodie")];
    assert_eq!(find(&members, "ELO", false), ["Élodie"]);
    assert_eq!(find(&[member("x", "Elodie")], "éLo", false), ["Elodie"]);
}

#[test]
fn a_word_inside_the_name_starts_with_the_query() {
    let members = [member("al", "Alice Stone")];
    assert_eq!(find(&members, "sto", false), ["Alice Stone"]);
}

#[test]
fn the_start_of_a_word_comes_before_a_mere_inside() {
    let members = [member("joann", "Joann"), member("ann", "Ann Lee")];
    assert_eq!(find(&members, "ann", false), ["Ann Lee", "Joann"]);
}

#[test]
fn equal_scores_are_ordered_by_name() {
    let members = [member("zoe", "Zoe Ba"), member("abe", "Abe Ba")];
    assert_eq!(find(&members, "ba", false), ["Abe Ba", "Zoe Ba"]);
}

#[test]
fn the_user_is_never_offered_to_themselves() {
    let members = [member("me", "Me Myself"), member("meg", "Meg")];
    assert_eq!(find(&members, "me", false), ["Meg"]);
}

#[test]
fn the_local_part_of_the_id_finds_the_person() {
    let members = [member("kdubois", "Kevin")];
    assert_eq!(find(&members, "dubo", false), ["Kevin"]);
}

#[test]
fn a_full_matrix_id_finds_the_person() {
    let members = [member("bob", "Robert"), member("amy", "Amy")];
    assert_eq!(find(&members, "@bob:localhost", false), ["Robert"]);
    assert_eq!(find(&members, "bob:localhost", false), ["Robert"]);
}

#[test]
fn the_matrix_name_is_searched_too() {
    let m = Member {
        user_id: user("jd"),
        name: "Johnny".into(),
        matrix_name: Some("Jonathan Doe".into()),
    };
    assert_eq!(find(&[m], "doe", false), ["Johnny"]);
}

#[test]
fn nobody_matches_nothing() {
    assert!(find(&[member("bob", "Bob")], "zzz", true).is_empty());
}

#[test]
fn a_bare_at_offers_25_people_not_more() {
    let got = find(&crowd(30), "", false);
    assert_eq!(got.len(), MAX_MENTIONS);
    assert_eq!(got[0], "Person 00");
    assert_eq!(got[24], "Person 24");
}

#[test]
fn a_bare_at_with_few_people_offers_them_all() {
    assert_eq!(find(&crowd(3), "", false).len(), 3);
}

// ------------------------------------------------------------ @room

#[test]
fn a_bare_at_ends_with_everyone_for_who_may_notify() {
    let got = find(&crowd(3), "", true);
    assert_eq!(got.len(), 4);
    assert_eq!(got[3], "@room");
}

#[test]
fn everyone_comes_after_the_25_people() {
    let got = find(&crowd(30), "", true);
    assert_eq!(got.len(), 26);
    assert_eq!(got[25], "@room");
    assert_eq!(got[24], "Person 24");
}

#[test]
fn a_bare_at_without_anyone_offers_no_everyone() {
    assert!(find(&[], "", true).is_empty());
}

#[test]
fn everyone_is_not_offered_to_who_may_not_notify() {
    assert!(!find(&crowd(3), "", false).contains(&"@room".to_string()));
    assert!(!find(&crowd(3), "all", false).contains(&"@room".to_string()));
}

#[test]
fn a_start_of_room_all_or_everyone_puts_everyone_first() {
    for query in ["r", "ro", "room", "a", "all", "e", "everyone"] {
        let got = find(
            &[member("allan", "Allan"), member("ron", "Ron")],
            query,
            true,
        );
        assert_eq!(got[0], "@room", "{query}");
    }
}

#[test]
fn everyone_first_even_when_nobody_matches() {
    assert_eq!(find(&[member("bob", "Bob")], "every", true), ["@room"]);
}

#[test]
fn another_query_does_not_offer_everyone() {
    let got = find(&[member("bob", "Bob")], "b", true);
    assert_eq!(got, ["Bob"]);
}

// ------------------------------------------------------------ rooms after "#"

fn room(local: &str, name: &str) -> LinkableRoom {
    LinkableRoom {
        alias: alias(local),
        name: name.into(),
    }
}

fn room_names<'a>(found: &[&'a LinkableRoom]) -> Vec<&'a str> {
    found.iter().map(|r| r.name.as_str()).collect()
}

#[test]
fn a_room_is_found_by_a_word_of_its_name() {
    let rooms = [room("target", "Target Team"), room("misc", "Misc")];
    assert_eq!(room_names(&match_rooms(&rooms, "tea")), ["Target Team"]);
}

#[test]
fn a_room_is_found_by_its_alias() {
    let rooms = [room("project-alpha", "Alpha Room")];
    assert_eq!(room_names(&match_rooms(&rooms, "project")), ["Alpha Room"]);
}

#[test]
fn rooms_ignore_accents_and_case() {
    let rooms = [room("cafe", "Café du Coin")];
    assert_eq!(room_names(&match_rooms(&rooms, "CAFE")), ["Café du Coin"]);
}

#[test]
fn a_start_of_word_comes_before_an_inside_for_rooms() {
    let rooms = [room("inside", "Reteam"), room("start", "Team Two")];
    assert_eq!(
        room_names(&match_rooms(&rooms, "tea")),
        ["Team Two", "Reteam"]
    );
}

#[test]
fn a_bare_hash_offers_at_most_25_rooms() {
    let rooms: Vec<LinkableRoom> = (0..30)
        .map(|i| room(&format!("r{i:02}"), &format!("Room {i:02}")))
        .collect();
    assert_eq!(match_rooms(&rooms, "").len(), 25);
}

// ------------------------------------------------------------ properties

proptest! {
    #[test]
    fn mentions_never_exceed_25_people_and_are_unique(count in 0usize..60) {
        let picked: Vec<Mention> = (0..count)
            .map(|i| Mention::user(user(&format!("u{}", i % 40)), &format!("name{} ", i % 40)))
            .collect();
        let text: String = picked.iter().map(|m| m.name.clone()).collect();
        let got = extract(&text, &picked).user_ids;
        prop_assert!(got.len() <= MAX_MENTIONS);
        let mut seen = got.clone();
        seen.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        seen.dedup();
        prop_assert_eq!(seen.len(), got.len());
    }

    #[test]
    fn linking_never_leaks_a_tag_of_the_user(text in "[a-z <>&\"/]{0,30}") {
        let out = link_names(to_message(&format!("{text} bob")), &[bob()]);
        let html = out.formatted_body.unwrap_or_default();
        prop_assert!(!html.contains("<script"));
        prop_assert!(!html.contains("<img"));
        prop_assert_eq!(out.body, format!("{text} bob"));
    }

    #[test]
    fn the_member_list_never_exceeds_26_and_never_holds_the_user(
        count in 0usize..60,
        query in "[a-z0-9 ]{0,4}",
    ) {
        let mut members = crowd(count);
        members.push(member("me", "Person me"));
        let got = match_members(&members, &query, &user("me"), true);
        prop_assert!(got.len() <= MAX_MENTIONS + 1);
        prop_assert!(!got.iter().any(|c| matches!(c, MentionCandidate::Person(m) if m.user_id == user("me"))));
    }
}
