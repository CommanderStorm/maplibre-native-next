// Owned references to webgpu.h objects, and the thread-affinity check.
//
// Browser WebGPU objects live in the JS realm of the worker that created
// them: emdawnwebgpu keeps the JS objects (and its table of pending
// futures) per worker, so a webgpu.h handle used, waited on or released on
// another pthread reaches the wrong table. Every handle remembers the thread
// that created it and asserts on each use and on release that it is still
// on that thread. In safe code the assertion can't fire: the handles hold
// raw pointers, so they are neither Send nor Sync on wasm32, where wgpu
// does not require it, and wgpu's own types become !Send there too.

use crate::sys;

/// Identifies the current thread: the address of a thread-local.
pub(crate) fn current_thread() -> usize {
    thread_local! {
        static KEY: u8 = const { 0 };
    }
    KEY.with(|key| core::ptr::from_ref(key) as usize)
}

#[cold]
#[track_caller]
pub(crate) fn wrong_thread(what: &str) -> ! {
    panic!(
        "mln_wgpu_webgpu_h: {what} used on a thread other than the one that created it; \
         browser WebGPU objects belong to the worker that created them"
    )
}

macro_rules! handles {
    ($($name:ident($raw:ident, $add_ref:ident, $release:ident);)*) => {$(
        /// An owned reference (one webgpu.h refcount) to a
        #[doc = concat!("`", stringify!($raw), "`.")]
        pub(crate) struct $name {
            raw: sys::$raw,
            thread: usize,
        }

        impl $name {
            /// Takes over one reference. `None` for a null handle.
            ///
            /// # Safety
            ///
            /// `raw` is null or a valid handle the caller owns a reference to.
            #[allow(dead_code)]
            pub(crate) unsafe fn from_raw(raw: sys::$raw) -> Option<Self> {
                (!raw.is_null()).then(|| Self { raw, thread: crate::handle::current_thread() })
            }

            #[track_caller]
            pub(crate) fn raw(&self) -> sys::$raw {
                if self.thread != crate::handle::current_thread() {
                    crate::handle::wrong_thread(stringify!($raw));
                }
                self.raw
            }
        }

        impl Clone for $name {
            fn clone(&self) -> Self {
                unsafe { sys::$add_ref(self.raw()) };
                Self { raw: self.raw, thread: self.thread }
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                unsafe { sys::$release(self.raw()) };
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, concat!(stringify!($raw), "({:p})"), self.raw)
            }
        }
    )*};
}

handles! {
    InstanceHandle(WGPUInstance, wgpuInstanceAddRef, wgpuInstanceRelease);
    AdapterHandle(WGPUAdapter, wgpuAdapterAddRef, wgpuAdapterRelease);
    DeviceHandle(WGPUDevice, wgpuDeviceAddRef, wgpuDeviceRelease);
    QueueHandle(WGPUQueue, wgpuQueueAddRef, wgpuQueueRelease);
    BufferHandle(WGPUBuffer, wgpuBufferAddRef, wgpuBufferRelease);
    TextureHandle(WGPUTexture, wgpuTextureAddRef, wgpuTextureRelease);
    TextureViewHandle(WGPUTextureView, wgpuTextureViewAddRef, wgpuTextureViewRelease);
    SamplerHandle(WGPUSampler, wgpuSamplerAddRef, wgpuSamplerRelease);
    ShaderModuleHandle(WGPUShaderModule, wgpuShaderModuleAddRef, wgpuShaderModuleRelease);
    BindGroupLayoutHandle(WGPUBindGroupLayout, wgpuBindGroupLayoutAddRef, wgpuBindGroupLayoutRelease);
    BindGroupHandle(WGPUBindGroup, wgpuBindGroupAddRef, wgpuBindGroupRelease);
    PipelineLayoutHandle(WGPUPipelineLayout, wgpuPipelineLayoutAddRef, wgpuPipelineLayoutRelease);
    RenderPipelineHandle(WGPURenderPipeline, wgpuRenderPipelineAddRef, wgpuRenderPipelineRelease);
    CommandEncoderHandle(WGPUCommandEncoder, wgpuCommandEncoderAddRef, wgpuCommandEncoderRelease);
    CommandBufferHandle(WGPUCommandBuffer, wgpuCommandBufferAddRef, wgpuCommandBufferRelease);
    RenderPassHandle(WGPURenderPassEncoder, wgpuRenderPassEncoderAddRef, wgpuRenderPassEncoderRelease);
    SurfaceHandle(WGPUSurface, wgpuSurfaceAddRef, wgpuSurfaceRelease);
}

/// wgpu's interface traits require `Send + Sync` on native targets only.
/// Natively this backend is never instantiated (every webgpu.h function is a
/// panicking stub there); these impls only let it type-check. On wasm32 the
/// types stay `!Send + !Sync`, which is what keeps them on their thread.
macro_rules! native_send_sync {
    ($($name:ty),* $(,)?) => {$(
        #[cfg(not(target_arch = "wasm32"))]
        unsafe impl Send for $name {}
        #[cfg(not(target_arch = "wasm32"))]
        unsafe impl Sync for $name {}
    )*};
}
pub(crate) use native_send_sync;
