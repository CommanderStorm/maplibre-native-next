// Ports the map side of render-test/runner.{hpp,cpp}: `TestRunnerMapObserver`,
// `TestRunner::Impl` (observer, HeadlessFrontend, Map), `swapBehavior`,
// `resetContext` and the map/style calls of the operations, over mln_map and
// mln_platform_default.

use mln_actor::RunLoop;
use mln_gfx::{ContextMode, SwapBehaviour};
use mln_map::{BoundOptions, Map, MapLoadError, MapObserver, MapOptions, ProjectionMode, RenderMode};
use mln_platform_default::HeadlessFrontend;
use mln_util::image::PremultipliedImage;
use render_test::metadata::{MapMode, TestMetadata};
use render_test::runner::{CameraOptions, ImplFactory, MapError, ResourceOptions, TestRunnerImpl};
use render_test_style as style_ops;
use mln_storage::{ClientOptions, FileSource, FileSourceManager, FileSourceType};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

/// `TestRunnerMapObserver` (runner.hpp).
#[derive(Default)]
pub struct TestRunnerMapObserver {
    pub map_load_failure: Cell<bool>,
    pub finish_rendering_map: Cell<bool>,
    pub idle: Cell<bool>,
}

impl TestRunnerMapObserver {
    pub fn reset(&self) {
        self.map_load_failure.set(false);
        self.finish_rendering_map.set(false);
        self.idle.set(false);
    }
}

impl MapObserver for TestRunnerMapObserver {
    fn on_did_fail_loading_map(&self, _error: MapLoadError, _what: &str) {
        self.map_load_failure.set(true);
    }

    fn on_did_finish_rendering_map(&self, mode: RenderMode) {
        if !self.finish_rendering_map.get() {
            self.finish_rendering_map.set(mode == RenderMode::Full);
        }
    }

    fn on_did_become_idle(&self) {
        self.idle.set(true);
    }
}

fn map_mode(mode: MapMode) -> mln_map::MapMode {
    match mode {
        MapMode::Continuous => mln_map::MapMode::Continuous,
        MapMode::Static => mln_map::MapMode::Static,
        MapMode::Tile => mln_map::MapMode::Tile,
    }
}

/// `swapBehavior(mode)` (runner.cpp).
fn swap_behavior(mode: MapMode) -> SwapBehaviour {
    if mode == MapMode::Continuous { SwapBehaviour::Flush } else { SwapBehaviour::NoFlush }
}

/// `TestRunner::Impl`.
pub struct Impl {
    pub observer: Rc<TestRunnerMapObserver>,
    pub frontend: HeadlessFrontend,
    /// `std::shared_ptr<FileSource> fileSource`: the ProxyFileSource over
    /// the cache database of ResourceOptions::cachePath.
    pub file_source: Option<Arc<dyn FileSource>>,
    pub map: Map,
}

impl Impl {
    /// `TestRunner::Impl::Impl(metadata, resourceOptions, clientOptions)`.
    pub fn new(metadata: &TestMetadata, resource_options: &ResourceOptions) -> Result<Impl, String> {
        let client_options = ClientOptions::default();
        let resource_options = mln_storage::ResourceOptions::default()
            .with_cache_path(resource_options.cache_path.clone())
            .with_api_key(resource_options.api_key.clone());
        let observer = Rc::new(TestRunnerMapObserver::default());
        let frontend = HeadlessFrontend::with_options(
            metadata.size,
            metadata.pixel_ratio,
            swap_behavior(metadata.map_mode),
            ContextMode::Unique,
            true,
        );
        let file_source = FileSourceManager::get().get_file_source(
            FileSourceType::ResourceLoader,
            &resource_options,
            &client_options,
        );
        let mut map = Map::with_resource_options(
            &frontend,
            observer.clone(),
            MapOptions::new()
                .with_map_mode(map_mode(metadata.map_mode))
                .with_size(metadata.size)
                .with_pixel_ratio(metadata.pixel_ratio)
                .with_cross_source_collisions(metadata.cross_source_collisions),
            &resource_options,
            &client_options,
        );
        if let Some(max_pitch) = metadata.max_pitch {
            map.set_bounds(&BoundOptions::default().with_max_pitch(f64::from(max_pitch)))?;
        }
        Ok(Impl {
            observer,
            frontend,
            file_source,
            map,
        })
    }
}

impl TestRunnerImpl for Impl {
    fn reset_context(&mut self, metadata: &TestMetadata) -> Result<(), MapError> {
        self.frontend.with_renderer(|renderer| renderer.clear_data());
        self.frontend.set_size(metadata.size);
        self.map.set_size(metadata.size).map_err(MapError::Exception)?;
        self.map.set_projection_mode(
            &ProjectionMode::default()
                .with_axonometric(metadata.axonometric)
                .with_x_skew(metadata.x_skew)
                .with_y_skew(metadata.y_skew),
        );
        self.map.set_debug(mln_map::MapDebugOptions(metadata.debug.0));
        style_ops::load_json(&mut self.map.get_style_mut(), &metadata.document_json);
        Ok(())
    }

    fn jump_to_default_camera(&mut self) -> Result<(), MapError> {
        let camera = style_ops::default_camera(&self.map.get_style());
        self.map.jump_to(&camera);
        Ok(())
    }

    fn render(&mut self) -> Result<PremultipliedImage, String> {
        self.frontend.render(&mut self.map).map(|result| result.image)
    }

    fn jump_to(&mut self, camera: &CameraOptions) -> Result<(), MapError> {
        let camera = mln_map::CameraOptions::default()
            .with_center(camera.center)
            .with_center_altitude(camera.center_altitude)
            .with_zoom(camera.zoom)
            .with_bearing(camera.bearing)
            .with_pitch(camera.pitch)
            .with_roll(camera.roll);
        self.map.jump_to(&camera);
        Ok(())
    }

    fn load_style_json(&mut self, json: &str) -> Result<(), MapError> {
        style_ops::load_json(&mut self.map.get_style_mut(), json);
        Ok(())
    }

    fn load_style_url(&mut self, url: &str) -> Result<(), MapError> {
        style_ops::load_url(&mut self.map.get_style_mut(), url);
        Ok(())
    }

    fn set_layer_property(
        &mut self,
        layer: &str,
        name: &str,
        value: &serde_json::Value,
    ) -> Result<(), MapError> {
        style_ops::set_layer_property(&mut self.map.get_style_mut(), layer, name, value)
    }

    fn set_layer_zoom_range(&mut self, layer: &str, min: f32, max: f32) -> Result<(), MapError> {
        style_ops::set_layer_zoom_range(&mut self.map.get_style_mut(), layer, min, max)
    }

    fn add_layer(&mut self, json: &str) -> Result<(), MapError> {
        style_ops::add_layer(&mut self.map.get_style_mut(), json)
    }

    fn remove_layer(&mut self, id: &str) -> Result<bool, MapError> {
        Ok(style_ops::remove_layer(&mut self.map.get_style_mut(), id))
    }

    fn set_layer_filter(&mut self, layer: &str, filter: &serde_json::Value) -> Result<(), MapError> {
        style_ops::set_layer_filter(&mut self.map.get_style_mut(), layer, filter)
    }

    fn set_global_state_property(&mut self, name: &str, value: &maplibre_native_base::Value) -> Result<(), MapError> {
        style_ops::set_global_state_property(&mut self.map.get_style_mut(), name, value)
    }

    fn set_feature_state(
        &mut self,
        source_id: &str,
        source_layer: Option<&str>,
        feature_id: &str,
        state: &maplibre_native_base::PropertyMap,
    ) -> Result<bool, MapError> {
        if self.frontend.render(&mut self.map).is_err() {
            return Ok(false);
        }
        self.frontend
            .with_renderer(|renderer| renderer.set_feature_state(source_id, source_layer, feature_id, state));
        Ok(true)
    }

    fn get_feature_state(
        &mut self,
        source_id: &str,
        source_layer: Option<&str>,
        feature_id: &str,
    ) -> Result<bool, MapError> {
        if self.frontend.render(&mut self.map).is_err() {
            return Ok(false);
        }
        self.frontend.with_renderer(|renderer| {
            let _ = renderer.get_feature_state(source_id, source_layer, feature_id);
        });
        Ok(true)
    }

    fn remove_feature_state(
        &mut self,
        source_id: &str,
        source_layer: Option<&str>,
        feature_id: Option<&str>,
        state_key: Option<&str>,
    ) -> Result<bool, MapError> {
        if self.frontend.render(&mut self.map).is_err() {
            return Ok(false);
        }
        self.frontend.with_renderer(|renderer| {
            renderer.remove_feature_state(source_id, source_layer, feature_id, state_key)
        });
        Ok(true)
    }

    // TODO(port): add_source/remove_source, add_image/remove_image (file
    // source), sleep, setTileLod*: the trait's NotPorted defaults until the
    // APIs exist.
}

/// The factory `TestRunner::run` uses for `maps[key]`. Creates the thread's
/// `util::RunLoop` first, as `runRenderTests` does, and keeps it alive.
pub fn factory() -> ImplFactory {
    let run_loop = RunLoop::new();
    Box::new(move |metadata: &TestMetadata, resource_options: &ResourceOptions| {
        let _ = &run_loop;
        Ok(Box::new(Impl::new(metadata, resource_options)?) as Box<dyn TestRunnerImpl>)
    })
}
