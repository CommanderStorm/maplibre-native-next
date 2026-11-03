// Ports vendor/wagyu/include/mapbox/geometry/wagyu/util.hpp (for T =
// int32_t). Integer arithmetic that C++ does in `T` before widening wraps
// here, as x86-64 does (C++: undefined on overflow; the clipper's inputs are
// int16 coordinates, which never overflow).

use crate::almost_equal::almost_equals;
use crate::point::GPoint;

pub(crate) fn values_are_equal(x: f64, y: f64) -> bool {
    almost_equals(x, y)
}

pub(crate) fn value_is_zero(val: f64) -> bool {
    values_are_equal(val, 0.0)
}

pub(crate) fn greater_than_or_equal(x: f64, y: f64) -> bool {
    x > y || values_are_equal(x, y)
}

pub(crate) fn greater_than(x: f64, y: f64) -> bool {
    !values_are_equal(x, y) && x > y
}

pub(crate) fn less_than(x: f64, y: f64) -> bool {
    !values_are_equal(x, y) && x < y
}

/// `slopes_equal(pt1, pt2, pt3)` (all three overloads).
pub(crate) fn slopes_equal3(pt1: (i32, i32), pt2: (i32, i32), pt3: (i32, i32)) -> bool {
    i64::from(pt1.1.wrapping_sub(pt2.1)) * i64::from(pt2.0.wrapping_sub(pt3.0))
        == i64::from(pt1.0.wrapping_sub(pt2.0)) * i64::from(pt2.1.wrapping_sub(pt3.1))
}

pub(crate) fn gp_slopes_equal3(pt1: GPoint, pt2: GPoint, pt3: GPoint) -> bool {
    slopes_equal3((pt1.x, pt1.y), (pt2.x, pt2.y), (pt3.x, pt3.y))
}

/// `::llround` (glibc): round half away from zero. Out-of-range and NaN
/// values never reach it (coordinates are int16-sized).
pub(crate) fn llround(value: f64) -> i64 {
    value.round() as i64
}

/// `wround<int32_t>`: `static_cast<int32_t>(::llround(value))`.
pub(crate) fn wround(value: f64) -> i32 {
    llround(value) as i32
}

/// `std::max(double, double)`: `(a < b) ? b : a`.
pub(crate) fn cpp_max(a: f64, b: f64) -> f64 {
    if a < b { b } else { a }
}

/// `std::min(double, double)`: `(b < a) ? b : a`.
pub(crate) fn cpp_min(a: f64, b: f64) -> f64 {
    if b < a { b } else { a }
}
