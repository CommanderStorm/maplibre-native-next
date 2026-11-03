// Ports test/math/clamp.test.cpp, test/math/minmax.test.cpp and
// test/math/wrap.test.cpp.
//
// C++ `util::min(a, b, c)` is variadic and expands to `min(a, min(b, c))`;
// the nested calls below are that expansion. `numeric_limits<int32_t>::
// quiet_NaN()` is 0 for integral types.

use mln_math::{clamp, clamp_f64, max, max_f64, min, min_f64, wrap};

/// gtest `ASSERT_DOUBLE_EQ`: equal within 4 ULPs.
fn assert_double_eq(expected: f64, actual: f64) {
    fn biased(f: f64) -> u64 {
        let bits = f.to_bits();
        if bits & (1 << 63) != 0 { !bits + 1 } else { bits | (1 << 63) }
    }
    let ok = if expected.is_nan() || actual.is_nan() {
        false
    } else {
        let (a, b) = (biased(expected), biased(actual));
        a.abs_diff(b) <= 4
    };
    assert!(ok, "expected {expected:e}, got {actual:e}");
}

// gtest: Math.ClampFloatingPoint
#[test]
fn clamp_floating_point() {
    let lowest_value = f64::MIN;
    let maximum_value = f64::MAX;
    let quiet_nan = f64::NAN;
    assert_double_eq(0., clamp_f64(0., quiet_nan, quiet_nan));
    assert_double_eq(0., clamp_f64(0., lowest_value, maximum_value));
}

// gtest: Math.ClampIntegral
#[test]
fn clamp_integral() {
    let lowest_value = i32::MIN;
    let maximum_value = i32::MAX;
    let quiet_nan: i32 = 0; // numeric_limits<int32_t>::quiet_NaN()
    assert_eq!(0, clamp(0, quiet_nan, quiet_nan));
    assert_eq!(0, clamp(0, lowest_value, maximum_value));
}

// gtest: Math.MinMaxFloatingPoint
#[test]
fn min_max_floating_point() {
    let minimum_value = f64::MIN_POSITIVE; // numeric_limits<double>::min()
    let lowest_value = f64::MIN;
    let maximum_value = f64::MAX;
    let quiet_nan = f64::NAN;
    assert_double_eq(lowest_value, min_f64(minimum_value, lowest_value));
    assert_double_eq(lowest_value, min_f64(f64::NAN, lowest_value));
    assert_double_eq(lowest_value, min_f64(quiet_nan, lowest_value));
    assert_double_eq(0., min_f64(2., min_f64(1., 0.)));
    assert_double_eq(maximum_value, max_f64(f64::NAN, maximum_value));
    assert_double_eq(maximum_value, max_f64(quiet_nan, maximum_value));
    assert_double_eq(10., max_f64(8., max_f64(9., 10.)));
}

// gtest: Math.MinMaxIntegral
#[test]
fn min_max_integral() {
    let minimum_value = i32::MIN;
    let lowest_value = i32::MIN;
    let maximum_value = i32::MAX;
    let quiet_nan: i32 = 0; // numeric_limits<int32_t>::quiet_NaN()
    assert_eq!(lowest_value, min(minimum_value, lowest_value));
    assert_eq!(lowest_value, min(quiet_nan, lowest_value));
    assert_eq!(0, min(2, min(1, 0)));
    assert_eq!(maximum_value, max(quiet_nan, maximum_value));
    assert_eq!(10, max(8, max(9, 10)));
}

// gtest: Math.WrapHigherValue
#[test]
fn wrap_higher_value() {
    assert_double_eq(4.0, wrap(16.0, 0.0, 12.0));
}

// gtest: Math.WrapLowerValue
#[test]
fn wrap_lower_value() {
    assert_double_eq(9.0, wrap(-3.0, 0.0, 12.0));
}

// gtest: Math.WrapInRangeValue
#[test]
fn wrap_in_range_value() {
    assert_double_eq(4.0, wrap(4.0, 0.0, 12.0));
}

// gtest: Math.WrapMaxValue
#[test]
fn wrap_max_value() {
    assert_double_eq(0.0, wrap(12.0, 0.0, 12.0));
}

// gtest: Math.WrapMinValue
#[test]
fn wrap_min_value() {
    assert_double_eq(0.0, wrap(0.0, 0.0, 12.0));
}
