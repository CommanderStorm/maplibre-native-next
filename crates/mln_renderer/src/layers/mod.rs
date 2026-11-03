//! Ports src/mln/renderer/layers (the layer types M1 and M3 render).

pub mod background_layer_tweaker;
pub mod circle_layer_tweaker;
pub mod fill_layer_tweaker;
pub mod line_layer_tweaker;
pub mod render_background_layer;
pub mod render_circle_layer;
pub mod render_fill_layer;
pub mod render_line_layer;

pub use background_layer_tweaker::BackgroundLayerTweaker;
pub use circle_layer_tweaker::CircleLayerTweaker;
pub use fill_layer_tweaker::FillLayerTweaker;
pub use line_layer_tweaker::LineLayerTweaker;
pub use render_background_layer::RenderBackgroundLayer;
pub use render_circle_layer::RenderCircleLayer;
pub use render_fill_layer::RenderFillLayer;
pub use render_line_layer::RenderLineLayer;

use crate::bucket::{Bucket, BucketPtr};
use crate::layer_tweaker::LayerTweakerPtr;
use crate::render_tile::RenderTile;
use mln_gfx::vertex_vector::{VertexVector, VertexVectorBasePtr};
use mln_gfx::Drawable;
use mln_style::layer_properties::{LayerProperties, get_crossfade, get_evaluated};
use mln_style::layers::{BackgroundLayerProperties, BackgroundPaintProperties};
use mln_style::CrossfadeParameters;
use mln_util::immutable::Immutable;
use std::any::Any;
use std::rc::Rc;

/// `getEvaluated<BackgroundLayerProperties>(properties)`.
pub(crate) fn background_layer_properties(
    properties: &Immutable<dyn LayerProperties>,
) -> &BackgroundPaintProperties::PossiblyEvaluated {
    get_evaluated::<BackgroundLayerProperties>(properties)
}

/// `getCrossfade<BackgroundLayerProperties>(properties)`.
pub(crate) fn background_crossfade(properties: &Immutable<dyn LayerProperties>) -> &CrossfadeParameters {
    get_crossfade::<BackgroundLayerProperties>(properties)
}

/// What `drawable->setBinders(bucket, &binders)` stores: the bucket (kept
/// alive by the drawable) and the id of the layer whose binders
/// (`bucket.paintPropertyBinders.at(id)`) the drawable uses.
pub struct DrawableBinders {
    pub bucket: BucketPtr,
    pub layer_id: String,
}

/// The value for `Drawable::set_binders`.
pub fn drawable_binders_slot(bucket: &BucketPtr, layer_id: &str) -> Option<Rc<dyn Any>> {
    Some(Rc::new(DrawableBinders { bucket: bucket.clone(), layer_id: layer_id.to_owned() }) as Rc<dyn Any>)
}

/// `drawable.getBucket()` + `drawable.getBinders()`, downcast to the bucket
/// type the layer created: the bucket and the layer id of its binders.
pub fn get_drawable_bucket<'a, B: Bucket>(drawable: &'a dyn Drawable) -> Option<(&'a B, &'a str)> {
    let slot = drawable.get_binders()?.downcast_ref::<DrawableBinders>()?;
    let bucket = (*slot.bucket).downcast_ref::<B>()?;
    Some((bucket, slot.layer_id.as_str()))
}

/// What `drawable->setRenderTile(renderTilesOwner, &tile)` stores: the
/// owning vector and the tile's index in it.
pub struct DrawableRenderTile {
    pub owner: Rc<Vec<RenderTile>>,
    pub index: usize,
}

impl DrawableRenderTile {
    pub fn tile(&self) -> &RenderTile {
        &self.owner[self.index]
    }
}

/// The value for `Drawable::set_render_tile`.
pub fn drawable_render_tile_slot(owner: &Rc<Vec<RenderTile>>, tile: &RenderTile) -> Option<Rc<dyn Any>> {
    let index = owner.iter().position(|t| std::ptr::eq(t, tile))?;
    Some(Rc::new(DrawableRenderTile { owner: owner.clone(), index }) as Rc<dyn Any>)
}

/// `drawable.getRenderTile()`.
pub fn get_drawable_render_tile(drawable: &dyn Drawable) -> Option<&RenderTile> {
    let slot = drawable.get_render_tile()?.downcast_ref::<DrawableRenderTile>()?;
    Some(slot.tile())
}

/// `drawable.getLayerTweaker() == layerTweaker`.
pub fn drawable_has_layer_tweaker(drawable: &dyn Drawable, layer_tweaker: &Option<LayerTweakerPtr>) -> bool {
    let current = drawable.get_layer_tweaker().and_then(|slot| slot.downcast_ref::<LayerTweakerPtr>());
    match (current, layer_tweaker) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

/// A shared gfx vertex vector holding a copy of a bucket's vertices (C++
/// shares the bucket's own `std::shared_ptr<VertexVector>`).
pub fn shared_vertices<V: bytemuck::Pod>(vertices: &[V]) -> VertexVectorBasePtr {
    Rc::new(VertexVector::from_vec(vertices.to_vec()))
}

/// `checkTweakDrawable` for a tweaker whose `RefCell` is already borrowed
/// mutably by the running `execute`: compares the drawable's tweaker with
/// `self_ptr`.
pub fn check_tweak_drawable_ptr<T: ?Sized>(drawable: &dyn Drawable, self_ptr: *const T) -> bool {
    match drawable.get_layer_tweaker().and_then(|slot| slot.downcast_ref::<LayerTweakerPtr>()) {
        None => true,
        Some(tweaker) => std::ptr::addr_eq(tweaker.as_ptr(), self_ptr),
    }
}

/// `util::cast<float>(mat4)`.
pub fn cast_matrix(m: &[f64; 16]) -> [f32; 16] {
    std::array::from_fn(|i| m[i] as f32)
}
