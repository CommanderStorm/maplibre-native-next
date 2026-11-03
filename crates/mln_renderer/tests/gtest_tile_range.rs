// Ports test/util/tile_range.test.cpp (util::TileRange lives in
// mln_renderer::util::tile_range).

use mln_renderer::util::tile_range::TileRange;
use mln_tile_id::CanonicalTileID;
use mln_util::geo::{LatLng, LatLngBounds};

fn ll(lat: f64, lon: f64) -> LatLng {
    LatLng::new(lat, lon).unwrap()
}

fn c(z: u8, x: u32, y: u32) -> CanonicalTileID {
    CanonicalTileID::new(z, x, y)
}

/// `LatLngBounds(const CanonicalTileID&)`.
fn bounds_from(id: CanonicalTileID) -> LatLngBounds {
    LatLngBounds::from_canonical_tile(id.z, id.x, id.y)
}

// gtest: TileRange.ContainsWorld
#[test]
fn contains_world() {
    let range = TileRange::from_lat_lng_bounds(&LatLngBounds::world(), 0);
    assert!(range.contains(&c(0, 0, 0)));
    assert!(!range.contains(&c(10, 0, 0)));
}

// gtest: TileRange.ContainsBoundsFromTile
#[test]
fn contains_bounds_from_tile() {
    {
        let bounds = bounds_from(c(3, 7, 0));
        let range = TileRange::from_lat_lng_bounds(&bounds, 3);
        assert!(range.contains(&c(3, 7, 0)));
    }
    {
        let bounds = bounds_from(c(10, 162, 395));
        let range = TileRange::from_lat_lng_bounds(&bounds, 10);
        assert!(range.contains(&c(10, 162, 395)));
    }
}

// gtest: TileRange.ContainsMultiZoom
#[test]
fn contains_multi_zoom() {
    let wrapped_bounds = LatLngBounds::hull(ll(37.6609, -122.5744), ll(37.8271, -122.3204));
    let range = TileRange::from_lat_lng_bounds_range(&wrapped_bounds, 5, 13);
    assert!(!range.contains(&c(0, 0, 0)));
    assert!(!range.contains(&c(5, 3, 11)));
    assert!(!range.contains(&c(6, 9, 22)));
    assert!(range.contains(&c(5, 5, 12)));
    assert!(range.contains(&c(6, 10, 24)));
    assert!(range.contains(&c(13, 1310, 3166)));
}

// gtest: TileRange.ContainsIntersectingTiles
#[test]
fn contains_intersecting_tiles() {
    let bounds = LatLngBounds::hull(ll(37.6609, -122.5744), ll(37.8271, -122.3204));
    let range = TileRange::from_lat_lng_bounds(&bounds, 13);
    assert!(!range.contains(&c(13, 1316, 3100)));
    assert!(range.contains(&c(13, 1310, 3166)));
}

// gtest: TileRange.ContainsWrappedBounds
#[test]
fn contains_wrapped_bounds() {
    let wrapped_bounds = LatLngBounds::hull(ll(37.6609, 237.6796), ll(37.8271, 237.4256));
    let range = TileRange::from_lat_lng_bounds(&wrapped_bounds, 13);
    assert!(!range.contains(&c(13, 1316, 3100)));
    assert!(range.contains(&c(13, 1310, 3166)));
}

// gtest: TileRange.ContainsBoundsCrossingAntimeridian
#[test]
fn contains_bounds_crossing_antimeridian() {
    {
        let crossing_bounds = LatLngBounds::hull(ll(-20.9615, -214.309), ll(19.477, -155.830));
        let range = TileRange::from_lat_lng_bounds(&crossing_bounds, 1);
        assert!(range.contains(&c(1, 1, 1)));
        assert!(range.contains(&c(1, 0, 0)));
    }
    {
        let crossing_bounds = LatLngBounds::hull(ll(-20.9615, -214.309), ll(19.477, -155.830));
        let range = TileRange::from_lat_lng_bounds(&crossing_bounds, 6);
        assert!(!range.contains(&c(6, 55, 34)));
        assert!(!range.contains(&c(6, 5, 28)));
        assert!(range.contains(&c(6, 63, 28)));
    }
}
