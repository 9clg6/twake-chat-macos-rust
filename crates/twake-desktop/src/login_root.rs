//! The sign-in window: draws a `LoginScreenView` and sends what the user does to the controller.
//! No rule here (ARCHITECTURE section 6): the texts and the buttons come from `twake_ui`.

use std::sync::Arc;

use futures::channel::mpsc::Sender;
use tokio::sync::watch;
use twake_app::login_screen::LoginView;
use twake_gpui::{
    AppContext, Context, Entity, FontWeight, IntoElement, ParentElement, Render, Styled, Window,
    div, px,
};
use twake_i18n::Locale;
use twake_ui::login_screen::{LoginAction, LoginButton, LoginScreenView};
use twake_ui_kit::button::{ButtonColor, ButtonVariant};
use twake_ui_kit::fonts::FAMILY;
use twake_ui_kit::theme::Theme;
use twake_ui_kit::view::{Submit, TextInput, button, hsla};

use crate::controller::Command;

const CARD_WIDTH: f32 = 420.0;

/// The sign-in screen, following the view the controller publishes.
pub struct LoginRoot {
    shown: LoginScreenView,
    server: Entity<TextInput>,
    commands: Sender<Command>,
    locale: Arc<Locale>,
    theme: Theme,
}

impl LoginRoot {
    /// The screen of the view the controller has published so far.
    pub fn new(
        mut views: watch::Receiver<LoginView>,
        commands: Sender<Command>,
        locale: Arc<Locale>,
        theme: Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let server = cx.new(|cx| TextInput::new(theme, "https://matrix.example.org", window, cx));
        cx.subscribe(&server, |this, input, _: &Submit, cx| {
            let text = input.read(cx).text().to_owned();
            let _ = this.commands.try_send(Command::Server(text));
        })
        .detach();
        let shown = LoginScreenView::from(&views.borrow_and_update().clone(), &locale);
        cx.spawn(async move |this, cx| {
            while views.changed().await.is_ok() {
                let view = views.borrow_and_update().clone();
                if this.update(cx, |me, cx| me.show(&view, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            shown,
            server,
            commands,
            locale,
            theme,
        }
    }

    fn show(&mut self, view: &LoginView, cx: &mut Context<Self>) {
        self.shown = LoginScreenView::from(view, &self.locale);
        cx.notify();
    }

    fn button(&self, index: usize, spec: &LoginButton) -> impl IntoElement {
        let commands = self.commands.clone();
        let server = self.server.clone();
        let action = spec.action;
        let main = index == 0;
        button(format!("login-{index}"), spec.label.clone(), self.theme)
            .variant(if main {
                ButtonVariant::Contained
            } else {
                ButtonVariant::Text
            })
            .color(ButtonColor::Primary)
            .on_click(move |_, _, cx| {
                let mut commands = commands.clone();
                let command = match action {
                    LoginAction::SubmitServer => Command::Server(server.read(cx).text().to_owned()),
                    other => Command::Action(other),
                };
                let _ = commands.try_send(command);
            })
    }
}

impl Render for LoginRoot {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let colors = self.theme.colors();
        let asks_server = self
            .shown
            .buttons
            .iter()
            .any(|b| b.action == LoginAction::SubmitServer);
        let mut card = div().flex().flex_col().gap_3().w(px(CARD_WIDTH)).child(
            div()
                .text_xl()
                .font_weight(FontWeight::SEMIBOLD)
                .child(self.shown.title.clone()),
        );
        if let Some(message) = &self.shown.message {
            card = card.child(
                div()
                    .text_color(hsla(colors.text.secondary))
                    .child(message.clone()),
            );
        }
        if asks_server {
            card = card.child(self.server.clone());
        }
        if let Some(problem) = &self.shown.problem {
            card = card.child(
                div()
                    .text_color(hsla(colors.error.main))
                    .child(problem.clone()),
            );
        }
        if self.shown.busy {
            card = card.child(div().text_color(hsla(colors.text.secondary)).child("…"));
        }
        let mut row = div().flex().gap_2();
        for (index, spec) in self.shown.buttons.iter().enumerate() {
            row = row.child(self.button(index, spec));
        }
        div()
            .font_family(FAMILY)
            .flex()
            .size_full()
            .justify_center()
            .items_center()
            .bg(hsla(colors.background.default))
            .text_color(hsla(colors.text.primary))
            .child(card.child(row))
    }
}
