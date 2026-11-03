// Ports vendor/wagyu/include/mapbox/geometry/wagyu/scanbeam.hpp.

use crate::local_minimum::LocalMinimum;

/// `insert_sorted_scanbeam`.
pub(crate) fn insert_sorted_scanbeam(scanbeam: &mut Vec<i32>, t: i32) {
    let i = scanbeam.partition_point(|&x| x < t);
    if i == scanbeam.len() || t < scanbeam[i] {
        scanbeam.insert(i, t);
    }
}

/// `pop_from_scanbeam`.
pub(crate) fn pop_from_scanbeam(y: &mut i32, scanbeam: &mut Vec<i32>) -> bool {
    match scanbeam.pop() {
        Some(v) => {
            *y = v;
            true
        }
        None => false,
    }
}

/// `setup_scanbeam`.
pub(crate) fn setup_scanbeam(minima_list: &[LocalMinimum], scanbeam: &mut Vec<i32>) {
    scanbeam.reserve(minima_list.len());
    for lm in minima_list {
        scanbeam.push(lm.y);
    }
    scanbeam.sort_unstable();
}
