// Ports test/tile/custom_geometry_tile.test.cpp.
//
// TODO(port): CustomGeometrySource (style/sources/custom_geometry_source),
// CustomTileLoader (style/custom_tile_loader) and CustomGeometryTile
// (tile/custom_geometry_tile) are not ported, so these are stubs. Each test
// builds a CustomGeometryTile for 0/0/0 on the RasterTileTest-like fixture
// (`super::RasterTileTest::tile_parameters`) with a CustomTileLoader actor.

// gtest: CustomGeometryTile.InvokeFetchTile
#[test]
#[ignore = "TODO(port): CustomGeometrySource, CustomTileLoader, CustomGeometryTile"]
fn invoke_fetch_tile() {
    // TODO(port): body. setNecessity(Required) must call the loader's
    // fetchTile with CanonicalTileID(0, 0, 0).
}

// gtest: CustomGeometryTile.InvokeCancelTile
#[test]
#[ignore = "TODO(port): CustomGeometrySource, CustomTileLoader, CustomGeometryTile"]
fn invoke_cancel_tile() {
    // TODO(port): body. setNecessity(Required) then setNecessity(Optional)
    // must call the loader's cancelTile with CanonicalTileID(0, 0, 0).
}

// gtest: CustomGeometryTile.InvokeTileChanged
#[test]
#[ignore = "TODO(port): CustomGeometrySource, CustomTileLoader, CustomGeometryTile"]
fn invoke_tile_changed() {
    // TODO(port): body. querySourceFeatures is empty before setTileData,
    // returns the one feature after the tile completes, and the bucket for
    // the circle layer never disappears in onTileChanged.
}
