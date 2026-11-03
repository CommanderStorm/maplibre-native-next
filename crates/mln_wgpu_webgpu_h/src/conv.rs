// wgpu types -> webgpu.h types (and back for features, limits and adapter
// info). Mirrors wgpu's own WebGPU backend (wgpu/src/backend/webgpu.rs).

use crate::sys::*;

macro_rules! map_enum {
    ($fn:ident($ty:ty) -> $out:ident { $($from:pat => $to:ident,)* }) => {
        pub(crate) fn $fn(value: $ty) -> $out {
            #[allow(unreachable_patterns)]
            match value {
                $($from => $to,)*
                _ => unimplemented!(concat!("mln_wgpu_webgpu_h: {:?} has no ", stringify!($out)), value),
            }
        }
    };
}

pub(crate) fn texture_format(format: wgpu::TextureFormat) -> WGPUTextureFormat {
    use wgpu::{AstcBlock as B, AstcChannel as C, TextureFormat as F};
    match format {
        F::R8Unorm => WGPUTextureFormat_R8Unorm,
        F::R8Snorm => WGPUTextureFormat_R8Snorm,
        F::R8Uint => WGPUTextureFormat_R8Uint,
        F::R8Sint => WGPUTextureFormat_R8Sint,
        F::R16Unorm => WGPUTextureFormat_R16Unorm,
        F::R16Snorm => WGPUTextureFormat_R16Snorm,
        F::R16Uint => WGPUTextureFormat_R16Uint,
        F::R16Sint => WGPUTextureFormat_R16Sint,
        F::R16Float => WGPUTextureFormat_R16Float,
        F::Rg8Unorm => WGPUTextureFormat_RG8Unorm,
        F::Rg8Snorm => WGPUTextureFormat_RG8Snorm,
        F::Rg8Uint => WGPUTextureFormat_RG8Uint,
        F::Rg8Sint => WGPUTextureFormat_RG8Sint,
        F::R32Float => WGPUTextureFormat_R32Float,
        F::R32Uint => WGPUTextureFormat_R32Uint,
        F::R32Sint => WGPUTextureFormat_R32Sint,
        F::Rg16Unorm => WGPUTextureFormat_RG16Unorm,
        F::Rg16Snorm => WGPUTextureFormat_RG16Snorm,
        F::Rg16Uint => WGPUTextureFormat_RG16Uint,
        F::Rg16Sint => WGPUTextureFormat_RG16Sint,
        F::Rg16Float => WGPUTextureFormat_RG16Float,
        F::Rgba8Unorm => WGPUTextureFormat_RGBA8Unorm,
        F::Rgba8UnormSrgb => WGPUTextureFormat_RGBA8UnormSrgb,
        F::Rgba8Snorm => WGPUTextureFormat_RGBA8Snorm,
        F::Rgba8Uint => WGPUTextureFormat_RGBA8Uint,
        F::Rgba8Sint => WGPUTextureFormat_RGBA8Sint,
        F::Bgra8Unorm => WGPUTextureFormat_BGRA8Unorm,
        F::Bgra8UnormSrgb => WGPUTextureFormat_BGRA8UnormSrgb,
        F::Rgb10a2Uint => WGPUTextureFormat_RGB10A2Uint,
        F::Rgb10a2Unorm => WGPUTextureFormat_RGB10A2Unorm,
        F::Rg11b10Ufloat => WGPUTextureFormat_RG11B10Ufloat,
        F::Rgb9e5Ufloat => WGPUTextureFormat_RGB9E5Ufloat,
        F::Rg32Float => WGPUTextureFormat_RG32Float,
        F::Rg32Uint => WGPUTextureFormat_RG32Uint,
        F::Rg32Sint => WGPUTextureFormat_RG32Sint,
        F::Rgba16Unorm => WGPUTextureFormat_RGBA16Unorm,
        F::Rgba16Snorm => WGPUTextureFormat_RGBA16Snorm,
        F::Rgba16Uint => WGPUTextureFormat_RGBA16Uint,
        F::Rgba16Sint => WGPUTextureFormat_RGBA16Sint,
        F::Rgba16Float => WGPUTextureFormat_RGBA16Float,
        F::Rgba32Float => WGPUTextureFormat_RGBA32Float,
        F::Rgba32Uint => WGPUTextureFormat_RGBA32Uint,
        F::Rgba32Sint => WGPUTextureFormat_RGBA32Sint,
        F::Stencil8 => WGPUTextureFormat_Stencil8,
        F::Depth16Unorm => WGPUTextureFormat_Depth16Unorm,
        F::Depth24Plus => WGPUTextureFormat_Depth24Plus,
        F::Depth24PlusStencil8 => WGPUTextureFormat_Depth24PlusStencil8,
        F::Depth32Float => WGPUTextureFormat_Depth32Float,
        F::Depth32FloatStencil8 => WGPUTextureFormat_Depth32FloatStencil8,
        F::Bc1RgbaUnorm => WGPUTextureFormat_BC1RGBAUnorm,
        F::Bc1RgbaUnormSrgb => WGPUTextureFormat_BC1RGBAUnormSrgb,
        F::Bc2RgbaUnorm => WGPUTextureFormat_BC2RGBAUnorm,
        F::Bc2RgbaUnormSrgb => WGPUTextureFormat_BC2RGBAUnormSrgb,
        F::Bc3RgbaUnorm => WGPUTextureFormat_BC3RGBAUnorm,
        F::Bc3RgbaUnormSrgb => WGPUTextureFormat_BC3RGBAUnormSrgb,
        F::Bc4RUnorm => WGPUTextureFormat_BC4RUnorm,
        F::Bc4RSnorm => WGPUTextureFormat_BC4RSnorm,
        F::Bc5RgUnorm => WGPUTextureFormat_BC5RGUnorm,
        F::Bc5RgSnorm => WGPUTextureFormat_BC5RGSnorm,
        F::Bc6hRgbUfloat => WGPUTextureFormat_BC6HRGBUfloat,
        F::Bc6hRgbFloat => WGPUTextureFormat_BC6HRGBFloat,
        F::Bc7RgbaUnorm => WGPUTextureFormat_BC7RGBAUnorm,
        F::Bc7RgbaUnormSrgb => WGPUTextureFormat_BC7RGBAUnormSrgb,
        F::Etc2Rgb8Unorm => WGPUTextureFormat_ETC2RGB8Unorm,
        F::Etc2Rgb8UnormSrgb => WGPUTextureFormat_ETC2RGB8UnormSrgb,
        F::Etc2Rgb8A1Unorm => WGPUTextureFormat_ETC2RGB8A1Unorm,
        F::Etc2Rgb8A1UnormSrgb => WGPUTextureFormat_ETC2RGB8A1UnormSrgb,
        F::Etc2Rgba8Unorm => WGPUTextureFormat_ETC2RGBA8Unorm,
        F::Etc2Rgba8UnormSrgb => WGPUTextureFormat_ETC2RGBA8UnormSrgb,
        F::EacR11Unorm => WGPUTextureFormat_EACR11Unorm,
        F::EacR11Snorm => WGPUTextureFormat_EACR11Snorm,
        F::EacRg11Unorm => WGPUTextureFormat_EACRG11Unorm,
        F::EacRg11Snorm => WGPUTextureFormat_EACRG11Snorm,
        F::Astc { block, channel: channel @ (C::Unorm | C::UnormSrgb) } => {
            let srgb = matches!(channel, C::UnormSrgb);
            let (unorm, unorm_srgb) = match block {
                B::B4x4 => (WGPUTextureFormat_ASTC4x4Unorm, WGPUTextureFormat_ASTC4x4UnormSrgb),
                B::B5x4 => (WGPUTextureFormat_ASTC5x4Unorm, WGPUTextureFormat_ASTC5x4UnormSrgb),
                B::B5x5 => (WGPUTextureFormat_ASTC5x5Unorm, WGPUTextureFormat_ASTC5x5UnormSrgb),
                B::B6x5 => (WGPUTextureFormat_ASTC6x5Unorm, WGPUTextureFormat_ASTC6x5UnormSrgb),
                B::B6x6 => (WGPUTextureFormat_ASTC6x6Unorm, WGPUTextureFormat_ASTC6x6UnormSrgb),
                B::B8x5 => (WGPUTextureFormat_ASTC8x5Unorm, WGPUTextureFormat_ASTC8x5UnormSrgb),
                B::B8x6 => (WGPUTextureFormat_ASTC8x6Unorm, WGPUTextureFormat_ASTC8x6UnormSrgb),
                B::B8x8 => (WGPUTextureFormat_ASTC8x8Unorm, WGPUTextureFormat_ASTC8x8UnormSrgb),
                B::B10x5 => (WGPUTextureFormat_ASTC10x5Unorm, WGPUTextureFormat_ASTC10x5UnormSrgb),
                B::B10x6 => (WGPUTextureFormat_ASTC10x6Unorm, WGPUTextureFormat_ASTC10x6UnormSrgb),
                B::B10x8 => (WGPUTextureFormat_ASTC10x8Unorm, WGPUTextureFormat_ASTC10x8UnormSrgb),
                B::B10x10 => (WGPUTextureFormat_ASTC10x10Unorm, WGPUTextureFormat_ASTC10x10UnormSrgb),
                B::B12x10 => (WGPUTextureFormat_ASTC12x10Unorm, WGPUTextureFormat_ASTC12x10UnormSrgb),
                B::B12x12 => (WGPUTextureFormat_ASTC12x12Unorm, WGPUTextureFormat_ASTC12x12UnormSrgb),
            };
            if srgb { unorm_srgb } else { unorm }
        }
        _ => unimplemented!("mln_wgpu_webgpu_h: texture format {format:?} has no WebGPU equivalent"),
    }
}

pub(crate) fn optional_texture_format(format: Option<wgpu::TextureFormat>) -> WGPUTextureFormat {
    format.map_or(WGPUTextureFormat_Undefined, texture_format)
}

map_enum!(vertex_format(wgpu::VertexFormat) -> WGPUVertexFormat {
    wgpu::VertexFormat::Uint8 => WGPUVertexFormat_Uint8,
    wgpu::VertexFormat::Uint8x2 => WGPUVertexFormat_Uint8x2,
    wgpu::VertexFormat::Uint8x4 => WGPUVertexFormat_Uint8x4,
    wgpu::VertexFormat::Sint8 => WGPUVertexFormat_Sint8,
    wgpu::VertexFormat::Sint8x2 => WGPUVertexFormat_Sint8x2,
    wgpu::VertexFormat::Sint8x4 => WGPUVertexFormat_Sint8x4,
    wgpu::VertexFormat::Unorm8 => WGPUVertexFormat_Unorm8,
    wgpu::VertexFormat::Unorm8x2 => WGPUVertexFormat_Unorm8x2,
    wgpu::VertexFormat::Unorm8x4 => WGPUVertexFormat_Unorm8x4,
    wgpu::VertexFormat::Snorm8 => WGPUVertexFormat_Snorm8,
    wgpu::VertexFormat::Snorm8x2 => WGPUVertexFormat_Snorm8x2,
    wgpu::VertexFormat::Snorm8x4 => WGPUVertexFormat_Snorm8x4,
    wgpu::VertexFormat::Uint16 => WGPUVertexFormat_Uint16,
    wgpu::VertexFormat::Uint16x2 => WGPUVertexFormat_Uint16x2,
    wgpu::VertexFormat::Uint16x4 => WGPUVertexFormat_Uint16x4,
    wgpu::VertexFormat::Sint16 => WGPUVertexFormat_Sint16,
    wgpu::VertexFormat::Sint16x2 => WGPUVertexFormat_Sint16x2,
    wgpu::VertexFormat::Sint16x4 => WGPUVertexFormat_Sint16x4,
    wgpu::VertexFormat::Unorm16 => WGPUVertexFormat_Unorm16,
    wgpu::VertexFormat::Unorm16x2 => WGPUVertexFormat_Unorm16x2,
    wgpu::VertexFormat::Unorm16x4 => WGPUVertexFormat_Unorm16x4,
    wgpu::VertexFormat::Snorm16 => WGPUVertexFormat_Snorm16,
    wgpu::VertexFormat::Snorm16x2 => WGPUVertexFormat_Snorm16x2,
    wgpu::VertexFormat::Snorm16x4 => WGPUVertexFormat_Snorm16x4,
    wgpu::VertexFormat::Float16 => WGPUVertexFormat_Float16,
    wgpu::VertexFormat::Float16x2 => WGPUVertexFormat_Float16x2,
    wgpu::VertexFormat::Float16x4 => WGPUVertexFormat_Float16x4,
    wgpu::VertexFormat::Float32 => WGPUVertexFormat_Float32,
    wgpu::VertexFormat::Float32x2 => WGPUVertexFormat_Float32x2,
    wgpu::VertexFormat::Float32x3 => WGPUVertexFormat_Float32x3,
    wgpu::VertexFormat::Float32x4 => WGPUVertexFormat_Float32x4,
    wgpu::VertexFormat::Uint32 => WGPUVertexFormat_Uint32,
    wgpu::VertexFormat::Uint32x2 => WGPUVertexFormat_Uint32x2,
    wgpu::VertexFormat::Uint32x3 => WGPUVertexFormat_Uint32x3,
    wgpu::VertexFormat::Uint32x4 => WGPUVertexFormat_Uint32x4,
    wgpu::VertexFormat::Sint32 => WGPUVertexFormat_Sint32,
    wgpu::VertexFormat::Sint32x2 => WGPUVertexFormat_Sint32x2,
    wgpu::VertexFormat::Sint32x3 => WGPUVertexFormat_Sint32x3,
    wgpu::VertexFormat::Sint32x4 => WGPUVertexFormat_Sint32x4,
    wgpu::VertexFormat::Unorm10_10_10_2 => WGPUVertexFormat_Unorm10_10_10_2,
    wgpu::VertexFormat::Unorm8x4Bgra => WGPUVertexFormat_Unorm8x4BGRA,
});

map_enum!(vertex_step_mode(wgpu::VertexStepMode) -> WGPUVertexStepMode {
    wgpu::VertexStepMode::Vertex => WGPUVertexStepMode_Vertex,
    wgpu::VertexStepMode::Instance => WGPUVertexStepMode_Instance,
});

map_enum!(primitive_topology(wgpu::PrimitiveTopology) -> WGPUPrimitiveTopology {
    wgpu::PrimitiveTopology::PointList => WGPUPrimitiveTopology_PointList,
    wgpu::PrimitiveTopology::LineList => WGPUPrimitiveTopology_LineList,
    wgpu::PrimitiveTopology::LineStrip => WGPUPrimitiveTopology_LineStrip,
    wgpu::PrimitiveTopology::TriangleList => WGPUPrimitiveTopology_TriangleList,
    wgpu::PrimitiveTopology::TriangleStrip => WGPUPrimitiveTopology_TriangleStrip,
});

map_enum!(index_format(wgpu::IndexFormat) -> WGPUIndexFormat {
    wgpu::IndexFormat::Uint16 => WGPUIndexFormat_Uint16,
    wgpu::IndexFormat::Uint32 => WGPUIndexFormat_Uint32,
});

map_enum!(front_face(wgpu::FrontFace) -> WGPUFrontFace {
    wgpu::FrontFace::Ccw => WGPUFrontFace_CCW,
    wgpu::FrontFace::Cw => WGPUFrontFace_CW,
});

pub(crate) fn cull_mode(face: Option<wgpu::Face>) -> WGPUCullMode {
    match face {
        None => WGPUCullMode_None,
        Some(wgpu::Face::Front) => WGPUCullMode_Front,
        Some(wgpu::Face::Back) => WGPUCullMode_Back,
    }
}

map_enum!(compare_function(wgpu::CompareFunction) -> WGPUCompareFunction {
    wgpu::CompareFunction::Never => WGPUCompareFunction_Never,
    wgpu::CompareFunction::Less => WGPUCompareFunction_Less,
    wgpu::CompareFunction::Equal => WGPUCompareFunction_Equal,
    wgpu::CompareFunction::LessEqual => WGPUCompareFunction_LessEqual,
    wgpu::CompareFunction::Greater => WGPUCompareFunction_Greater,
    wgpu::CompareFunction::NotEqual => WGPUCompareFunction_NotEqual,
    wgpu::CompareFunction::GreaterEqual => WGPUCompareFunction_GreaterEqual,
    wgpu::CompareFunction::Always => WGPUCompareFunction_Always,
});

pub(crate) fn optional_compare_function(compare: Option<wgpu::CompareFunction>) -> WGPUCompareFunction {
    compare.map_or(WGPUCompareFunction_Undefined, compare_function)
}

map_enum!(stencil_operation(wgpu::StencilOperation) -> WGPUStencilOperation {
    wgpu::StencilOperation::Keep => WGPUStencilOperation_Keep,
    wgpu::StencilOperation::Zero => WGPUStencilOperation_Zero,
    wgpu::StencilOperation::Replace => WGPUStencilOperation_Replace,
    wgpu::StencilOperation::Invert => WGPUStencilOperation_Invert,
    wgpu::StencilOperation::IncrementClamp => WGPUStencilOperation_IncrementClamp,
    wgpu::StencilOperation::DecrementClamp => WGPUStencilOperation_DecrementClamp,
    wgpu::StencilOperation::IncrementWrap => WGPUStencilOperation_IncrementWrap,
    wgpu::StencilOperation::DecrementWrap => WGPUStencilOperation_DecrementWrap,
});

map_enum!(blend_factor(wgpu::BlendFactor) -> WGPUBlendFactor {
    wgpu::BlendFactor::Zero => WGPUBlendFactor_Zero,
    wgpu::BlendFactor::One => WGPUBlendFactor_One,
    wgpu::BlendFactor::Src => WGPUBlendFactor_Src,
    wgpu::BlendFactor::OneMinusSrc => WGPUBlendFactor_OneMinusSrc,
    wgpu::BlendFactor::SrcAlpha => WGPUBlendFactor_SrcAlpha,
    wgpu::BlendFactor::OneMinusSrcAlpha => WGPUBlendFactor_OneMinusSrcAlpha,
    wgpu::BlendFactor::Dst => WGPUBlendFactor_Dst,
    wgpu::BlendFactor::OneMinusDst => WGPUBlendFactor_OneMinusDst,
    wgpu::BlendFactor::DstAlpha => WGPUBlendFactor_DstAlpha,
    wgpu::BlendFactor::OneMinusDstAlpha => WGPUBlendFactor_OneMinusDstAlpha,
    wgpu::BlendFactor::SrcAlphaSaturated => WGPUBlendFactor_SrcAlphaSaturated,
    wgpu::BlendFactor::Constant => WGPUBlendFactor_Constant,
    wgpu::BlendFactor::OneMinusConstant => WGPUBlendFactor_OneMinusConstant,
    wgpu::BlendFactor::Src1 => WGPUBlendFactor_Src1,
    wgpu::BlendFactor::OneMinusSrc1 => WGPUBlendFactor_OneMinusSrc1,
    wgpu::BlendFactor::Src1Alpha => WGPUBlendFactor_Src1Alpha,
    wgpu::BlendFactor::OneMinusSrc1Alpha => WGPUBlendFactor_OneMinusSrc1Alpha,
});

map_enum!(blend_operation(wgpu::BlendOperation) -> WGPUBlendOperation {
    wgpu::BlendOperation::Add => WGPUBlendOperation_Add,
    wgpu::BlendOperation::Subtract => WGPUBlendOperation_Subtract,
    wgpu::BlendOperation::ReverseSubtract => WGPUBlendOperation_ReverseSubtract,
    wgpu::BlendOperation::Min => WGPUBlendOperation_Min,
    wgpu::BlendOperation::Max => WGPUBlendOperation_Max,
});

pub(crate) fn blend_component(c: &wgpu::BlendComponent) -> WGPUBlendComponent {
    WGPUBlendComponent {
        operation: blend_operation(c.operation),
        srcFactor: blend_factor(c.src_factor),
        dstFactor: blend_factor(c.dst_factor),
    }
}

map_enum!(address_mode(wgpu::AddressMode) -> WGPUAddressMode {
    wgpu::AddressMode::ClampToEdge => WGPUAddressMode_ClampToEdge,
    wgpu::AddressMode::Repeat => WGPUAddressMode_Repeat,
    wgpu::AddressMode::MirrorRepeat => WGPUAddressMode_MirrorRepeat,
});

map_enum!(filter_mode(wgpu::FilterMode) -> WGPUFilterMode {
    wgpu::FilterMode::Nearest => WGPUFilterMode_Nearest,
    wgpu::FilterMode::Linear => WGPUFilterMode_Linear,
});

map_enum!(mipmap_filter_mode(wgpu::MipmapFilterMode) -> WGPUMipmapFilterMode {
    wgpu::MipmapFilterMode::Nearest => WGPUMipmapFilterMode_Nearest,
    wgpu::MipmapFilterMode::Linear => WGPUMipmapFilterMode_Linear,
});

map_enum!(texture_dimension(wgpu::TextureDimension) -> WGPUTextureDimension {
    wgpu::TextureDimension::D1 => WGPUTextureDimension_1D,
    wgpu::TextureDimension::D2 => WGPUTextureDimension_2D,
    wgpu::TextureDimension::D3 => WGPUTextureDimension_3D,
});

map_enum!(texture_view_dimension(wgpu::TextureViewDimension) -> WGPUTextureViewDimension {
    wgpu::TextureViewDimension::D1 => WGPUTextureViewDimension_1D,
    wgpu::TextureViewDimension::D2 => WGPUTextureViewDimension_2D,
    wgpu::TextureViewDimension::D2Array => WGPUTextureViewDimension_2DArray,
    wgpu::TextureViewDimension::Cube => WGPUTextureViewDimension_Cube,
    wgpu::TextureViewDimension::CubeArray => WGPUTextureViewDimension_CubeArray,
    wgpu::TextureViewDimension::D3 => WGPUTextureViewDimension_3D,
});

pub(crate) fn optional_texture_view_dimension(
    dimension: Option<wgpu::TextureViewDimension>,
) -> WGPUTextureViewDimension {
    dimension.map_or(WGPUTextureViewDimension_Undefined, texture_view_dimension)
}

map_enum!(texture_aspect(wgpu::TextureAspect) -> WGPUTextureAspect {
    wgpu::TextureAspect::All => WGPUTextureAspect_All,
    wgpu::TextureAspect::StencilOnly => WGPUTextureAspect_StencilOnly,
    wgpu::TextureAspect::DepthOnly => WGPUTextureAspect_DepthOnly,
});

pub(crate) fn texture_sample_type(t: wgpu::TextureSampleType) -> WGPUTextureSampleType {
    match t {
        wgpu::TextureSampleType::Float { filterable: true } => WGPUTextureSampleType_Float,
        wgpu::TextureSampleType::Float { filterable: false } => WGPUTextureSampleType_UnfilterableFloat,
        wgpu::TextureSampleType::Depth => WGPUTextureSampleType_Depth,
        wgpu::TextureSampleType::Sint => WGPUTextureSampleType_Sint,
        wgpu::TextureSampleType::Uint => WGPUTextureSampleType_Uint,
    }
}

map_enum!(storage_texture_access(wgpu::StorageTextureAccess) -> WGPUStorageTextureAccess {
    wgpu::StorageTextureAccess::WriteOnly => WGPUStorageTextureAccess_WriteOnly,
    wgpu::StorageTextureAccess::ReadOnly => WGPUStorageTextureAccess_ReadOnly,
    wgpu::StorageTextureAccess::ReadWrite => WGPUStorageTextureAccess_ReadWrite,
});

map_enum!(power_preference(wgpu::PowerPreference) -> WGPUPowerPreference {
    wgpu::PowerPreference::None => WGPUPowerPreference_Undefined,
    wgpu::PowerPreference::LowPower => WGPUPowerPreference_LowPower,
    wgpu::PowerPreference::HighPerformance => WGPUPowerPreference_HighPerformance,
});

map_enum!(error_filter(wgpu::ErrorFilter) -> WGPUErrorFilter {
    wgpu::ErrorFilter::Validation => WGPUErrorFilter_Validation,
    wgpu::ErrorFilter::OutOfMemory => WGPUErrorFilter_OutOfMemory,
    wgpu::ErrorFilter::Internal => WGPUErrorFilter_Internal,
});

pub(crate) fn map_mode(mode: wgpu::MapMode) -> WGPUMapMode {
    match mode {
        wgpu::MapMode::Read => WGPUMapMode_Read,
        wgpu::MapMode::Write => WGPUMapMode_Write,
    }
}

// The WebGPU bits of these flags have the same values in wgpu and webgpu.h;
// wgpu-only bits are dropped.
pub(crate) fn buffer_usage(usage: wgpu::BufferUsages) -> WGPUBufferUsage {
    usage.bits() as WGPUBufferUsage & 0x3FF
}

pub(crate) fn texture_usage(usage: wgpu::TextureUsages) -> WGPUTextureUsage {
    usage.bits() as WGPUTextureUsage & 0x1F
}

pub(crate) fn shader_stage(stages: wgpu::ShaderStages) -> WGPUShaderStage {
    stages.bits() as WGPUShaderStage & 0x7
}

pub(crate) fn color_write_mask(mask: wgpu::ColorWrites) -> WGPUColorWriteMask {
    mask.bits() as WGPUColorWriteMask & 0xF
}

pub(crate) fn extent(e: wgpu::Extent3d) -> WGPUExtent3D {
    WGPUExtent3D { width: e.width, height: e.height, depthOrArrayLayers: e.depth_or_array_layers }
}

pub(crate) fn origin(o: wgpu::Origin3d) -> WGPUOrigin3D {
    WGPUOrigin3D { x: o.x, y: o.y, z: o.z }
}

pub(crate) fn color(c: wgpu::Color) -> WGPUColor {
    WGPUColor { r: c.r, g: c.g, b: c.b, a: c.a }
}

pub(crate) fn texel_copy_buffer_layout(layout: wgpu::TexelCopyBufferLayout) -> WGPUTexelCopyBufferLayout {
    WGPUTexelCopyBufferLayout {
        offset: layout.offset,
        bytesPerRow: layout.bytes_per_row.unwrap_or(WGPU_COPY_STRIDE_UNDEFINED),
        rowsPerImage: layout.rows_per_image.unwrap_or(WGPU_COPY_STRIDE_UNDEFINED),
    }
}

pub(crate) fn texel_copy_texture_info(info: &wgpu::TexelCopyTextureInfo<'_>) -> WGPUTexelCopyTextureInfo {
    WGPUTexelCopyTextureInfo {
        texture: crate::resource::texture(info.texture).raw.raw(),
        mipLevel: info.mip_level,
        origin: origin(info.origin),
        aspect: texture_aspect(info.aspect),
    }
}

pub(crate) fn texel_copy_buffer_info(info: &wgpu::TexelCopyBufferInfo<'_>) -> WGPUTexelCopyBufferInfo {
    WGPUTexelCopyBufferInfo {
        layout: texel_copy_buffer_layout(info.layout),
        buffer: crate::resource::buffer(info.buffer).raw.raw(),
    }
}

const FEATURES: [(wgpu::Features, WGPUFeatureName); 16] = [
    (wgpu::Features::DEPTH_CLIP_CONTROL, WGPUFeatureName_DepthClipControl),
    (wgpu::Features::DEPTH32FLOAT_STENCIL8, WGPUFeatureName_Depth32FloatStencil8),
    (wgpu::Features::TEXTURE_COMPRESSION_BC, WGPUFeatureName_TextureCompressionBC),
    (wgpu::Features::TEXTURE_COMPRESSION_BC_SLICED_3D, WGPUFeatureName_TextureCompressionBCSliced3D),
    (wgpu::Features::TEXTURE_COMPRESSION_ETC2, WGPUFeatureName_TextureCompressionETC2),
    (wgpu::Features::TEXTURE_COMPRESSION_ASTC, WGPUFeatureName_TextureCompressionASTC),
    (wgpu::Features::TEXTURE_COMPRESSION_ASTC_SLICED_3D, WGPUFeatureName_TextureCompressionASTCSliced3D),
    (wgpu::Features::TIMESTAMP_QUERY, WGPUFeatureName_TimestampQuery),
    (wgpu::Features::INDIRECT_FIRST_INSTANCE, WGPUFeatureName_IndirectFirstInstance),
    (wgpu::Features::SHADER_F16, WGPUFeatureName_ShaderF16),
    (wgpu::Features::RG11B10UFLOAT_RENDERABLE, WGPUFeatureName_RG11B10UfloatRenderable),
    (wgpu::Features::BGRA8UNORM_STORAGE, WGPUFeatureName_BGRA8UnormStorage),
    (wgpu::Features::FLOAT32_FILTERABLE, WGPUFeatureName_Float32Filterable),
    (wgpu::Features::FLOAT32_BLENDABLE, WGPUFeatureName_Float32Blendable),
    (wgpu::Features::DUAL_SOURCE_BLENDING, WGPUFeatureName_DualSourceBlending),
    (wgpu::Features::CLIP_DISTANCES, WGPUFeatureName_ClipDistances),
];

/// # Safety
///
/// `supported` comes from wgpu{Adapter,Device}GetFeatures.
pub(crate) unsafe fn features(supported: &WGPUSupportedFeatures) -> wgpu::Features {
    let names: &[WGPUFeatureName] = if supported.features.is_null() {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(supported.features, supported.featureCount) }
    };
    FEATURES.iter().filter(|(_, name)| names.contains(name)).fold(wgpu::Features::empty(), |acc, (f, _)| acc | *f)
}

/// The webgpu.h names of `features`; panics on features WebGPU doesn't have.
pub(crate) fn feature_names(features: wgpu::Features) -> Vec<WGPUFeatureName> {
    let mut rest = features;
    let names = FEATURES
        .iter()
        .filter(|(f, _)| features.contains(*f))
        .map(|(f, name)| {
            rest.remove(*f);
            *name
        })
        .collect();
    if !rest.is_empty() {
        unimplemented!("mln_wgpu_webgpu_h: features {rest:?} have no WebGPU equivalent");
    }
    names
}

// (webgpu.h field, wgpu field) for every WebGPU limit.
macro_rules! with_limits {
    ($m:ident) => {
        $m! {
            maxTextureDimension1D: max_texture_dimension_1d,
            maxTextureDimension2D: max_texture_dimension_2d,
            maxTextureDimension3D: max_texture_dimension_3d,
            maxTextureArrayLayers: max_texture_array_layers,
            maxBindGroups: max_bind_groups,
            maxBindGroupsPlusVertexBuffers: max_bind_groups_plus_vertex_buffers,
            maxBindingsPerBindGroup: max_bindings_per_bind_group,
            maxDynamicUniformBuffersPerPipelineLayout: max_dynamic_uniform_buffers_per_pipeline_layout,
            maxDynamicStorageBuffersPerPipelineLayout: max_dynamic_storage_buffers_per_pipeline_layout,
            maxSampledTexturesPerShaderStage: max_sampled_textures_per_shader_stage,
            maxSamplersPerShaderStage: max_samplers_per_shader_stage,
            maxStorageBuffersPerShaderStage: max_storage_buffers_per_shader_stage,
            maxStorageTexturesPerShaderStage: max_storage_textures_per_shader_stage,
            maxUniformBuffersPerShaderStage: max_uniform_buffers_per_shader_stage,
            maxUniformBufferBindingSize: max_uniform_buffer_binding_size,
            maxStorageBufferBindingSize: max_storage_buffer_binding_size,
            minUniformBufferOffsetAlignment: min_uniform_buffer_offset_alignment,
            minStorageBufferOffsetAlignment: min_storage_buffer_offset_alignment,
            maxVertexBuffers: max_vertex_buffers,
            maxBufferSize: max_buffer_size,
            maxVertexAttributes: max_vertex_attributes,
            maxVertexBufferArrayStride: max_vertex_buffer_array_stride,
            maxInterStageShaderVariables: max_inter_stage_shader_variables,
            maxColorAttachments: max_color_attachments,
            maxColorAttachmentBytesPerSample: max_color_attachment_bytes_per_sample,
            maxComputeWorkgroupStorageSize: max_compute_workgroup_storage_size,
            maxComputeInvocationsPerWorkgroup: max_compute_invocations_per_workgroup,
            maxComputeWorkgroupSizeX: max_compute_workgroup_size_x,
            maxComputeWorkgroupSizeY: max_compute_workgroup_size_y,
            maxComputeWorkgroupSizeZ: max_compute_workgroup_size_z,
            maxComputeWorkgroupsPerDimension: max_compute_workgroups_per_dimension,
        }
    };
}

pub(crate) fn limits_from(raw: &WGPULimits) -> wgpu::Limits {
    let mut limits = wgpu::Limits::default();
    macro_rules! copy {
        ($($c:ident: $r:ident,)*) => { $(limits.$r = raw.$c as _;)* };
    }
    with_limits!(copy);
    limits
}

pub(crate) fn limits_to(limits: &wgpu::Limits) -> WGPULimits {
    let mut raw = undefined_limits();
    macro_rules! copy {
        ($($c:ident: $r:ident,)*) => { $(raw.$c = limits.$r as _;)* };
    }
    with_limits!(copy);
    raw
}

/// `WGPU_LIMITS_INIT`: every limit undefined.
pub(crate) fn undefined_limits() -> WGPULimits {
    let mut raw: WGPULimits = unsafe { core::mem::zeroed() };
    macro_rules! undefined {
        ($($c:ident: $r:ident,)*) => { $(raw.$c = undefined(raw.$c);)* };
    }
    fn undefined<T: Undefined>(_: T) -> T {
        T::UNDEFINED
    }
    trait Undefined {
        const UNDEFINED: Self;
    }
    impl Undefined for u32 {
        const UNDEFINED: u32 = WGPU_LIMIT_U32_UNDEFINED;
    }
    impl Undefined for u64 {
        const UNDEFINED: u64 = WGPU_LIMIT_U64_UNDEFINED;
    }
    with_limits!(undefined);
    raw.maxImmediateSize = WGPU_LIMIT_U32_UNDEFINED;
    raw
}

/// # Safety
///
/// `info` comes from wgpu{Adapter,Device}GetInfo.
pub(crate) unsafe fn adapter_info(info: &WGPUAdapterInfo) -> wgpu::AdapterInfo {
    let vendor = unsafe { to_string(info.vendor) };
    let architecture = unsafe { to_string(info.architecture) };
    // Browsers often leave description and device empty (privacy); fall
    // back to "vendor architecture", e.g. "google swiftshader".
    let name = [info.description, info.device]
        .into_iter()
        .map(|s| unsafe { to_string(s) })
        .find(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{vendor} {architecture}").trim().to_owned());
    wgpu::AdapterInfo {
        name,
        vendor: info.vendorID,
        device: info.deviceID,
        device_type: match info.adapterType {
            WGPUAdapterType_DiscreteGPU => wgpu::DeviceType::DiscreteGpu,
            WGPUAdapterType_IntegratedGPU => wgpu::DeviceType::IntegratedGpu,
            WGPUAdapterType_CPU => wgpu::DeviceType::Cpu,
            _ => wgpu::DeviceType::Other,
        },
        device_pci_bus_id: String::new(),
        driver: vendor,
        driver_info: architecture,
        backend: wgpu::Backend::BrowserWebGpu,
        subgroup_min_size: info.subgroupMinSize,
        subgroup_max_size: info.subgroupMaxSize,
        transient_saves_memory: None,
        limit_bucket: None,
    }
}
