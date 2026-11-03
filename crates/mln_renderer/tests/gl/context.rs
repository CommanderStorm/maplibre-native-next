// Ports test/gl/context.test.cpp.
//
// The whole C++ file is `#if MLN_RENDER_BACKEND_OPENGL`: it tests
// gl::Context with ContextMode::Shared: custom GL drawing outside the render
// loop, then test::checkImage("test/fixtures/shared_context").
// The port has no OpenGL backend (mln_webgpu only), so these stay ignored;
// the backend-independent bucket tests of test/gl are in bucket_test.rs.

// gtest: GLContextMode.Shared
#[test]
#[ignore = "OpenGL backend only (gl::Context, shared GL context); the port has only the WebGPU backend"]
fn shared() {}
