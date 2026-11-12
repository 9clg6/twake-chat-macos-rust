//! The window of a virtualised list: which items to build for a scroll position.
//!
//! Need (not the code): `ARCHITECTURE.md` section 8 (scroll a 10 000-message fixture without a
//! dropped frame) and the performance rule that long lists draw only what is on screen.
//! Item `i` meets the window [scroll_top, scroll_top + viewport) when its top is before the end and
//! its bottom after the start. Heights here are whole numbers, so every sum is exact in `f32`.
//!
//! Public items used are listed in `docs/dev/ui-screens-api-sketch.md` section 6.

use std::ops::Range;

use proptest::prelude::*;
use twake_ui::virtual_list::HeightIndex;

fn index(heights: &[f32]) -> HeightIndex {
    HeightIndex::new(heights)
}

/// Tops 0, 10, 30, 60; total 100.
fn four() -> HeightIndex {
    index(&[10.0, 20.0, 30.0, 40.0])
}

/// The indexes that meet the window, by brute force.
fn oracle(heights: &[f32], start: f32, end: f32) -> Vec<usize> {
    let mut top = 0.0;
    let mut found = Vec::new();
    for (i, h) in heights.iter().enumerate() {
        if top < end && top + h > start {
            found.push(i);
        }
        top += h;
    }
    found
}

// ------------------------------------------------------------ heights

#[test]
fn the_total_height_is_the_sum_of_the_heights() {
    assert_eq!(four().total_height(), 100.0);
    assert_eq!(four().len(), 4);
}

#[test]
fn an_empty_list_has_no_height_and_no_window() {
    let empty = index(&[]);
    assert_eq!(empty.total_height(), 0.0);
    assert_eq!(empty.visible_range(0.0, 800.0, 3), 0..0);
}

#[test]
fn the_offset_of_an_item_is_the_height_of_those_before_it() {
    let list = four();
    let tops: Vec<f32> = (0..=4).map(|i| list.offset(i)).collect();
    assert_eq!(tops, [0.0, 10.0, 30.0, 60.0, 100.0]);
}

// ------------------------------------------------------------ window

#[test]
fn the_window_holds_the_items_it_touches() {
    assert_eq!(four().visible_range(0.0, 15.0, 0), 0..2);
    assert_eq!(four().visible_range(25.0, 10.0, 0), 1..3);
}

#[test]
fn an_item_that_ends_where_the_window_starts_is_out() {
    assert_eq!(four().visible_range(10.0, 10.0, 0), 1..2);
}

#[test]
fn an_item_that_starts_where_the_window_ends_is_out() {
    assert_eq!(four().visible_range(0.0, 30.0, 0), 0..2);
}

#[test]
fn a_window_taller_than_the_list_holds_everything() {
    assert_eq!(four().visible_range(0.0, 10_000.0, 0), 0..4);
}

#[test]
fn the_overscan_adds_items_on_each_side() {
    assert_eq!(four().visible_range(25.0, 10.0, 1), 0..4);
}

#[test]
fn the_overscan_stops_at_the_ends_of_the_list() {
    assert_eq!(four().visible_range(0.0, 5.0, 3), 0..4);
    assert_eq!(four().visible_range(95.0, 5.0, 3), 0..4);
    assert_eq!(four().visible_range(95.0, 5.0, 0), 3..4);
}

#[test]
fn a_negative_scroll_counts_as_the_top() {
    assert_eq!(
        four().visible_range(-50.0, 15.0, 0),
        four().visible_range(0.0, 15.0, 0)
    );
}

#[test]
fn a_scroll_past_the_end_holds_no_item_without_overscan() {
    assert_eq!(four().visible_range(500.0, 100.0, 0), 4..4);
}

#[test]
fn a_scroll_past_the_end_keeps_the_overscan_items_before_the_end() {
    assert_eq!(four().visible_range(500.0, 100.0, 2), 2..4);
}

#[test]
fn ten_thousand_items_build_only_a_screenful() {
    let heights: Vec<f32> = (0..10_000)
        .map(|i| if i % 2 == 0 { 40.0 } else { 60.0 })
        .collect();
    let list = index(&heights);
    assert_eq!(list.total_height(), 500_000.0);
    // 5 000 pairs of 100 px: the window [250_000, 250_800) opens on item 5 000 and ends in item 5 015.
    let range = list.visible_range(250_000.0, 800.0, 0);
    assert_eq!(range, 5_000..5_016);
    assert_eq!(list.visible_range(250_000.0, 800.0, 5), 4_995..5_021);
}

#[test]
fn the_last_item_of_ten_thousand_is_reached() {
    let heights = vec![50.0_f32; 10_000];
    let list = index(&heights);
    let range = list.visible_range(list.total_height() - 800.0, 800.0, 0);
    assert_eq!(range, 9_984..10_000);
}

// ------------------------------------------------------------ properties

fn heights_strategy() -> impl Strategy<Value = Vec<f32>> {
    prop::collection::vec(1_u16..=200, 0..300).prop_map(|v| v.into_iter().map(f32::from).collect())
}

fn whole(n: u32) -> f32 {
    n as f32
}

proptest! {
    #[test]
    fn the_total_height_is_the_sum(heights in heights_strategy()) {
        let sum: f32 = heights.iter().sum();
        prop_assert_eq!(index(&heights).total_height(), sum);
    }

    #[test]
    fn the_range_is_never_out_of_bounds(
        heights in heights_strategy(),
        scroll in -500_i32..80_000,
        viewport in 0_u32..2_000,
        overscan in 0_usize..20,
    ) {
        let range: Range<usize> = index(&heights).visible_range(scroll as f32, whole(viewport), overscan);
        prop_assert!(range.start <= range.end);
        prop_assert!(range.end <= heights.len());
    }

    #[test]
    fn the_range_covers_every_item_the_viewport_touches(
        heights in heights_strategy(),
        scroll in 0_u32..80_000,
        viewport in 1_u32..2_000,
        overscan in 0_usize..20,
    ) {
        let (start, end) = (whole(scroll), whole(scroll) + whole(viewport));
        let range = index(&heights).visible_range(whole(scroll), whole(viewport), overscan);
        for i in oracle(&heights, start, end) {
            prop_assert!(range.contains(&i), "item {} missing from {:?}", i, range);
        }
    }

    #[test]
    fn without_overscan_the_range_is_exactly_the_items_the_viewport_touches(
        heights in heights_strategy(),
        scroll in 0_u32..80_000,
        viewport in 1_u32..2_000,
    ) {
        let (start, end) = (whole(scroll), whole(scroll) + whole(viewport));
        let touched = oracle(&heights, start, end);
        let range = index(&heights).visible_range(whole(scroll), whole(viewport), 0);
        let built: Vec<usize> = range.collect();
        prop_assert_eq!(built, touched);
    }

    #[test]
    fn the_overscan_widens_the_range_by_at_most_that_many_items_each_side(
        heights in heights_strategy(),
        scroll in 0_u32..80_000,
        viewport in 1_u32..2_000,
        overscan in 0_usize..20,
    ) {
        let list = index(&heights);
        let plain = list.visible_range(whole(scroll), whole(viewport), 0);
        let wide = list.visible_range(whole(scroll), whole(viewport), overscan);
        prop_assert!(wide.start <= plain.start && plain.start - wide.start <= overscan);
        prop_assert!(wide.end >= plain.end && wide.end - plain.end <= overscan);
    }

    #[test]
    fn scrolling_down_never_moves_the_window_up(
        heights in heights_strategy(),
        scroll in 0_u32..80_000,
        step in 0_u32..5_000,
        viewport in 1_u32..2_000,
        overscan in 0_usize..20,
    ) {
        let list = index(&heights);
        let before = list.visible_range(whole(scroll), whole(viewport), overscan);
        let after = list.visible_range(whole(scroll + step), whole(viewport), overscan);
        prop_assert!(before.start <= after.start);
        prop_assert!(before.end <= after.end);
    }

    #[test]
    fn the_offsets_grow_with_the_index(heights in heights_strategy()) {
        let list = index(&heights);
        let tops: Vec<f32> = (0..=heights.len()).map(|i| list.offset(i)).collect();
        prop_assert!(tops.windows(2).all(|pair| pair[0] < pair[1]));
        prop_assert_eq!(tops.last().copied(), Some(list.total_height()));
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn ten_thousand_items_keep_the_range_small_and_inside(
        scroll in 0_u32..450_000,
        viewport in 1_u32..1_200,
    ) {
        let heights: Vec<f32> = (0..10_000_u32).map(|i| 20.0 + (i % 7) as f32 * 10.0).collect();
        let list = index(&heights);
        let range = list.visible_range(whole(scroll), whole(viewport), 5);
        prop_assert!(range.end <= 10_000);
        prop_assert!(range.len() <= (viewport as usize) / 20 + 12);
    }
}
