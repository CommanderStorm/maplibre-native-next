//! The engine behind the `@bartlwrs/mln-next-web` package: maplibre-native-next
//! built for wasm32-unknown-unknown, rendering with wgpu's WebGPU backend
//! straight into a canvas. The package's JavaScript `Map` (js/) drives an
//! [`Engine`], in a worker or on the page:
//!
//! - Commands change the camera (`jump_to`, `ease_to`, `fly_to`, `fit_bounds`)
//!   or apply gestures (`move_by`, `scale_by`, `rotate_by`, `pitch_by`).
//! - `frame()` runs once per animation frame: it runs the RunLoop's tasks
//!   (responses, map updates) and draws the map's latest update, if it is new.
//!   Nothing yields between acquiring the canvas texture and submitting, so a
//!   frame can't come out blank.
//! - `state()` reports the camera and the transform's projection matrices, so
//!   that the page can answer `getCenter()` or `unproject()` synchronously, as
//!   the core does. `take_events()` reports the map's events.
//!
//! Without shared memory, the background schedulers start no threads:
//! `work(budget_ms)` runs their tasks (tile parsing and layout, sprites)
//! after each frame, on the same thread. Built with shared memory
//! (`+atomics`, build.sh), the thread pools run on Web Workers instead:
//! JavaScript provides `mlnSpawnThread`, which starts a worker on this module
//! and memory that calls `mln_thread_entry`. The engine then has to live in a
//! worker itself, since the page's main thread may not block on a lock.

use maplibre_native_next::geojson::{self, GeoJSON};
use maplibre_native_next::mln_actor::RunLoop;
#[cfg(not(target_feature = "atomics"))]
use maplibre_native_next::mln_actor::thread_pool::{has_background_tasks, run_background_tasks};
use maplibre_native_next::mln_gfx::{BackendScope, RendererBackend, ScopeType};
use maplibre_native_next::mln_map::{
    AnimationOptions, BoundOptions, CameraOptions, Map, MapLoadError, MapObserver, MapOptions,
};
use maplibre_native_next::mln_renderer::query::RenderedQueryOptions;
use maplibre_native_next::mln_renderer::{Renderer, RendererFrontend, RendererObserver, UpdateParameters};
use maplibre_native_next::mln_storage::{ClientOptions, ResourceOptions};
use maplibre_native_next::mln_util::chrono;
use maplibre_native_next::mln_util::geo::{EdgeInsets, LatLng, LatLngBounds, ScreenCoordinate};
use maplibre_native_next::mln_util::logging::{Event, EventSeverity, Log, Observer};
use maplibre_native_next::mln_util::size::Size;
#[cfg(not(target_feature = "atomics"))]
use maplibre_native_next::mln_util::time::Instant;
use maplibre_native_next::mln_webgpu::SurfaceBackend;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;
#[cfg(not(target_feature = "atomics"))]
use std::time::Duration;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();
    Log::set_observer(Box::new(ConsoleLog));
    #[cfg(target_feature = "atomics")]
    maplibre_native_next::mln_actor::spawn::set_spawner(threads::spawn);
}

/// Whether this module was built with shared memory (the thread pools run on
/// Web Workers) or without (`work` runs their tasks).
#[wasm_bindgen]
pub fn threaded() -> bool {
    cfg!(target_feature = "atomics")
}

/// The threaded build's threads: Web Workers on this module and memory.
#[cfg(target_feature = "atomics")]
mod threads {
    use wasm_bindgen::prelude::*;

    type Entry = Box<dyn FnOnce() + Send + 'static>;

    #[wasm_bindgen]
    extern "C" {
        /// Provided by the engine's worker: starts a worker that initializes
        /// this module with `module` and `memory` and calls
        /// `mln_thread_entry(entry)`.
        #[wasm_bindgen(js_name = mlnSpawnThread)]
        fn spawn_thread(module: JsValue, memory: JsValue, entry: u32, name: &str);
    }

    pub fn spawn(name: String, f: Entry) {
        let entry = Box::into_raw(Box::new(f)) as usize as u32;
        spawn_thread(wasm_bindgen::module(), wasm_bindgen::memory(), entry, &name);
    }

    /// Runs a spawned thread's closure, on its worker.
    #[wasm_bindgen]
    pub fn mln_thread_entry(entry: u32) {
        // SAFETY: `entry` is the box `spawn` leaked for this thread; each
        // worker runs it once.
        let f = unsafe { Box::from_raw(entry as usize as *mut Entry) };
        f();
    }
}

// MARK: - Logging

/// The port's log records go to the browser console.
struct ConsoleLog;

impl Observer for ConsoleLog {
    fn on_record(&self, severity: EventSeverity, event: Event, _code: i64, msg: &str) -> bool {
        use maplibre_native_next::mln_util::enum_::Enum;
        let line = JsValue::from_str(&format!("[{}] {msg}", event.to_string()));
        match severity {
            EventSeverity::Error => web_sys::console::error_1(&line),
            EventSeverity::Warning => web_sys::console::warn_1(&line),
            _ => web_sys::console::log_1(&line),
        }
        true
    }
}

// MARK: - Events

/// The map's events, queued for `take_events`: (type, detail).
#[derive(Default)]
struct Events {
    queue: RefCell<Vec<(&'static str, String)>>,
    loaded: Cell<bool>,
}

impl Events {
    fn push(&self, kind: &'static str, detail: impl Into<String>) {
        self.queue.borrow_mut().push((kind, detail.into()));
    }
}

impl MapObserver for Events {
    fn on_did_finish_loading_style(&self) {
        self.push("styledata", "");
    }

    fn on_did_finish_loading_map(&self) {
        if !self.loaded.replace(true) {
            self.push("load", "");
        }
    }

    fn on_did_become_idle(&self) {
        self.push("idle", "");
    }

    fn on_did_fail_loading_map(&self, error: MapLoadError, what: &str) {
        self.push("error", format!("the map failed to load ({error:?}): {what}"));
    }

    fn on_style_image_missing(&self, id: &str) {
        self.push("styleimagemissing", id);
    }
}

// MARK: - Frontend

/// What `GLFWRendererFrontend` holds: the renderer, and the map's latest
/// update until the next frame draws it.
struct FrontendState {
    backend: Rc<SurfaceBackend>,
    renderer: RefCell<Option<Renderer>>,
    parameters: RefCell<Option<Arc<UpdateParameters>>>,
    dirty: Cell<bool>,
    pixel_ratio: f64,
}

#[derive(Clone)]
struct Frontend(Rc<FrontendState>);

impl Frontend {
    /// Draws the latest update into the canvas, if it is new. Returns
    /// whether it drew.
    fn render(&self) -> bool {
        let state = &self.0;
        if !state.dirty.get() {
            return false;
        }
        let Some(parameters) = state.parameters.borrow().clone() else {
            return false;
        };
        // The canvas takes the size of the map this frame draws, in the same
        // task as the draw: resizing it when the page resizes would draw the
        // old map size into the new canvas until the map's resized update
        // arrives.
        state.backend.set_framebuffer_size(physical(parameters.transform_state.get_size(), state.pixel_ratio));
        if !state.backend.acquire_next_texture() {
            return false;
        }
        state.dirty.set(false);
        let backend: &dyn RendererBackend = &*state.backend;
        let _scope = BackendScope::new(backend, ScopeType::Implicit);
        if let Some(renderer) = state.renderer.borrow_mut().as_mut() {
            renderer.render(&parameters);
        }
        true
    }
}

impl RendererFrontend for Frontend {
    fn reset(&self) {
        let renderer = self.0.renderer.borrow_mut().take();
        drop(renderer);
    }

    fn set_observer(&self, observer: Weak<dyn RendererObserver>) {
        if let Some(renderer) = self.0.renderer.borrow_mut().as_mut() {
            renderer.set_observer(Some(observer));
        }
    }

    fn update(&self, parameters: Arc<UpdateParameters>) {
        *self.0.parameters.borrow_mut() = Some(parameters);
        self.0.dirty.set(true);
    }
}

fn physical(size: Size, pixel_ratio: f64) -> Size {
    Size::new(
        (f64::from(size.width) * pixel_ratio).round().max(1.0) as u32,
        (f64::from(size.height) * pixel_ratio).round().max(1.0) as u32,
    )
}

fn js_error(message: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&message.to_string()).into()
}

/// `value`, or `None` for NaN: the JavaScript side passes NaN for an option
/// it leaves out.
fn given(value: f64) -> Option<f64> {
    (!value.is_nan()).then_some(value)
}

fn animation(duration_ms: f64) -> AnimationOptions {
    AnimationOptions {
        duration: given(duration_ms).map(|ms| chrono::Duration::from_nanos((ms.max(0.0) * 1e6) as i64)),
        ..Default::default()
    }
}

fn camera(longitude: f64, latitude: f64, zoom: f64, bearing: f64, pitch: f64) -> CameraOptions {
    let center = match (given(longitude), given(latitude)) {
        (Some(longitude), Some(latitude)) => LatLng::new(latitude, longitude).ok(),
        _ => None,
    };
    CameraOptions::default()
        .with_center(center)
        .with_zoom(given(zoom))
        .with_bearing(given(bearing))
        .with_pitch(given(pitch))
}

// MARK: - The engine

/// A continuous map rendering into a canvas.
#[wasm_bindgen]
pub struct Engine {
    // Dropped first: the map resets the renderer through the frontend.
    map: Map,
    frontend: Frontend,
    events: Rc<Events>,
    run_loop: RunLoop,
    /// The camera `state()` last reported.
    reported: RefCell<Vec<f64>>,
}

#[wasm_bindgen]
impl Engine {
    /// Creates the WebGPU device, a surface on `canvas` (an HTMLCanvasElement
    /// or an OffscreenCanvas) and the map, at `width` x `height` CSS pixels.
    pub async fn create(canvas: JsValue, width: u32, height: u32, pixel_ratio: f64) -> Result<Engine, JsValue> {
        let target = if canvas.is_instance_of::<web_sys::HtmlCanvasElement>() {
            wgpu::SurfaceTarget::Canvas(canvas.unchecked_into())
        } else {
            wgpu::SurfaceTarget::OffscreenCanvas(canvas.unchecked_into())
        };
        let size = Size::new(width.max(1), height.max(1));
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance.create_surface(target).map_err(js_error)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(|e| js_error(format!("WebGPU found no GPU adapter: {e}")))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor { label: Some("MapLibre WebGPU Device"), ..Default::default() })
            .await
            .map_err(|e| js_error(format!("WebGPU device: {e}")))?;
        let format = surface.get_capabilities(&adapter).formats.first().copied().unwrap_or(wgpu::TextureFormat::Bgra8Unorm);
        let backend = Rc::new(SurfaceBackend::from_host(instance, device, queue, surface, format, physical(size, pixel_ratio)));

        let run_loop = RunLoop::get();
        let renderer_backend: Rc<dyn RendererBackend> = backend.clone();
        let renderer = Renderer::new(renderer_backend, pixel_ratio as f32);
        let frontend = Frontend(Rc::new(FrontendState {
            backend,
            renderer: RefCell::new(Some(renderer)),
            parameters: RefCell::new(None),
            dirty: Cell::new(false),
            pixel_ratio,
        }));
        let events = Rc::new(Events::default());
        let map = Map::with_resource_options(
            &frontend,
            events.clone(),
            MapOptions::new().with_size(size).with_pixel_ratio(pixel_ratio as f32),
            &ResourceOptions::default(),
            &ClientOptions::default(),
        );
        Ok(Engine { map, frontend, events, run_loop, reported: RefCell::new(Vec::new()) })
    }

    // MARK: Style

    /// Loads a style from a URL.
    pub fn set_style_url(&mut self, url: &str) {
        self.map.get_style_mut().load_url(url);
    }

    /// Loads a style from its JSON.
    pub fn set_style_json(&mut self, json: &str) {
        self.map.get_style_mut().load_json(json);
    }

    // MARK: Camera

    /// Moves the camera at once. NaN leaves a value unchanged.
    pub fn jump_to(&mut self, longitude: f64, latitude: f64, zoom: f64, bearing: f64, pitch: f64) {
        self.map.jump_to(&camera(longitude, latitude, zoom, bearing, pitch));
    }

    /// Animates the camera over `duration_ms`. NaN leaves a value unchanged.
    pub fn ease_to(&mut self, longitude: f64, latitude: f64, zoom: f64, bearing: f64, pitch: f64, duration_ms: f64) {
        self.map.ease_to(&camera(longitude, latitude, zoom, bearing, pitch), &animation(duration_ms));
    }

    /// Flies the camera along a zoom-out-and-in path. `duration_ms` NaN: the
    /// core picks it from `velocity` (screenfuls a second), NaN for its own.
    #[allow(clippy::too_many_arguments)]
    pub fn fly_to(
        &mut self,
        longitude: f64,
        latitude: f64,
        zoom: f64,
        bearing: f64,
        pitch: f64,
        duration_ms: f64,
        velocity: f64,
    ) {
        let mut options = animation(duration_ms);
        options.velocity = given(velocity);
        self.map.fly_to(&camera(longitude, latitude, zoom, bearing, pitch), &options);
    }

    /// Moves the camera to show the bounds, inside `padding` CSS pixels.
    #[allow(clippy::too_many_arguments)]
    pub fn fit_bounds(
        &mut self,
        west: f64,
        south: f64,
        east: f64,
        north: f64,
        padding_top: f64,
        padding_left: f64,
        padding_bottom: f64,
        padding_right: f64,
        bearing: f64,
        pitch: f64,
        max_zoom: f64,
        duration_ms: f64,
    ) -> Result<(), JsValue> {
        let south_west = LatLng::new(south, west).map_err(js_error)?;
        let north_east = LatLng::new(north, east).map_err(js_error)?;
        let padding = EdgeInsets::new(padding_top, padding_left, padding_bottom, padding_right).map_err(js_error)?;
        let mut options = self.map.camera_for_lat_lng_bounds(
            &LatLngBounds::hull(south_west, north_east),
            &padding,
            given(bearing),
            given(pitch),
        );
        if let (Some(max_zoom), Some(zoom)) = (given(max_zoom), options.zoom) {
            options.zoom = Some(zoom.min(max_zoom));
        }
        // The core returns the bounds' center with the padding to apply in
        // the transform, where it would stay for later camera changes; GL JS
        // keeps none. Symmetric padding needs no offset (asymmetric padding
        // isn't offset either).
        options.padding = None;
        self.map.ease_to(&options, &animation(duration_ms));
        Ok(())
    }

    /// Pans by (`dx`, `dy`) CSS pixels.
    pub fn move_by(&mut self, dx: f64, dy: f64, duration_ms: f64) {
        self.map.move_by(&ScreenCoordinate::new(dx, dy), &animation(duration_ms));
    }

    /// Zooms by a factor of `scale` around the CSS pixel (`x`, `y`), or the
    /// center for NaN.
    pub fn scale_by(&mut self, scale: f64, x: f64, y: f64, duration_ms: f64) {
        let anchor = match (given(x), given(y)) {
            (Some(x), Some(y)) => Some(ScreenCoordinate::new(x, y)),
            _ => None,
        };
        self.map.scale_by(scale, anchor, &animation(duration_ms));
    }

    /// Rotates as a drag from (`x0`, `y0`) to (`x1`, `y1`) turns around the
    /// center.
    pub fn rotate_by(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        self.map.rotate_by(&ScreenCoordinate::new(x0, y0), &ScreenCoordinate::new(x1, y1), &AnimationOptions::default());
    }

    /// Tilts by `degrees` (positive tilts toward the horizon).
    pub fn pitch_by(&mut self, degrees: f64, duration_ms: f64) {
        self.map.pitch_by(-degrees, &animation(duration_ms));
    }

    /// Marks a gesture's start or end: the core then defers expensive work
    /// such as symbol placement updates.
    pub fn set_gesture_in_progress(&mut self, in_progress: bool) {
        self.map.set_gesture_in_progress(in_progress);
    }

    /// Stops any camera animation.
    pub fn cancel_transitions(&mut self) {
        self.map.cancel_transitions();
    }

    /// Limits zoom and pitch. NaN leaves a limit unchanged.
    pub fn set_limits(&mut self, min_zoom: f64, max_zoom: f64, min_pitch: f64, max_pitch: f64) -> Result<(), JsValue> {
        let options = BoundOptions {
            min_zoom: given(min_zoom),
            max_zoom: given(max_zoom),
            min_pitch: given(min_pitch),
            max_pitch: given(max_pitch),
            ..Default::default()
        };
        self.map.set_bounds(&options).map_err(js_error)
    }

    /// Resizes the map to `width` x `height` CSS pixels. The canvas follows
    /// with the first frame at that size.
    pub fn resize(&mut self, width: u32, height: u32) {
        let _ = self.map.set_size(Size::new(width.max(1), height.max(1)));
    }

    /// Renders again even if nothing changed.
    pub fn trigger_repaint(&mut self) {
        self.map.trigger_repaint();
    }

    // MARK: Frames

    /// One animation frame: runs the RunLoop's tasks and draws the map's
    /// latest update if it is new. Returns whether it drew.
    pub fn frame(&mut self) -> bool {
        self.run_loop.run_once();
        self.frontend.render()
    }

    /// Runs background tasks (tile parsing and layout, sprites) for up to
    /// `budget_ms`, then the RunLoop's tasks they produced. Returns whether
    /// background tasks remain. Does nothing in the threaded build.
    pub fn work(&mut self, budget_ms: f64) -> bool {
        #[cfg(not(target_feature = "atomics"))]
        {
            let deadline = Instant::now() + Duration::from_secs_f64(budget_ms.max(0.0) / 1000.0);
            let remaining = run_background_tasks(deadline);
            self.run_loop.run_once();
            remaining
        }
        #[cfg(target_feature = "atomics")]
        {
            let _ = budget_ms;
            false
        }
    }

    /// Whether background tasks are waiting for `work`.
    pub fn has_work(&self) -> bool {
        #[cfg(not(target_feature = "atomics"))]
        return has_background_tasks();
        #[cfg(target_feature = "atomics")]
        return false;
    }

    /// Whether the camera is moving: a gesture or an animation is in progress.
    pub fn is_moving(&self) -> bool {
        self.map.is_gesture_in_progress() || self.map.is_panning() || self.map.is_scaling() || self.map.is_rotating()
    }

    /// Whether every tile and resource the map needs is loaded.
    pub fn is_fully_loaded(&self) -> bool {
        self.map.is_fully_loaded()
    }

    /// The camera and projection, when they changed since the last call
    /// (empty otherwise): longitude, latitude, zoom, bearing, pitch, width,
    /// height, scale, then the transform's coordinate matrix and its inverse
    /// (16 values each, column-major). See js/geo.js for their use.
    pub fn state(&self) -> Vec<f64> {
        let options = self.map.get_camera_options(None);
        let center = options.center.unwrap_or_default();
        let transform = self.map.get_transform_state();
        let size = transform.get_size();
        let mut state = vec![
            center.longitude(),
            center.latitude(),
            options.zoom.unwrap_or(0.0),
            options.bearing.unwrap_or(0.0),
            options.pitch.unwrap_or(0.0),
            f64::from(size.width),
            f64::from(size.height),
            transform.get_scale(),
        ];
        state.extend_from_slice(&transform.get_coord_matrix());
        state.extend_from_slice(&transform.get_inverted_matrix());
        let mut reported = self.reported.borrow_mut();
        if *reported == state {
            return Vec::new();
        }
        reported.clone_from(&state);
        state
    }

    /// The attributions of the style's sources (HTML), without duplicates.
    pub fn attributions(&self) -> Vec<String> {
        let mut attributions: Vec<String> = Vec::new();
        for source in self.map.get_style().get_sources() {
            if let Some(attribution) = source.get_attribution()
                && !attribution.is_empty()
                && !attributions.contains(&attribution)
            {
                attributions.push(attribution);
            }
        }
        attributions
    }

    /// The map's events since the last call, as [type, detail] pairs.
    pub fn take_events(&self) -> js_sys::Array {
        let events = js_sys::Array::new();
        for (kind, detail) in self.events.queue.borrow_mut().drain(..) {
            events.push(&js_sys::Array::of2(&JsValue::from_str(kind), &JsValue::from_str(&detail)));
        }
        events
    }

    // MARK: Queries

    /// The rendered features at a CSS pixel (`x1`, `y1` NaN) or in a box, in
    /// `layers` (all when empty), as a JSON array of
    /// `{feature, source, sourceLayer}`, where `feature` is GeoJSON.
    pub fn query_rendered_features(&self, x0: f64, y0: f64, x1: f64, y1: f64, layers: Vec<String>) -> String {
        let renderer = self.frontend.0.renderer.borrow();
        let Some(renderer) = renderer.as_ref() else {
            return "[]".to_owned();
        };
        let options = RenderedQueryOptions::new((!layers.is_empty()).then_some(layers), None);
        let features = match (given(x1), given(y1)) {
            (Some(x1), Some(y1)) => renderer.query_rendered_features_in_box(
                &ScreenCoordinate::new(x0.min(x1), y0.min(y1)),
                &ScreenCoordinate::new(x0.max(x1), y0.max(y1)),
                &options,
            ),
            _ => renderer.query_rendered_features_at_point(&ScreenCoordinate::new(x0, y0), &options),
        };
        let mut json = String::from("[");
        for (i, feature) in features.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }
            let geojson = geojson::stringify(&GeoJSON::Feature(feature.to_geojson_feature()));
            json.push_str(&format!(
                "{{\"feature\":{geojson},\"source\":{},\"sourceLayer\":{}}}",
                serde_json::to_string(&feature.source).unwrap_or_default(),
                serde_json::to_string(&feature.source_layer).unwrap_or_default(),
            ));
        }
        json.push(']');
        json
    }
}
