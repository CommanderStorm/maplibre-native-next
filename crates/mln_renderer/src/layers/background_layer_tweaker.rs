// Ports src/mln/renderer/layers/background_layer_tweaker.hpp and
// src/mln/renderer/layers/background_layer_tweaker.cpp (MLN_UBO_CONSOLIDATION
// branches: the port's backend is WebGPU).

use crate::layer_tweaker::{get_tile_matrix, LayerTweaker, LayerTweakerBase};
use crate::paint_parameters::PaintParameters;
use mln_gfx::renderer::layer_group::visit_layer_group_drawables;
use mln_gfx::shaders::background_layer_ubo::{BackgroundDrawableUBO, BackgroundDrawableUnionUBO, BackgroundPropsUBO};
use mln_gfx::shaders::shader_defines::{ID_BACKGROUND_DRAWABLE_UBO, ID_BACKGROUND_PROPS_UBO};
use mln_gfx::{LayerGroupBase, UniformBufferPtr};
use mln_style::layer_properties::LayerProperties;
use super::{background_crossfade, background_layer_properties};
use mln_style::layers::{BackgroundColor, BackgroundOpacity, BackgroundPattern};
use mln_style_types::TranslateAnchorType;
use mln_util::immutable::Immutable;

pub struct BackgroundLayerTweaker {
    base: LayerTweakerBase,
    drawable_uniform_buffer: Option<UniformBufferPtr>,
}

impl BackgroundLayerTweaker {
    pub fn new(id: String, properties: Immutable<dyn LayerProperties>) -> Self {
        BackgroundLayerTweaker {
            base: LayerTweakerBase::new(id, properties),
            drawable_uniform_buffer: None,
        }
    }
}

/// `util::cast<float>(mat4)`.
fn cast_matrix(m: &[f64; 16]) -> [f32; 16] {
    std::array::from_fn(|i| m[i] as f32)
}

impl LayerTweaker for BackgroundLayerTweaker {
    fn base(&self) -> &LayerTweakerBase {
        &self.base
    }

    fn base_mut(&mut self) -> &mut LayerTweakerBase {
        &mut self.base
    }

    fn execute(&mut self, layer_group: &mut dyn LayerGroupBase, parameters: &PaintParameters<'_>) {
        if layer_group.empty() {
            return;
        }

        let context = parameters.context;

        let properties = self.base.evaluated_properties.clone();
        let evaluated = background_layer_properties(&properties);
        let _crossfade = background_crossfade(&properties);
        let has_pattern = !evaluated.get::<BackgroundPattern>().to.empty();
        // TODO(port): background-pattern (PatternAtlas::getPattern, M3).
        let image_pos_a: Option<()> = if has_pattern { parameters.pattern_atlas.borrow().get_pattern("") } else { None };
        let image_pos_b: Option<()> = if has_pattern { parameters.pattern_atlas.borrow().get_pattern("") } else { None };

        if has_pattern && (image_pos_a.is_none() || image_pos_b.is_none()) {
            // The pattern isn't valid, disable the whole thing.
            layer_group.set_enabled(false);
            return;
        }
        layer_group.set_enabled(true);

        // properties are re-evaluated every time
        self.base.properties_updated = false;

        if has_pattern {
            // TODO(port): BackgroundPatternPropsUBO (unreachable until
            // patterns exist: getPattern returns nothing, see above).
        } else {
            let props_ubo = BackgroundPropsUBO {
                color: evaluated.get::<BackgroundColor>().to_float_array(),
                opacity: *evaluated.get::<BackgroundOpacity>(),
                pad1: 0.0,
                pad2: 0.0,
                pad3: 0.0,
            };
            layer_group
                .mutable_uniform_buffers()
                .create_or_update_pod(ID_BACKGROUND_PROPS_UBO, &props_ubo, context);
        }

        let mut i: u32 = 0;
        let mut drawable_ubo_vector = vec![BackgroundDrawableUnionUBO::default(); layer_group.get_drawable_count()];

        let self_ptr: *const Self = self;
        let check = |d: &dyn mln_gfx::Drawable| match d
            .get_layer_tweaker()
            .and_then(|slot| slot.downcast_ref::<crate::layer_tweaker::LayerTweakerPtr>())
        {
            // checkTweakDrawable (the trait method needs `&self`, which the
            // RefCell of the running tweaker already lends out mutably).
            None => true,
            Some(tweaker) => std::ptr::addr_eq(tweaker.as_ptr(), self_ptr),
        };
        visit_layer_group_drawables(layer_group, |drawable| {
            debug_assert!(drawable.get_tile_id().is_some());
            let Some(tile_id) = drawable.get_tile_id() else {
                return;
            };
            if !check(drawable) {
                return;
            }

            // We assume that drawables don't change between pattern and
            // non-pattern.

            let tile_id = tile_id.to_unwrapped();
            let matrix = get_tile_matrix(
                &tile_id,
                parameters,
                &[0.0, 0.0],
                TranslateAnchorType::Viewport,
                false,
                false,
                drawable,
                false,
            );

            if has_pattern {
                // TODO(port): textures and BackgroundPatternDrawableUBO
                // (unreachable until patterns exist).
            } else {
                drawable_ubo_vector[i as usize] =
                    BackgroundDrawableUnionUBO::from_drawable(&BackgroundDrawableUBO { matrix: cast_matrix(&matrix) });
            }

            drawable.set_ubo_index(i);
            i += 1;
        });

        let bytes: &[u8] = bytemuck::cast_slice(&drawable_ubo_vector);
        let drawable_ubo_vector_size = bytes.len();
        match &self.drawable_uniform_buffer {
            Some(buffer) if buffer.get_size() >= drawable_ubo_vector_size => buffer.update(bytes),
            _ => {
                self.drawable_uniform_buffer = Some(context.create_uniform_buffer(bytes, false, true));
            }
        }

        layer_group
            .mutable_uniform_buffers()
            .set(ID_BACKGROUND_DRAWABLE_UBO, self.drawable_uniform_buffer.clone());
    }
}
