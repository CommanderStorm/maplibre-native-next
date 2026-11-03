//! A wgpu backend over the WebGPU C API (`webgpu.h`) of Dawn's
//! emdawnwebgpu port, so wgpu renders with the browser's WebGPU inside an
//! Emscripten module (`--use-port=emdawnwebgpu`). wgpu 30 only has its
//! GLES backend on `wasm32-unknown-emscripten`; this implements the
//! `wgpu::custom` interfaces instead, for what maplibre-native-next uses:
//! adapters and devices, buffers (writes and mapped readback), textures,
//! views, samplers, WGSL shader modules, bind groups, render pipelines,
//! command encoders, render passes, copies, error scopes and uncaptured
//! errors, and surfaces on canvases ([`canvas_surface_target`]). Compute,
//! queries and render bundles are `unimplemented!`.
//!
//! # Threads
//!
//! - Use the backend from a pthread, never the browser main thread. wgpu's
//!   blocking points (adapter and device requests, `Device::poll(Wait)`,
//!   popping error scopes, compilation info) wait with
//!   `wgpuInstanceWaitAny`, which suspends the wasm stack (JSPI or
//!   Asyncify) until the browser resolves the promise. Link with `-sJSPI`
//!   (or `-sASYNCIFY`) and run the work under `-sPROXY_TO_PTHREAD` or in a
//!   `std::thread`.
//! - Every object belongs to the thread that created its instance: browser
//!   WebGPU objects live in that worker's JS realm. On wasm32 wgpu doesn't
//!   require `Send`/`Sync`, and with this backend compiled in its types are
//!   neither, so safe code can't move them; every use also asserts it is on
//!   the creating thread.
//! - Map and work-done callbacks run inside `Device::poll`, as on
//!   wgpu-core. The device-lost and uncaptured-error callbacks run when the
//!   thread yields to its event loop (inside a blocking wait).
//!
//! On other targets the crate compiles (webgpu.h functions are panicking
//! stubs) but must not be used.

// webgpu.h constants keep their C names, also in patterns.
#![allow(non_upper_case_globals)]

mod command;
mod conv;
mod device;
mod event;
mod handle;
mod instance;
mod resource;
mod surface;
mod sys;

pub use command::{CommandBuffer, CommandEncoder, RenderPass};
pub use device::{Device, Queue};
pub use instance::{Adapter, Instance};
pub use resource::{
    BindGroup, BindGroupLayout, Buffer, MappedRange, PipelineLayout, RenderPipeline, Sampler, ShaderModule,
    StagingBuffer, Texture, TextureView,
};
pub use surface::{Surface, SurfaceOutputDetail, canvas_surface_target};

/// Creates a `wgpu::Instance` on the browser's WebGPU through webgpu.h.
///
/// Call it on the thread that will use the instance and everything created
/// from it (see the crate docs). Panics if webgpu.h can't create an
/// instance that can block (no JSPI or Asyncify).
pub fn create_instance(desc: wgpu::InstanceDescriptor) -> wgpu::Instance {
    use wgpu::custom::InstanceInterface;
    wgpu::Instance::from_custom(Instance::new(desc))
}
