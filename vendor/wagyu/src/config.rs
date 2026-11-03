// Ports vendor/wagyu/include/mapbox/geometry/wagyu/config.hpp.

/// `enum clip_type : std::uint8_t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipType {
    Intersection = 0,
    Union,
    Difference,
    XOr,
}

/// `enum polygon_type : std::uint8_t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PolygonType {
    #[default]
    Subject = 0,
    Clip,
}

/// `enum fill_type : std::uint8_t`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillType {
    EvenOdd = 0,
    NonZero,
    Positive,
    Negative,
}
