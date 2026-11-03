// Ports test/gl/resource_pool.test.cpp.
//
// The whole C++ file is `#if MLN_RENDER_BACKEND_OPENGL`: it tests
// gl::Context::getTexturePool(): allocation, reuse and release of GL
// textures, with context.empty().
// The port has no OpenGL backend (mln_webgpu only), so these stay ignored;
// the backend-independent bucket tests of test/gl are in bucket_test.rs.

// gtest: ResourcePool.TexturePool
#[test]
#[ignore = "OpenGL backend only (gl::TexturePool); the port has only the WebGPU backend"]
fn texture_pool() {}
