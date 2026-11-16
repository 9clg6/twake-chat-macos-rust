//! The window of a virtualised list: which items to build for a scroll position.

use std::ops::Range;

/// Prefix sums of item heights, built once per change of the list; a window is found in O(log n).
#[derive(Debug, Clone)]
pub struct HeightIndex {
    /// `tops[i]` is the top of item `i`; the last entry is the total height.
    tops: Vec<f32>,
}

impl HeightIndex {
    /// The index of items of these heights, in pixels.
    pub fn new(item_heights: &[f32]) -> Self {
        let mut tops = Vec::with_capacity(item_heights.len() + 1);
        let mut sum = 0.0;
        tops.push(sum);
        for height in item_heights {
            sum += height;
            tops.push(sum);
        }
        Self { tops }
    }

    /// The number of items.
    pub fn len(&self) -> usize {
        self.tops.len().saturating_sub(1)
    }

    /// Whether there is no item.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The height of the whole list.
    pub fn total_height(&self) -> f32 {
        self.tops.last().copied().unwrap_or(0.0)
    }

    /// The top of item `index`; the total height for `len`.
    pub fn offset(&self, index: usize) -> f32 {
        self.tops.get(index).copied().unwrap_or(self.total_height())
    }

    /// Items that meet `[scroll_top, scroll_top + viewport)`, plus `overscan` items on each side,
    /// clamped to the list. A negative `scroll_top` (rubber band) counts as 0.
    pub fn visible_range(&self, scroll_top: f32, viewport: f32, overscan: usize) -> Range<usize> {
        let start = scroll_top.max(0.0);
        let end = start + viewport;
        // First item whose bottom is after the start; first item whose top is not before the end.
        let first = self.tops[1..].partition_point(|bottom| *bottom <= start);
        let last = self.tops[..self.len()].partition_point(|top| *top < end);
        let first = first.min(last);
        first.saturating_sub(overscan)..(last + overscan).min(self.len())
    }
}
