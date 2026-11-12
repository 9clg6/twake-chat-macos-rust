//! `MxcUri`, `TxnId` and `Timestamp`: small values parsed once at the boundary.
//!
//! Need: Matrix spec, "Matrix Content (mxc) URIs" (`mxc://<server-name>/<media-id>`) and
//! the client-server API `txnId` of a send; `docs/spec/features/roomlist.md` D31 (avatar of a
//! row) and `timeline.md` D495 (a message in flight is known by its transaction id only).
//! Public items used are listed in `docs/dev/app-core-api-sketch.md`.

#![cfg(test)]

use twake_domain::{IdError, MxcUri, Timestamp, TxnId};

#[test]
fn an_mxc_uri_gives_its_server_and_its_media_id() {
    let uri = MxcUri::parse("mxc://matrix.example.org/AbCd1234").expect("valid");
    assert_eq!(uri.server_name(), "matrix.example.org");
    assert_eq!(uri.media_id(), "AbCd1234");
    assert_eq!(uri.as_str(), "mxc://matrix.example.org/AbCd1234");
}

#[test]
fn an_mxc_uri_keeps_the_port_of_the_server() {
    let uri = MxcUri::parse("mxc://localhost:8008/xyz").expect("valid");
    assert_eq!(uri.server_name(), "localhost:8008");
    assert_eq!(uri.media_id(), "xyz");
}

#[test]
fn an_empty_mxc_uri_is_empty() {
    assert_eq!(MxcUri::parse(""), Err(IdError::Empty));
}

#[test]
fn an_mxc_uri_refuses_another_scheme() {
    assert!(MxcUri::parse("https://matrix.example.org/AbCd1234").is_err());
    assert!(MxcUri::parse("matrix.example.org/AbCd1234").is_err());
}

#[test]
fn an_mxc_uri_needs_a_server_and_a_media_id() {
    for bad in ["mxc://", "mxc://server", "mxc://server/", "mxc:///media"] {
        assert!(MxcUri::parse(bad).is_err(), "{bad:?} must be refused");
    }
}

#[test]
fn an_mxc_uri_media_id_is_one_path_segment_without_blanks() {
    for bad in ["mxc://server/a/b", "mxc://server/a b", "mxc://server/a\nb"] {
        assert!(MxcUri::parse(bad).is_err(), "{bad:?} must be refused");
    }
}

#[test]
fn a_transaction_id_keeps_its_text() {
    let txn = TxnId::parse("txn-17").expect("valid");
    assert_eq!(txn.as_str(), "txn-17");
}

#[test]
fn an_empty_transaction_id_is_refused() {
    assert_eq!(TxnId::parse(""), Err(IdError::Empty));
}

#[test]
fn a_transaction_id_with_a_blank_is_refused() {
    assert!(TxnId::parse("txn 17").is_err());
    assert!(TxnId::parse("txn\t17").is_err());
}

#[test]
fn two_transaction_ids_with_the_same_text_are_equal() {
    assert_eq!(
        TxnId::parse("a1").expect("valid"),
        TxnId::parse("a1").expect("valid")
    );
    assert_ne!(
        TxnId::parse("a1").expect("valid"),
        TxnId::parse("a2").expect("valid")
    );
}

#[test]
fn a_timestamp_gives_back_its_milliseconds() {
    assert_eq!(
        Timestamp::from_millis(1_700_000_000_123).as_millis(),
        1_700_000_000_123
    );
}

#[test]
fn timestamps_are_ordered_by_time() {
    assert!(Timestamp::from_millis(10) < Timestamp::from_millis(11));
    assert!(Timestamp::from_millis(2_000) > Timestamp::from_millis(1_999));
    assert_eq!(Timestamp::from_millis(5), Timestamp::from_millis(5));
}
