// Ports the structs of include/mln/shaders/layer_ubo.hpp (the id enum is in
// shader_defines.rs).

use bytemuck::{Pod, Zeroable};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum AttributeSource {
    Constant,
    PerVertex,
    Computed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum ExpressionFunction {
    Constant,
    Linear,
    Exponential,
}

/// `struct alignas(16) GlobalPaintParamsUBO` (48 bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(C, align(16))]
pub struct GlobalPaintParamsUBO {
    /*  0 */ pub pattern_atlas_texsize: [f32; 2],
    /*  8 */ pub units_to_pixels: [f32; 2],
    /* 16 */ pub world_size: [f32; 2],
    /* 24 */ pub camera_to_center_distance: f32,
    /* 28 */ pub symbol_fade_change: f32,
    /* 32 */ pub aspect_ratio: f32,
    /* 36 */ pub pixel_ratio: f32,
    /* 40 */ pub map_zoom: f32,
    /* 44 */ pub pad1: f32,
    /* 48 */
}
const _: () = assert!(std::mem::size_of::<GlobalPaintParamsUBO>() == 3 * 16);
