//! Compile-only probe of the matrix-sdk 0.19.1 API shape assumed by ARCHITECTURE.md section 7.
//! Never run in CI: it builds the types but performs no network call (spike S2).
#![recursion_limit = "256"] // matrix-sdk futures overflow the default layout depth of 128
#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use matrix_sdk::Client;
use matrix_sdk::authentication::matrix::MatrixSession;
use matrix_sdk_ui::room_list_service::RoomListService;
use matrix_sdk_ui::sync_service::{State, SyncService};
use matrix_sdk_ui::timeline::RoomExt;

async fn build(dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .homeserver_url("https://localhost.invalid")
        .sqlite_store(dir, Some("passphrase-from-keychain"))
        .build()
        .await?;

    // Session restore signature (device id reuse, section 7).
    let _restore: Option<MatrixSession> = None;

    // The SDK's sliding sync owner: room-list sync + encryption sync.
    let sync: SyncService = SyncService::builder(client.clone())
        .with_offline_mode()
        .build()
        .await?;
    let _state: matrix_sdk::reqwest::Url; // reqwest re-export check
    let _s: State = sync.state().get();
    let _rooms: std::sync::Arc<RoomListService> = sync.room_list_service();

    // Timeline from a room.
    if let Some(room) = client.get_room(matrix_sdk::ruma::room_id!("!a:b.invalid")) {
        let _t = room.timeline().await?;
    }
    sync.start().await;
    sync.stop().await;
    Ok(())
}

fn main() {
    // Reference only: not executed.
    let _ = build;
}
