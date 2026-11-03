// Ports test/gl/bucket.test.cpp (Buckets.CircleBucket, FillBucket,
// LineBucket; the OpenGL headless backend is the WebGPU one here) and
// test/renderer/fill_bucket.test.cpp.

use maplibre_native_base::{Identifier, Point, PropertyMap, Value};
use mln_gfx::backend_scope::{BackendScope, ScopeType};
use mln_gfx::headless_backend::HeadlessBackend as _;
use mln_gfx::renderer_backend::RendererBackend;
use mln_gfx::shaders::segment::SegmentBase;
use mln_map_types::MapMode;
use mln_renderer::bucket::Bucket;
use mln_renderer::buckets::{CircleBucket, FillBucket, LineBucket};
use mln_renderer::layout::pattern_layout::PatternLayerMap;
use mln_style::layers::fill_layer_properties::FillLayoutProperties;
use mln_style::layers::line_layer_properties::LineLayoutProperties;
use mln_style_types::image::ImagePositions;
use mln_tile_id::geometry_tile_data::{GeometryCollection, GeometryTileFeature};
use mln_tile_id::CanonicalTileID;
use mln_util::geometry::FeatureType;
use mln_util::logging::{Event, EventSeverity, Log, Observer};
use mln_util::size::Size;
use mln_webgpu::headless_backend::HeadlessBackend;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// `StubGeometryTileFeature(id, type, geometry, properties)`.
struct StubGeometryTileFeature {
    properties: PropertyMap,
    id: Identifier,
    type_: FeatureType,
    geometry: GeometryCollection,
}

impl GeometryTileFeature for StubGeometryTileFeature {
    fn get_type(&self) -> FeatureType {
        self.type_
    }
    fn get_value(&self, key: &str) -> Option<Value> {
        self.properties.get(key).cloned()
    }
    fn get_id(&self) -> Identifier {
        self.id.clone()
    }
    fn get_geometries(&self) -> &GeometryCollection {
        &self.geometry
    }
}

fn feature(type_: FeatureType, geometry: &GeometryCollection) -> StubGeometryTileFeature {
    StubGeometryTileFeature {
        properties: PropertyMap::new(),
        id: Identifier::Null,
        type_,
        geometry: geometry.clone(),
    }
}

fn collection(rings: &[&[(i16, i16)]]) -> GeometryCollection {
    GeometryCollection::from(
        rings
            .iter()
            .map(|ring| ring.iter().map(|(x, y)| Point::new(*x, *y)).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
    )
}

/// Runs `f` with an upload pass of a headless backend (the C++ tests create
/// a `gl::HeadlessBackend`, a context and an upload pass).
fn with_upload_pass(f: impl FnOnce(&dyn mln_gfx::UploadPass)) {
    let backend = HeadlessBackend::create(Size { width: 512, height: 256 });
    let _scope = BackendScope::new(&backend, ScopeType::Explicit);
    let context = backend.get_context();
    let command_encoder = context.create_command_encoder();
    let upload_pass = command_encoder.create_upload_pass("upload", backend.get_default_renderable());
    f(&*upload_pass);
}

#[test]
fn circle_bucket() {
    let mut bucket = CircleBucket::new(&BTreeMap::new(), MapMode::Static, 1.0);
    assert!(!bucket.has_data());
    assert!(!bucket.needs_upload());

    // CircleBucket::addFeature() is a no-op.
    let point = collection(&[&[(0, 0)]]);
    bucket
        .add_feature(
            &feature(FeatureType::Point, &point),
            &point,
            &ImagePositions::new(),
            &PatternLayerMap::new(),
            0,
            &CanonicalTileID::new(0, 0, 0),
        )
        .unwrap();
    assert!(!bucket.has_data());
    assert!(!bucket.needs_upload());

    bucket.segments.push(SegmentBase::new(0, 0));
    assert!(bucket.has_data());
    assert!(bucket.needs_upload());

    with_upload_pass(|upload_pass| bucket.upload(upload_pass));
    assert!(bucket.has_data());
    assert!(!bucket.needs_upload());
}

#[test]
fn fill_bucket() {
    let layout = FillLayoutProperties::PossiblyEvaluated::default();
    let mut bucket = FillBucket::new(&layout, &BTreeMap::new(), 5.0, 1);
    assert!(!bucket.has_data());
    assert!(!bucket.needs_upload());

    let polygon = collection(&[&[(0, 0), (0, 1), (1, 1)]]);
    bucket
        .add_feature(
            &feature(FeatureType::Polygon, &polygon),
            &polygon,
            &ImagePositions::new(),
            &PatternLayerMap::new(),
            0,
            &CanonicalTileID::new(0, 0, 0),
        )
        .unwrap();
    assert!(bucket.has_data());
    assert!(bucket.needs_upload());

    with_upload_pass(|upload_pass| bucket.upload(upload_pass));
    assert!(!bucket.needs_upload());
}

#[test]
fn line_bucket() {
    let layout = LineLayoutProperties::PossiblyEvaluated::default();
    let mut bucket = LineBucket::new(layout, &BTreeMap::new(), 10.0, 1);
    assert!(!bucket.has_data());
    assert!(!bucket.needs_upload());

    // Ignore invalid feature type.
    let point = collection(&[&[(0, 0)]]);
    bucket
        .add_feature(
            &feature(FeatureType::Point, &point),
            &point,
            &ImagePositions::new(),
            &PatternLayerMap::new(),
            0,
            &CanonicalTileID::new(0, 0, 0),
        )
        .unwrap();
    assert!(!bucket.has_data());

    let line = collection(&[&[(0, 0), (1, 1)]]);
    bucket
        .add_feature(
            &feature(FeatureType::LineString, &line),
            &line,
            &ImagePositions::new(),
            &PatternLayerMap::new(),
            1,
            &CanonicalTileID::new(0, 0, 0),
        )
        .unwrap();
    assert!(bucket.has_data());
    assert!(bucket.needs_upload());

    with_upload_pass(|upload_pass| bucket.upload(upload_pass));
    assert!(!bucket.needs_upload());
}

type Records = Arc<Mutex<Vec<(EventSeverity, Event, i64, String)>>>;

struct Recorder(Records);

impl Observer for Recorder {
    fn on_record(&self, severity: EventSeverity, event: Event, code: i64, msg: &str) -> bool {
        self.0.lock().unwrap().push((severity, event, code, msg.to_owned()));
        true
    }
}

/// `FixtureLog` for the calling thread.
struct FixtureLog(Records);

impl FixtureLog {
    fn new() -> Self {
        let records: Records = Arc::new(Mutex::new(Vec::new()));
        Log::set_thread_observer(Some(Box::new(Recorder(records.clone()))));
        FixtureLog(records)
    }
    fn count(&self, severity: EventSeverity, event: Event, msg: &str) -> usize {
        self.0.lock().unwrap().iter().filter(|m| m.0 == severity && m.1 == event && m.2 == -1 && m.3 == msg).count()
    }
    fn is_empty(&self) -> bool {
        self.0.lock().unwrap().is_empty()
    }
}

impl Drop for FixtureLog {
    fn drop(&mut self) {
        Log::set_thread_observer(None);
    }
}

// FillBucket.TracksSDFPatternsPerLayer
#[test]
fn tracks_sdf_patterns_per_layer() {
    let log = FixtureLog::new();
    let layout = FillLayoutProperties::PossiblyEvaluated::default();
    let mut bucket = FillBucket::new(&layout, &BTreeMap::new(), 5.0, 1);

    bucket.record_sdf_pattern("sdf-layer", true);
    bucket.record_sdf_pattern("rgba-layer", false);

    assert!(bucket.is_sdf_pattern("sdf-layer"));
    assert!(!bucket.is_sdf_pattern("rgba-layer"));
    assert!(!bucket.is_sdf_pattern("missing-layer"));
    assert!(log.is_empty());
}

// FillBucket.WarnsOnceForMixedPatternTypesPerBucket
#[test]
fn warns_once_for_mixed_pattern_types_per_bucket() {
    let log = FixtureLog::new();
    let layout = FillLayoutProperties::PossiblyEvaluated::default();
    let mut first_bucket = FillBucket::new(&layout, &BTreeMap::new(), 5.0, 1);
    let mut second_bucket = FillBucket::new(&layout, &BTreeMap::new(), 5.0, 1);

    first_bucket.record_sdf_pattern("mixed-layer", true);
    first_bucket.record_sdf_pattern("mixed-layer", false);
    first_bucket.record_sdf_pattern("mixed-layer", false);
    second_bucket.record_sdf_pattern("mixed-layer", true);
    second_bucket.record_sdf_pattern("mixed-layer", false);

    assert_eq!(
        2,
        log.count(
            EventSeverity::Warning,
            Event::Style,
            "Style sheet warning: Cannot mix SDF and non-SDF fill patterns in layer \"mixed-layer\""
        )
    );
}
