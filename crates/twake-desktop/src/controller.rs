//! Drives `Boot` for the sign-in screen: the commands of the screen in, the view out (A034).
//! One task owns `Boot`; the screen only sends commands and shows the last view.

use futures::channel::mpsc::Receiver;
use futures::future::{Either, select};
use futures::{FutureExt, StreamExt};
use tokio::sync::watch;
use twake_app::boot::Boot;
use twake_app::login_screen::LoginView;
use twake_app::session::SessionEvent;
use twake_domain::session::{HomeserverUrl, Secret};
use twake_ui::login_screen::LoginAction;

/// What the screen asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// A button.
    Action(LoginAction),
    /// The server typed, with `SubmitServer`.
    Server(String),
    /// The browser came back with this login token.
    Token(Secret),
}

/// Starts `boot`, then serves `commands` until they end; `shown` always holds the current view.
/// While signed in it also waits for the server to end the session.
/// ponytail: a command that arrives while the watch is rebuilt could hide a 401 for an instant;
/// the next start finds it again.
pub async fn run(mut boot: Boot, mut commands: Receiver<Command>, shown: watch::Sender<LoginView>) {
    boot.start().await;
    loop {
        shown.send_replace(boot.view());
        let next = if matches!(boot.view(), LoginView::SignedIn { .. }) {
            match select(boot.watch_session().boxed(), commands.next()).await {
                // The watch ended with nobody lost (no client): only commands wake us now.
                Either::Left(((), _)) => commands.next().await,
                Either::Right((command, _)) => command,
            }
        } else {
            commands.next().await
        };
        let Some(command) = next else {
            return;
        };
        handle(&mut boot, command).await;
    }
}

async fn handle(boot: &mut Boot, command: Command) {
    match command {
        Command::Server(text) => enter_server(boot, text.trim()),
        Command::Token(token) => boot.sso_callback(token).await,
        Command::Action(action) => act(boot, action).await,
    }
}

/// ponytail: the address typed is the homeserver, no `.well-known` lookup yet (A034).
fn enter_server(boot: &mut Boot, text: &str) {
    boot.apply(SessionEvent::ServerEntered);
    boot.apply(match HomeserverUrl::parse(text) {
        Ok(url) => SessionEvent::HomeserverResolved(url),
        Err(_) => SessionEvent::DiscoveryFailed,
    });
}

async fn act(boot: &mut Boot, action: LoginAction) {
    match action {
        LoginAction::Cancel => boot.apply(SessionEvent::Cancelled),
        LoginAction::SignOut => sign_out(boot).await,
        LoginAction::AddAgain => boot.add_again(),
        LoginAction::RetryStart => boot.start().await,
        // The screen turns these two into `Server` and the browser; nothing to do here.
        LoginAction::SubmitServer | LoginAction::OpenBrowser => {}
    }
}

/// An ended session has nothing to erase: the machine leaves it by itself.
async fn sign_out(boot: &mut Boot) {
    if matches!(boot.view(), LoginView::SessionExpired { .. }) {
        boot.apply(SessionEvent::SignOut);
    } else {
        boot.sign_out().await;
    }
}
