//! A preferences store with no disk: one document in memory, and the streams of its changes.

use std::sync::Mutex;

use futures::future::BoxFuture;
use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use twake_app::ports::{PreferencesError, PreferencesPort};
use twake_domain::preferences::Preferences;

use crate::subscribers::{Subscribers, locked};

struct Stored {
    document: Preferences,
    watchers: Subscribers<Preferences>,
}

/// Holds the document; `unavailable` makes every call fail.
pub struct FakePreferences {
    stored: Mutex<Stored>,
    unreachable: bool,
}

impl Default for FakePreferences {
    fn default() -> Self {
        Self::with(Preferences::default())
    }
}

impl FakePreferences {
    /// A store with nothing saved: the defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// A store that already holds `preferences`.
    pub fn with(preferences: Preferences) -> Self {
        Self {
            stored: Mutex::new(Stored {
                document: preferences,
                watchers: Subscribers::new(),
            }),
            unreachable: false,
        }
    }

    /// Every call returns `Unavailable`; nothing is kept.
    pub fn unavailable(mut self) -> Self {
        self.unreachable = true;
        self
    }

    fn run<T: Send + 'static>(
        &self,
        action: impl FnOnce(&mut Stored) -> T,
    ) -> BoxFuture<'_, Result<T, PreferencesError>> {
        let result = if self.unreachable {
            Err(PreferencesError::Unavailable)
        } else {
            Ok(action(&mut locked(&self.stored)))
        };
        std::future::ready(result).boxed()
    }
}

impl PreferencesPort for FakePreferences {
    fn load(&self) -> BoxFuture<'_, Result<Preferences, PreferencesError>> {
        self.run(|stored| stored.document.clone())
    }

    fn save(&self, preferences: Preferences) -> BoxFuture<'_, Result<(), PreferencesError>> {
        self.run(|stored| {
            if stored.document != preferences {
                stored.document = preferences.clone();
                stored.watchers.send(&[preferences]);
            }
        })
    }

    fn subscribe(
        &self,
    ) -> BoxFuture<'_, Result<BoxStream<'static, Preferences>, PreferencesError>> {
        self.run(|stored| {
            let first = stored.document.clone();
            let batches = stored.watchers.join(vec![first]);
            batches.flat_map(futures::stream::iter).boxed()
        })
    }
}
