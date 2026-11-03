// test/gl/*.test.cpp: the OpenGL-only gtests (all ignored, see each file).
// test/gl/bucket.test.cpp is ported in tests/bucket_test.rs.

mod context;
#[path = "enum.rs"]
mod enum_;
mod gl_functions;
mod object;
mod resource_pool;
