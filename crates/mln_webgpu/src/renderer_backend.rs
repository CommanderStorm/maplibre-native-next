// Ports include/mln/webgpu/renderer_backend.hpp and src/mln/webgpu/renderer_backend.cpp
//
// The C++ backend stores raw `WGPUDevice`/`WGPUQueue`/instance handles in its
// `Impl` and every resource reaches them through `Context& -> getBackend()`.
// Here the `Impl` is shared (`Rc<BackendImpl>`) by the backend and the
// context, and resources hold an `Rc<Context>`.
//
// `getDefaultRenderable()` is pure virtual and implemented by the platform
// backends (HeadlessBackend), which embed this struct.
//
// TODO(port): `setSurface` (window surfaces) and the `getCurrentTextureView`
// / `getDepthStencilView` / `getFramebufferSize` virtuals used as render pass
// fallbacks when a renderable has no views of its own; the headless
// renderable always provides them.

use crate::context::Context;
use crate::shaders::shader_group::ShaderGroup;
use crate::shaders::shader_source::shader_source;
use mln_gfx::renderer_backend::{ContextMode, RendererBackendBase};
use mln_gfx::shader_registry::ShaderRegistry;
use mln_gfx::shaders::program_parameters::ProgramParameters;
use mln_gfx::shaders::shader_source::BuiltIn;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `RendererBackend::Impl`
#[derive(Default)]
pub struct BackendImpl {
    instance: RefCell<Option<wgpu::Instance>>,
    adapter: RefCell<Option<wgpu::Adapter>>,
    device: RefCell<Option<wgpu::Device>>,
    queue: RefCell<Option<wgpu::Queue>>,
    /// `wgpu::TextureFormat::Undefined` is `None`.
    depth_stencil_format: Cell<Option<wgpu::TextureFormat>>,
    color_format: Cell<Option<wgpu::TextureFormat>>,
    /// Port-only: buffers and textures dropped since the last submit, which
    /// `destroy_retired` destroys after the next one (see `retire_buffer`).
    retired_buffers: RefCell<Vec<wgpu::Buffer>>,
    retired_textures: RefCell<Vec<wgpu::Texture>>,
}

impl BackendImpl {
    pub fn set_instance(&self, instance: Option<wgpu::Instance>) {
        *self.instance.borrow_mut() = instance;
    }
    pub fn set_adapter(&self, adapter: Option<wgpu::Adapter>) {
        *self.adapter.borrow_mut() = adapter;
    }
    pub fn set_device(&self, device: Option<wgpu::Device>) {
        *self.device.borrow_mut() = device;
    }
    pub fn set_queue(&self, queue: Option<wgpu::Queue>) {
        *self.queue.borrow_mut() = queue;
    }
    pub fn get_instance(&self) -> Option<wgpu::Instance> {
        self.instance.borrow().clone()
    }
    pub fn get_adapter(&self) -> Option<wgpu::Adapter> {
        self.adapter.borrow().clone()
    }
    pub fn get_device(&self) -> Option<wgpu::Device> {
        self.device.borrow().clone()
    }
    pub fn get_queue(&self) -> Option<wgpu::Queue> {
        self.queue.borrow().clone()
    }
    pub fn set_depth_stencil_format(&self, format: Option<wgpu::TextureFormat>) {
        self.depth_stencil_format.set(format);
    }
    pub fn get_depth_stencil_format(&self) -> Option<wgpu::TextureFormat> {
        self.depth_stencil_format.get()
    }
    pub fn set_color_format(&self, format: Option<wgpu::TextureFormat>) {
        self.color_format.set(format);
    }
    pub fn get_color_format(&self) -> Option<wgpu::TextureFormat> {
        self.color_format.get()
    }

    /// Port-only: a buffer its owner dropped. wgpu's WebGPU backend
    /// (wasm32-unknown-unknown) only drops its reference to the JavaScript
    /// GPUBuffer, whose memory then waits for the garbage collector; a map
    /// replacing tiles runs ahead of it by hundreds of MiB. There the buffer
    /// is destroyed explicitly after the next submit (work recorded before
    /// it may still use the buffer). Elsewhere dropping it frees it.
    pub fn retire_buffer(&self, buffer: wgpu::Buffer) {
        #[cfg(all(target_family = "wasm", target_os = "unknown"))]
        self.retired_buffers.borrow_mut().push(buffer);
        #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
        drop(buffer);
    }

    /// Port-only: `retire_buffer` for textures.
    pub fn retire_texture(&self, texture: wgpu::Texture) {
        #[cfg(all(target_family = "wasm", target_os = "unknown"))]
        self.retired_textures.borrow_mut().push(texture);
        #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
        drop(texture);
    }

    /// Port-only: destroys what was retired before a submit that just
    /// happened; the browser frees each once the submitted work completes.
    pub fn destroy_retired(&self) {
        for buffer in self.retired_buffers.take() {
            buffer.destroy();
        }
        for texture in self.retired_textures.take() {
            texture.destroy();
        }
    }
}

/// `webgpu::RendererBackend`: embedded by the platform backends, which
/// implement `gfx::RendererBackend` by delegating to it.
pub struct RendererBackend {
    pub base: RendererBackendBase,
    impl_: Rc<BackendImpl>,
}

/// The shader types registered by `initShaders`, in the C++ order.
const REGISTERED_SHADERS: &[BuiltIn] = &[
    BuiltIn::BackgroundShader,
    BuiltIn::BackgroundPatternShader,
    BuiltIn::CircleShader,
    BuiltIn::ClippingMaskProgram,
    BuiltIn::CollisionBoxShader,
    BuiltIn::CollisionCircleShader,
    BuiltIn::DebugShader,
    BuiltIn::FillShader,
    BuiltIn::FillOutlineShader,
    BuiltIn::FillPatternShader,
    BuiltIn::FillOutlinePatternShader,
    BuiltIn::FillOutlineTriangulatedShader,
    BuiltIn::FillExtrusionShader,
    BuiltIn::FillExtrusionPatternShader,
    BuiltIn::HeatmapShader,
    BuiltIn::HeatmapTextureShader,
    BuiltIn::HillshadeShader,
    BuiltIn::HillshadePrepareShader,
    BuiltIn::ColorReliefShader,
    BuiltIn::LineShader,
    BuiltIn::LineGradientShader,
    BuiltIn::LinePatternShader,
    BuiltIn::LineSDFShader,
    BuiltIn::LineGradientSDFShader,
    BuiltIn::LocationIndicatorShader,
    BuiltIn::LocationIndicatorTexturedShader,
    BuiltIn::RasterShader,
    BuiltIn::SymbolIconShader,
    BuiltIn::SymbolSDFShader,
    BuiltIn::SymbolTextAndIconShader,
    BuiltIn::WideVectorShader,
];

impl RendererBackend {
    pub fn new(context_mode: ContextMode) -> Self {
        RendererBackend { base: RendererBackendBase::new(context_mode), impl_: Rc::new(BackendImpl::default()) }
    }

    pub fn backend_impl(&self) -> &Rc<BackendImpl> {
        &self.impl_
    }

    /// `createContext()`
    pub fn create_context(&self) -> Rc<Context> {
        Context::new(self.impl_.clone())
    }

    /// `activate()` / `deactivate()`: activation logic if needed (none).
    pub fn activate(&self) {}
    pub fn deactivate(&self) {}

    /// `initShaders`: register one `webgpu::ShaderGroup` per built-in shader.
    ///
    /// C++ registers all 31 types; a type whose WGSL is not part of this
    /// build is skipped (TODO(port): all sources once the
    /// `@maplibre//:webgpu-shader-headers` filegroup feeds the extractor).
    /// A duplicate registration is an error, as in C++ (which throws).
    pub fn init_shaders(&self, registry: &ShaderRegistry, program_parameters: &ProgramParameters) {
        for &shader_id in REGISTERED_SHADERS {
            let Some(source) = shader_source(shader_id) else {
                continue;
            };
            let group = Rc::new(ShaderGroup::new(shader_id, program_parameters.clone()));
            if !registry.register_shader_group(group, source.name) {
                panic!("Failed to register {} with shader registry!", source.name);
            }
        }
    }

    pub fn set_instance(&self, instance: Option<wgpu::Instance>) {
        self.impl_.set_instance(instance);
    }
    pub fn set_device(&self, device: Option<wgpu::Device>) {
        self.impl_.set_device(device);
    }
    pub fn set_queue(&self, queue: Option<wgpu::Queue>) {
        self.impl_.set_queue(queue);
    }
    pub fn get_instance(&self) -> Option<wgpu::Instance> {
        self.impl_.get_instance()
    }
    pub fn get_device(&self) -> Option<wgpu::Device> {
        self.impl_.get_device()
    }
    pub fn get_queue(&self) -> Option<wgpu::Queue> {
        self.impl_.get_queue()
    }
    pub fn set_depth_stencil_format(&self, format: Option<wgpu::TextureFormat>) {
        self.impl_.set_depth_stencil_format(format);
    }
    pub fn get_depth_stencil_format(&self) -> Option<wgpu::TextureFormat> {
        self.impl_.get_depth_stencil_format()
    }
    pub fn set_color_format(&self, format: Option<wgpu::TextureFormat>) {
        self.impl_.set_color_format(format);
    }
    pub fn get_color_format(&self) -> Option<wgpu::TextureFormat> {
        self.impl_.get_color_format()
    }
}

impl Drop for RendererBackend {
    fn drop(&mut self) {
        // `context.reset()`
        self.base.reset_context();
    }
}
