// Ports include/mln/shaders/shader_defines.hpp and the id enum of
// include/mln/shaders/layer_ubo.hpp, for `MLN_RENDER_BACKEND_WEBGPU`
// (`getEnumValue(packed, unpacked)` = packed).
//
// The values were checked against the C++ headers compiled with
// -DMLN_RENDER_BACKEND_WEBGPU=1 (idGlobalUBOIndex = 1, idBackgroundDrawableUBO
// = 2, idBackgroundPropsUBO = 4, layerUBOStartId = 5, maxUBOCountPerShader =
// 6, maxTextureCountPerShader = 3, maxAttributeCountPerShader = 11).
//
// TODO(port): the ids of the other layer types arrive with those layers
// (circle, fill and line: layers agent, M3).

// --- layer_ubo.hpp: global UBOs (WebGPU branch) ---
pub const ID_GLOBAL_PAINT_PARAMS_UBO: usize = 0;
pub const ID_GLOBAL_UBO_INDEX: usize = 1;
pub const GLOBAL_UBO_COUNT: usize = 2;

/// `MLN_UBO_CONSOLIDATION` (Metal, Vulkan, WebGPU).
pub const MLN_UBO_CONSOLIDATION: bool = true;

// --- layer SSBOs ---
pub const LAYER_SSBO_START_ID: usize = GLOBAL_UBO_COUNT;
pub const ID_DRAWABLE_RESERVED_VERTEX_ONLY_UBO: usize = LAYER_SSBO_START_ID;
pub const ID_DRAWABLE_RESERVED_FRAGMENT_ONLY_UBO: usize = LAYER_SSBO_START_ID + 1;
pub const DRAWABLE_RESERVED_UBO_COUNT: usize = LAYER_SSBO_START_ID + 2;

pub const ID_BACKGROUND_DRAWABLE_UBO: usize = ID_DRAWABLE_RESERVED_VERTEX_ONLY_UBO; // SSBO
pub const BACKGROUND_LAYER_SSBO_COUNT: usize = DRAWABLE_RESERVED_UBO_COUNT;

pub const ID_CIRCLE_DRAWABLE_UBO: usize = ID_DRAWABLE_RESERVED_VERTEX_ONLY_UBO; // SSBO
pub const CIRCLE_LAYER_SSBO_COUNT: usize = DRAWABLE_RESERVED_UBO_COUNT;

pub const ID_FILL_DRAWABLE_UBO: usize = ID_DRAWABLE_RESERVED_VERTEX_ONLY_UBO; // SSBO
pub const ID_FILL_TILE_PROPS_UBO: usize = DRAWABLE_RESERVED_UBO_COUNT; // SSBO
pub const FILL_LAYER_SSBO_COUNT: usize = ID_FILL_TILE_PROPS_UBO + 1;

pub const ID_LINE_DRAWABLE_UBO: usize = ID_DRAWABLE_RESERVED_VERTEX_ONLY_UBO; // SSBO
pub const ID_LINE_TILE_PROPS_UBO: usize = ID_DRAWABLE_RESERVED_FRAGMENT_ONLY_UBO; // SSBO
pub const LINE_LAYER_SSBO_COUNT: usize = DRAWABLE_RESERVED_UBO_COUNT;

/// `ClippingMask` UBO (`idClippingMaskUBO = idDrawableReservedVertexOnlyUBO`).
pub const ID_CLIPPING_MASK_UBO: usize = ID_DRAWABLE_RESERVED_VERTEX_ONLY_UBO;

// fill/color-relief/fill-extrusion reserve one more SSBO, so the max of the
// layer SSBO counts is FILL_LAYER_SSBO_COUNT.

// --- layer UBOs ---
pub const LAYER_UBO_START_ID: usize = FILL_LAYER_SSBO_COUNT;
pub const ID_BACKGROUND_PROPS_UBO: usize = BACKGROUND_LAYER_SSBO_COUNT; // packed
pub const BACKGROUND_LAYER_UBO_COUNT: usize = ID_BACKGROUND_PROPS_UBO + 1;
pub const ID_CIRCLE_EVALUATED_PROPS_UBO: usize = CIRCLE_LAYER_SSBO_COUNT; // packed
pub const CIRCLE_LAYER_UBO_COUNT: usize = ID_CIRCLE_EVALUATED_PROPS_UBO + 1;
pub const ID_FILL_EVALUATED_PROPS_UBO: usize = FILL_LAYER_SSBO_COUNT; // packed
pub const FILL_LAYER_UBO_COUNT: usize = ID_FILL_EVALUATED_PROPS_UBO + 1;
pub const ID_LINE_EVALUATED_PROPS_UBO: usize = LINE_LAYER_SSBO_COUNT; // packed
pub const ID_LINE_EXPRESSION_UBO: usize = ID_LINE_EVALUATED_PROPS_UBO + 1;
pub const LINE_LAYER_UBO_COUNT: usize = ID_LINE_EXPRESSION_UBO + 1;

// --- drawable SSBOs / UBOs ---
pub const DRAWABLE_SSBO_START_ID: usize = 6;
pub const DRAWABLE_UBO_START_ID: usize = 6;
pub const BACKGROUND_UBO_COUNT: usize = BACKGROUND_LAYER_UBO_COUNT; // packed

pub const MAX_UBO_COUNT_PER_SHADER: usize = 6;
pub const MAX_SSBO_COUNT_PER_LAYER: usize = LAYER_UBO_START_ID - LAYER_SSBO_START_ID;
pub const MAX_UBO_COUNT_PER_LAYER: usize = DRAWABLE_SSBO_START_ID - LAYER_UBO_START_ID;
pub const MAX_SSBO_COUNT_PER_DRAWABLE: usize = DRAWABLE_UBO_START_ID - DRAWABLE_SSBO_START_ID;
pub const MAX_UBO_COUNT_PER_DRAWABLE: usize = MAX_UBO_COUNT_PER_SHADER - DRAWABLE_UBO_START_ID;

// --- textures ---
pub const ID_BACKGROUND_IMAGE_TEXTURE: usize = 0;
pub const BACKGROUND_TEXTURE_COUNT: usize = 1;
pub const CIRCLE_TEXTURE_COUNT: usize = 0;
pub const ID_FILL_IMAGE_TEXTURE: usize = 0;
pub const FILL_TEXTURE_COUNT: usize = 1;
pub const ID_LINE_IMAGE_TEXTURE: usize = 0;
pub const ID_LINE_DASH_TEXTURE: usize = 1;
pub const LINE_TEXTURE_COUNT: usize = 2;
pub const MAX_TEXTURE_COUNT_PER_SHADER: usize = 3;

// --- vertex attributes ---
pub const ID_BACKGROUND_POS_VERTEX_ATTRIBUTE: usize = 0;
pub const BACKGROUND_VERTEX_ATTRIBUTE_COUNT: usize = 1;
pub const ID_CLIPPING_MASK_POS_VERTEX_ATTRIBUTE: usize = 0;

pub const ID_CIRCLE_POS_VERTEX_ATTRIBUTE: usize = 0;
// Data driven
pub const ID_CIRCLE_COLOR_VERTEX_ATTRIBUTE: usize = 1;
pub const ID_CIRCLE_RADIUS_VERTEX_ATTRIBUTE: usize = 2;
pub const ID_CIRCLE_BLUR_VERTEX_ATTRIBUTE: usize = 3;
pub const ID_CIRCLE_OPACITY_VERTEX_ATTRIBUTE: usize = 4;
pub const ID_CIRCLE_STROKE_COLOR_VERTEX_ATTRIBUTE: usize = 5;
pub const ID_CIRCLE_STROKE_WIDTH_VERTEX_ATTRIBUTE: usize = 6;
pub const ID_CIRCLE_STROKE_OPACITY_VERTEX_ATTRIBUTE: usize = 7;
pub const CIRCLE_VERTEX_ATTRIBUTE_COUNT: usize = 8;

pub const ID_FILL_POS_VERTEX_ATTRIBUTE: usize = 0;
// Data driven
pub const ID_FILL_COLOR_VERTEX_ATTRIBUTE: usize = 1;
pub const ID_FILL_OPACITY_VERTEX_ATTRIBUTE: usize = 2;
pub const ID_FILL_OUTLINE_COLOR_VERTEX_ATTRIBUTE: usize = 3;
pub const ID_FILL_PATTERN_FROM_VERTEX_ATTRIBUTE: usize = 4;
pub const ID_FILL_PATTERN_TO_VERTEX_ATTRIBUTE: usize = 5;
pub const FILL_VERTEX_ATTRIBUTE_COUNT: usize = 6;

pub const ID_LINE_POS_NORMAL_VERTEX_ATTRIBUTE: usize = 0;
pub const ID_LINE_DATA_VERTEX_ATTRIBUTE: usize = 1;
// Data driven
pub const ID_LINE_COLOR_VERTEX_ATTRIBUTE: usize = 2;
pub const ID_LINE_BLUR_VERTEX_ATTRIBUTE: usize = 3;
pub const ID_LINE_OPACITY_VERTEX_ATTRIBUTE: usize = 4;
pub const ID_LINE_GAP_WIDTH_VERTEX_ATTRIBUTE: usize = 5;
pub const ID_LINE_OFFSET_VERTEX_ATTRIBUTE: usize = 6;
pub const ID_LINE_WIDTH_VERTEX_ATTRIBUTE: usize = 7;
pub const ID_LINE_FLOOR_WIDTH_VERTEX_ATTRIBUTE: usize = 8;
pub const ID_LINE_PATTERN_FROM_VERTEX_ATTRIBUTE: usize = 9;
pub const ID_LINE_PATTERN_TO_VERTEX_ATTRIBUTE: usize = 10;
pub const LINE_VERTEX_ATTRIBUTE_COUNT: usize = 11;
pub const MAX_ATTRIBUTE_COUNT_PER_SHADER: usize = 11;
