// Ports test/gl/enum.test.cpp.
//
// The whole C++ file is `#if MLN_RENDER_BACKEND_OPENGL`: it tests the
// gl::Enum<T> conversions between gfx types and GLenum values.
// The port has no OpenGL backend (mln_webgpu only), so these stay ignored;
// the backend-independent bucket tests of test/gl are in bucket_test.rs.

// gtest: GL.DrawModeType
#[test]
#[ignore = "OpenGL backend only (gl::Enum<T> GLenum conversions); the port has only the WebGPU backend"]
fn draw_mode_type() {}

// gtest: GL.ColorBlendEquationType
#[test]
#[ignore = "OpenGL backend only (gl::Enum<T> GLenum conversions); the port has only the WebGPU backend"]
fn color_blend_equation_type() {}

// gtest: GL.ColorBlendFactorType
#[test]
#[ignore = "OpenGL backend only (gl::Enum<T> GLenum conversions); the port has only the WebGPU backend"]
fn color_blend_factor_type() {}

// gtest: GL.DepthFunctionType
#[test]
#[ignore = "OpenGL backend only (gl::Enum<T> GLenum conversions); the port has only the WebGPU backend"]
fn depth_function_type() {}

// gtest: GL.DepthMaskType
#[test]
#[ignore = "OpenGL backend only (gl::Enum<T> GLenum conversions); the port has only the WebGPU backend"]
fn depth_mask_type() {}

// gtest: GL.StencilFunctionType
#[test]
#[ignore = "OpenGL backend only (gl::Enum<T> GLenum conversions); the port has only the WebGPU backend"]
fn stencil_function_type() {}

// gtest: GL.StencilOpType
#[test]
#[ignore = "OpenGL backend only (gl::Enum<T> GLenum conversions); the port has only the WebGPU backend"]
fn stencil_op_type() {}

// gtest: GL.CullFaceSideType
#[test]
#[ignore = "OpenGL backend only (gl::Enum<T> GLenum conversions); the port has only the WebGPU backend"]
fn cull_face_side_type() {}

// gtest: GL.CullFaceWindingType
#[test]
#[ignore = "OpenGL backend only (gl::Enum<T> GLenum conversions); the port has only the WebGPU backend"]
fn cull_face_winding_type() {}

// gtest: GL.BufferUsageType
#[test]
#[ignore = "OpenGL backend only (gl::Enum<T> GLenum conversions); the port has only the WebGPU backend"]
fn buffer_usage_type() {}

// gtest: GL.TexturePixelType
#[test]
#[ignore = "OpenGL backend only (gl::Enum<T> GLenum conversions); the port has only the WebGPU backend"]
fn texture_pixel_type() {}

// gtest: GL.TextureChannelDataType
#[test]
#[ignore = "OpenGL backend only (gl::Enum<T> GLenum conversions); the port has only the WebGPU backend"]
fn texture_channel_data_type() {}

// gtest: GL.RenderbufferPixelType
#[test]
#[ignore = "OpenGL backend only (gl::Enum<T> GLenum conversions); the port has only the WebGPU backend"]
fn renderbuffer_pixel_type() {}

// gtest: GL.sizedFor
#[test]
#[ignore = "OpenGL backend only (gl::Enum<T> GLenum conversions); the port has only the WebGPU backend"]
fn sized_for() {}
