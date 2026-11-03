// Ports include/mln/map/map.hpp and src/mln/map/map.cpp (the parts M1 runs).

use crate::map_impl::{MapImpl, StillImageCallback, StillImageRequest, StyleMut};
use crate::map_observer::MapObserver;
use crate::map_options::MapOptions;
use mln_map_types::{
    AnimationOptions, BoundOptions, CameraOptions, ConstrainMode, MapDebugOptions, MapMode, NorthOrientation,
    ProjectionMode, TileLodMode, TransformState, ViewportMode,
};
use mln_renderer::{ExceptionPtr, RendererFrontend};
use mln_storage::{ClientOptions, FileSourceManager, FileSourceType, ResourceOptions};
use mln_style::Style;
use mln_util::exception::MisuseException;
use mln_util::geo::{EdgeInsets, ScreenCoordinate};
use mln_util::size::Size;
use std::cell::Ref;
use std::rc::Rc;

pub struct Map {
    impl_: Rc<MapImpl>,
}

impl Map {
    /// `Map(RendererFrontend&, MapObserver&, const MapOptions&)` with the
    /// default `ResourceOptions()` and `ClientOptions()`.
    ///
    /// C++ keeps a reference to the frontend; frontends are handles here
    /// (`Clone` shares the same renderer), so the map keeps a clone.
    pub fn new<F>(frontend: &F, observer: Rc<dyn MapObserver>, map_options: MapOptions) -> Map
    where
        F: RendererFrontend + Clone + 'static,
    {
        Self::with_resource_options(
            frontend,
            observer,
            map_options,
            &ResourceOptions::default(),
            &ClientOptions::default(),
        )
    }

    /// `Map(RendererFrontend&, MapObserver&, const MapOptions&, const
    /// ResourceOptions&, const ClientOptions&)`: the file source is the
    /// FileSourceManager's `ResourceLoader` for these options.
    // TODO(port): ActionJournalOptions.
    pub fn with_resource_options<F>(
        frontend: &F,
        observer: Rc<dyn MapObserver>,
        map_options: MapOptions,
        resource_options: &ResourceOptions,
        client_options: &ClientOptions,
    ) -> Map
    where
        F: RendererFrontend + Clone + 'static,
    {
        let file_source =
            FileSourceManager::get().get_file_source(FileSourceType::ResourceLoader, resource_options, client_options);
        Map {
            impl_: MapImpl::new(Box::new(frontend.clone()), observer, file_source, &map_options),
        }
    }

    /// `getClientOptions()`.
    pub fn get_client_options(&self) -> ClientOptions {
        match &self.impl_.file_source {
            Some(file_source) => file_source.get_client_options(),
            None => ClientOptions::default(),
        }
    }


    /// Register a callback that will get called (on the render thread) when
    /// all resources have been loaded and a complete render occurs.
    pub fn render_still(&mut self, callback: StillImageCallback) {
        let impl_ = &self.impl_;
        if impl_.mode != MapMode::Static && impl_.mode != MapMode::Tile {
            callback(Some(misuse("Map is not in static or tile image render modes")));
            return;
        }

        if impl_.still_image_request.borrow().is_some() {
            callback(Some(misuse("Map is currently rendering an image")));
            return;
        }

        let last_error = impl_.style.borrow().impl_.get_last_error();
        if let Some(error) = last_error {
            callback(Some(error));
            return;
        }

        *impl_.still_image_request.borrow_mut() = Some(StillImageRequest { callback });

        impl_.on_update();
    }

    /// `renderStill(const CameraOptions&, MapDebugOptions, StillImageCallback)`.
    pub fn render_still_with_camera(
        &mut self,
        camera: &CameraOptions,
        debug_options: MapDebugOptions,
        callback: StillImageCallback,
    ) {
        self.impl_.camera_mutated.set(true);
        self.impl_.debug_options.set(debug_options);
        self.impl_.transform.borrow_mut().jump_to(camera);
        self.render_still(callback);
    }

    /// Triggers a repaint.
    pub fn trigger_repaint(&mut self) {
        self.impl_.on_update();
    }

    // MARK: - Style

    /// `const style::Style& getStyle() const`.
    pub fn get_style(&self) -> Ref<'_, Style> {
        self.impl_.style.borrow()
    }

    /// `style::Style& getStyle()`: a guard that derefs to `&mut Style`; the
    /// style's observer calls made through it reach the map when it drops.
    pub fn get_style_mut(&mut self) -> StyleMut<'_> {
        StyleMut::new(&self.impl_)
    }

    // TODO(port): setStyle(std::unique_ptr<style::Style>).

    // MARK: - Transitions

    pub fn cancel_transitions(&mut self) {
        self.impl_.transform.borrow_mut().cancel_transitions();
        self.impl_.on_update();
    }

    pub fn set_gesture_in_progress(&mut self, in_progress: bool) {
        self.impl_.transform.borrow_mut().set_gesture_in_progress(in_progress);
        self.impl_.on_update();
    }

    pub fn is_gesture_in_progress(&self) -> bool {
        self.impl_.transform.borrow().is_gesture_in_progress()
    }

    pub fn is_rotating(&self) -> bool {
        self.impl_.transform.borrow().is_rotating()
    }

    pub fn is_scaling(&self) -> bool {
        self.impl_.transform.borrow().is_scaling()
    }

    pub fn is_panning(&self) -> bool {
        self.impl_.transform.borrow().is_panning()
    }

    // MARK: - Camera

    pub fn get_camera_options(&self, padding: Option<EdgeInsets>) -> CameraOptions {
        self.impl_.transform.borrow().get_camera_options(padding)
    }

    pub fn jump_to(&mut self, camera: &CameraOptions) {
        self.impl_.jump_to(camera);
    }

    pub fn ease_to(&mut self, camera: &CameraOptions, animation: &AnimationOptions) {
        self.impl_.camera_mutated.set(true);
        self.impl_.transform.borrow_mut().ease_to(camera, animation);
        self.impl_.on_update();
    }

    pub fn fly_to(&mut self, camera: &CameraOptions, animation: &AnimationOptions) {
        self.impl_.camera_mutated.set(true);
        self.impl_.transform.borrow_mut().fly_to(camera, animation, false);
        self.impl_.on_update();
    }

    pub fn move_by(&mut self, point: &ScreenCoordinate, animation: &AnimationOptions) {
        self.impl_.camera_mutated.set(true);
        self.impl_.transform.borrow_mut().move_by(point, animation);
        self.impl_.on_update();
    }

    pub fn pitch_by(&mut self, pitch: f64, animation: &AnimationOptions) {
        let current = mln_math::rad2deg(self.impl_.transform.borrow().get_pitch());
        self.ease_to(&CameraOptions::default().with_pitch(Some(current - pitch)), animation);
    }

    pub fn scale_by(&mut self, scale: f64, anchor: Option<ScreenCoordinate>, animation: &AnimationOptions) {
        let zoom = {
            let transform = self.impl_.transform.borrow();
            transform.get_zoom() + transform.get_state().scale_zoom(scale)
        };
        self.ease_to(&CameraOptions::default().with_zoom(Some(zoom)).with_anchor(anchor), animation);
    }

    pub fn rotate_by(&mut self, first: &ScreenCoordinate, second: &ScreenCoordinate, animation: &AnimationOptions) {
        self.impl_.camera_mutated.set(true);
        self.impl_.transform.borrow_mut().rotate_by(first, second, animation);
        self.impl_.on_update();
    }

    // TODO(port): cameraForLatLngBounds, cameraForLatLngs,
    // cameraForGeometry, latLngBoundsForCamera(Unwrapped).

    // MARK: - Bounds

    pub fn set_bounds(&mut self, options: &BoundOptions) -> Result<(), String> {
        let mut change_camera = false;
        let mut camera_options = CameraOptions::default();

        {
            let mut transform = self.impl_.transform.borrow_mut();

            if let Some(bounds) = options.bounds {
                change_camera = true;
                transform.set_lat_lng_bounds(bounds)?;
            }

            if let Some(min_zoom) = options.min_zoom {
                transform.set_min_zoom(min_zoom);
                if transform.get_zoom() < min_zoom {
                    change_camera = true;
                    camera_options = camera_options.with_zoom(options.min_zoom);
                }
            }

            if let Some(max_zoom) = options.max_zoom {
                transform.set_max_zoom(max_zoom);
                if transform.get_zoom() > max_zoom {
                    change_camera = true;
                    camera_options = camera_options.with_zoom(options.max_zoom);
                }
            }

            if let Some(max_pitch) = options.max_pitch {
                transform.set_max_pitch(max_pitch);
                if transform.get_pitch() > transform.get_state().get_max_pitch() {
                    change_camera = true;
                    camera_options = camera_options.with_pitch(options.max_pitch);
                }
            }

            if let Some(min_pitch) = options.min_pitch {
                transform.set_min_pitch(min_pitch);
                if transform.get_pitch() < transform.get_state().get_min_pitch() {
                    change_camera = true;
                    camera_options = camera_options.with_pitch(options.min_pitch);
                }
            }
        }

        if change_camera {
            self.jump_to(&camera_options);
        }
        Ok(())
    }

    /// Returns the current map bound options. All optional fields in
    /// BoundOptions are set.
    pub fn get_bounds(&self) -> BoundOptions {
        let transform = self.impl_.transform.borrow();
        let state = transform.get_state();
        BoundOptions::default()
            .with_lat_lng_bounds(state.get_lat_lng_bounds())
            .with_min_zoom(state.get_min_zoom())
            .with_max_zoom(state.get_max_zoom())
            .with_min_pitch(mln_math::rad2deg(state.get_min_pitch()))
            .with_max_pitch(mln_math::rad2deg(state.get_max_pitch()))
    }

    // MARK: - Map options

    pub fn set_north_orientation(&mut self, orientation: NorthOrientation) {
        self.impl_.transform.borrow_mut().set_north_orientation(orientation);
        self.impl_.on_update();
    }

    pub fn set_constrain_mode(&mut self, mode: ConstrainMode) {
        self.impl_.transform.borrow_mut().set_constrain_mode(mode);
        self.impl_.on_update();
    }

    pub fn set_viewport_mode(&mut self, mode: ViewportMode) {
        self.impl_.transform.borrow_mut().set_viewport_mode(mode);
        self.impl_.on_update();
    }

    /// C++ throws "failed to resize: size is empty"; the port returns it.
    pub fn set_size(&mut self, size: Size) -> Result<(), String> {
        self.impl_.transform.borrow_mut().resize(size)?;
        self.impl_.on_update();
        Ok(())
    }

    pub fn set_frustum_offset(&mut self, frustum_offset: &EdgeInsets) {
        self.impl_.transform.borrow_mut().set_frustum_offset(frustum_offset);
        self.impl_.on_update();
    }

    pub fn get_frustum_offset(&self) -> EdgeInsets {
        self.impl_.transform.borrow().get_frustum_offset()
    }

    pub fn get_map_options(&self) -> MapOptions {
        let transform = self.impl_.transform.borrow();
        let state = transform.get_state();
        MapOptions::new()
            .with_north_orientation(transform.get_north_orientation())
            .with_map_mode(self.impl_.mode)
            .with_constrain_mode(state.get_constrain_mode())
            .with_viewport_mode(state.get_viewport_mode())
            .with_cross_source_collisions(self.impl_.cross_source_collisions)
            .with_size(state.get_size())
            .with_pixel_ratio(self.impl_.pixel_ratio)
    }

    // MARK: - Projection mode

    pub fn set_projection_mode(&mut self, options: &ProjectionMode) {
        self.impl_.transform.borrow_mut().set_projection_mode(options);
        self.impl_.on_update();
    }

    pub fn get_projection_mode(&self) -> ProjectionMode {
        self.impl_.transform.borrow().get_projection_mode()
    }

    // MARK: - Transform

    pub fn get_transform_state(&self) -> TransformState {
        self.impl_.transform.borrow().get_state().clone()
    }

    // MARK: - Tile LOD

    pub fn set_tile_lod_min_radius(&mut self, radius: f64) {
        if self.impl_.tile_lod_min_radius.get() == radius {
            return;
        }
        self.impl_.tile_lod_min_radius.set(radius);
        self.impl_.on_update();
    }

    pub fn get_tile_lod_min_radius(&self) -> f64 {
        self.impl_.tile_lod_min_radius.get()
    }

    pub fn set_tile_lod_scale(&mut self, scale: f64) {
        if self.impl_.tile_lod_scale.get() == scale {
            return;
        }
        self.impl_.tile_lod_scale.set(scale);
        self.impl_.on_update();
    }

    pub fn get_tile_lod_scale(&self) -> f64 {
        self.impl_.tile_lod_scale.get()
    }

    pub fn set_tile_lod_pitch_threshold(&mut self, threshold: f64) {
        if self.impl_.tile_lod_pitch_threshold.get() == threshold {
            return;
        }
        self.impl_.tile_lod_pitch_threshold.set(threshold);
        self.impl_.on_update();
    }

    pub fn get_tile_lod_pitch_threshold(&self) -> f64 {
        self.impl_.tile_lod_pitch_threshold.get()
    }

    pub fn set_tile_lod_zoom_shift(&mut self, shift: f64) {
        if self.impl_.tile_lod_zoom_shift.get() == shift {
            return;
        }
        self.impl_.tile_lod_zoom_shift.set(shift);
        self.impl_.on_update();
    }

    pub fn get_tile_lod_zoom_shift(&self) -> f64 {
        self.impl_.tile_lod_zoom_shift.get()
    }

    pub fn set_tile_lod_mode(&mut self, mode: TileLodMode) {
        if self.impl_.tile_lod_mode.get() == mode {
            return;
        }
        self.impl_.tile_lod_mode.set(mode);
        self.impl_.on_update();
    }

    pub fn get_tile_lod_mode(&self) -> TileLodMode {
        self.impl_.tile_lod_mode.get()
    }

    // TODO(port): pixelForLatLng, latLngForPixel, annotations, prefetch zoom
    // delta, rendering stats view, free camera options, rendered features.

    // MARK: - Debug

    pub fn set_debug(&mut self, debug_options: MapDebugOptions) {
        self.impl_.debug_options.set(debug_options);
        self.impl_.on_update();
    }

    pub fn get_debug(&self) -> MapDebugOptions {
        self.impl_.debug_options.get()
    }

    pub fn is_fully_loaded(&self) -> bool {
        self.impl_.style.borrow().impl_.is_loaded() && self.impl_.renderer_fully_loaded.get()
    }
}

fn misuse(message: &str) -> ExceptionPtr {
    MisuseException::ptr(message)
}
