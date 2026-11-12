//! Composition root: builds the runtime, wires the adapters and opens the main window.

use gallery::Gallery;
use twake_gpui::{
    App, Application, Bounds, Context, IntoElement, Render, TitlebarOptions, Window,
    WindowAppearance, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use twake_tokens::{LIGHT, Rgba8};
use twake_ui_kit::view::IconAssets;

mod gallery;

const APP_TITLE: &str = "Twake Chat";
const WINDOW_SIZE: (f32, f32) = (1100.0, 720.0);

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
    Application::new()
        .with_assets(IconAssets)
        .run(move |cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(WINDOW_SIZE.0), px(WINDOW_SIZE.1)), cx);
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(APP_TITLE.into()),
                    ..Default::default()
                }),
                ..Default::default()
            };
            if gallery {
                let dark = matches!(
                    cx.window_appearance(),
                    WindowAppearance::Dark | WindowAppearance::VibrantDark
                );
                cx.open_window(options, |_, cx| cx.new(|_| Gallery::new(dark)))
                    .expect("open the gallery window");
            } else {
                cx.open_window(options, |_, cx| cx.new(|_| Root))
                    .expect("open the main window");
            }
            cx.activate(true);
        });
}
