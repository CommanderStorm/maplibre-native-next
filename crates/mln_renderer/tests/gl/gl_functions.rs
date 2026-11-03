// Ports test/gl/gl_functions.test.cpp.
//
// The whole C++ file is `#if MLN_RENDER_BACKEND_OPENGL`: it tests that every
// OpenGL ES 2.0/3.0 function pointer in platform/gl_functions is loaded.
// The port has no OpenGL backend (mln_webgpu only), so these stay ignored;
// the backend-independent bucket tests of test/gl are in bucket_test.rs.

// gtest: GLFunctions.OpenGLES
#[test]
#[ignore = "OpenGL backend only (platform::gl* function pointers); the port has only the WebGPU backend"]
fn open_gles() {}
