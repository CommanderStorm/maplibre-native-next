//! Ports the vendored `mapbox::geometry`, `mapbox::feature` and
//! `mapbox::base::Value` types (vendor/maplibre-native-base) that
//! `include/mln/util/geometry.hpp` and `include/mln/util/feature.hpp`
//! re-export into the `mln` namespace.
//!
//! Deliberately not `geo-types`: the port needs the exact generic coordinate
//! parameter and the `u64`/`i64`/`f64` distinction in [`Value`] that the
//! expression and conversion code relies on.

pub mod feature;
pub mod point;
pub mod value;

pub use feature::{Feature, Identifier, PropertyMap};
pub use point::{
    Geometry, GeometryCollection, LineString, LinearRing, MultiLineString, MultiPoint,
    MultiPolygon, Point, Polygon,
};
pub use value::{Value, ValueObject};
