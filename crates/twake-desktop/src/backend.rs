//! `AccountBackend` over `twake-matrix` and the keychain of `twake-store` (A034).
//!
//! The Matrix calls run on the Tokio runtime of the process; the futures handed back can be
//! awaited from any executor.

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use futures::StreamExt;
use futures::future::BoxFuture;
use futures::stream::{self, BoxStream};
use tokio::runtime::Handle;
use twake_app::boot::{AccountBackend, WipeFailed};
use twake_app::login_screen::LoginProblem;
use twake_domain::UserId;
use twake_domain::session::{DeviceId, HomeserverUrl, Secret, Session};
use twake_matrix::{
    Client, LoginError, account_data_dir, build_client, check_session, expiry, login_client,
    login_with_token, restore_session, sign_out,
};
use twake_store::{Keychain, forget_store_key_async, store_key_for_async};

/// The accounts of this run: their data folder, their keychain, the clients signed in.
pub struct MatrixBackend {
    root: PathBuf,
    keychain: Arc<dyn Keychain>,
    runtime: Handle,
    clients: Arc<Mutex<HashMap<UserId, Client>>>,
}

impl MatrixBackend {
    /// Account data under `root`, store keys in `keychain`, work on `runtime`.
    pub fn new(root: PathBuf, keychain: Arc<dyn Keychain>, runtime: Handle) -> Self {
        Self {
            root,
            keychain,
            runtime,
            clients: Arc::default(),
        }
    }

    /// Runs `work` on the runtime.
    fn on_runtime<T: Send + 'static>(
        &self,
        work: impl Future<Output = T> + Send + 'static,
        failed: T,
    ) -> BoxFuture<'_, T> {
        let handle = self.runtime.spawn(work);
        Box::pin(async move { handle.await.unwrap_or(failed) })
    }
}

/// The client of `session`: its folder, its key (made at the first use), the session put back.
async fn open_client(
    root: &std::path::Path,
    keychain: &Arc<dyn Keychain>,
    session: &Session,
) -> Result<Client, LoginProblem> {
    let key = store_key_for_async(Arc::clone(keychain), session.user_id.clone())
        .await
        .map_err(|_| LoginProblem::Server)?;
    let dir = account_data_dir(root, &session.user_id);
    let client = build_client(&session.homeserver, &dir, &key)
        .await
        .map_err(|_| LoginProblem::Server)?;
    restore_session(&client, session)
        .await
        .map_err(|_| LoginProblem::Server)?;
    Ok(client)
}

fn problem(error: LoginError) -> LoginProblem {
    match error {
        LoginError::InvalidToken => LoginProblem::InvalidToken,
        LoginError::Unreachable => LoginProblem::Unreachable,
        LoginError::Server => LoginProblem::Server,
    }
}

impl AccountBackend for MatrixBackend {
    fn restore(&self, session: Session) -> BoxFuture<'_, Result<(), LoginProblem>> {
        let (root, keychain, clients) = (
            self.root.clone(),
            Arc::clone(&self.keychain),
            Arc::clone(&self.clients),
        );
        self.on_runtime(
            async move {
                let client = open_client(&root, &keychain, &session).await?;
                check_session(&client).await.map_err(problem)?;
                lock(&clients).insert(session.user_id.clone(), client);
                Ok(())
            },
            Err(LoginProblem::Server),
        )
    }

    fn login_with_token(
        &self,
        homeserver: HomeserverUrl,
        token: Secret,
        device: Option<DeviceId>,
    ) -> BoxFuture<'_, Result<Session, LoginProblem>> {
        let (root, keychain, clients) = (
            self.root.clone(),
            Arc::clone(&self.keychain),
            Arc::clone(&self.clients),
        );
        self.on_runtime(
            async move {
                let bare = login_client(&homeserver)
                    .await
                    .map_err(|_| LoginProblem::Server)?;
                let session = login_with_token(&bare, &token, device.as_ref())
                    .await
                    .map_err(problem)?;
                let client = open_client(&root, &keychain, &session).await?;
                lock(&clients).insert(session.user_id.clone(), client);
                Ok(session)
            },
            Err(LoginProblem::Server),
        )
    }

    fn sign_out(&self, session: Session) -> BoxFuture<'_, Result<(), WipeFailed>> {
        let (root, keychain, clients) = (
            self.root.clone(),
            Arc::clone(&self.keychain),
            Arc::clone(&self.clients),
        );
        self.on_runtime(
            async move {
                // ponytail: no client in the map (never restored) still erases the folder, on a
                // client that is built and dropped at once.
                let known = lock(&clients).remove(&session.user_id);
                let client = match known {
                    Some(client) => client,
                    None => open_client(&root, &keychain, &session)
                        .await
                        .map_err(|_| WipeFailed)?,
                };
                sign_out(client, &root, &session.user_id)
                    .await
                    .map_err(|_| WipeFailed)?;
                // A034: after a failed erase the key stays, the data is still on the disk.
                let _ = forget_store_key_async(keychain, session.user_id).await;
                Ok(())
            },
            Err(WipeFailed),
        )
    }

    fn session_lost(&self, user: UserId) -> BoxStream<'static, ()> {
        match lock(&self.clients).get(&user) {
            Some(client) => expiry(client).map(|_| ()).boxed(),
            None => stream::empty().boxed(),
        }
    }
}

fn lock(
    clients: &Mutex<HashMap<UserId, Client>>,
) -> std::sync::MutexGuard<'_, HashMap<UserId, Client>> {
    clients
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
