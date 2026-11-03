// Ports src/mln/renderer/layers/render_symbol_layer.hpp and
// src/mln/renderer/layers/render_symbol_layer.cpp (drawable path,
// MLN_USE_SYMBOL_INSTANCING off), plus gfx/symbol_drawable_data.hpp.
//
// C++ drawables share the bucket's vertex vectors and see placement's
// changes through them (`updateModified`). Here a drawable gets copies, so
// when the bucket's vertex data changes (placement, sorting: the bucket's
// `version`) the tile's drawables are re-created.
//
// TODO(port): collision debug drawables (CollisionLayerTweaker, collision
// box/circle buffers), captureRenderedFeatures (queries), placementData
// (placement), SymbolLayerPaintPropertyOverrides::setOverrides (per-section
// text-color overrides of the evaluated paint properties).

use super::symbol_layer_tweaker::SymbolLayerTweaker;
use super::{TileDrawables, drawable_binders_slot, drawable_render_tile_slot, shared_vertices};
use crate::buckets::symbol_bucket::{Buffer, PaintProperties, SymbolBucket};
use crate::change_request::{ChangeRequest, UniqueChangeRequestVec};
use crate::gfx::drawable_atlases_tweaker::DrawableAtlasesTweaker;
use crate::layer_tweaker::{LayerTweakerPtr, layer_tweaker_slot};
use crate::paint_parameters::PaintParameters;
use crate::paint_property_binder::read_data_driven_paint_property;
use crate::render_layer::{LayerPrepareParameters, RenderLayer, RenderLayerBase};
use crate::render_pass::RenderPass;
use crate::render_tree::RenderTree;
use crate::update_parameters::UpdateParameters;
use mln_gfx::index_vector::IndexVectorBase;
use mln_gfx::shader_group::{ShaderGroupPtr, StringIDSetsPair};
use mln_gfx::shaders::shader_defines::*;
use mln_gfx::vertex_attribute::VertexAttributeArray;
use mln_gfx::{AttributeDataType, ColorMode, Context, CullFaceMode, DepthMaskType, DrawMode, DrawableTweakerPtr, ShaderRegistry};
use mln_map_types::TransformState;
use mln_style::layer;
use mln_style::layer::static_immutable_cast;
use mln_style::layer_properties::{LayerProperties, get_evaluated};
use mln_style::layers::symbol_layer_impl::{Impl as SymbolLayerImpl, SymbolLayerPaintPropertyOverrides};
use mln_style::layers::symbol_layer_properties::*;
use mln_style::properties::Property;
use mln_style::{PropertyEvaluationParameters, TransitionParameters};
use mln_style_types::{AlignmentType, IconTextFitType, SymbolPlacementType, TranslateAnchorType};
use mln_tile_id::{OverscaledTileID, UnwrappedTileID};
use mln_util::Color;
use mln_util::identity::SimpleIdentity;
use mln_util::immutable::Immutable;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::Ordering;

fn impl_cast(impl_: &Immutable<dyn layer::Impl>) -> &SymbolLayerImpl {
    debug_assert!(std::ptr::eq(impl_.get_type_info(), SymbolLayerImpl::static_type_info()));
    impl_.downcast_ref::<SymbolLayerImpl>().expect("RenderSymbolLayer: not a symbol layer impl")
}

const SYMBOL_ICON_SHADER_NAME: &str = "SymbolIconShader";
const SYMBOL_SDF_SHADER_NAME: &str = "SymbolSDFShader";
const SYMBOL_TEXT_AND_ICON_SHADER_NAME: &str = "SymbolTextAndIconShader";
const POS_OFFSET_ATTRIB_NAME: &str = "a_pos_offset";

/// `SymbolType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolType {
    Text,
    IconRGBA,
    IconSDF,
}

/// `style::SymbolPropertyValues`.
#[derive(Clone, Copy, Debug)]
pub struct SymbolPropertyValues {
    pub pitch_alignment: AlignmentType,
    pub rotation_alignment: AlignmentType,
    pub keep_upright: bool,
    pub translate: [f32; 2],
    pub translate_anchor: TranslateAnchorType,
    pub has_halo: bool,
    pub has_fill: bool,
}

/// `gfx::SymbolDrawableData`.
#[derive(Clone, Copy, Debug)]
pub struct SymbolDrawableData {
    pub is_halo: bool,
    pub bucket_variable_placement: bool,
    pub symbol_type: SymbolType,
    pub pitch_alignment: AlignmentType,
    pub rotation_alignment: AlignmentType,
    pub placement: SymbolPlacementType,
    pub text_fit: IconTextFitType,
    pub is_offset: bool,
}

/// `iconPropertyValues(evaluated, layout)`.
pub fn icon_property_values(
    evaluated: &SymbolPaintProperties::PossiblyEvaluated,
    layout: &SymbolLayoutProperties::PossiblyEvaluated,
) -> SymbolPropertyValues {
    SymbolPropertyValues {
        pitch_alignment: *layout.get::<IconPitchAlignment>(),
        rotation_alignment: *layout.get::<IconRotationAlignment>(),
        keep_upright: *layout.get::<IconKeepUpright>(),
        translate: *evaluated.get::<IconTranslate>(),
        translate_anchor: *evaluated.get::<IconTranslateAnchor>(),
        has_halo: evaluated.get::<IconHaloColor>().constant_or(&Color::black()).a > 0.0
            && evaluated.get::<IconHaloWidth>().constant_or(&1.0) != 0.0,
        has_fill: evaluated.get::<IconColor>().constant_or(&Color::black()).a > 0.0,
    }
}

/// `textPropertyValues(evaluated, layout)`.
pub fn text_property_values(
    evaluated: &SymbolPaintProperties::PossiblyEvaluated,
    layout: &SymbolLayoutProperties::PossiblyEvaluated,
) -> SymbolPropertyValues {
    SymbolPropertyValues {
        pitch_alignment: *layout.get::<TextPitchAlignment>(),
        rotation_alignment: *layout.get::<TextRotationAlignment>(),
        keep_upright: *layout.get::<TextKeepUpright>(),
        translate: *evaluated.get::<TextTranslate>(),
        translate_anchor: *evaluated.get::<TextTranslateAnchor>(),
        has_halo: evaluated.get::<TextHaloColor>().constant_or(&Color::black()).a > 0.0
            && evaluated.get::<TextHaloWidth>().constant_or(&1.0) != 0.0,
        has_fill: evaluated.get::<TextColor>().constant_or(&Color::black()).a > 0.0,
    }
}

/// `updateTileAttributes(buffer, isText, paintProps, evaluated, attribs,
/// propertiesAsUniforms)`.
fn update_tile_attributes(
    buffer: &Buffer,
    is_text: bool,
    paint_props: &PaintProperties,
    evaluated: &SymbolPaintProperties::PossiblyEvaluated,
    attribs: &mut VertexAttributeArray,
    properties_as_uniforms: &mut StringIDSetsPair,
) {
    let mut id = ID_SYMBOL_OPACITY_ATTRIBUTE;
    let pau = properties_as_uniforms;
    if is_text {
        let b = &paint_props.text_binders;
        let ib = &b.interleaved_vertex_buffer;
        let shared = shared_vertices(&ib.shared_vertex_vector);
        read_data_driven_paint_property(attribs, &b.text_opacity, ib, &shared, &["a_opacity"], evaluated.get::<TextOpacity>().is_constant(), pau, &mut id);
        read_data_driven_paint_property(attribs, &b.text_color, ib, &shared, &["a_fill_color"], evaluated.get::<TextColor>().is_constant(), pau, &mut id);
        read_data_driven_paint_property(attribs, &b.text_halo_color, ib, &shared, &["a_halo_color"], evaluated.get::<TextHaloColor>().is_constant(), pau, &mut id);
        read_data_driven_paint_property(attribs, &b.text_halo_width, ib, &shared, &["a_halo_width"], evaluated.get::<TextHaloWidth>().is_constant(), pau, &mut id);
        read_data_driven_paint_property(attribs, &b.text_halo_blur, ib, &shared, &["a_halo_blur"], evaluated.get::<TextHaloBlur>().is_constant(), pau, &mut id);
    } else {
        let b = &paint_props.icon_binders;
        let ib = &b.interleaved_vertex_buffer;
        let shared = shared_vertices(&ib.shared_vertex_vector);
        read_data_driven_paint_property(attribs, &b.icon_opacity, ib, &shared, &["a_opacity"], evaluated.get::<IconOpacity>().is_constant(), pau, &mut id);
        read_data_driven_paint_property(attribs, &b.icon_color, ib, &shared, &["a_fill_color"], evaluated.get::<IconColor>().is_constant(), pau, &mut id);
        read_data_driven_paint_property(attribs, &b.icon_halo_color, ib, &shared, &["a_halo_color"], evaluated.get::<IconHaloColor>().is_constant(), pau, &mut id);
        read_data_driven_paint_property(attribs, &b.icon_halo_width, ib, &shared, &["a_halo_width"], evaluated.get::<IconHaloWidth>().is_constant(), pau, &mut id);
        read_data_driven_paint_property(attribs, &b.icon_halo_blur, ib, &shared, &["a_halo_blur"], evaluated.get::<IconHaloBlur>().is_constant(), pau, &mut id);
    }

    let layout_size = std::mem::size_of::<crate::buckets::symbol_bucket::SymbolLayoutVertex>() as u32;
    let layout_data = shared_vertices(&buffer.attribute_data);
    if let Some(attr) = attribs.set(ID_SYMBOL_POS_OFFSET_ATTRIBUTE, -1, AttributeDataType::Invalid, 0) {
        attr.set_shared_raw_data(Some(layout_data.clone()), 0, 0, layout_size, AttributeDataType::Short4);
    }
    if let Some(attr) = attribs.set(ID_SYMBOL_DATA_ATTRIBUTE, -1, AttributeDataType::Invalid, 0) {
        attr.set_shared_raw_data(Some(layout_data.clone()), 8, 0, layout_size, AttributeDataType::UShort4);
    }
    if let Some(attr) = attribs.set(ID_SYMBOL_PIXEL_OFFSET_ATTRIBUTE, -1, AttributeDataType::Invalid, 0) {
        attr.set_shared_raw_data(Some(layout_data), 16, 0, layout_size, AttributeDataType::Short4);
    }

    if let Some(attr) = attribs.set(ID_SYMBOL_PROJECTED_POS_ATTRIBUTE, -1, AttributeDataType::Invalid, 0) {
        attr.set_shared_raw_data(
            Some(shared_vertices(&buffer.dynamic_attribute_data)),
            0,
            0,
            std::mem::size_of::<crate::buckets::symbol_bucket::SymbolDynamicLayoutVertex>() as u32,
            AttributeDataType::Float3,
        );
    }
    if let Some(attr) = attribs.set(ID_SYMBOL_FADE_OPACITY_ATTRIBUTE, -1, AttributeDataType::Invalid, 0) {
        attr.set_shared_raw_data(
            Some(shared_vertices(&buffer.opacity_attribute_data)),
            0,
            0,
            std::mem::size_of::<crate::buckets::symbol_bucket::SymbolOpacityVertex>() as u32,
            AttributeDataType::Float,
        );
    }
}

/// A `SegmentGroup` (a `RenderableSegment` plus its segments).
struct SegmentGroup {
    sort_key: f32,
    type_: SymbolType,
    tile_index: usize,
    tile_id: UnwrappedTileID,
    overscaled_z: u8,
    /// One segment (`sortFeaturesByKey`) or all segments of the buffer.
    segment: Option<usize>,
}

impl SegmentGroup {
    /// `operator<(RenderableSegment, RenderableSegment)`.
    fn less(lhs: &SegmentGroup, rhs: &SegmentGroup) -> bool {
        // Sort renderable segments by a sort key.
        if lhs.sort_key < rhs.sort_key {
            return true;
        }

        // In cases when sort key is the same, sort by the type of a segment
        // (text over icons), and for segments of the same type with the same
        // sort key, sort by a tile id.
        if lhs.sort_key == rhs.sort_key {
            if lhs.type_ != SymbolType::Text && rhs.type_ == SymbolType::Text {
                return true;
            }

            if lhs.type_ == rhs.type_ {
                return lhs.tile_id < rhs.tile_id;
            }
        }

        false
    }
}

pub struct RenderSymbolLayer {
    base: RenderLayerBase,
    unevaluated: SymbolPaintProperties::Unevaluated,
    has_format_section_overrides: bool,

    icon_size: f32,
    text_size: f32,

    symbol_icon_group: Option<ShaderGroupPtr>,
    symbol_sdf_group: Option<ShaderGroupPtr>,
    symbol_text_and_icon_group: Option<ShaderGroupPtr>,

    /// The bucket version each tile's drawables were built from.
    tile_bucket_versions: HashMap<OverscaledTileID, u32>,
}

impl RenderSymbolLayer {
    pub fn new(impl_: Immutable<SymbolLayerImpl>) -> Self {
        let unevaluated = impl_.paint.untransitioned();
        let properties: Immutable<dyn LayerProperties> = Arc::new(SymbolLayerProperties::new(impl_));
        let mut base = RenderLayerBase::new(properties);
        base.style_dependencies = unevaluated.get_dependencies();
        RenderSymbolLayer {
            base,
            unevaluated,
            has_format_section_overrides: false,
            icon_size: 1.0,
            text_size: 16.0,
            symbol_icon_group: None,
            symbol_sdf_group: None,
            symbol_text_and_icon_group: None,
            tile_bucket_versions: HashMap::new(),
        }
    }
}

impl RenderLayer for RenderSymbolLayer {
    fn base(&self) -> &RenderLayerBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut RenderLayerBase {
        &mut self.base
    }

    fn transition(&mut self, parameters: &TransitionParameters) {
        let prior = std::mem::take(&mut self.unevaluated);
        self.unevaluated = impl_cast(&self.base.base_impl).paint.transitioned(parameters, prior);
        self.has_format_section_overrides =
            SymbolLayerPaintPropertyOverrides::has_overrides(impl_cast(&self.base.base_impl).layout.get::<TextField>());
        self.base.style_dependencies = self.unevaluated.get_dependencies();
    }

    fn evaluate(&mut self, parameters: &PropertyEvaluationParameters) {
        let previous_evaluated = get_evaluated::<SymbolLayerProperties>(&self.base.evaluated_properties).clone();
        let mut properties = SymbolLayerProperties::with_evaluated(
            static_immutable_cast::<SymbolLayerImpl>(&self.base.base_impl),
            self.unevaluated.evaluate_with_previous(parameters, &previous_evaluated),
        );

        // TODO(port): SymbolLayerPaintPropertyOverrides::setOverrides when
        // `has_format_section_overrides`.

        let evaluated = &properties.evaluated;
        let has_icon_opacity = evaluated.get::<IconColor>().constant_or(&IconColor::default_value()).a > 0.0
            || evaluated.get::<IconHaloColor>().constant_or(&IconHaloColor::default_value()).a > 0.0;
        let has_text_opacity = evaluated.get::<TextColor>().constant_or(&TextColor::default_value()).a > 0.0
            || evaluated.get::<TextHaloColor>().constant_or(&TextHaloColor::default_value()).a > 0.0;

        self.base.passes = if (evaluated.get::<IconOpacity>().constant_or(&1.0) > 0.0
            && has_icon_opacity
            && self.icon_size > 0.0)
            || (evaluated.get::<TextOpacity>().constant_or(&1.0) > 0.0 && has_text_opacity && self.text_size > 0.0)
        {
            RenderPass::Translucent
        } else {
            RenderPass::None
        };
        properties.base.render_passes = self.base.passes.underlying_type();
        self.base.evaluated_properties = Arc::new(properties);

        // The symbol tweaker supports updating properties. When the style
        // changes, it will be replaced in `RenderLayer::layerChanged`
        if let Some(layer_tweaker) = &self.base.layer_tweaker {
            layer_tweaker.borrow_mut().update_properties(self.base.evaluated_properties.clone());
        }
    }

    fn has_transition(&self) -> bool {
        self.unevaluated.has_transition()
    }

    fn has_crossfade(&self) -> bool {
        false
    }

    /// `prepare(const LayerPrepareParameters&)`.
    fn prepare(&mut self, params: &mut LayerPrepareParameters<'_>) {
        let Some(source) = params.source else {
            self.base.render_tiles = None;
            self.base.render_tile_ids.clear();
            return;
        };
        self.base.render_tiles = source.get_render_tiles_sorted_by_y_position();
        self.base.render_tiles_owner = source.get_raw_render_tiles();

        self.update_render_tile_ids();
        if self.base.render_tiles.is_some() {
            self.add_render_passes_from_tiles();
        }

        // TODO(port): placementData (placement).
    }

    fn update(
        &mut self,
        shaders: &ShaderRegistry,
        context: &dyn Context,
        state: &TransformState,
        _update_parameters: &Arc<UpdateParameters>,
        _paint_parameters: &PaintParameters<'_>,
        _render_tree: &RenderTree,
        changes: &mut UniqueChangeRequestVec,
    ) {
        let passes = self.base.passes;
        let Some(render_tiles) = self.base.render_tiles.clone().filter(|t| !t.is_empty()) else {
            self.remove_all_drawables();
            return;
        };
        if passes == RenderPass::None {
            self.remove_all_drawables();
            return;
        }

        // Set up a layer group
        if self.base.layer_group.is_none() {
            if let Some(layer_group) =
                context.create_tile_layer_group(self.base.layer_index, /*initial_capacity=*/ 64, self.get_id().to_owned())
            {
                self.set_layer_group(Some(layer_group), changes);
            }
        }
        let Some(layer_group) = self.base.layer_group.clone() else {
            return;
        };

        if self.base.layer_tweaker.is_none() {
            let tweaker: LayerTweakerPtr = Rc::new(RefCell::new(SymbolLayerTweaker::new(
                self.get_id().to_owned(),
                self.base.evaluated_properties.clone(),
            )));
            self.base.layer_tweaker = Some(tweaker.clone());
            changes.push(ChangeRequest::AddLayerTweaker(layer_group.clone(), Rc::downgrade(&tweaker)));
        }

        if self.symbol_icon_group.is_none() {
            self.symbol_icon_group = shaders.get_shader_group(SYMBOL_ICON_SHADER_NAME);
        }
        if self.symbol_sdf_group.is_none() {
            self.symbol_sdf_group = shaders.get_shader_group(SYMBOL_SDF_SHADER_NAME);
        }
        if self.symbol_text_and_icon_group.is_none() {
            self.symbol_text_and_icon_group = shaders.get_shader_group(SYMBOL_TEXT_AND_ICON_SHADER_NAME);
        }

        // remove drawables that are dropped out of scope
        {
            let mut group = layer_group.borrow_mut();
            let tile_layer_group = group.as_tile_layer_group_mut().expect("symbol layer group is a TileLayerGroup");
            let base = &self.base;
            let removed = tile_layer_group.remove_drawables_if(|drawable| {
                // If the render pass has changed or the tile has dropped out
                // of the cover set, remove it.
                drawable.get_render_pass() != passes
                    || drawable.get_tile_id().is_some_and(|id| !base.render_tile_ids.contains_key(&id))
            });
            self.base.stats.drawables_removed += removed;
        }
        let render_tile_ids = &self.base.render_tile_ids;
        self.tile_bucket_versions.retain(|id, _| render_tile_ids.contains_key(id));

        let layout = &impl_cast(&self.base.base_impl).layout;
        let sort_features_by_key = !layout.get::<SymbolSortKey>().is_undefined();
        let is_offset = !layout.get::<IconOffset>().is_undefined();

        let current_zoom = state.get_zoom() as f32;
        let layer_prefix = format!("{}/", self.get_id());
        let layer_tweaker = self.base.layer_tweaker.clone();

        let mut renderable_segments: Vec<SegmentGroup> = Vec::new();

        let tiles: Vec<&crate::render_tile::RenderTile> = render_tiles.iter().collect();
        for (tile_index, tile) in tiles.iter().enumerate() {
            let tile_id = *tile.get_overscaled_tile_id();

            let base_impl = self.base.base_impl.clone();
            let render_data = super::render_data_for_pass(tile, &*base_impl, passes);
            let Some(render_data) = render_data.filter(|r| r.bucket.has_data()) else {
                self.remove_tile(passes, &tile_id);
                self.tile_bucket_versions.remove(&tile_id);
                continue;
            };
            let Some(bucket) = (*render_data.bucket).downcast_ref::<SymbolBucket>() else {
                continue;
            };

            let prev_bucket_id = self.get_render_tile_bucket_id(&tile_id);
            let version = bucket.version.load(Ordering::SeqCst);
            if (prev_bucket_id != SimpleIdentity::EMPTY && prev_bucket_id != bucket.base.bucket_id)
                || self.tile_bucket_versions.get(&tile_id).is_some_and(|v| *v != version)
            {
                // This tile was previously set up from a different bucket
                // (or the bucket's vertex data changed), drop and re-create
                // any drawables for it.
                self.remove_tile(passes, &tile_id);
            }
            self.set_render_tile_bucket_id(&tile_id, bucket.base.bucket_id);
            self.tile_bucket_versions.insert(tile_id, version);

            debug_assert!(bucket.paint_properties.contains_key(self.get_id()));
            if !bucket.paint_properties.contains_key(self.get_id()) {
                continue;
            }

            // If we already have drawables for this tile, update them.
            let update_existing = |drawable: &mut dyn mln_gfx::Drawable| {
                // This drawable was produced on a previous style/bucket, and
                // should not be updated.
                super::drawable_has_layer_tweaker(drawable, &layer_tweaker)
            };
            if self.update_tile(passes, &tile_id, update_existing) {
                continue;
            }

            let mut serial_key = 1.0_f32;
            let state_guard = bucket.state();
            let mut add_renderables = |buffer: &Buffer, type_: SymbolType| {
                if sort_features_by_key {
                    // Features need to be rendered in a specific order, so we
                    // add each segment individually
                    for (i, segment) in buffer.segments.iter().enumerate() {
                        renderable_segments.push(SegmentGroup {
                            sort_key: segment.sort_key,
                            type_,
                            tile_index,
                            tile_id: tile.id,
                            overscaled_z: tile_id.overscaled_z,
                            segment: Some(i),
                        });
                    }
                } else if !buffer.segments.is_empty() {
                    // Features can be rendered in the order produced, and as
                    // grouped by the bucket
                    renderable_segments.push(SegmentGroup {
                        sort_key: serial_key,
                        type_,
                        tile_index,
                        tile_id: tile.id,
                        overscaled_z: tile_id.overscaled_z,
                        segment: None,
                    });
                    serial_key += 1.0;
                }
            };

            let Some(atlases) = tile.get_atlas_textures() else {
                continue;
            };
            let has_icon_texture = atlases.icon.borrow().is_some();
            let has_glyph_texture = atlases.glyph.borrow().is_some();

            if !state_guard.icon.segments.is_empty() && has_icon_texture {
                add_renderables(&state_guard.icon, SymbolType::IconRGBA);
            }
            if !state_guard.sdf_icon.segments.is_empty() && has_icon_texture {
                add_renderables(&state_guard.sdf_icon, SymbolType::IconSDF);
            }
            if !state_guard.text.segments.is_empty() && has_glyph_texture {
                add_renderables(&state_guard.text, SymbolType::Text);
            }

            // TODO(port): addCollisionDrawables (collision debug boxes).
        }

        // std::multiset<SegmentGroup>: a stable sort keeps equal elements in
        // insertion order, as multiset::emplace does.
        renderable_segments.sort_by(|a, b| {
            if SegmentGroup::less(a, b) {
                std::cmp::Ordering::Less
            } else if SegmentGroup::less(b, a) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });

        // We'll be processing renderables across tiles, potentially
        // out-of-order, so keep track of some things by tile ID so we don't
        // have to re-build them multiple times.
        #[derive(Default)]
        struct TileInfo {
            text_tweaker: Option<DrawableTweakerPtr>,
            icon_tweaker: Option<DrawableTweakerPtr>,
        }
        let mut tile_cache: HashMap<UnwrappedTileID, TileInfo> = HashMap::new();
        let mut builder: Option<mln_gfx::UniqueDrawableBuilder> = None;
        let mut properties_as_uniforms: StringIDSetsPair = Default::default();

        for group in &renderable_segments {
            let is_text = group.type_ == SymbolType::Text;
            let sdf_icons = group.type_ == SymbolType::IconSDF;

            let tile = tiles[group.tile_index];
            let tile_id = tile.id.overscale_to(group.overscaled_z);
            let base_impl = self.base.base_impl.clone();
            let Some(render_data) = super::render_data_for_pass(tile, &*base_impl, passes) else {
                continue;
            };
            let Some(bucket) = (*render_data.bucket).downcast_ref::<SymbolBucket>() else {
                continue;
            };
            let state_guard = bucket.state();
            let buffer = if is_text {
                &state_guard.text
            } else if sdf_icons {
                &state_guard.sdf_icon
            } else {
                &state_guard.icon
            };

            if buffer.triangles.is_empty() {
                continue;
            }

            let evaluated = get_evaluated::<SymbolLayerProperties>(&render_data.layer_properties);
            let Some(bucket_paint_properties) = bucket.paint_properties.get(self.get_id()) else {
                continue;
            };
            let bucket_paint_properties = bucket_paint_properties.read().unwrap_or_else(|e| e.into_inner());

            let bucket_layout = &*bucket.layout;
            let values = if is_text {
                text_property_values(evaluated, bucket_layout)
            } else {
                icon_property_values(evaluated, bucket_layout)
            };

            let Some(atlases) = tile.get_atlas_textures() else {
                debug_assert!(false);
                continue;
            };

            let tile_info = tile_cache.entry(tile.id).or_default();

            properties_as_uniforms.0.clear();
            properties_as_uniforms.1.clear();

            let vertex_attribs = context.create_vertex_attribute_array();
            update_tile_attributes(
                buffer,
                is_text,
                &bucket_paint_properties,
                evaluated,
                &mut vertex_attribs.borrow_mut(),
                &mut properties_as_uniforms,
            );

            let text_halo = evaluated.get::<TextHaloColor>().constant_or(&Color::black()).a > 0.0
                && evaluated.get::<TextHaloWidth>().constant_or(&1.0) != 0.0;
            let text_fill = evaluated.get::<TextColor>().constant_or(&Color::black()).a > 0.0;

            let icon_halo = evaluated.get::<IconHaloColor>().constant_or(&Color::black()).a > 0.0
                && evaluated.get::<IconHaloWidth>().constant_or(&1.0) != 0.0;
            let icon_fill = evaluated.get::<IconColor>().constant_or(&Color::black()).a > 0.0;

            if let Some(builder) = builder.as_mut() {
                builder.clear_tweakers();
            }

            let mut draw = |shader_group: &Option<ShaderGroupPtr>, is_halo: bool, suffix: &str| {
                let Some(shader_group) = shader_group else {
                    return;
                };

                // We can use the same tweakers for all the segments in a tile
                if is_text && tile_info.text_tweaker.is_none() {
                    let text_size_is_zoom_constant =
                        bucket.text_size_binder.evaluate_for_zoom(current_zoom).is_zoom_constant;
                    tile_info.text_tweaker = Some(Rc::new(DrawableAtlasesTweaker::new(
                        Some(atlases.clone()),
                        Some(ID_SYMBOL_IMAGE_ICON_TEXTURE),
                        Some(ID_SYMBOL_IMAGE_TEXTURE),
                        is_text,
                        false,
                        values.rotation_alignment,
                        false,
                        text_size_is_zoom_constant,
                    )));
                }
                if !is_text && tile_info.icon_tweaker.is_none() {
                    let icon_scaled =
                        bucket_layout.get::<IconSize>().constant_or(&1.0) != 1.0 || bucket.icons_need_linear;
                    tile_info.icon_tweaker = Some(Rc::new(DrawableAtlasesTweaker::new(
                        Some(atlases.clone()),
                        Some(ID_SYMBOL_IMAGE_ICON_TEXTURE),
                        Some(ID_SYMBOL_IMAGE_TEXTURE),
                        is_text,
                        sdf_icons,
                        values.rotation_alignment,
                        icon_scaled,
                        false,
                    )));
                }

                let builder = builder.get_or_insert_with(|| {
                    let mut builder = context.create_drawable_builder(layer_prefix.clone());
                    builder.set_sub_layer_index(0);
                    builder.set_render_pass(passes);
                    builder.set_cull_face_mode(&CullFaceMode::disabled());
                    builder.set_depth_type(DepthMaskType::ReadOnly);
                    builder.set_color_mode(
                        &if (passes.underlying_type() & RenderPass::Translucent.underlying_type()) != 0 {
                            ColorMode::alpha_blended()
                        } else {
                            ColorMode::unblended()
                        },
                    );
                    builder
                });

                let Some(shader) =
                    shader_group.get_or_create_shader(context, &properties_as_uniforms, POS_OFFSET_ATTRIB_NAME)
                else {
                    return;
                };
                builder.set_shader(Some(shader));

                builder.clear_tweakers();
                let tweaker = if is_text { tile_info.text_tweaker.clone() } else { tile_info.icon_tweaker.clone() };
                if let Some(tweaker) = tweaker {
                    builder.add_tweaker(tweaker);
                }
                builder.set_drawable_name(format!("{layer_prefix}{suffix}"));
                builder.set_vertex_attributes(Some(vertex_attribs.clone()));

                builder.set_raw_vertices(Vec::new(), buffer.attribute_data.len(), AttributeDataType::Short4);

                let triangle_indices = Some(Rc::new(RefCell::new(IndexVectorBase::new(buffer.triangles.clone()))));
                match group.segment {
                    Some(i) => builder.set_segments_shared(
                        DrawMode::Triangles(),
                        triangle_indices,
                        std::slice::from_ref(&buffer.segments[i]),
                    ),
                    None => builder.set_segments_shared(DrawMode::Triangles(), triangle_indices, &buffer.segments),
                }

                builder.flush(context);

                let mut layer_group_ref = layer_group.borrow_mut();
                let tile_layer_group = layer_group_ref.as_tile_layer_group_mut().expect("TileLayerGroup");
                for mut drawable in builder.clear_drawables() {
                    drawable.set_tile_id(tile_id);
                    drawable.set_layer_tweaker(layer_tweaker.as_ref().and_then(layer_tweaker_slot));
                    drawable.set_render_tile(drawable_render_tile_slot(render_tiles.owner(), tile));
                    drawable.set_binders(drawable_binders_slot(&render_data.bucket, &base_impl.id));

                    let has_variable_placement = state_guard.has_variable_placement;
                    drawable.set_data(Some(Box::new(SymbolDrawableData {
                        is_halo,
                        bucket_variable_placement: has_variable_placement,
                        symbol_type: group.type_,
                        pitch_alignment: values.pitch_alignment,
                        rotation_alignment: values.rotation_alignment,
                        placement: *bucket_layout.get::<SymbolPlacement>(),
                        text_fit: *bucket_layout.get::<IconTextFit>(),
                        is_offset,
                    })));

                    tile_layer_group.add_drawable(passes, &tile_id, drawable);
                    self.base.stats.drawables_added += 1;
                }
            };

            if is_text {
                if bucket.icons_in_text {
                    if text_halo {
                        draw(&self.symbol_text_and_icon_group, true, "halo");
                    }
                    if text_fill {
                        draw(&self.symbol_text_and_icon_group, false, "fill");
                    }
                } else {
                    if text_halo {
                        draw(&self.symbol_sdf_group, true, "halo");
                    }
                    if text_fill {
                        draw(&self.symbol_sdf_group, false, "fill");
                    }
                }
            } else if sdf_icons {
                // icons
                if icon_halo {
                    draw(&self.symbol_sdf_group, true, "halo");
                }
                if icon_fill {
                    draw(&self.symbol_sdf_group, false, "fill");
                }
            } else {
                draw(&self.symbol_icon_group, false, "icon");
            }
        }
    }
}
