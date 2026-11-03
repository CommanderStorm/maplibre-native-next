// Ports test/gl/object.test.cpp.
//
// The whole C++ file is `#if MLN_RENDER_BACKEND_OPENGL`: it tests
// gl::State<T> (value caching, dirty tracking) and
// gl::UniqueTexture/UniqueBuffer lifetimes on a gl::Context.
// The port has no OpenGL backend (mln_webgpu only), so these stay ignored;
// the backend-independent bucket tests of test/gl are in bucket_test.rs.

// gtest: GLObject.Value
#[test]
#[ignore = "OpenGL backend only (gl::State, gl::Object unique resources); the port has only the WebGPU backend"]
fn value() {}

// gtest: GLObject.Store
#[test]
#[ignore = "OpenGL backend only (gl::State, gl::Object unique resources); the port has only the WebGPU backend"]
fn store() {}
