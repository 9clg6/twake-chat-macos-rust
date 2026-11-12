//! The messages ticked in the dark bar (D183).

use twake_domain::EventId;

/// The ticked messages and the anchor of a Shift+click.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    ids: Vec<EventId>,
    anchor: Option<EventId>,
}

fn place(order: &[EventId], id: &EventId) -> Option<usize> {
    order.iter().position(|e| e == id)
}

impl Selection {
    /// Not active.
    pub fn new() -> Self {
        Self::default()
    }

    /// "Select" in the menu: active, with `id` alone, which is the anchor.
    pub fn start(&mut self, id: EventId) {
        self.ids = vec![id.clone()];
        self.anchor = Some(id);
    }

    /// Whether the mode is on, that is whether something is ticked.
    pub fn is_active(&self) -> bool {
        !self.ids.is_empty()
    }

    /// How many are ticked. No `is_empty`: `is_active` says it, and the tests compare `len() > 0`
    /// (clippy `len_zero` would fire in them if `is_empty` existed).
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether `id` is ticked.
    pub fn contains(&self, id: &EventId) -> bool {
        self.ids.contains(id)
    }

    /// A press: ticks `id`, or unticks it; the anchor moves to it. Unticking the last leaves
    /// the mode.
    pub fn toggle(&mut self, id: EventId) {
        match self.ids.iter().position(|e| *e == id) {
            Some(at) => {
                self.ids.remove(at);
            }
            None => self.ids.push(id.clone()),
        }
        self.anchor = Some(id);
    }

    /// Shift+click: ticks from the anchor to `id` included, in either direction, keeping what
    /// is ticked. Without an anchor, or if either is missing from `order`, it is a `toggle`.
    pub fn extend_to(&mut self, id: EventId, order: &[EventId]) {
        let from = self.anchor.as_ref().and_then(|a| place(order, a));
        let (Some(from), Some(to)) = (from, place(order, &id)) else {
            return self.toggle(id);
        };
        for event in &order[from.min(to)..=from.max(to)] {
            if !self.contains(event) {
                self.ids.push(event.clone());
            }
        }
        self.anchor = Some(id);
    }

    /// Escape: leaves the mode, nothing done.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// The ticked ids in timeline order, once each; those missing from `order` are dropped.
    pub fn ids(&self, order: &[EventId]) -> Vec<EventId> {
        let mut seen: Vec<EventId> = Vec::new();
        for event in order.iter().filter(|e| self.contains(e)) {
            if !seen.contains(event) {
                seen.push(event.clone());
            }
        }
        seen
    }
}
