//! Composition root: builds the runtime, wires the adapters and opens the main window.

use std::path::PathBuf;
use std::sync::Arc;

use gallery::Gallery;
use preview::Preview;
use tokio::runtime::{Builder, Runtime};
use twake_app::boot::Boot;
use twake_app::login_screen::LoginView;
use twake_desktop::backend::MatrixBackend;
use twake_desktop::controller;
use twake_desktop::login_root::LoginRoot;
use twake_domain::session::Secret;
use twake_gpui::{
    App, Bounds, TitlebarOptions, WindowAppearance, WindowBounds, WindowOptions, prelude::*, px,
    size,
};
use twake_i18n::Locale;
use twake_store::{Keychain, KeychainError, KeychainSessionStore, OsKeychain, platform_store};
use twake_ui_kit::fonts;
use twake_ui_kit::theme::Theme;
use twake_ui_kit::view::IconAssets;

mod gallery;
mod gallery_parts;
mod preview;
mod preview_data;

const APP_TITLE: &str = "Twake Chat";
const WINDOW_SIZE: (f32, f32) = (1100.0, 720.0);
const GALLERY_SIZE: (f32, f32) = (1800.0, 960.0);

/// The keychain of the OS; where there is none, one that always fails, so the screen says so
/// (A005: never a plaintext fallback).
struct NoKeychain;

impl Keychain for NoKeychain {
    fn get(&self, _entry: &str) -> Result<Option<Secret>, KeychainError> {
        Err(KeychainError::default())
    }

    fn set(&self, _entry: &str, _value: &Secret) -> Result<(), KeychainError> {
        Err(KeychainError::default())
    }

    fn delete(&self, _entry: &str) -> Result<(), KeychainError> {
        Err(KeychainError::default())
    }
}

/// Where the accounts keep their data: `TWAKE_DATA_DIR`, else the folder of the OS.
/// ponytail: no `dirs` crate; the Windows folder comes with the Windows build.
fn data_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("TWAKE_DATA_DIR") {
        return PathBuf::from(dir);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/twake-chat")
    } else {
        home.join(".local/share/twake-chat")
    }
}

/// The controller of the sign-in on the runtime; its commands and its views.
fn start_boot(
    runtime: &Runtime,
) -> (
    futures::channel::mpsc::Sender<controller::Command>,
    tokio::sync::watch::Receiver<LoginView>,
) {
    let keychain: Arc<dyn Keychain> = match platform_store() {
        Ok(store) => Arc::new(OsKeychain::new(store)),
        Err(_) => Arc::new(NoKeychain),
    };
    let store = Arc::new(KeychainSessionStore::new(Arc::clone(&keychain)));
    let backend = Arc::new(MatrixBackend::new(
        data_root(),
        keychain,
        runtime.handle().clone(),
    ));
    let boot = Boot::new(store, backend);
    let (commands, command_rx) = futures::channel::mpsc::channel(8);
    let (views, view_rx) = tokio::sync::watch::channel(boot.view());
    runtime.spawn(controller::run(boot, command_rx, views));
    (commands, view_rx)
}

// Start-up is the one place where `expect` is allowed (ARCHITECTURE section 4).
#[allow(clippy::expect_used)]
fn main() {
    let gallery = std::env::args().any(|arg| arg == "--gallery");
    let preview = std::env::args().any(|arg| arg == "--preview");
    let bench = std::env::var_os("TWAKE_PREVIEW_BENCH").is_some();
    twake_matrix::install_tls_provider();
    let runtime = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("build the runtime");
    let sign_in = (!gallery && !preview).then(|| start_boot(&runtime));
    twake_gpui::gpui_platform::application()
        .with_assets(IconAssets)
        .run(move |cx: &mut App| {
            fonts::register(cx.text_system()).expect("register the embedded Inter");
            let (width, height) = if gallery { GALLERY_SIZE } else { WINDOW_SIZE };
            let bounds = Bounds::centered(None, size(px(width), px(height)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(APP_TITLE.into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            let dark = matches!(
                cx.window_appearance(),
                WindowAppearance::Dark | WindowAppearance::VibrantDark
            );
            if gallery {
                cx.open_window(options, |_, cx| cx.new(|_| Gallery::new(dark)))
                    .expect("open the gallery window");
            } else if preview {
                cx.open_window(options, |window, cx| {
                    cx.new(|cx| Preview::new(dark, bench, window, cx))
                })
                .expect("open the preview window");
            } else {
                let Some((commands, views)) = sign_in else {
                    return;
                };
                let theme = Theme::system(dark);
                let lang = std::env::var("LANG").unwrap_or_default();
                let locale = Arc::new(Locale::bundled(&lang, 0));
                cx.open_window(options, move |window, cx| {
                    cx.new(|cx| LoginRoot::new(views, commands, locale, theme, window, cx))
                })
                .expect("open the main window");
            }
            cx.activate(true);
        });
}
