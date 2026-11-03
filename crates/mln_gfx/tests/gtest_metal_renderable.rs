// Ports test/renderer/metal_renderable.test.cpp (built on Metal only in C++,
// `#if MLN_RENDER_BACKEND_METAL`).
//
// The test checks `mtl::RenderableResource::bind()` keeping the pending
// MTL::CommandBuffer. The port has no Metal backend: it renders through wgpu
// (crates/mln_webgpu), which owns command buffer submission, so there is no
// backend-independent part to port.

// gtest: MetalRenderable.BindKeepsThePendingCommandBuffer
#[test]
#[ignore = "Metal backend only (mtl::RenderableResource command buffers); the port renders through wgpu and has no Metal backend"]
fn bind_keeps_the_pending_command_buffer() {
    // test/renderer/metal_renderable.test.cpp:16
}
