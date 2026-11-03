// Ports vendor/wagyu/include/mapbox/geometry/wagyu/local_minimum.hpp. The
// `std::deque<local_minimum>` is a Vec; the left and right bounds are in the
// bound arena.

use crate::bound::BoundPtr;

/// `local_minimum<int32_t>`.
#[derive(Clone, Debug)]
pub(crate) struct LocalMinimum {
    pub left_bound: BoundPtr,
    pub right_bound: BoundPtr,
    pub y: i32,
    pub minimum_has_horizontal: bool,
}

/// `local_minimum_sorter`.
pub(crate) fn local_minimum_sorter(lm1: &LocalMinimum, lm2: &LocalMinimum) -> bool {
    if lm2.y == lm1.y {
        return lm2.minimum_has_horizontal != lm1.minimum_has_horizontal && lm1.minimum_has_horizontal;
    }
    lm2.y < lm1.y
}
