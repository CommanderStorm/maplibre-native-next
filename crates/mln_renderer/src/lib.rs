//! The map renderer: sources and tiles, layouts and buckets, render layers,
//! and the orchestrator that turns a style into each frame's render tree.

pub(crate) mod algorithm;
pub mod bucket;
pub(crate) mod bucket_geometry;
pub(crate) mod bucket_parameters;
pub(crate) mod buckets;
pub(crate) mod change_request;
pub(crate) mod frame_projection;
pub(crate) mod geometry;
pub(crate) mod gfx;
pub(crate) mod globe_depth_pass;
pub(crate) mod globe_tile_mesh;
pub(crate) mod group_by_layout;
pub(crate) mod image_manager;
pub(crate) mod layer_group;
pub(crate) mod layer_tweaker;
pub mod layermanager;
pub(crate) mod layers;
pub(crate) mod layout;
pub(crate) mod line_atlas;
pub(crate) mod paint_parameters;
pub(crate) mod paint_property_binder;
pub(crate) mod paint_property_statistics;
pub(crate) mod pattern_atlas;
pub mod query;
pub(crate) mod render_layer;
pub(crate) mod render_light;
pub(crate) mod render_orchestrator;
pub(crate) mod render_pass;
pub(crate) mod render_sky;
pub(crate) mod render_source;
pub(crate) mod render_static_data;
pub(crate) mod render_target;
pub(crate) mod render_tile;
pub(crate) mod render_tree;
pub(crate) mod renderer;
pub(crate) mod renderer_event;
pub(crate) mod sky_pass;
pub mod source_state;
pub(crate) mod sources;
pub(crate) mod style_diff;
pub mod text;
pub mod tile;
pub(crate) mod tile_mask;
pub(crate) mod tile_parameters;
pub(crate) mod tile_pyramid;
pub(crate) mod tile_render_data;
pub(crate) mod update_parameters;
pub mod update_slot;
pub mod util;

pub use frame_projection::{ClipDepthRange, FrameProjection};
pub use renderer::Renderer;
pub use renderer_event::{
    FeatureInfo, FontStack, FrameRenderedFeaturesMap, GlyphRange, LayerFeaturesMap, NDCBound, OverscaledTileID,
    RenderMode, RendererEvent, RendererEvents, RenderingStats, SourceLayerID, TileOperation,
};
pub use update_parameters::UpdateParameters;
pub use update_slot::{Update, UpdateReader, UpdateSlot};

pub(crate) use paint_property_binder::paint_property_binders;

#[cfg(test)]
mod tests;
