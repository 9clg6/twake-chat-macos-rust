//! `--gallery`: every button variant, colour and state, a few icons, the components of `gallery_parts`,
//! and a light/dark switch.

use crate::gallery_parts::components;
use twake_gpui::{
    Context, Div, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, div, px, rgb,
};
use twake_icons::{ASSISTANT_COLOR, CHECK, SEND};
use twake_tokens::Rgba8;
use twake_ui_kit::button::{ButtonColor, ButtonVariant};
use twake_ui_kit::icon::{IconColor, IconSpec};
use twake_ui_kit::theme::Theme;
use twake_ui_kit::view::{button, icon};

const VARIANTS: [(ButtonVariant, &str); 3] = [
    (ButtonVariant::Contained, "Contained"),
    (ButtonVariant::Outlined, "Outlined"),
    (ButtonVariant::Text, "Text"),
];
const COLORS: [(ButtonColor, &str); 3] = [
    (ButtonColor::Primary, "Primary"),
    (ButtonColor::Secondary, "Secondary"),
    (ButtonColor::Error, "Error"),
];

pub(crate) fn packed(color: Rgba8) -> u32 {
    u32::from(color.r) << 16 | u32::from(color.g) << 8 | u32::from(color.b)
}

/// The gallery window.
pub struct Gallery {
    theme: Theme,
}

impl Gallery {
    /// A gallery following the OS appearance (`dark`).
    pub fn new(dark: bool) -> Self {
        Self {
            theme: Theme::system(dark),
        }
    }
}

fn button_rows(theme: Theme) -> Div {
    let mut rows = div().flex().flex_col().gap_3();
    for (variant, variant_name) in VARIANTS {
        for (color, color_name) in COLORS {
            let id = format!("{variant_name}-{color_name}");
            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(div().w(px(160.0)).child(id.clone()))
                    .child(
                        button(format!("{id}-rest"), "Rest", theme)
                            .variant(variant)
                            .color(color),
                    )
                    .child(
                        button(format!("{id}-off"), "Disabled", theme)
                            .variant(variant)
                            .color(color)
                            .disabled(true),
                    ),
            );
        }
    }
    rows
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let colors = theme.colors();
        let rows = button_rows(theme);
        let icons = [&SEND, &CHECK, &ASSISTANT_COLOR].into_iter().fold(
            div().flex().gap_4(),
            |row, glyph| {
                let spec = IconSpec {
                    icon: glyph,
                    size_px: 24.0,
                    color: IconColor::Current,
                };
                row.child(icon(&spec).text_color(rgb(packed(colors.text.icon))))
            },
        );
        div()
            .id("gallery")
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .items_start()
            .gap_4()
            .size_full()
            .p_6()
            .bg(rgb(packed(colors.background.default)))
            .text_color(rgb(packed(colors.text.primary)))
            .child(button("theme", "Light / dark", theme).on_click(cx.listener(
                |this, _, _, cx| {
                    this.theme.toggle();
                    cx.notify();
                },
            )))
            .child(
                div()
                    .flex()
                    .gap_8()
                    .child(div().flex().flex_col().gap_4().child(rows).child(icons))
                    .child(components(theme)),
            )
    }
}
