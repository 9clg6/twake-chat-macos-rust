//! Shared plumbing of the fakes: a lock that survives a panic, and the senders of a stream.

use std::sync::{Mutex, MutexGuard, PoisonError};

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender, unbounded};
use futures::stream::{BoxStream, StreamExt};

pub(crate) fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The live subscribers of a stream of batches.
pub(crate) struct Subscribers<T>(Vec<UnboundedSender<Vec<T>>>);

impl<T: Clone + Send + 'static> Subscribers<T> {
    pub(crate) fn new() -> Self {
        Self(Vec::new())
    }

    /// A new stream whose first batch is `first`.
    pub(crate) fn join(&mut self, first: Vec<T>) -> BoxStream<'static, Vec<T>> {
        let (tx, rx): (_, UnboundedReceiver<Vec<T>>) = unbounded();
        // The receiver is alive in this scope, so the send cannot fail.
        let _ = tx.unbounded_send(first);
        self.0.push(tx);
        rx.boxed()
    }

    /// One batch to every live subscriber; a dropped one is forgotten.
    pub(crate) fn send(&mut self, batch: &[T]) {
        self.0
            .retain(|tx| tx.unbounded_send(batch.to_vec()).is_ok());
    }
}
