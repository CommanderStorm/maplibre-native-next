// Ports the WebGPU `ShaderSource<BuiltIn, Backend::Type::WebGPU>`
// specialisations: the WGSL strings come from include/mln/shaders/webgpu/*.hpp
// through the generated `wgsl_sources` module (see BUILD.bazel); the
// attribute/texture tables from src/mln/shaders/webgpu/*.cpp are below.
//
// TODO(port): the attribute/texture tables of the other shader types
// (fill_extrusion.cpp, symbol.cpp, raster.cpp, ...) arrive with their layers.

use super::wgsl_sources;
use mln_gfx::gfx_types::AttributeDataType;
use mln_gfx::shaders::shader_defines::*;
use mln_gfx::shaders::shader_source::BuiltIn;

/// The WGSL members of a `ShaderSource` specialisation, as extracted.
#[derive(Debug)]
pub struct WgslShaderSource {
    /// The `BuiltIn` enumerator name.
    pub built_in: &'static str,
    pub name: &'static str,
    pub vertex: Option<&'static str>,
    pub fragment: Option<&'static str>,
    pub prelude: Option<&'static str>,
}

/// `shaders::AttributeInfo` (include/mln/shaders/webgpu/shader_program.hpp)
#[derive(Clone, Copy, Debug)]
pub struct AttributeInfo {
    pub index: usize,
    pub data_type: AttributeDataType,
    pub id: usize,
}

/// `shaders::TextureInfo`
#[derive(Clone, Copy, Debug)]
pub struct TextureInfo {
    pub index: usize,
    pub id: usize,
}

/// `shaders::UniformBlockInfo`
#[derive(Clone, Copy, Debug)]
pub struct UniformBlockInfo {
    pub index: usize,
    pub vertex: bool,
    pub fragment: bool,
    pub size: usize,
    pub id: usize,
}

/// A complete specialisation: WGSL plus the `attributes`,
/// `instanceAttributes` and `textures` tables.
#[derive(Debug)]
pub struct ShaderSource {
    pub wgsl: &'static WgslShaderSource,
    pub name: &'static str,
    pub attributes: &'static [AttributeInfo],
    pub instance_attributes: &'static [AttributeInfo],
    pub textures: &'static [TextureInfo],
}

// src/mln/shaders/webgpu/background.cpp
const BACKGROUND_ATTRIBUTES: &[AttributeInfo] =
    &[AttributeInfo { index: 3, data_type: AttributeDataType::Short2, id: ID_BACKGROUND_POS_VERTEX_ATTRIBUTE }];
const BACKGROUND_PATTERN_TEXTURES: &[TextureInfo] = &[TextureInfo { index: 0, id: ID_BACKGROUND_IMAGE_TEXTURE }];

// src/mln/shaders/webgpu/clipping_mask.cpp
const CLIPPING_MASK_ATTRIBUTES: &[AttributeInfo] =
    &[AttributeInfo { index: 2, data_type: AttributeDataType::Short2, id: ID_CLIPPING_MASK_POS_VERTEX_ATTRIBUTE }];

static BACKGROUND_SHADER: ShaderSource = ShaderSource {
    wgsl: &wgsl_sources::BACKGROUND_SHADER,
    name: "BackgroundShader",
    attributes: BACKGROUND_ATTRIBUTES,
    instance_attributes: &[],
    textures: &[],
};

static BACKGROUND_PATTERN_SHADER: ShaderSource = ShaderSource {
    wgsl: &wgsl_sources::BACKGROUND_PATTERN_SHADER,
    name: "BackgroundPatternShader",
    attributes: BACKGROUND_ATTRIBUTES,
    instance_attributes: &[],
    textures: BACKGROUND_PATTERN_TEXTURES,
};

static CLIPPING_MASK_PROGRAM: ShaderSource = ShaderSource {
    wgsl: &wgsl_sources::CLIPPING_MASK_PROGRAM,
    name: "ClippingMaskProgram",
    attributes: CLIPPING_MASK_ATTRIBUTES,
    instance_attributes: &[],
    textures: &[],
};

const fn attr(index: usize, data_type: AttributeDataType, id: usize) -> AttributeInfo {
    AttributeInfo { index, data_type, id }
}

const fn tex(index: usize, id: usize) -> TextureInfo {
    TextureInfo { index, id }
}

use AttributeDataType::{Float2, Float4, Short2, UByte4, UShort4};

// src/mln/shaders/webgpu/circle.cpp
const CIRCLE_ATTRIBUTES: &[AttributeInfo] = &[
    attr(0, Short2, ID_CIRCLE_POS_VERTEX_ATTRIBUTE),
    attr(1, Float4, ID_CIRCLE_COLOR_VERTEX_ATTRIBUTE),
    attr(2, Float2, ID_CIRCLE_RADIUS_VERTEX_ATTRIBUTE),
    attr(3, Float2, ID_CIRCLE_BLUR_VERTEX_ATTRIBUTE),
    attr(4, Float2, ID_CIRCLE_OPACITY_VERTEX_ATTRIBUTE),
    attr(5, Float4, ID_CIRCLE_STROKE_COLOR_VERTEX_ATTRIBUTE),
    attr(6, Float2, ID_CIRCLE_STROKE_WIDTH_VERTEX_ATTRIBUTE),
    attr(7, Float2, ID_CIRCLE_STROKE_OPACITY_VERTEX_ATTRIBUTE),
];

// src/mln/shaders/webgpu/fill.cpp
const FILL_ATTRIBUTES: &[AttributeInfo] = &[
    attr(0, Short2, ID_FILL_POS_VERTEX_ATTRIBUTE),
    attr(1, Float4, ID_FILL_COLOR_VERTEX_ATTRIBUTE),
    attr(2, Float2, ID_FILL_OPACITY_VERTEX_ATTRIBUTE),
];
const FILL_OUTLINE_ATTRIBUTES: &[AttributeInfo] = &[
    attr(0, Short2, ID_FILL_POS_VERTEX_ATTRIBUTE),
    attr(1, Float4, ID_FILL_OUTLINE_COLOR_VERTEX_ATTRIBUTE),
    attr(2, Float2, ID_FILL_OPACITY_VERTEX_ATTRIBUTE),
];
const FILL_PATTERN_ATTRIBUTES: &[AttributeInfo] = &[
    attr(4, Short2, ID_FILL_POS_VERTEX_ATTRIBUTE),
    attr(5, Float4, ID_FILL_COLOR_VERTEX_ATTRIBUTE),
    attr(6, UShort4, ID_FILL_PATTERN_FROM_VERTEX_ATTRIBUTE),
    attr(7, UShort4, ID_FILL_PATTERN_TO_VERTEX_ATTRIBUTE),
    attr(8, Float2, ID_FILL_OPACITY_VERTEX_ATTRIBUTE),
];
const FILL_PATTERN_TEXTURES: &[TextureInfo] = &[tex(0, ID_FILL_IMAGE_TEXTURE)];
const FILL_OUTLINE_TRIANGULATED_ATTRIBUTES: &[AttributeInfo] = &[
    attr(0, Short2, ID_LINE_POS_NORMAL_VERTEX_ATTRIBUTE),
    attr(1, UByte4, ID_LINE_DATA_VERTEX_ATTRIBUTE),
];

// src/mln/shaders/webgpu/line.cpp
const LINE_ATTRIBUTES: &[AttributeInfo] = &[
    attr(4, Short2, ID_LINE_POS_NORMAL_VERTEX_ATTRIBUTE),
    attr(5, UByte4, ID_LINE_DATA_VERTEX_ATTRIBUTE),
    attr(6, Float4, ID_LINE_COLOR_VERTEX_ATTRIBUTE),
    attr(7, Float2, ID_LINE_BLUR_VERTEX_ATTRIBUTE),
    attr(8, Float2, ID_LINE_OPACITY_VERTEX_ATTRIBUTE),
    attr(9, Float2, ID_LINE_GAP_WIDTH_VERTEX_ATTRIBUTE),
    attr(10, Float2, ID_LINE_OFFSET_VERTEX_ATTRIBUTE),
    attr(11, Float2, ID_LINE_WIDTH_VERTEX_ATTRIBUTE),
];
const LINE_GRADIENT_ATTRIBUTES: &[AttributeInfo] = &[
    attr(4, Short2, ID_LINE_POS_NORMAL_VERTEX_ATTRIBUTE),
    attr(5, UByte4, ID_LINE_DATA_VERTEX_ATTRIBUTE),
    attr(6, Float2, ID_LINE_BLUR_VERTEX_ATTRIBUTE),
    attr(7, Float2, ID_LINE_OPACITY_VERTEX_ATTRIBUTE),
    attr(8, Float2, ID_LINE_GAP_WIDTH_VERTEX_ATTRIBUTE),
    attr(9, Float2, ID_LINE_OFFSET_VERTEX_ATTRIBUTE),
    attr(10, Float2, ID_LINE_WIDTH_VERTEX_ATTRIBUTE),
];
const LINE_IMAGE_TEXTURES: &[TextureInfo] = &[tex(0, ID_LINE_IMAGE_TEXTURE)];
const LINE_PATTERN_ATTRIBUTES: &[AttributeInfo] = &[
    attr(4, Short2, ID_LINE_POS_NORMAL_VERTEX_ATTRIBUTE),
    attr(5, UByte4, ID_LINE_DATA_VERTEX_ATTRIBUTE),
    attr(6, Float2, ID_LINE_BLUR_VERTEX_ATTRIBUTE),
    attr(7, Float2, ID_LINE_OPACITY_VERTEX_ATTRIBUTE),
    attr(8, Float2, ID_LINE_GAP_WIDTH_VERTEX_ATTRIBUTE),
    attr(9, Float2, ID_LINE_OFFSET_VERTEX_ATTRIBUTE),
    attr(10, Float2, ID_LINE_WIDTH_VERTEX_ATTRIBUTE),
    attr(11, UShort4, ID_LINE_PATTERN_FROM_VERTEX_ATTRIBUTE),
    attr(12, UShort4, ID_LINE_PATTERN_TO_VERTEX_ATTRIBUTE),
];
const LINE_SDF_ATTRIBUTES: &[AttributeInfo] = &[
    attr(4, Short2, ID_LINE_POS_NORMAL_VERTEX_ATTRIBUTE),
    attr(5, UByte4, ID_LINE_DATA_VERTEX_ATTRIBUTE),
    attr(6, Float4, ID_LINE_COLOR_VERTEX_ATTRIBUTE),
    attr(7, Float2, ID_LINE_BLUR_VERTEX_ATTRIBUTE),
    attr(8, Float2, ID_LINE_OPACITY_VERTEX_ATTRIBUTE),
    attr(9, Float2, ID_LINE_GAP_WIDTH_VERTEX_ATTRIBUTE),
    attr(10, Float2, ID_LINE_OFFSET_VERTEX_ATTRIBUTE),
    attr(11, Float2, ID_LINE_WIDTH_VERTEX_ATTRIBUTE),
    attr(12, Float2, ID_LINE_FLOOR_WIDTH_VERTEX_ATTRIBUTE),
];
const LINE_GRADIENT_SDF_ATTRIBUTES: &[AttributeInfo] = &[
    attr(4, Short2, ID_LINE_POS_NORMAL_VERTEX_ATTRIBUTE),
    attr(5, UByte4, ID_LINE_DATA_VERTEX_ATTRIBUTE),
    attr(6, Float2, ID_LINE_BLUR_VERTEX_ATTRIBUTE),
    attr(7, Float2, ID_LINE_OPACITY_VERTEX_ATTRIBUTE),
    attr(8, Float2, ID_LINE_GAP_WIDTH_VERTEX_ATTRIBUTE),
    attr(9, Float2, ID_LINE_OFFSET_VERTEX_ATTRIBUTE),
    attr(10, Float2, ID_LINE_WIDTH_VERTEX_ATTRIBUTE),
    attr(11, Float2, ID_LINE_FLOOR_WIDTH_VERTEX_ATTRIBUTE),
];
// A texture at location L binds its sampler at L and the texture at L + 1,
// so the dash atlas sits at 2.
const LINE_GRADIENT_SDF_TEXTURES: &[TextureInfo] = &[tex(0, ID_LINE_IMAGE_TEXTURE), tex(2, ID_LINE_DASH_TEXTURE)];

macro_rules! shader_source {
    ($name:ident, $wgsl:ident, $str:literal, $attrs:expr, $textures:expr) => {
        static $name: ShaderSource = ShaderSource {
            wgsl: &wgsl_sources::$wgsl,
            name: $str,
            attributes: $attrs,
            instance_attributes: &[],
            textures: $textures,
        };
    };
}

shader_source!(CIRCLE_SHADER, CIRCLE_SHADER, "CircleShader", CIRCLE_ATTRIBUTES, &[]);
shader_source!(FILL_SHADER, FILL_SHADER, "FillShader", FILL_ATTRIBUTES, &[]);
shader_source!(FILL_OUTLINE_SHADER, FILL_OUTLINE_SHADER, "FillOutlineShader", FILL_OUTLINE_ATTRIBUTES, &[]);
shader_source!(
    FILL_PATTERN_SHADER,
    FILL_PATTERN_SHADER,
    "FillPatternShader",
    FILL_PATTERN_ATTRIBUTES,
    FILL_PATTERN_TEXTURES
);
shader_source!(
    FILL_OUTLINE_PATTERN_SHADER,
    FILL_OUTLINE_PATTERN_SHADER,
    "FillOutlinePatternShader",
    FILL_PATTERN_ATTRIBUTES,
    FILL_PATTERN_TEXTURES
);
shader_source!(
    FILL_OUTLINE_TRIANGULATED_SHADER,
    FILL_OUTLINE_TRIANGULATED_SHADER,
    "FillOutlineTriangulatedShader",
    FILL_OUTLINE_TRIANGULATED_ATTRIBUTES,
    &[]
);
shader_source!(LINE_SHADER, LINE_SHADER, "LineShader", LINE_ATTRIBUTES, &[]);
shader_source!(
    LINE_GRADIENT_SHADER,
    LINE_GRADIENT_SHADER,
    "LineGradientShader",
    LINE_GRADIENT_ATTRIBUTES,
    LINE_IMAGE_TEXTURES
);
shader_source!(
    LINE_PATTERN_SHADER,
    LINE_PATTERN_SHADER,
    "LinePatternShader",
    LINE_PATTERN_ATTRIBUTES,
    LINE_IMAGE_TEXTURES
);
shader_source!(LINE_SDF_SHADER, LINE_SDF_SHADER, "LineSDFShader", LINE_SDF_ATTRIBUTES, LINE_IMAGE_TEXTURES);
shader_source!(
    LINE_GRADIENT_SDF_SHADER,
    LINE_GRADIENT_SDF_SHADER,
    "LineGradientSDFShader",
    LINE_GRADIENT_SDF_ATTRIBUTES,
    LINE_GRADIENT_SDF_TEXTURES
);

/// `ShaderSource<shader_id, WebGPU>`, or `None` for types not ported yet.
pub fn shader_source(shader_id: BuiltIn) -> Option<&'static ShaderSource> {
    match shader_id {
        BuiltIn::BackgroundShader => Some(&BACKGROUND_SHADER),
        BuiltIn::BackgroundPatternShader => Some(&BACKGROUND_PATTERN_SHADER),
        BuiltIn::CircleShader => Some(&CIRCLE_SHADER),
        BuiltIn::ClippingMaskProgram => Some(&CLIPPING_MASK_PROGRAM),
        BuiltIn::FillShader => Some(&FILL_SHADER),
        BuiltIn::FillOutlineShader => Some(&FILL_OUTLINE_SHADER),
        BuiltIn::FillPatternShader => Some(&FILL_PATTERN_SHADER),
        BuiltIn::FillOutlinePatternShader => Some(&FILL_OUTLINE_PATTERN_SHADER),
        BuiltIn::FillOutlineTriangulatedShader => Some(&FILL_OUTLINE_TRIANGULATED_SHADER),
        BuiltIn::LineShader => Some(&LINE_SHADER),
        BuiltIn::LineGradientShader => Some(&LINE_GRADIENT_SHADER),
        BuiltIn::LinePatternShader => Some(&LINE_PATTERN_SHADER),
        BuiltIn::LineSDFShader => Some(&LINE_SDF_SHADER),
        BuiltIn::LineGradientSDFShader => Some(&LINE_GRADIENT_SDF_SHADER),
        _ => None,
    }
}

/// `ShaderSource<BuiltIn::Prelude, WebGPU>::prelude` (common.hpp)
pub fn prelude() -> &'static str {
    wgsl_sources::PRELUDE.prelude.expect("Prelude without prelude")
}
