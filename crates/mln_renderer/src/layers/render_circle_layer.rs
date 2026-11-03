// Ports src/mln/renderer/layers/render_circle_layer.hpp and
// src/mln/renderer/layers/render_circle_layer.cpp (drawable path).
//
// TODO(port): queryIntersectsFeature (queries, M6) and
// captureRenderedFeatures (UpdateParameters::captureRenderedFeatures).

use super::circle_layer_tweaker::CircleLayerTweaker;
use super::{drawable_binders_slot, drawable_has_layer_tweaker, drawable_render_tile_slot, shared_vertices};
use crate::buckets::circle_bucket::CircleBucket;
use crate::change_request::{ChangeRequest, UniqueChangeRequestVec};
use crate::layer_tweaker::{layer_tweaker_slot, LayerTweakerPtr};
use crate::paint_parameters::PaintParameters;
use crate::paint_property_binder::read_data_driven_paint_property;
use crate::render_layer::{RenderLayer, RenderLayerBase};
use crate::render_pass::RenderPass;
use crate::render_tree::RenderTree;
use crate::update_parameters::UpdateParameters;
use mln_gfx::index_vector::IndexVectorBase;
use mln_gfx::shader_group::{ShaderGroupPtr, StringIDSetsPair};
use mln_gfx::shaders::shader_defines::*;
use mln_gfx::{AttributeDataType, ColorMode, Context, CullFaceMode, DepthMaskType, DrawMode, ShaderRegistry};
use mln_map_types::TransformState;
use mln_style::layer;
use mln_style::layer::static_immutable_cast;
use mln_style::layer_properties::{get_evaluated, LayerProperties};
use mln_style::layers::circle_layer_impl::Impl as CircleLayerImpl;
use mln_style::layers::circle_layer_properties::*;
use mln_style::{PropertyEvaluationParameters, TransitionParameters};
use mln_util::identity::SimpleIdentity;
use mln_util::immutable::Immutable;
use mln_util::Color;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

fn impl_cast(impl_: &Immutable<dyn layer::Impl>) -> &CircleLayerImpl {
    debug_assert!(std::ptr::eq(impl_.get_type_info(), CircleLayerImpl::static_type_info()));
    impl_.downcast_ref::<CircleLayerImpl>().expect("RenderCircleLayer: not a circle layer impl")
}

const CIRCLE_SHADER_GROUP_NAME: &str = "CircleShader";

pub struct RenderCircleLayer {
    base: RenderLayerBase,
    // Paint properties
    unevaluated: CirclePaintProperties::Unevaluated,
    circle_shader_group: Option<ShaderGroupPtr>,
}

impl RenderCircleLayer {
    pub fn new(impl_: Immutable<CircleLayerImpl>) -> Self {
        let unevaluated = impl_.paint.untransitioned();
        let properties: Immutable<dyn LayerProperties> = Arc::new(CircleLayerProperties::new(impl_));
        let mut base = RenderLayerBase::new(properties);
        base.style_dependencies = unevaluated.get_dependencies();
        RenderCircleLayer { base, unevaluated, circle_shader_group: None }
    }
}

impl RenderLayer for RenderCircleLayer {
    fn base(&self) -> &RenderLayerBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut RenderLayerBase {
        &mut self.base
    }

    fn transition(&mut self, parameters: &TransitionParameters) {
        let prior = std::mem::take(&mut self.unevaluated);
        self.unevaluated = impl_cast(&self.base.base_impl).paint.transitioned(parameters, prior);
        self.base.style_dependencies = self.unevaluated.get_dependencies();
    }

    fn evaluate(&mut self, parameters: &PropertyEvaluationParameters) {
        let previous_evaluated = get_evaluated::<CircleLayerProperties>(&self.base.evaluated_properties).clone();
        let mut properties = CircleLayerProperties::with_evaluated(
            static_immutable_cast::<CircleLayerImpl>(&self.base.base_impl),
            self.unevaluated.evaluate_with_previous(parameters, &previous_evaluated),
        );
        let evaluated = &properties.evaluated;

        self.base.passes = if (evaluated.get::<CircleRadius>().constant_or(&1.0) > 0.0
            || evaluated.get::<CircleStrokeWidth>().constant_or(&1.0) > 0.0)
            && (evaluated.get::<CircleColor>().constant_or(&Color::black()).a > 0.0
                || evaluated.get::<CircleStrokeColor>().constant_or(&Color::black()).a > 0.0)
            && (evaluated.get::<CircleOpacity>().constant_or(&1.0) > 0.0
                || evaluated.get::<CircleStrokeOpacity>().constant_or(&1.0) > 0.0)
        {
            RenderPass::Translucent
        } else {
            RenderPass::None
        };
        properties.base.render_passes = self.base.passes.underlying_type();
        self.base.evaluated_properties = Arc::new(properties);

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

    fn update(
        &mut self,
        shaders: &ShaderRegistry,
        context: &dyn Context,
        _state: &TransformState,
        _update_parameters: &Arc<UpdateParameters>,
        _paint_parameters: &PaintParameters<'_>,
        _render_tree: &RenderTree,
        changes: &mut UniqueChangeRequestVec,
    ) {
        // TODO(port): stats.renderedFeatures.clear() (rendered-feature capture).

        let Some(render_tiles) = self.base.render_tiles.clone().filter(|t| !t.is_empty()) else {
            self.remove_all_drawables();
            return;
        };

        // Set up a layer group
        if self.base.layer_group.is_none() {
            match context.create_tile_layer_group(self.base.layer_index, /*initial_capacity=*/ 64, self.get_id().to_owned()) {
                Some(layer_group) => self.set_layer_group(Some(layer_group), changes),
                None => return,
            }
        }
        let layer_group = self.base.layer_group.clone().expect("layer group");

        if self.base.layer_tweaker.is_none() {
            let tweaker: LayerTweakerPtr = Rc::new(RefCell::new(CircleLayerTweaker::new(
                self.get_id().to_owned(),
                self.base.evaluated_properties.clone(),
            )));
            self.base.layer_tweaker = Some(tweaker.clone());
            // layerGroup->addLayerTweaker(layerTweaker)
            changes.push(ChangeRequest::AddLayerTweaker(layer_group.clone(), Rc::downgrade(&tweaker)));
        }

        if self.circle_shader_group.is_none() {
            self.circle_shader_group = shaders.get_shader_group(CIRCLE_SHADER_GROUP_NAME);
        }
        let Some(circle_shader_group) = self.circle_shader_group.clone() else {
            self.remove_all_drawables();
            return;
        };

        let render_pass = RenderPass::Translucent;

        if (render_pass.underlying_type() & self.base.evaluated_properties.render_passes) == 0 {
            return;
        }

        {
            let mut group = layer_group.borrow_mut();
            let tile_layer_group = group.as_tile_layer_group_mut().expect("circle layer group is a TileLayerGroup");
            let base = &self.base;
            let removed = tile_layer_group
                .remove_drawables_if(|drawable| drawable.get_tile_id().is_some_and(|id| !base.has_render_tile(&id)));
            self.base.stats.drawables_removed += removed;
        }

        let evaluated_properties = self.base.evaluated_properties.clone();
        let evaluated = get_evaluated::<CircleLayerProperties>(&evaluated_properties);
        let layer_tweaker = self.base.layer_tweaker.clone();
        let mut properties_as_uniforms: StringIDSetsPair = Default::default();

        for tile in render_tiles.iter() {
            let tile_id = *tile.get_overscaled_tile_id();

            let render_data = self.get_render_data_for_pass(tile, render_pass);
            let Some(render_data) = render_data.filter(|r| r.bucket.has_data()) else {
                self.remove_tile(render_pass, &tile_id);
                continue;
            };

            let Some(bucket) = (*render_data.bucket).downcast_ref::<CircleBucket>() else {
                continue;
            };
            let vertex_count = bucket.vertices.len();
            let Some(paint_property_binders) = bucket.paint_property_binders.get(self.get_id()) else {
                continue;
            };

            let prev_bucket_id = self.base.get_render_tile_bucket_id(&tile_id);
            if prev_bucket_id != SimpleIdentity::EMPTY && prev_bucket_id != bucket.base.bucket_id {
                // This tile was previously set up from a different bucket,
                // drop and re-create any drawables for it.
                self.remove_tile(render_pass, &tile_id);
            }
            self.base.set_render_tile_bucket_id(&tile_id, bucket.base.bucket_id);

            // TODO(port): captureRenderedFeatures (updateParameters->captureRenderedFeatures).

            // If there are already drawables for this tile, update their UBOs
            // and move on to the next tile.
            let update_existing = |drawable: &mut dyn mln_gfx::Drawable| {
                // A drawable produced on a previous style/bucket should not be updated.
                drawable_has_layer_tweaker(drawable, &layer_tweaker)
            };
            if self.update_tile(render_pass, &tile_id, update_existing) {
                continue;
            }

            properties_as_uniforms.0.clear();
            properties_as_uniforms.1.clear();

            let circle_vertex_attrs = context.create_vertex_attribute_array();
            {
                let buffer = &paint_property_binders.interleaved_vertex_buffer;
                let shared = shared_vertices(&buffer.shared_vertex_vector);
                let mut attrs = circle_vertex_attrs.borrow_mut();
                let mut id = ID_CIRCLE_COLOR_VERTEX_ATTRIBUTE;
                let b = paint_property_binders;
                let pau = &mut properties_as_uniforms;
                read_data_driven_paint_property(&mut attrs, &b.circle_color, buffer, &shared, &["a_color"], evaluated.get::<CircleColor>().is_constant(), pau, &mut id);
                read_data_driven_paint_property(&mut attrs, &b.circle_radius, buffer, &shared, &["a_radius"], evaluated.get::<CircleRadius>().is_constant(), pau, &mut id);
                read_data_driven_paint_property(&mut attrs, &b.circle_blur, buffer, &shared, &["a_blur"], evaluated.get::<CircleBlur>().is_constant(), pau, &mut id);
                read_data_driven_paint_property(&mut attrs, &b.circle_opacity, buffer, &shared, &["a_opacity"], evaluated.get::<CircleOpacity>().is_constant(), pau, &mut id);
                read_data_driven_paint_property(&mut attrs, &b.circle_stroke_color, buffer, &shared, &["a_stroke_color"], evaluated.get::<CircleStrokeColor>().is_constant(), pau, &mut id);
                read_data_driven_paint_property(&mut attrs, &b.circle_stroke_width, buffer, &shared, &["a_stroke_width"], evaluated.get::<CircleStrokeWidth>().is_constant(), pau, &mut id);
                read_data_driven_paint_property(&mut attrs, &b.circle_stroke_opacity, buffer, &shared, &["a_stroke_opacity"], evaluated.get::<CircleStrokeOpacity>().is_constant(), pau, &mut id);
            }

            let Some(circle_shader) = circle_shader_group.get_or_create_shader(context, &properties_as_uniforms, "a_pos")
            else {
                continue;
            };

            if let Some(attr) =
                circle_vertex_attrs.borrow_mut().set(ID_CIRCLE_POS_VERTEX_ATTRIBUTE, -1, AttributeDataType::Invalid, 0)
            {
                attr.set_shared_raw_data(
                    Some(shared_vertices(&bucket.vertices)),
                    0, // offsetof(CircleLayoutVertex, a1)
                    0,
                    std::mem::size_of::<crate::buckets::circle_bucket::CircleLayoutVertex>() as u32,
                    AttributeDataType::Short2,
                );
            }

            let mut circle_builder = context.create_drawable_builder("circle".to_owned());
            circle_builder.set_shader(Some(circle_shader));
            circle_builder.set_depth_type(DepthMaskType::ReadOnly);
            circle_builder.set_color_mode(&ColorMode::alpha_blended());
            circle_builder.set_cull_face_mode(&CullFaceMode::disabled());

            circle_builder.set_render_pass(render_pass);
            circle_builder.set_vertex_attributes(Some(circle_vertex_attrs));

            circle_builder.set_raw_vertices(Vec::new(), vertex_count, AttributeDataType::Short2);
            circle_builder.set_segments_shared(
                DrawMode::Triangles(),
                Some(Rc::new(RefCell::new(IndexVectorBase::new(bucket.triangles.clone())))),
                &bucket.segments,
            );

            circle_builder.flush(context);

            let mut group = layer_group.borrow_mut();
            let tile_layer_group = group.as_tile_layer_group_mut().expect("TileLayerGroup");
            for mut drawable in circle_builder.clear_drawables() {
                drawable.set_tile_id(tile_id);
                drawable.set_layer_tweaker(layer_tweaker.as_ref().and_then(layer_tweaker_slot));
                drawable.set_binders(drawable_binders_slot(&render_data.bucket, self.get_id()));
                drawable.set_render_tile(drawable_render_tile_slot(render_tiles.owner(), tile));

                tile_layer_group.add_drawable(render_pass, &tile_id, drawable);
                self.base.stats.drawables_added += 1;
            }
        }
    }
}
