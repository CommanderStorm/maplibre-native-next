//! maplibre-native-next in the browser without Emscripten: the port built
//! for wasm32-unknown-unknown, rendering with wgpu's WebGPU backend straight
//! into a canvas, with a small JavaScript API through wasm-bindgen.
//!
//! Everything runs on the page's thread. The map, its renderer, the file
//! sources (fetch) and the "worker" tasks (tile parsing, layout, sprites)
//! share it: the background schedulers start no threads in this build, and
//! `work` runs their queued tasks within a time budget. JavaScript drives it:
//!
//! - `render()` in each animation frame: applies the input, runs the
//!   RunLoop's tasks (responses, map updates) and draws when the map has a
//!   new update. Nothing yields between acquiring the canvas texture and
//!   submitting, so frames can't come out blank.
//! - `work(budget_ms)` right after the frame (a posted message, so the frame
//!   is presented first): runs background tasks until the budget is spent,
//!   then delivers their results.

use maplibre_native_next::mln_actor::RunLoop;
use maplibre_native_next::mln_actor::thread_pool::{has_background_tasks, run_background_tasks};
use maplibre_native_next::mln_gfx::{BackendScope, RendererBackend, ScopeType};
use maplibre_native_next::mln_map::{CameraOptions, Map, MapLoadError, MapObserver, MapOptions};
use maplibre_native_next::mln_renderer::{Renderer, RendererFrontend, RendererObserver, UpdateParameters};
use maplibre_native_next::mln_storage::{ClientOptions, ResourceOptions};
use maplibre_native_next::mln_util::geo::LatLng;
use maplibre_native_next::mln_util::logging::{Event, EventSeverity, Log, Observer};
use maplibre_native_next::mln_util::size::Size;
use maplibre_native_next::mln_util::time::Instant;
use maplibre_native_next::mln_webgpu::SurfaceBackend;
use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;
use wasm_bindgen::prelude::*;

const TILE_SIZE: f64 = 512.0;
const MAX_ZOOM: f64 = 19.0;

#[wasm_bindgen(start)]
fn start() {
    console_error_panic_hook::set_once();
    Log::set_observer(Box::new(ConsoleLog));
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

#[derive(Default)]
struct ConsoleMapObserver {
    loaded: Cell<bool>,
}

impl MapObserver for ConsoleMapObserver {
    fn on_did_fail_loading_map(&self, error: MapLoadError, what: &str) {
        web_sys::console::error_1(&JsValue::from_str(&format!("map failed to load ({error:?}): {what}")));
    }

    fn on_did_finish_loading_map(&self) {
        if !self.loaded.replace(true) {
            web_sys::console::log_1(&JsValue::from_str("map loaded"));
        }
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

// MARK: - Camera (as the Emscripten demo's main.c)

#[derive(Clone, Copy)]
struct View {
    latitude: f64,
    longitude: f64,
    zoom: f64,
    width: u32,
    height: u32,
}

fn x_of(longitude: f64, world: f64) -> f64 {
    (longitude + 180.0) / 360.0 * world
}

fn y_of(latitude: f64, world: f64) -> f64 {
    let s = (latitude * PI / 180.0).sin();
    (0.5 - ((1.0 + s) / (1.0 - s)).ln() / (4.0 * PI)) * world
}

fn longitude_of(x: f64, world: f64) -> f64 {
    x / world * 360.0 - 180.0
}

fn latitude_of(y: f64, world: f64) -> f64 {
    let n = PI - 2.0 * PI * y / world;
    180.0 / PI * n.sinh().atan()
}

#[derive(Default)]
struct Input {
    pan_x: f64,
    pan_y: f64,
    zoom_delta: f64,
    anchor_x: f64,
    anchor_y: f64,
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

// MARK: - The JavaScript API

/// A continuous map rendering into a canvas.
#[wasm_bindgen]
pub struct WebMap {
    // Dropped first: the map resets the renderer through the frontend.
    map: Map,
    frontend: Frontend,
    _observer: Rc<ConsoleMapObserver>,
    run_loop: RunLoop,
    pixel_ratio: f64,
    view: View,
    input: Input,
}

#[wasm_bindgen]
impl WebMap {
    /// Creates the WebGPU device, a surface on `canvas` and the map, at
    /// `width` x `height` CSS pixels.
    pub async fn create(canvas: web_sys::HtmlCanvasElement, width: u32, height: u32, pixel_ratio: f64) -> Result<WebMap, JsValue> {
        let size = Size::new(width.max(1), height.max(1));
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas)).map_err(js_error)?;
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
        }));
        let observer = Rc::new(ConsoleMapObserver::default());
        let map = Map::with_resource_options(
            &frontend,
            observer.clone(),
            MapOptions::new().with_size(size).with_pixel_ratio(pixel_ratio as f32),
            &ResourceOptions::default(),
            &ClientOptions::default(),
        );
        Ok(WebMap {
            map,
            frontend,
            _observer: observer,
            run_loop,
            pixel_ratio,
            view: View { latitude: 0.0, longitude: 0.0, zoom: 0.0, width: size.width, height: size.height },
            input: Input::default(),
        })
    }

    /// Loads a style from a URL.
    pub fn set_style_url(&mut self, url: &str) {
        self.map.get_style_mut().load_url(url);
    }

    /// Moves the camera.
    pub fn jump_to(&mut self, latitude: f64, longitude: f64, zoom: f64) {
        self.view.latitude = latitude;
        self.view.longitude = longitude;
        self.view.zoom = zoom;
        self.jump();
    }

    /// Pans by a drag of (`dx`, `dy`) CSS pixels, at the next frame.
    pub fn pan(&mut self, dx: f64, dy: f64) {
        self.input.pan_x += dx;
        self.input.pan_y += dy;
    }

    /// Zooms by `delta` around the CSS pixel (`x`, `y`), at the next frame.
    pub fn zoom(&mut self, delta: f64, x: f64, y: f64) {
        self.input.anchor_x = x;
        self.input.anchor_y = y;
        self.input.zoom_delta += delta;
    }

    /// Resizes the map to `width` x `height` CSS pixels.
    pub fn resize(&mut self, width: u32, height: u32) {
        let size = Size::new(width.max(1), height.max(1));
        if size.width == self.view.width && size.height == self.view.height {
            return;
        }
        self.view.width = size.width;
        self.view.height = size.height;
        let _ = self.map.set_size(size);
        self.frontend.0.backend.set_framebuffer_size(physical(size, self.pixel_ratio));
    }

    /// One animation frame: applies the input, runs the RunLoop's tasks and
    /// draws the map's latest update if it is new. Returns whether it drew.
    pub fn render(&mut self) -> bool {
        if self.take_input() {
            self.jump();
        }
        self.run_loop.run_once();
        self.frontend.render()
    }

    /// Runs background tasks (tile parsing and layout, sprites) for up to
    /// `budget_ms`, then the RunLoop's tasks they produced. Returns whether
    /// background tasks remain.
    pub fn work(&mut self, budget_ms: f64) -> bool {
        let deadline = Instant::now() + Duration::from_secs_f64(budget_ms.max(0.0) / 1000.0);
        let remaining = run_background_tasks(deadline);
        self.run_loop.run_once();
        remaining
    }

    /// Whether background tasks are waiting for `work`.
    pub fn has_work(&self) -> bool {
        has_background_tasks()
    }

    /// Whether the map has an update that the next `render` would draw.
    pub fn needs_render(&self) -> bool {
        self.frontend.0.dirty.get()
    }

    /// Whether every tile and resource the map needs is loaded.
    pub fn is_fully_loaded(&self) -> bool {
        self.map.is_fully_loaded()
    }
}

impl WebMap {
    /// Applies pending input to the view; returns whether it changed.
    fn take_input(&mut self) -> bool {
        let input = std::mem::take(&mut self.input);
        let (dx, dy, dz) = (input.pan_x, input.pan_y, input.zoom_delta);
        if dx == 0.0 && dy == 0.0 && dz == 0.0 {
            return false;
        }
        let v = &mut self.view;
        let mut world = TILE_SIZE * 2f64.powf(v.zoom);
        let mut x = x_of(v.longitude, world) - dx;
        let mut y = y_of(v.latitude, world) - dy;
        if dz != 0.0 {
            // Keep the point under the cursor in place.
            let ax = input.anchor_x - f64::from(v.width) / 2.0;
            let ay = input.anchor_y - f64::from(v.height) / 2.0;
            let zoom = (v.zoom + dz).clamp(0.0, MAX_ZOOM);
            let scale = 2f64.powf(zoom - v.zoom);
            x = (x + ax) * scale - ax;
            y = (y + ay) * scale - ay;
            v.zoom = zoom;
            world = TILE_SIZE * 2f64.powf(v.zoom);
        }
        x = (x % world + world) % world;
        y = y.clamp(0.0, world);
        v.longitude = longitude_of(x, world);
        v.latitude = latitude_of(y, world);
        true
    }

    fn jump(&mut self) {
        let center = LatLng::new(self.view.latitude, self.view.longitude).ok();
        self.map.jump_to(&CameraOptions::default().with_center(center).with_zoom(Some(self.view.zoom)));
    }
}
