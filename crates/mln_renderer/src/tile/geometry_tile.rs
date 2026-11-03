// Ports src/mln/tile/geometry_tile.hpp and src/mln/tile/geometry_tile.cpp.
//
// C++ `GeometryTile` is both the tile the TilePyramid owns and the target of
// its worker's replies (an actor reference to `this`). Here the state the
// replies change is `GeometryTileInner`, an `Rc` registered as the target of
// the tile's mailbox on the render thread; `GeometryTile` owns it, the
// mailbox and the worker actor. Correlation between GeometryTile and
// GeometryTileWorker works as in C++ (see the C++ file's comment):
// `correlationID` for the tile's pending state, the worker's
// `imageCorrelationID` for image replies.
//
// Deviation: C++ runs the worker synchronously on the render thread when the
// source asks for it (`TileParameters::isUpdateSynchronous`, a GeoJSON
// source with `synchronousUpdate`); the port always uses the thread pool.
// TODO(port): the synchronous OptionalActor.
//
// TODO(port): glyph and image requests (GlyphRequestor/ImageRequestor,
// onGlyphsAvailable, onImagesAvailable, getGlyphs, getImages), image patches
// in GeometryTileRenderData, queries, feature state.

use crate::bucket::BucketPtr;
use crate::geometry::feature_index::FeatureIndex;
use crate::render_layer::LayerRenderData;
use crate::render_source::SourcePrepareParameters;
use crate::tile::geometry_tile_worker::{GeometryTileWorker, SharedTileData};
use crate::tile::tile::{Kind, Tile, TileBase, TileOperation};
use crate::tile::tile_observer::TileObserver;
use crate::tile_parameters::TileParameters;
use crate::tile_render_data::{TileAtlasTextures, TileRenderData};
use mln_actor::actor::{Actor, PoolActorRef};
use mln_actor::{ActorRef, Mailbox, scheduler};
use mln_gfx::UploadPass;
use mln_map_types::{MapDebugOptions, MapMode};
use mln_style::expression::GlobalStateMap;
use mln_style::layer;
use mln_style::layer::layer_type_info;
use mln_style::layer_properties::LayerProperties;
use mln_style_types::VisibilityType;
use mln_style_types::image::ImagePosition;
use mln_tile_id::OverscaledTileID;
use mln_util::exception::ExceptionPtr;
use mln_util::immutable::Immutable;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// `GeometryTile::LayoutResult`.
// TODO(port): glyphAtlas, imageAtlas, dynamicTextureAtlas (M5).
#[derive(Clone)]
pub struct LayoutResult {
    pub layer_render_data: HashMap<String, LayerRenderData>,
    pub feature_index: Arc<FeatureIndex>,
}

impl LayoutResult {
    pub fn new(layer_render_data: HashMap<String, LayerRenderData>, feature_index: Arc<FeatureIndex>) -> Self {
        LayoutResult {
            layer_render_data,
            feature_index,
        }
    }

    /// `getLayerRenderData(const style::Layer::Impl&)`.
    pub fn get_layer_render_data(&self, layer_impl: &dyn layer::Impl) -> Option<&LayerRenderData> {
        let result = self.layer_render_data.get(&layer_impl.id)?;
        if !std::ptr::eq(
            result.layer_properties.base_impl.get_type_info(),
            layer_impl.get_type_info(),
        ) {
            // Layer data might be outdated, see issue #12432.
            return None;
        }
        Some(result)
    }
}

/// `GeometryTileRenderData`.
struct GeometryTileRenderData {
    atlas_textures: Option<Rc<TileAtlasTextures>>,
    layout_result: Option<Rc<LayoutResult>>,
    // TODO(port): std::vector<ImagePatch> imagePatches (sprite updates).
}

impl TileRenderData for GeometryTileRenderData {
    fn get_atlas_textures(&self) -> Option<&Rc<TileAtlasTextures>> {
        self.atlas_textures.as_ref()
    }

    fn get_pattern(&self, _pattern: &str) -> Option<ImagePosition> {
        // TODO(port): layoutResult->imageAtlas.patternPositions (patterns).
        None
    }

    fn get_layer_render_data(&self, layer_impl: &dyn layer::Impl) -> Option<&LayerRenderData> {
        self.layout_result
            .as_ref()
            .and_then(|result| result.get_layer_render_data(layer_impl))
    }

    fn get_bucket(&self, layer_impl: &dyn layer::Impl) -> Option<&BucketPtr> {
        self.get_layer_render_data(layer_impl).map(|data| &data.bucket)
    }

    fn upload(&self, upload_pass: &dyn UploadPass) {
        let Some(layout_result) = &self.layout_result else {
            return;
        };

        for entry in layout_result.layer_render_data.values() {
            if entry.bucket.needs_upload() {
                entry.bucket.upload(upload_pass);
            }
        }

        debug_assert!(self.atlas_textures.is_some());

        // TODO(port): glyph/icon dynamic textures and image patches (M5).
    }

    fn prepare(&mut self, _parameters: &SourcePrepareParameters<'_>) {
        if self.layout_result.is_none() {
            // return
        }
        // TODO(port): populateImagePatches (sprite image updates).
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FadeState {
    Loaded,
    NeedsFirstPlacement,
    NeedsSecondPlacement,
    CanRemove,
}

/// The render-thread state of a `GeometryTile`, also the target of its
/// worker's replies.
pub struct GeometryTileInner {
    pub base: TileBase,

    /// Used to signal the worker that it should abandon parsing this tile as
    /// soon as possible.
    obsolete: Arc<AtomicBool>,

    worker: PoolActorRef<GeometryTileWorker>,
    mailbox: Arc<Mailbox>,

    correlation_id: Cell<u64>,

    layout_result: RefCell<Option<Rc<LayoutResult>>>,
    atlas_textures: RefCell<Option<Rc<TileAtlasTextures>>>,

    mode: MapMode,

    show_collision_boxes: Cell<bool>,

    fade_state: Cell<FadeState>,
}

impl GeometryTileInner {
    fn available_images(&self) -> Immutable<BTreeSet<String>> {
        // TODO(port): imageManager->getAvailableImages() (images).
        Arc::new(BTreeSet::new())
    }

    fn mark_obsolete(&self) {
        self.obsolete.store(true, Ordering::SeqCst);
        // `mailbox->abandon()`: no reply reaches the tile any more.
        self.mailbox.close();
    }

    /// `setError(std::exception_ptr)`.
    pub fn set_error(&self, err: ExceptionPtr) {
        self.base.loaded.set(true);
        self.base.observer().on_tile_error(&self.base, err);
    }

    /// `setData(std::unique_ptr<const GeometryTileData>)`.
    pub fn set_data(&self, data: Option<SharedTileData>) {
        if self.obsolete.load(Ordering::SeqCst) {
            return;
        }

        if !self.base.pending.get() {
            self.base.on_tile_action(TileOperation::StartParse);
        }

        // Mark the tile as pending again if it was complete before to
        // prevent signaling a complete state despite pending parse
        // operations.
        self.base.pending.set(true);

        self.correlation_id.set(self.correlation_id.get() + 1);
        let correlation_id = self.correlation_id.get();
        let available_images = self.available_images();
        self.worker.invoke(move |worker: &mut GeometryTileWorker| {
            worker.set_data(data, available_images, correlation_id)
        });
    }

    /// `reset()`: resets the tile's data and layers and leaves the tile in
    /// pending state, waiting for the new data and layers to come.
    pub fn reset(&self) {
        // If there is pending work, indicate that work has been cancelled.
        // Clear the pending status.
        if self.base.pending.get() {
            self.base.on_tile_action(TileOperation::Cancelled);
            self.base.pending.set(false);
        }

        // Reset the tile to an unloaded state to avoid signaling completion
        // after clearing the tile's pending status.
        self.base.loaded.set(false);

        // Reset the worker to the `NeedsParse` state.
        self.correlation_id.set(self.correlation_id.get() + 1);
        let correlation_id = self.correlation_id.get();
        self.worker
            .invoke(move |worker: &mut GeometryTileWorker| worker.reset(correlation_id));
    }

    /// `onLayout(std::shared_ptr<LayoutResult>, correlationID)`.
    pub fn on_layout(&self, result: LayoutResult, result_correlation_id: u64) {
        self.base.loaded.set(true);
        self.base.renderable.set(true);
        if result_correlation_id == self.correlation_id.get() {
            self.base.pending.set(false);
            self.base.on_tile_action(TileOperation::EndParse);
        }

        // TODO(port): ErrorScope, bucket->check (symbol guards).

        *self.layout_result.borrow_mut() = Some(Rc::new(result));
        if self.atlas_textures.borrow().is_none() {
            *self.atlas_textures.borrow_mut() = Some(Rc::new(TileAtlasTextures::default()));
        }

        self.base.observer().on_tile_changed(&self.base);
    }

    /// `onError(std::exception_ptr, correlationID)`.
    pub fn on_error(&self, err: ExceptionPtr, result_correlation_id: u64) {
        self.base.loaded.set(true);
        if result_correlation_id == self.correlation_id.get() {
            self.base.pending.set(false);
            self.base.on_tile_action(TileOperation::Error);
        }
        self.base.observer().on_tile_error(&self.base, err);
    }

    /// `getFeatureIndex()`.
    pub fn get_feature_index(&self) -> Option<Arc<FeatureIndex>> {
        self.layout_result
            .borrow()
            .as_ref()
            .map(|result| result.feature_index.clone())
    }
}

/// `GeometryTile`.
pub struct GeometryTile {
    inner: Rc<GeometryTileInner>,
    /// `OptionalActor<GeometryTileWorker> worker`; dropped after the
    /// mailbox is closed.
    _worker: Actor<GeometryTileWorker>,
}

impl GeometryTile {
    /// `GeometryTile(const OverscaledTileID&, std::string sourceID, const
    /// TileParameters&, TileObserver*)`.
    pub fn new(
        id: OverscaledTileID,
        source_id: String,
        parameters: &TileParameters<'_>,
        observer: Option<Rc<dyn TileObserver>>,
    ) -> Self {
        let obsolete = Arc::new(AtomicBool::new(false));
        let mailbox = Mailbox::new();
        let show_collision_boxes = parameters.debug_options.contains(MapDebugOptions::COLLISION);
        let worker = {
            let parent: ActorRef<GeometryTileInner> = ActorRef::new(&mailbox);
            let source_id = source_id.clone();
            let obsolete = obsolete.clone();
            let mode = parameters.mode;
            let pixel_ratio = parameters.pixel_ratio;
            let capture_rendered_features = parameters.capture_rendered_features;
            Actor::new_tagged(&parameters.thread_pool, move |self_ref| {
                GeometryTileWorker::new(
                    self_ref,
                    parent,
                    id,
                    source_id,
                    obsolete,
                    mode,
                    pixel_ratio,
                    show_collision_boxes,
                    capture_rendered_features,
                )
            })
        };
        let inner = Rc::new(GeometryTileInner {
            base: TileBase::new(Kind::Geometry, id, source_id, observer),
            obsolete,
            worker: worker.self_ref(),
            mailbox: mailbox.clone(),
            correlation_id: Cell::new(0),
            layout_result: RefCell::new(None),
            atlas_textures: RefCell::new(None),
            mode: parameters.mode,
            show_collision_boxes: Cell::new(show_collision_boxes),
            fade_state: Cell::new(FadeState::Loaded),
        });
        // `mailbox(std::make_shared<Mailbox>(*Scheduler::GetCurrent()))`
        mailbox.open(&scheduler::get_current(), inner.clone() as Rc<dyn Any>);
        GeometryTile { inner, _worker: worker }
    }

    pub fn inner(&self) -> &Rc<GeometryTileInner> {
        &self.inner
    }

    pub fn set_error(&self, err: ExceptionPtr) {
        self.inner.set_error(err);
    }

    pub fn set_data(&self, data: Option<SharedTileData>) {
        self.inner.set_data(data);
    }

    pub fn reset(&self) {
        self.inner.reset();
    }

    pub fn get_feature_index(&self) -> Option<Arc<FeatureIndex>> {
        self.inner.get_feature_index()
    }
}

impl Drop for GeometryTile {
    fn drop(&mut self) {
        self.inner.mark_obsolete();

        // TODO(port): glyphManager->removeRequestor, imageManager->removeRequestor.

        if self.inner.base.pending.get() {
            // This tile never finished loading or was abandoned, emit a
            // cancellation event
            self.inner.base.on_tile_action(TileOperation::Cancelled);
        }
    }
}

impl Tile for GeometryTile {
    fn base(&self) -> &TileBase {
        &self.inner.base
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn create_render_data(&self) -> Box<dyn TileRenderData> {
        Box::new(GeometryTileRenderData {
            atlas_textures: self.inner.atlas_textures.borrow().clone(),
            layout_result: self.inner.layout_result.borrow().clone(),
        })
    }

    fn cancel(&self) {
        self.inner.mark_obsolete();
    }

    fn set_layers(&self, layers: &[Immutable<dyn LayerProperties>], global_state: Option<Arc<GlobalStateMap>>) {
        let inner = &self.inner;
        // Mark the tile as pending again if it was complete before to
        // prevent signaling a complete state despite pending parse
        // operations.
        if !inner.base.pending.get() {
            inner.base.pending.set(true);
            inner.base.on_tile_action(TileOperation::StartParse);
        }

        let id = inner.base.id();
        let mut impls: Vec<Immutable<dyn LayerProperties>> = Vec::with_capacity(layers.len());

        for layer in layers {
            // Skip irrelevant layers.
            let layer_impl = &layer.base_impl;
            debug_assert!(layer_impl.get_type_info().source != layer_type_info::Source::NotRequired);
            debug_assert!(layer_impl.source == inner.base.source_id);
            debug_assert!(layer_impl.visibility != VisibilityType::None);
            let overscaled_z = f32::from(id.overscaled_z);
            if overscaled_z < layer_impl.min_zoom.floor() || overscaled_z >= layer_impl.max_zoom.ceil() {
                continue;
            }

            impls.push(layer.clone());
        }

        inner.correlation_id.set(inner.correlation_id.get() + 1);
        let correlation_id = inner.correlation_id.get();
        let available_images = inner.available_images();
        inner.worker.invoke(move |worker: &mut GeometryTileWorker| {
            worker.set_layers(impls, global_state, available_images, correlation_id)
        });
    }

    fn set_show_collision_boxes(&self, show_collision_boxes: bool) {
        let inner = &self.inner;
        if inner.show_collision_boxes.get() != show_collision_boxes {
            inner.show_collision_boxes.set(show_collision_boxes);
            inner.correlation_id.set(inner.correlation_id.get() + 1);
            let correlation_id = inner.correlation_id.get();
            inner.worker.invoke(move |worker: &mut GeometryTileWorker| {
                worker.set_show_collision_boxes(show_collision_boxes, correlation_id)
            });
        }
    }

    fn set_observer(&self, observer: Rc<dyn TileObserver>) {
        self.inner.base.set_observer(observer);
        // TODO(port): worker.self().invoke(&GeometryTileWorker::setObserver,
        // observer) (symbol errors reported from the worker thread).
    }

    fn layer_properties_updated(&self, layer_properties: &Immutable<dyn LayerProperties>) -> bool {
        let mut layout_result = self.inner.layout_result.borrow_mut();
        let Some(result) = layout_result.as_mut() else {
            return false;
        };
        let layer_impl = &layer_properties.base_impl;
        let Some(render_data) = result.get_layer_render_data(&**layer_impl) else {
            return false;
        };

        if !Arc::ptr_eq(&render_data.layer_properties, layer_properties)
            && render_data.layer_properties.constants_mask() == layer_properties.constants_mask()
        {
            // C++ updates the shared LayoutResult in place; render data
            // created from it before (the previous frame's RenderTiles) see
            // the change too, but are replaced in the next prepare. The port
            // updates a copy if the result is shared.
            let result = Rc::make_mut(result);
            if let Some(render_data) = result.layer_render_data.get_mut(&layer_impl.id) {
                render_data.layer_properties = layer_properties.clone();
            }
        }

        true
    }

    fn hold_for_fade(&self) -> bool {
        self.inner.mode == MapMode::Continuous
            && matches!(
                self.inner.fade_state.get(),
                FadeState::NeedsFirstPlacement | FadeState::NeedsSecondPlacement
            )
    }

    fn mark_rendered_ideal(&self) {
        self.inner.fade_state.set(FadeState::Loaded);
    }

    fn mark_rendered_previously(&self) {
        if self.inner.fade_state.get() == FadeState::Loaded {
            self.inner.fade_state.set(FadeState::NeedsFirstPlacement);
        }
    }

    fn performed_fade_placement(&self) {
        match self.inner.fade_state.get() {
            FadeState::NeedsFirstPlacement => self.inner.fade_state.set(FadeState::NeedsSecondPlacement),
            FadeState::NeedsSecondPlacement => self.inner.fade_state.set(FadeState::CanRemove),
            _ => {}
        }
    }
}
