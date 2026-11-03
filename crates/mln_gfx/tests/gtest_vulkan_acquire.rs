// Ports test/renderer/vulkan_acquire.test.cpp (built on Vulkan only in C++,
// `#if MLN_RENDER_BACKEND_VULKAN`).
//
// The tests drive `vulkan::Context::beginFrame` swapchain acquisition
// (timeouts, out-of-date swapchains, lost surfaces) through a stubbed
// Vulkan dispatcher. The port has no Vulkan backend of its own: it renders
// through wgpu (crates/mln_webgpu), which owns swapchain acquisition and
// recovery, and wgpu offers no way to stub a surface. No backend-independent
// part to port.

// gtest: VulkanContext.AcquireTimeoutAbortsFrame
#[test]
#[ignore = "Vulkan backend only (vulkan::Context swapchain acquisition through a stubbed dispatcher); the port renders through wgpu, which owns acquisition"]
fn acquire_timeout_aborts_frame() {
    // test/renderer/vulkan_acquire.test.cpp:138
}

// gtest: VulkanContext.FailedSwapchainRebuildIsRetried
#[test]
#[ignore = "Vulkan backend only (vulkan::Context swapchain rebuild through a stubbed dispatcher); the port renders through wgpu, which owns swapchain recovery"]
fn failed_swapchain_rebuild_is_retried() {
    // test/renderer/vulkan_acquire.test.cpp:171
}

// gtest: VulkanContext.FailedSurfaceRecreationIsRetried
#[test]
#[ignore = "Vulkan backend only (vulkan::SurfaceRenderableResource surface recreation through a stubbed dispatcher); the port renders through wgpu, which owns surface recovery"]
fn failed_surface_recreation_is_retried() {
    // test/renderer/vulkan_acquire.test.cpp:189
}
