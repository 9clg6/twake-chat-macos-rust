//! GPUI drawing of the pure half: `button(...)` applies a `ResolvedButton`, `icon(...)` draws an `IconSpec`.
//! No test draws these (a window is needed); the styles they apply are tested in `button` and `icon`.

use std::borrow::Cow;

mod parts;
mod text_element;
mod text_input;
pub mod text_keys;
mod text_layout;

pub use parts::{
    chip, dialog, icon_button, menu_item, menu_panel, menu_separator, presence_dot, tooltip,
};
pub use text_input::{Submit, TextInput};

use twake_gpui::{
    App, AssetSource, Background, BoxShadow, ClickEvent, FocusHandle, FontWeight, Hsla,
    InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, Svg, ViewElement, Window, div, linear_color_stop,
    linear_gradient, point, px, svg,
};
use twake_icons::by_id;
use twake_tokens::{Gradient, GradientStop, LineHeight, Rgba8};

use crate::avatar;
use crate::button::{
    ButtonColor, ButtonSize, ButtonState, ButtonStyle, ButtonVariant, ResolvedButton,
};
use crate::color::to_gpui_rgba;
use crate::icon::{IconColor, IconSpec};
use crate::theme::Theme;

type ClickListener = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

const ICON_DIR: &str = "icons/";

/// A token colour as a GPUI colour.
pub fn hsla(color: Rgba8) -> Hsla {
    to_gpui_rgba(color).into()
}

/// The background of the avatar of this index: its gradient of twake-mui.
pub fn avatar_background(index: usize) -> Background {
    let Gradient {
        angle_deg,
        from,
        to,
    } = *avatar::gradient(index);
    let stop = |s: GradientStop| linear_color_stop(hsla(s.color), s.at_percent / 100.0);
    linear_gradient(angle_deg, stop(from), stop(to))
}

/// Serves `icons/<id>.svg` from `twake-icons`: GPUI 0.2.2 loads an SVG only by asset path.
pub struct IconAssets;

impl AssetSource for IconAssets {
    fn load(&self, path: &str) -> twake_gpui::Result<Option<Cow<'static, [u8]>>> {
        let icon = path
            .strip_prefix(ICON_DIR)
            .and_then(|name| name.strip_suffix(".svg"))
            .and_then(by_id);
        Ok(icon.map(|icon| Cow::Owned(icon.to_svg().into_bytes())))
    }

    fn list(&self, _path: &str) -> twake_gpui::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

/// An icon drawn as a mask in `Fixed` colour. `Current` sets none: GPUI 0.2.2 does not hand the text
/// colour of the parent to an `svg()`, so the caller sets `text_color` (see docs/dev/gpui-notes.md).
/// GPUI paints an SVG as an alpha mask: a multicolour icon (A013) comes out in one colour.
pub fn icon(spec: &IconSpec) -> Svg {
    let drawn = svg()
        .path(format!("{ICON_DIR}{}.svg", spec.icon.id))
        .size(px(spec.size_px));
    match spec.color {
        IconColor::Current => drawn,
        IconColor::Fixed(color) => drawn.text_color(hsla(color)),
    }
}

/// A twake-mui button.
pub struct Button {
    id: SharedString,
    label: SharedString,
    variant: ButtonVariant,
    color: ButtonColor,
    size: ButtonSize,
    theme: Theme,
    disabled: bool,
    shown: Option<ButtonState>,
    focus: Option<FocusHandle>,
    on_click: Option<ClickListener>,
}

/// A medium contained primary button; chain the setters for the rest.
pub fn button(id: impl Into<SharedString>, label: impl Into<SharedString>, theme: Theme) -> Button {
    Button {
        id: id.into(),
        label: label.into(),
        variant: ButtonVariant::Contained,
        color: ButtonColor::Primary,
        size: ButtonSize::Medium,
        theme,
        disabled: false,
        shown: None,
        focus: None,
        on_click: None,
    }
}

impl Button {
    /// Fill style.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Tone.
    pub fn color(mut self, color: ButtonColor) -> Self {
        self.color = color;
        self
    }

    /// Size.
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// A disabled button takes no hover and no click.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Draws the button frozen in this state (the gallery); hover and click stay live.
    pub fn shown_as(mut self, state: ButtonState) -> Self {
        self.shown = Some(state);
        self
    }

    /// Tracks this handle and shows the ring while it is focused.
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus = Some(handle);
        self
    }

    /// Click listener.
    pub fn on_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(listener));
        self
    }

    fn resolve(&self, state: ButtonState) -> ResolvedButton {
        ButtonStyle::resolve(self.variant, self.color, self.size, state, &self.theme)
    }
}

fn line_height(font_size: f32, height: LineHeight) -> f32 {
    match height {
        LineHeight::Ratio(ratio) => font_size * ratio,
        LineHeight::Px(px) => px,
    }
}

// `#[derive(IntoElement)]` names the `gpui` crate, which only `twake-gpui` may depend on (A010).
impl IntoElement for Button {
    type Element = ViewElement<Self>;

    fn into_element(self) -> Self::Element {
        ViewElement::new(self)
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let state = match (self.disabled, self.shown) {
            (true, _) => ButtonState::Disabled,
            (false, shown) => shown.unwrap_or(ButtonState::Rest),
        };
        let rest = self.resolve(state);
        let font = rest.font;
        let mut base = div()
            .id(self.id.clone())
            .flex()
            .items_center()
            .justify_center()
            .min_h(px(rest.height_px))
            .px(px(rest.padding_x))
            .py(px(rest.padding_y))
            .rounded(px(rest.radius_px))
            .bg(hsla(rest.background))
            .text_color(hsla(rest.foreground))
            .text_size(px(font.font_size))
            .font_weight(FontWeight(f32::from(font.font_weight)))
            .line_height(px(line_height(font.font_size, font.line_height)));
        if let Some(border) = rest.border {
            base = base
                .border(px(border.width_px))
                .border_color(hsla(border.color));
        }
        base = parts::ringed(base, rest.focus_ring);
        if self.disabled {
            return base
                .cursor_not_allowed()
                .child(self.label)
                .into_any_element();
        }
        let hover = self.resolve(ButtonState::Hover);
        let pressed = self.resolve(ButtonState::Pressed);
        let ring = self.resolve(ButtonState::FocusVisible).focus_ring;
        let mut base = base
            .cursor_pointer()
            .hover(move |style| style.bg(hsla(hover.background)))
            .active(move |style| style.bg(hsla(pressed.background)));
        if let (Some(handle), Some(ring)) = (self.focus.as_ref(), ring) {
            base = base.track_focus(handle).focus(move |style| {
                // Inset: an outer spread would show through a translucent fill and overflow the box.
                style.shadow(vec![BoxShadow {
                    color: hsla(ring.color),
                    offset: point(px(0.0), px(0.0)),
                    blur_radius: px(0.0),
                    spread_radius: px(ring.width_px),
                    inset: true,
                }])
            });
        }
        if let Some(listener) = self.on_click {
            base = base.on_click(listener);
        }
        base.child(self.label).into_any_element()
    }
}
