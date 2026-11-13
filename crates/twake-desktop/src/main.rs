//! Composition root: builds the runtime, wires the adapters and opens the main window.

use gallery::Gallery;
use preview::Preview;
use twake_gpui::{
    App, Application, Bounds, Context, IntoElement, Render, TitlebarOptions, Window,
    WindowAppearance, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use twake_tokens::{LIGHT, Rgba8};
use twake_ui_kit::view::IconAssets;

mod gallery;
mod gallery_parts;
mod preview;
mod preview_data;

const APP_TITLE: &str = "Twake Chat";
const WINDOW_SIZE: (f32, f32) = (1100.0, 720.0);
const GALLERY_SIZE: (f32, f32) = (1800.0, 960.0);

fn packed(color: Rgba8) -> u32 {
    u32::from(color.r) << 16 | u32::from(color.g) << 8 | u32::from(color.b)
}

struct Root;

impl Render for Root {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .size_full()
            .justify_center()
            .items_center()
            .bg(rgb(packed(LIGHT.background.paper)))
            .text_color(rgb(packed(LIGHT.text.primary)))
            .text_xl()
            .child(APP_TITLE)
    }
}

// Start-up is the one place where `expect` is allowed (ARCHITECTURE section 4).
#[allow(clippy::expect_used)]
fn main() {
    let gallery = std::env::args().any(|arg| arg == "--gallery");
    let preview = std::env::args().any(|arg| arg == "--preview");
    let bench = std::env::var_os("TWAKE_PREVIEW_BENCH").is_some();
    Application::new()
        .with_assets(IconAssets)
        .run(move |cx: &mut App| {
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
                cx.open_window(options, |_, cx| cx.new(|_| Root))
                    .expect("open the main window");
            }
            cx.activate(true);
        });
}
