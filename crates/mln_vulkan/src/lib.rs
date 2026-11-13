//! The Vulkan backend: the `mln_gfx` traits implemented on the `ash`
//! bindings, with its shaders and its offscreen, window and borrowed render
//! targets.
//!
//! The C++ backend calls Vulkan through vulkan-hpp, allocates with
//! `VulkanMemoryAllocator` and compiles its GLSL with glslang; this port makes
//! the same calls through ash, allocates with gpu-allocator and compiles
//! with naga. `vk::Unique*` handles become owners that destroy their object
//! when dropped, and the `Context&` members of resources become
//! `Rc<Context>` or `Weak<Context>`.
//!
//! The GLSL comes from the C++ headers, with the port's changes
//! (`shaders::glsl_sources`).
//!
//! A `RendererBackend` renders on a `Device`, which backends on any thread
//! share, into an `OffscreenTarget`, a `SurfaceTarget` or a `BorrowedTarget`
//! built on it.
//!
//! The crate is empty on Apple platforms and in the browser build.

#![cfg(not(any(target_vendor = "apple", target_family = "wasm")))]
// Vulkan through ash is the FFI here.
#![allow(unsafe_code)]

#[cfg(target_os = "android")]
mod android_hardware_buffer;
pub(crate) mod borrowed_target;
pub(crate) mod buffer_resource;
pub(crate) mod command_encoder;
pub(crate) mod context;
pub(crate) mod custom_layer;
pub(crate) mod descriptor_set;
pub mod device;
pub(crate) mod draw_target;
pub(crate) mod drawable;
pub(crate) mod drawable_builder;
pub mod exported_target;
pub(crate) mod layer_group;
pub(crate) mod offscreen_target;
pub(crate) mod offscreen_texture;
pub(crate) mod pipeline;
pub(crate) mod render_pass;
pub(crate) mod renderer_backend;
pub(crate) mod shaders;
pub(crate) mod surface_resource;
pub(crate) mod surface_target;
pub(crate) mod texture2d;
pub(crate) mod tile_layer_group;
pub(crate) mod uniform_buffer;
pub(crate) mod upload_pass;
pub(crate) mod vertex_buffer_resource;
#[cfg(windows)]
mod win32;
mod window;

pub use ash;
pub use borrowed_target::{BorrowedDepth, BorrowedImage, BorrowedImages, BorrowedTarget, ImageOwner, PendingReads};
pub use context::Context;
pub use custom_layer::{CustomLayerEncoder, CustomLayerPass};
pub use device::{AdapterId, Device, HostDevice, QueueLease, QueueLock, RawDevice};
pub(crate) use device::Features;
pub use exported_target::ExportedTarget;
pub(crate) use buffer_resource::BufferResource;
pub use offscreen_target::OffscreenTarget;
pub use renderer_backend::RendererBackend;
pub use shaders::compiler::{Stage as ShaderStage, compile as compile_glsl};
pub use surface_target::{HostSurface, Overlay, OverlayTarget, SurfaceTarget};
pub use texture2d::Texture2D;

#[cfg(test)]
mod tests;
