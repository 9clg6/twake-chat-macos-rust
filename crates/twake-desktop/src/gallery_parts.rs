//! The component half of `--gallery`: ghost buttons, icon buttons, chips, a menu, a tooltip, the presence
//! dot and a dialog, each in its states (rest, hover, keyboard focus, disabled).

use twake_gpui::{Div, IntoElement, ParentElement, Styled, div, px, rgb};
use twake_icons::{COPY, CROSS_SMALL, SEND};
use twake_ui_kit::button::{ButtonColor, ButtonState, ButtonVariant};
use twake_ui_kit::chip::{ChipColor, ChipProps, ChipState, ChipVariant};
use twake_ui_kit::dialog::{DialogSize, DialogStyle};
use twake_ui_kit::icon_button::{IconButtonColor, IconButtonSize, IconButtonState};
use twake_ui_kit::menu::MenuItemState;
use twake_ui_kit::presence::{PresenceDot, PresenceState};
use twake_ui_kit::theme::Theme;
use twake_ui_kit::view::{
    avatar_background, button, chip, dialog, hsla, icon_button, menu_item, menu_panel,
    menu_separator, presence_dot, tooltip,
};

const BUTTON_STATES: [(ButtonState, &str); 4] = [
    (ButtonState::Rest, "Rest"),
    (ButtonState::Hover, "Hover"),
    (ButtonState::FocusVisible, "Focus"),
    (ButtonState::Disabled, "Disabled"),
];
const ICON_STATES: [IconButtonState; 4] = [
    IconButtonState::Rest,
    IconButtonState::Hover,
    IconButtonState::FocusVisible,
    IconButtonState::Disabled,
];
const CHIP_STATES: [ChipState; 4] = [
    ChipState::Rest,
    ChipState::Hover,
    ChipState::FocusVisible,
    ChipState::Disabled,
];

fn row(label: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(div().w(px(130.0)).text_sm().child(label.to_owned()))
}

fn title(text: &str) -> Div {
    div().text_lg().mt_2().child(text.to_owned())
}

fn ghost_rows(theme: Theme) -> Div {
    let mut col = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(title("Button, ghost"));
    for (color, name) in [
        (ButtonColor::Primary, "Primary"),
        (ButtonColor::Success, "Success"),
        (ButtonColor::Warning, "Warning"),
        (ButtonColor::Error, "Error"),
    ] {
        let mut r = row(name);
        for (state, label) in BUTTON_STATES {
            let id = format!("ghost-{name}-{label}");
            r = r.child(
                button(id, label, theme)
                    .variant(ButtonVariant::Ghost)
                    .color(color)
                    .shown_as(state)
                    .disabled(state == ButtonState::Disabled),
            );
        }
        col = col.child(r);
    }
    col
}

fn icon_button_rows(theme: Theme) -> Div {
    let mut col = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(title("Icon button (medium)"));
    for (color, name) in [
        (IconButtonColor::Default, "Default"),
        (IconButtonColor::Primary, "Primary"),
        (IconButtonColor::Error, "Error"),
    ] {
        let mut r = row(name);
        for state in ICON_STATES {
            r = r.child(icon_button(
                &SEND,
                16.0,
                (IconButtonSize::Medium, color, state),
                &theme,
            ));
        }
        col = col.child(r);
    }
    col
}

fn chip_rows(theme: Theme) -> Div {
    let mut col = div().flex().flex_col().gap_2().child(title("Chip"));
    let base = ChipProps {
        clickable: true,
        ..ChipProps::default()
    };
    let variants = [
        ("Filled", base),
        (
            "Filled primary",
            ChipProps {
                color: ChipColor::Primary,
                ..base
            },
        ),
        (
            "Outlined",
            ChipProps {
                variant: ChipVariant::Outlined,
                ..base
            },
        ),
        (
            "Outlined success",
            ChipProps {
                variant: ChipVariant::Outlined,
                color: ChipColor::Success,
                ..base
            },
        ),
        (
            "Deletable",
            ChipProps {
                deletable: true,
                ..base
            },
        ),
    ];
    for (name, props) in variants {
        let mut r = row(name);
        for state in CHIP_STATES {
            r = r.child(chip("Label", props, state, &theme));
        }
        col = col.child(r);
    }
    col
}

fn avatar(index: usize, state: Option<PresenceState>, theme: &Theme) -> Div {
    let base = div()
        .relative()
        .size(px(48.0))
        .rounded_full()
        .bg(avatar_background(index));
    match PresenceDot::resolve(state, 48.0, theme) {
        Some(dot) => base.child(presence_dot(&dot)),
        None => base,
    }
}

fn menu_and_friends(theme: Theme) -> Div {
    let mut menu = menu_panel(&theme).w(px(220.0));
    for (label, state) in [
        ("Rest", MenuItemState::Rest),
        ("Hover", MenuItemState::Hover),
        ("Selected", MenuItemState::Selected),
        ("Keyboard focus", MenuItemState::FocusVisible),
    ] {
        menu = menu.child(menu_item(label, Some(&COPY), state, &theme));
    }
    menu = menu.child(menu_separator(&theme)).child(menu_item(
        "Disabled",
        Some(&CROSS_SMALL),
        MenuItemState::Disabled,
        &theme,
    ));
    div().flex().items_start().gap_6().child(menu).child(
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(tooltip("Send message", &theme))
            .child(
                div()
                    .flex()
                    .gap_4()
                    .child(avatar(0, Some(PresenceState::Online), &theme))
                    .child(avatar(3, Some(PresenceState::Away), &theme))
                    .child(avatar(6, Some(PresenceState::Offline), &theme)),
            ),
    )
}

fn dialog_card(theme: Theme) -> Div {
    let backdrop = DialogStyle::resolve(DialogSize::Small, 1000.0, &theme).backdrop;
    let actions = vec![
        button("dialog-cancel", "Cancel", theme)
            .variant(ButtonVariant::Text)
            .into_any_element(),
        button("dialog-ok", "Confirm", theme).into_any_element(),
    ];
    div().p_6().bg(hsla(backdrop)).child(dialog(
        "Delete chat?",
        "The history is hidden for you alone.",
        actions,
        (DialogSize::Small, 1000.0),
        &theme,
    ))
}

/// Every component of the second lot, in two columns.
pub fn components(theme: Theme) -> Div {
    let colors = theme.colors();
    let column = || div().flex().flex_col().gap_3();
    div()
        .flex()
        .gap_8()
        .text_color(rgb(crate::gallery::packed(colors.text.primary)))
        .child(
            column()
                .child(ghost_rows(theme))
                .child(icon_button_rows(theme))
                .child(chip_rows(theme)),
        )
        .child(
            column()
                .child(title("Menu, tooltip, presence"))
                .child(menu_and_friends(theme))
                .child(title("Dialog"))
                .child(dialog_card(theme)),
        )
}
