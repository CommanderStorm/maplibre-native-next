// Ports src/mln/geometry/feature_index.hpp and
// src/mln/geometry/feature_index.cpp (the parts the tile workers use).
//
// TODO(port): the GridIndex (`grid`) and `query`, `lookupSymbolFeatures`,
// `translateQueryGeometry`, DynamicFeatureIndex (queryRenderedFeatures,
// M6). Until then `insert` records the indexed subfeatures with their
// envelopes in insertion order.

use mln_tile_id::geometry_tile_data::{GeometryCollection, GeometryTileData};
use mln_util::constants;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// `IndexedSubfeature` (the `Ref` and owning variants are one type here).
#[derive(Clone, Debug, PartialEq)]
pub struct IndexedSubfeature {
    pub index: usize,
    pub source_layer_name: String,
    pub bucket_leader_id: String,
    pub sort_index: usize,
    // Only used for symbol features
    pub bucket_instance_id: u32,
    pub collision_group_id: u16,
}

/// A grid entry: the subfeature and its envelope (`{min, max}` as floats).
#[derive(Clone, Debug, PartialEq)]
pub struct GridEntry {
    pub subfeature: IndexedSubfeature,
    pub min: [f32; 2],
    pub max: [f32; 2],
}

/// `FeatureIndex`.
pub struct FeatureIndex {
    // TODO(port): GridIndex<RefIndexedSubfeature> grid (EXTENT, EXTENT,
    // EXTENT / 16).
    entries: Vec<GridEntry>,
    sort_index: usize,

    unique_layer_ids: BTreeSet<String>,
    bucket_layer_ids: BTreeMap<String, Vec<String>>,
    tile_data: Option<Arc<dyn GeometryTileData + Send + Sync>>,
}

impl std::fmt::Debug for FeatureIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FeatureIndex")
            .field("entries", &self.entries.len())
            .field("bucket_layer_ids", &self.bucket_layer_ids)
            .finish_non_exhaustive()
    }
}

impl FeatureIndex {
    /// `FeatureIndex(std::unique_ptr<const GeometryTileData>)`. C++ gets a
    /// `clone()` of the tile data, which shares the data; this shares the
    /// `Arc`.
    pub fn new(tile_data: Option<Arc<dyn GeometryTileData + Send + Sync>>) -> Self {
        FeatureIndex {
            entries: Vec::new(),
            sort_index: 0,
            unique_layer_ids: BTreeSet::new(),
            bucket_layer_ids: BTreeMap::new(),
            tile_data,
        }
    }

    /// `getData()`.
    pub fn get_data(&self) -> Option<&(dyn GeometryTileData + Send + Sync)> {
        self.tile_data.as_deref()
    }

    /// Set the expected number of elements per cell to avoid small
    /// re-allocations for populated cells.
    pub fn reserve(&mut self, _value: usize) {}

    /// `insert(geometries, index, sourceLayerName, bucketLeaderID)`.
    pub fn insert(
        &mut self,
        geometries: &GeometryCollection,
        index: usize,
        source_layer_name: &str,
        bucket_leader_id: &str,
    ) {
        self.unique_layer_ids.insert(source_layer_name.to_owned());
        self.bucket_layer_ids.entry(bucket_leader_id.to_owned()).or_default();

        let feature_sort_index = self.sort_index;
        self.sort_index += 1;
        for ring in geometries.iter() {
            // mapbox::geometry::envelope(ring)
            let Some(first) = ring.first() else {
                continue;
            };
            let (mut min_x, mut min_y, mut max_x, mut max_y) = (first.x, first.y, first.x, first.y);
            for p in ring.iter() {
                min_x = min_x.min(p.x);
                min_y = min_y.min(p.y);
                max_x = max_x.max(p.x);
                max_y = max_y.max(p.y);
            }
            let extent = constants::EXTENT;
            if i32::from(min_x) < extent && i32::from(min_y) < extent && max_x >= 0 && max_y >= 0 {
                self.entries.push(GridEntry {
                    subfeature: IndexedSubfeature {
                        index,
                        source_layer_name: source_layer_name.to_owned(),
                        bucket_leader_id: bucket_leader_id.to_owned(),
                        sort_index: feature_sort_index,
                        bucket_instance_id: 0,
                        collision_group_id: 0,
                    },
                    min: [f32::from(min_x), f32::from(min_y)],
                    max: [f32::from(max_x), f32::from(max_y)],
                });
            }
        }
    }

    /// `setBucketLayerIDs(bucketLeaderID, layerIDs)`.
    pub fn set_bucket_layer_ids(&mut self, bucket_leader_id: &str, layer_ids: &[String]) {
        self.bucket_layer_ids
            .insert(bucket_leader_id.to_owned(), layer_ids.to_vec());
    }

    /// The indexed entries in insertion order (stand-in for the grid).
    pub fn entries(&self) -> &[GridEntry] {
        &self.entries
    }

    pub fn bucket_layer_ids(&self) -> &BTreeMap<String, Vec<String>> {
        &self.bucket_layer_ids
    }
}
