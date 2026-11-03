// Ports test/tile/vector_tile.test.cpp.
//
// TODO(port): VectorTile.setError, VectorTile.onError, VectorTile.Issue8542
// (they need VectorMVTTile, TileParameters and the renderer) and
// VectorTileData.MLTParseResults (MLT decoding).

use crate::VectorMVTTileData;
use maplibre_native_base::Identifier;
use mln_tile_id::GeometryTileData;
use mln_util::geometry::FeatureType;
use std::path::PathBuf;
use std::sync::Arc;

fn read_file(path: &str) -> Vec<u8> {
    let base = std::env::var("RUNFILES_DIR")
        .or_else(|_| std::env::var("TEST_SRCDIR"))
        .expect("RUNFILES_DIR or TEST_SRCDIR");
    std::fs::read(PathBuf::from(base).join("maplibre+").join(path)).unwrap()
}

#[test]
fn parse_results() {
    let data = VectorMVTTileData::new(Arc::new(read_file("test/fixtures/map/issue12432/0-0-0.mvt")));

    let layer_names = data.layer_names().unwrap();
    assert_eq!(layer_names.len(), 2);
    assert_eq!(layer_names[0], "admin");
    assert_eq!(layer_names[1], "water");

    assert!(data.get_layer("invalid").is_none());

    let layer = data.get_layer("admin").unwrap();
    assert_eq!(layer.get_name(), "admin");
    assert_eq!(layer.feature_count(), 17154);

    // ASSERT_THROW(layer->getFeature(17154u), std::out_of_range)
    let typed = data.try_get_layer("admin").unwrap().unwrap();
    assert!(matches!(typed.try_get_feature(17154), Err(crate::Error::OutOfRange(_))));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        layer.get_feature(17154);
    }));
    assert!(r.is_err());

    let feature = layer.get_feature(0);
    assert_eq!(feature.get_type(), FeatureType::LineString);
    assert!(matches!(feature.get_id(), Identifier::Uint(_)));
    assert_eq!(feature.get_id(), Identifier::Uint(1));

    let properties = feature.get_properties();
    assert_eq!(properties.len(), 3);
    assert_eq!(properties.get("disputed"), feature.get_value("disputed").as_ref());

    assert_eq!(feature.get_value("invalid"), None);
}
