// Ports src/mln/renderer/change_request.hpp and src/mln/renderer/change_request.cpp.
//
// The C++ class hierarchy (`ChangeRequest::execute(RenderOrchestrator&)`) is
// an enum here; `RenderOrchestrator::process_changes` executes it.

use crate::layer_tweaker::LayerTweaker;
use mln_gfx::LayerGroupBasePtr;
use std::cell::RefCell;
use std::rc::Weak;

pub enum ChangeRequest {
    /// `AddLayerGroupRequest`.
    AddLayerGroup(LayerGroupBasePtr),
    /// `RemoveLayerGroupRequest`.
    RemoveLayerGroup(LayerGroupBasePtr),
    /// `UpdateLayerGroupIndexRequest`.
    UpdateLayerGroupIndex(LayerGroupBasePtr, i32),
    /// Not a C++ request: `layerGroup->addLayerTweaker(tweaker)`. C++ layer
    /// groups keep a list of weak tweaker pointers; LayerTweaker is a
    /// renderer type and the layer groups live in mln_gfx, so the
    /// orchestrator keeps the lists. The request is executed by
    /// `processChanges`, before the tweakers first run in the same frame.
    AddLayerTweaker(LayerGroupBasePtr, Weak<RefCell<dyn LayerTweaker>>),
    // TODO(port): AddRenderTargetRequest / RemoveRenderTargetRequest (render
    // targets: hillshade, heatmap, custom drawable layers).
}

pub type UniqueChangeRequest = ChangeRequest;
pub type UniqueChangeRequestVec = Vec<ChangeRequest>;
