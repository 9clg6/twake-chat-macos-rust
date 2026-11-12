//! Composition root: builds the runtime, wires the adapters and opens the main window.

use twake_gpui::{
    App, Application, Bounds, Context, IntoElement, Render, TitlebarOptions, Window, WindowBounds,
    WindowOptions, div, prelude::*, px, rgb, size,
};
use twake_tokens::THEME;

const APP_TITLE: &str = "Twake Chat";
const WINDOW_SIZE: (f32, f32) = (1100.0, 720.0);

struct Root;

impl Render for Root {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .size_full()
            .justify_center()
            .items_center()
            .bg(rgb(THEME.background))
            .text_color(rgb(THEME.text))
            .text_xl()
            .child(APP_TITLE)
    }
}

// Start-up is the one place where `expect` is allowed (ARCHITECTURE section 4).
#[allow(clippy::expect_used)]
fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(WINDOW_SIZE.0), px(WINDOW_SIZE.1)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(APP_TITLE.into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| Root),
        )
        .expect("open the main window");
        cx.activate(true);
    });
}
