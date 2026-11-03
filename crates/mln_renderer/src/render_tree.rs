// Ports src/mln/renderer/render_tree.hpp and the RenderTreeImpl / LayerRenderItem
// parts of src/mln/renderer/render_orchestrator.cpp.
//
// C++ `RenderTree` is an abstract class with one implementation
// (`RenderTreeImpl` in render_orchestrator.cpp); the port has one struct.
// `std::set<LayerRenderItem>` (ordered and deduplicated by `index`) is a
// `BTreeMap<u32, LayerRenderItem>`.

use crate::paint_parameters::TransformParameters;
use crate::render_layer::RenderLayerPtr;
use crate::render_pass::RenderPass;
use mln_map_types::{MapDebugOptions, MapMode, TransformState};
use mln_util::chrono::TimePoint;
use mln_util::Color;
use std::collections::BTreeMap;

/// `LayerRenderItem`.
#[derive(Clone)]
pub struct LayerRenderItem {
    pub layer: RenderLayerPtr,
    // TODO(port): RenderSource* source (sources: M3).
    pub index: u32,
}

impl LayerRenderItem {
    pub fn new(layer: RenderLayerPtr, index: u32) -> Self {
        LayerRenderItem { layer, index }
    }

    pub fn has_render_pass(&self, pass: RenderPass) -> bool {
        self.layer.borrow().has_render_pass(pass)
    }

    pub fn get_name(&self) -> String {
        self.layer.borrow().get_id().to_owned()
    }
}

/// `RenderTreeParameters`.
pub struct RenderTreeParameters {
    pub transform_params: TransformParameters,
    pub map_mode: MapMode,
    pub debug_options: MapDebugOptions,
    pub time_point: TimePoint,
    // TODO(port): EvaluatedLight light (RenderLight; only 3D layers use it).
    pub has_3d: bool,
    pub opaque_pass_cut_off: u32,
    /// C++ default-constructs `Color` (transparent black).
    pub background_color: Color,
    pub symbol_fade_change: f32,
    pub needs_repaint: bool,
    pub loaded: bool,
    pub placement_changed: bool,
}

impl RenderTreeParameters {
    pub fn new(state: &TransformState, map_mode: MapMode, debug_options: MapDebugOptions, time_point: TimePoint) -> Self {
        RenderTreeParameters {
            transform_params: TransformParameters::new(state),
            map_mode,
            debug_options,
            time_point,
            has_3d: false,
            opaque_pass_cut_off: 0,
            background_color: Color::new_unchecked(0.0, 0.0, 0.0, 0.0),
            symbol_fade_change: 0.0,
            needs_repaint: false,
            loaded: false,
            placement_changed: false,
        }
    }
}

/// `RenderTree` / `RenderTreeImpl`.
pub struct RenderTree {
    parameters: Box<RenderTreeParameters>,
    pub(crate) layer_render_items: BTreeMap<u32, LayerRenderItem>,
    // TODO(port): sourceRenderItems, layersNeedPlacement, placement,
    // updateSymbolOpacities (sources, symbols).
    start_time: std::time::Instant,
}

impl RenderTree {
    pub(crate) fn new(
        parameters: Box<RenderTreeParameters>,
        layer_render_items: BTreeMap<u32, LayerRenderItem>,
        start_time: std::time::Instant,
    ) -> Self {
        RenderTree {
            parameters,
            layer_render_items,
            start_time,
        }
    }

    /// `prepare()`: updates the symbol buckets of the layers that need
    /// placement.
    pub fn prepare(&mut self) {
        // TODO(port): placement->updateLayerBuckets for layersNeedPlacement.
    }

    pub fn get_layer_render_item_map(&self) -> &BTreeMap<u32, LayerRenderItem> {
        &self.layer_render_items
    }

    pub fn get_layer_render_items(&self) -> Vec<LayerRenderItem> {
        self.layer_render_items.values().cloned().collect()
    }

    pub fn get_parameters(&self) -> &RenderTreeParameters {
        &self.parameters
    }

    /// `getElapsedTime()`, in seconds.
    pub fn get_elapsed_time(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
}
