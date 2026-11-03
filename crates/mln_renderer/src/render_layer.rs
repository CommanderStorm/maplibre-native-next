// Ports src/mln/renderer/render_layer.hpp and src/mln/renderer/render_layer.cpp
// (the parts layers without a source use; render tiles, buckets, placement
// data and feature queries arrive with sources, M3).
//
// C++ `RenderLayer` is an abstract class with data members; the data is
// `RenderLayerBase` and the virtual interface is the `RenderLayer` trait.
// Render layers are shared between the orchestrator, its ordered list and
// the render tree's items (C++: `unique_ptr` + `reference_wrapper`s), so
// they are `Rc<RefCell<dyn RenderLayer>>`.

use crate::change_request::{ChangeRequest, UniqueChangeRequestVec};
use crate::layer_tweaker::LayerTweakerPtr;
use crate::paint_parameters::PaintParameters;
use crate::pattern_atlas::PatternAtlas;
use crate::render_pass::RenderPass;
use crate::render_tree::RenderTree;
use crate::update_parameters::UpdateParameters;
use mln_gfx::{Context, LayerGroupBasePtr, ShaderRegistry, UploadPass};
use mln_map_types::TransformState;
use mln_style::expression::Dependency;
use mln_style::layer;
use mln_style::layer::layer_type_info;
use mln_style::layer_properties::LayerProperties;
use mln_style::{PropertyEvaluationParameters, TransitionParameters};
use mln_style_types::VisibilityType;
use mln_util::immutable::Immutable;
use mln_util::Color;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

pub type RenderLayerPtr = Rc<RefCell<dyn RenderLayer>>;
pub type RenderLayerReferences = Vec<RenderLayerPtr>;

/// `LayerPrepareParameters`.
pub struct LayerPrepareParameters<'a> {
    // TODO(port): RenderSource* source, ImageManager&, LineAtlas&.
    pub pattern_atlas: &'a RefCell<PatternAtlas>,
    pub state: &'a TransformState,
}

/// `RenderLayer::Stats`.
#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub drawables_added: usize,
    pub drawables_removed: usize,
    // TODO(port): renderedFeatures (rendered-feature capture).
}

/// The data members of `RenderLayer`.
pub struct RenderLayerBase {
    pub evaluated_properties: Immutable<dyn LayerProperties>,
    pub base_impl: Immutable<dyn layer::Impl>,

    pub passes: RenderPass,
    pub layer_group: Option<LayerGroupBasePtr>,
    pub layer_tweaker: Option<LayerTweakerPtr>,
    pub layer_index: i32,
    pub style_dependencies: Dependency,
    pub is_renderable: bool,

    pub stats: Stats,
    // TODO(port): renderTiles, renderTilesOwner, placementData,
    // renderTileIDs, sourceForState, hasRenderFailures.
}

impl RenderLayerBase {
    /// `RenderLayer(Immutable<style::LayerProperties>)`.
    pub fn new(properties: Immutable<dyn LayerProperties>) -> Self {
        let base_impl = properties.base_impl.clone();
        RenderLayerBase {
            evaluated_properties: properties,
            base_impl,
            passes: RenderPass::None,
            layer_group: None,
            layer_tweaker: None,
            layer_index: 0,
            style_dependencies: Dependency::NONE,
            is_renderable: false,
            stats: Stats::default(),
        }
    }
}

pub trait RenderLayer {
    fn base(&self) -> &RenderLayerBase;
    fn base_mut(&mut self) -> &mut RenderLayerBase;

    /// Begin transitions for any properties that have changed since the last
    /// frame.
    fn transition(&mut self, parameters: &TransitionParameters);

    /// Fully evaluate possibly-transitioning paint properties based on a
    /// zoom and some evaluated layout properties.
    fn evaluate(&mut self, parameters: &PropertyEvaluationParameters);

    /// Returns true if any paint properties have active transitions.
    fn has_transition(&self) -> bool;

    /// Returns true if the layer has a pattern property and is actively
    /// crossfading.
    fn has_crossfade(&self) -> bool;

    /// Returns true if layer writes to depth buffer by drawing using
    /// PaintParameters::depthModeFor3D().
    fn is_3d(&self) -> bool {
        false
    }

    fn upload(&mut self, _upload_pass: &dyn UploadPass) {}

    fn render(&mut self, _parameters: &mut PaintParameters<'_>) {}

    /// `prepare(const LayerPrepareParameters&)`. The C++ default reads the
    /// source's render tiles.
    // TODO(port): the default body (sources, M3). Every layer that runs in
    // M1 (background) overrides it.
    fn prepare(&mut self, _parameters: &mut LayerPrepareParameters<'_>) {}

    fn mark_context_destroyed(&mut self) {
        // no-op
    }

    /// TODO: Only for background layers.
    fn get_solid_background(&self) -> Option<Color> {
        None
    }

    /// Update a layer with its render data (drawables).
    #[allow(clippy::too_many_arguments)]
    fn update(
        &mut self,
        _shaders: &ShaderRegistry,
        _context: &dyn Context,
        _state: &TransformState,
        _update_parameters: &Arc<UpdateParameters>,
        _paint_parameters: &PaintParameters<'_>,
        _render_tree: &RenderTree,
        _changes: &mut UniqueChangeRequestVec,
    ) {
    }

    /// Called when the style layer is replaced (same ID and type), and the
    /// render layer is reused.
    fn layer_changed(
        &mut self,
        _parameters: &TransitionParameters,
        _new_layer: &Immutable<dyn layer::Impl>,
        _changes: &mut UniqueChangeRequestVec,
    ) {
        // When a layer changes, the bucket won't be replaced until the new
        // source(s) load. If we remove the drawables here, they will just be
        // re-created based on the current data.

        // Detach the layer tweaker, if any. This keeps the existing tiles
        // up-to-date with the most recent evaluated properties, while a new
        // one will be created along with new drawables when tiles are loaded
        // and new buckets are available.
        self.base_mut().layer_tweaker = None;
    }

    /// Called when the style layer is removed.
    fn layer_removed(&mut self, changes: &mut UniqueChangeRequestVec) {
        self.remove_all_drawables();
        let group = self.base().layer_group.clone();
        activate_layer_group(&group, false, changes);
    }

    /// Called when the layer index changes.
    fn layer_index_changed(&mut self, new_layer_index: i32, changes: &mut UniqueChangeRequestVec) {
        self.base_mut().layer_index = new_layer_index;

        // Submit a change request to update the layer index of our tile layer
        // group
        let group = self.base().layer_group.clone();
        change_layer_index(&group, new_layer_index, changes);
    }

    /// Called when the layer is included or excluded from rendering.
    fn mark_layer_renderable(&mut self, will_render: bool, changes: &mut UniqueChangeRequestVec) {
        self.base_mut().is_renderable = will_render;

        // This layer is either being freshly included in the renderable set
        // or excluded
        let group = self.base().layer_group.clone();
        activate_layer_group(&group, will_render, changes);
    }

    /// Remove all drawables for the tile from the layer group.
    fn remove_all_drawables(&mut self) -> usize {
        let base = self.base_mut();
        if let Some(group) = &base.layer_group {
            let count = group.borrow().get_drawable_count();
            base.stats.drawables_removed += count;
            group.borrow_mut().clear_drawables();
            return count;
        }
        0
    }

    // Non-virtual members of the C++ class.

    fn get_id(&self) -> &str {
        &self.base().base_impl.id
    }

    /// `transition(const TransitionParameters&, Immutable<style::Layer::Impl> newImpl)`.
    fn transition_with_impl(&mut self, parameters: &TransitionParameters, new_impl: Immutable<dyn layer::Impl>) {
        self.base_mut().base_impl = new_impl;
        self.transition(parameters);
    }

    /// Returns true if the layer has to be placed.
    fn needs_placement(&self) -> bool {
        // TODO(port): placementData is filled by symbol layers (M5); it is
        // always empty until then.
        let placement_data_empty = true;
        self.base().base_impl.get_type_info().cross_tile_index == layer_type_info::CrossTileIndex::Required
            && !placement_data_empty
    }

    fn get_layer_index(&self) -> i32 {
        self.base().layer_index
    }

    /// Checks whether the given render pass is used by this layer.
    fn has_render_pass(&self, pass: RenderPass) -> bool {
        (self.base().passes & pass) != RenderPass::None
    }

    /// Checks whether this layer needs to be rendered in the given render
    /// pass.
    fn needs_rendering(&self) -> bool {
        self.base().passes != RenderPass::None && self.base().base_impl.visibility != VisibilityType::None
    }

    /// Checks whether this layer can be rendered.
    fn supports_zoom(&self, zoom: f32) -> bool {
        // TODO: shall we use rounding or epsilon comparisons?
        self.base().base_impl.min_zoom <= zoom && self.base().base_impl.max_zoom >= zoom
    }

    fn is_layer_renderable(&self) -> bool {
        self.base().is_renderable
    }

    fn get_style_dependencies(&self) -> Dependency {
        self.base().style_dependencies
    }

    /// `setLayerGroup(LayerGroupBasePtr, UniqueChangeRequestVec&)`.
    fn set_layer_group(&mut self, layer_group: Option<LayerGroupBasePtr>, changes: &mut UniqueChangeRequestVec) {
        // Remove the active layer group, if any, before replacing it.
        let old = self.base().layer_group.clone();
        activate_layer_group(&old, false, changes);

        self.base_mut().layer_group = layer_group;

        // Add the new layer group, if we're currently renderable.
        let group = self.base().layer_group.clone();
        let renderable = self.base().is_renderable;
        activate_layer_group(&group, renderable, changes);
    }
}

/// (Un-)Register the layer group with the orchestrator.
pub fn activate_layer_group(layer_group: &Option<LayerGroupBasePtr>, activate: bool, changes: &mut UniqueChangeRequestVec) {
    if let Some(layer_group) = layer_group {
        if activate {
            // The RenderTree has determined this layer should be included in
            // the renderable set for a frame
            changes.push(ChangeRequest::AddLayerGroup(layer_group.clone()));
        } else {
            // The RenderTree is informing us we should not render anything
            changes.push(ChangeRequest::RemoveLayerGroup(layer_group.clone()));
        }
    }
}

/// `changeLayerIndex(const LayerGroupBasePtr&, int32_t, UniqueChangeRequestVec&)`.
pub fn change_layer_index(group: &Option<LayerGroupBasePtr>, new_index: i32, changes: &mut UniqueChangeRequestVec) {
    if let Some(group) = group {
        if group.borrow().get_layer_index() != new_index {
            changes.push(ChangeRequest::UpdateLayerGroupIndex(group.clone(), new_index));
        }
    }
}
