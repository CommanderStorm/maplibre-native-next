// Ports test/tile/tile_lod.test.cpp.
//
// TODO(port): every test renders a CustomGeometrySource (one point per
// fetched tile, counted in `fetchTileFunction`) through Map +
// HeadlessFrontend in MapMode::Static with MapDebugOptions::TileBorders and
// compares against test/fixtures/tile_lod/<name>/expected.png
// (test::checkImages). CustomGeometrySource is not ported, so these are
// stubs. The Map::set_tile_lod_* setters they use exist.

// gtest: TileLOD.disabled
#[test]
#[ignore = "TODO(port): CustomGeometrySource (and test::checkImage)"]
fn disabled() {
    // TODO(port): body. zoom 10, setTileLodPitchThreshold(pi): 4 tiles,
    // image "disabled".
}

// gtest: TileLOD.zoomShift
#[test]
#[ignore = "TODO(port): CustomGeometrySource"]
fn zoom_shift() {
    // TODO(port): body. zoom 12; zoom shift 4.0: 64 tiles; -4.0: 4 tiles.
}

// gtest: TileLOD.pitchThreshold
#[test]
#[ignore = "TODO(port): CustomGeometrySource (and test::checkImage)"]
fn pitch_threshold() {
    // TODO(port): body. zoom 10, shift 4, min radius 1, scale 1, threshold
    // 45 deg; pitch 44: 140 tiles; pitch 46: 22 tiles, images
    // "pitchThreshold-line" / "pitchThreshold-polyline" (threshold 0.0015).
}

// gtest: TileLOD.scale
#[test]
#[ignore = "TODO(port): CustomGeometrySource (and test::checkImage)"]
fn scale() {
    // TODO(port): body. zoom 0, min radius 1, scale 0.4, threshold -1,
    // shift 5: 112 tiles, image "scale".
}

// gtest: TileLOD.radius
#[test]
#[ignore = "TODO(port): CustomGeometrySource (and test::checkImage)"]
fn radius() {
    // TODO(port): body. zoom 0, min radius 5, scale 1, threshold -1,
    // shift 5: 172 tiles, image "radius".
}
