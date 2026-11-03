// Ports the render-source tests of test/style/source.test.cpp that apply to
// the ported source types (GeoJSON).
//
// TODO(port): the vector, raster, raster-dem, image and custom geometry
// source tests (M4 sources).

use mln_actor::{RunLoop, TaggedScheduler, scheduler};
use mln_map::transform::{NullTransformObserver, Transform};
use mln_map_types::{CameraOptions, ConstrainMode, MapDebugOptions, MapMode, TileLodMode, ViewportMode};
use mln_renderer::render_source::{self as render_source, RenderSource, RenderSourceObserver};
use mln_storage::response::{Error as ResponseError, Reason};
use mln_storage::stub_file_source::{ResponseType, StubFileSource};
use mln_storage::{Kind, Response};
use mln_style::layers::line_layer::Impl as LineLayerImpl;
use mln_style::layers::{LineLayer, LineLayerProperties};
use mln_style::sources::tile_source::{UrlOrTileset, VectorSource};
use mln_style_types::SourceType;
use mln_tile_id::OverscaledTileID;
use mln_util::exception::ExceptionPtr;
use mln_util::tileset::{Tileset, VectorEncoding};
use std::cell::RefCell;
use mln_renderer::sources::render_geojson_source::RenderGeoJSONSource;
use mln_renderer::tile_parameters::TileParameters;
use mln_style::layer::{Layer, static_immutable_cast};
use mln_style::layer_properties::LayerProperties;
use mln_style::layers::circle_layer::Impl as CircleLayerImpl;
use mln_style::layers::{CircleLayer, CircleLayerProperties};
use mln_style::source::{self, Source};
use mln_style::sources::geojson_source::{GeoJSONOptions, GeoJSONSource, create_geojson_data, get_sequenced};
use mln_style::sources::geojson_source_impl;
use mln_util::geo::LatLng;
use mln_util::identity::SimpleIdentity;
use mln_util::immutable::Immutable;
use mln_util::size::Size;
use std::rc::Rc;
use std::sync::Arc;

/// `SourceTest`.
struct SourceTest {
    _run_loop: RunLoop,
    transform: Transform,
    thread_pool: TaggedScheduler,
}

impl SourceTest {
    fn new() -> Self {
        let mut transform = Transform::new(
            Rc::new(NullTransformObserver),
            ConstrainMode::HeightOnly,
            ViewportMode::Default,
        );
        transform.resize(Size::new(512, 512)).unwrap();
        transform.jump_to(&CameraOptions {
            center: Some(LatLng::default()),
            zoom: Some(0.0),
            ..Default::default()
        });
        SourceTest {
            _run_loop: RunLoop::new(),
            transform,
            thread_pool: TaggedScheduler::new(scheduler::get_background(), SimpleIdentity::new()),
        }
    }

    fn tile_parameters(&self, map_mode: MapMode) -> TileParameters<'_> {
        self.tile_parameters_with(map_mode, None)
    }

    fn tile_parameters_with(
        &self,
        map_mode: MapMode,
        file_source: Option<Arc<dyn mln_storage::FileSource>>,
    ) -> TileParameters<'_> {
        TileParameters {
            pixel_ratio: 1.0,
            debug_options: MapDebugOptions::NO_DEBUG,
            transform_state: self.transform.get_state(),
            file_source,
            mode: map_mode,
            prefetch_zoom_delta: 0,
            thread_pool: self.thread_pool.clone(),
            tile_lod_min_radius: 3.0,
            tile_lod_scale: 1.0,
            tile_lod_pitch_threshold: (60.0 / 180.0) * std::f64::consts::PI,
            tile_lod_zoom_shift: 0.0,
            tile_lod_mode: TileLodMode::Default,
            global_state: None,
            is_update_synchronous: false,
            capture_rendered_features: false,
            image_manager: mln_renderer::image_manager::ImageManager::create(),
            glyph_manager: mln_renderer::text::glyph_manager::GlyphManager::new(None),
            dynamic_texture_atlas: std::sync::Arc::new(
                mln_renderer::gfx::dynamic_texture_atlas::DynamicTextureAtlas::new(),
            ),
        }
    }
}

impl Drop for SourceTest {
    fn drop(&mut self) {
        self.thread_pool.wait_for_empty();
    }
}

#[test]
fn geojson_source_tiles_after_data_reset() {
    let test = SourceTest::new();
    let source = GeoJSONSource::with_default_options("source");
    let geojson = geojson::parse(
        r#"{"geometry": {"type": "Point", "coordinates": [1.1, 1.1]}, "type": "Feature", "properties": {}}"#,
    )
    .unwrap();
    let geojson_data = create_geojson_data(&geojson, get_sequenced(), &GeoJSONOptions::default_options()).unwrap();
    source.set_geojson_data(Some(geojson_data.clone()));
    let mut render_source = RenderGeoJSONSource::new(
        source::static_immutable_cast::<geojson_source_impl::Impl>(&source.base_impl()),
        &test.thread_pool,
    );

    let layer = CircleLayer::new("id", "source");
    let layer_properties: Immutable<dyn LayerProperties> = Arc::new(CircleLayerProperties::new(
        static_immutable_cast::<CircleLayerImpl>(&layer.base_impl()),
    ));
    let layers = vec![layer_properties];

    let continuous = test.tile_parameters(MapMode::Continuous);
    render_source.update(source.base_impl(), &layers, true, true, &continuous);
    // loaded == false, means that the source contains pending tiles.
    assert!(!render_source.is_loaded());

    source.set_geojson_data(None);
    render_source.update(source.base_impl(), &layers, true, true, &continuous);
    assert!(!render_source.is_loaded()); // Tiles remain in continuous mode.

    source.set_geojson_data(Some(geojson_data));
    render_source.update(source.base_impl(), &layers, true, true, &continuous);
    assert!(!render_source.is_loaded());

    source.set_geojson_data(None);
    render_source.update(source.base_impl(), &layers, true, true, &test.tile_parameters(MapMode::Static));
    assert!(render_source.is_loaded()); // Tiles are reset in static mode.
}

/// `StubRenderSourceObserver` (test/src/mln/test/stub_render_source_observer.hpp).
#[derive(Default)]
struct StubRenderSourceObserver {
    tile_changed: RefCell<Option<Box<dyn Fn(&str, &OverscaledTileID)>>>,
    tile_error: RefCell<Option<Box<dyn Fn(&str, &OverscaledTileID, ExceptionPtr)>>>,
}

impl RenderSourceObserver for StubRenderSourceObserver {
    fn on_tile_changed(&self, source_id: &str, tile_id: &OverscaledTileID) {
        if let Some(f) = &*self.tile_changed.borrow() {
            f(source_id, tile_id);
        }
    }

    fn on_tile_error(&self, source_id: &str, tile_id: &OverscaledTileID, error: ExceptionPtr) {
        if let Some(f) = &*self.tile_error.borrow() {
            f(source_id, tile_id, error);
        }
    }
}

fn water_line_layer() -> Vec<Immutable<dyn LayerProperties>> {
    let mut layer = LineLayer::new("id", "source");
    layer.set_source_layer("water");
    vec![Arc::new(LineLayerProperties::new(static_immutable_cast::<LineLayerImpl>(&layer.base_impl())))]
}

fn vector_source(file_source: &StubFileSource) -> VectorSource {
    let tileset = Tileset {
        tiles: vec!["tiles".to_owned()],
        ..Default::default()
    };
    let source = VectorSource::new("source", UrlOrTileset::Tileset(tileset), None, None, VectorEncoding::Mapbox);
    source.load_description(file_source);
    source
}

#[test]
fn vector_tile_empty() {
    let test = SourceTest::new();
    let file_source = Arc::new(StubFileSource::new(ResponseType::Asynchronous));
    file_source.set_response(Kind::Tile, |_resource| {
        let mut response = Response::new();
        response.no_content = true;
        Some(response)
    });

    let layers = water_line_layer();
    let source = vector_source(&file_source);

    let observer = Rc::new(StubRenderSourceObserver::default());
    let changed = Rc::new(RefCell::new(false));
    {
        let changed = changed.clone();
        let handle = test._run_loop.handle();
        *observer.tile_changed.borrow_mut() = Some(Box::new(move |source_id, _tile_id| {
            assert_eq!("source", source_id);
            *changed.borrow_mut() = true;
            handle.stop();
        }));
    }
    *observer.tile_error.borrow_mut() = Some(Box::new(|_, _, _| panic!("Should never be called")));

    let mut render_source = render_source::create(&source.base_impl(), &test.thread_pool).unwrap();
    render_source.set_observer(observer.clone());
    let fs: Arc<dyn mln_storage::FileSource> = file_source.clone();
    render_source.update(source.base_impl(), &layers, true, true, &test.tile_parameters_with(MapMode::Continuous, Some(fs)));

    test._run_loop.run();
    assert!(*changed.borrow());
}

#[test]
fn vector_tile_fail() {
    let test = SourceTest::new();
    let file_source = Arc::new(StubFileSource::new(ResponseType::Asynchronous));
    file_source.set_response(Kind::Tile, |_resource| {
        let mut response = Response::new();
        response.error = Some(Box::new(ResponseError::new(Reason::Other, "Failed by the test case")));
        Some(response)
    });

    let layers = water_line_layer();
    let source = vector_source(&file_source);

    let observer = Rc::new(StubRenderSourceObserver::default());
    let failed = Rc::new(RefCell::new(false));
    {
        let failed = failed.clone();
        let handle = test._run_loop.handle();
        *observer.tile_error.borrow_mut() = Some(Box::new(move |source_id, tile_id, error| {
            assert_eq!("source", source_id);
            assert_eq!(OverscaledTileID::from_zxy(0, 0, 0), *tile_id);
            assert_eq!("Failed by the test case", error.to_string());
            *failed.borrow_mut() = true;
            handle.stop();
        }));
    }

    let mut render_source = render_source::create(&source.base_impl(), &test.thread_pool).unwrap();
    assert_eq!(SourceType::Vector, render_source.base().base_impl.type_);
    render_source.set_observer(observer.clone());
    let fs: Arc<dyn mln_storage::FileSource> = file_source.clone();
    render_source.update(source.base_impl(), &layers, true, true, &test.tile_parameters_with(MapMode::Continuous, Some(fs)));

    test._run_loop.run();
    assert!(*failed.borrow());
}
