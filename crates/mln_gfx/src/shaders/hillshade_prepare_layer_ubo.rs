// Ports include/mln/shaders/hillshade_prepare_layer_ubo.hpp

use bytemuck::{Pod, Zeroable};

/// `struct alignas(16) HillshadePrepareDrawableUBO` (64 bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(C, align(16))]
pub struct HillshadePrepareDrawableUBO {
    /*  0 */ pub matrix: [f32; 4 * 4],
    /* 64 */
}
const _: () = assert!(std::mem::size_of::<HillshadePrepareDrawableUBO>() == 4 * 16);

/// `struct alignas(16) HillshadePrepareTilePropsUBO` (32 bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(C, align(16))]
pub struct HillshadePrepareTilePropsUBO {
    /*  0 */ pub unpack: [f32; 4],
    /* 16 */ pub dimension: [f32; 2],
    /* 24 */ pub zoom: f32,
    /* 28 */ pub maxzoom: f32,
    /* 32 */
}
const _: () = assert!(std::mem::size_of::<HillshadePrepareTilePropsUBO>() == 2 * 16);
