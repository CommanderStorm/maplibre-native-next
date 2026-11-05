// Ports include/mln/gfx/uniform_buffer.hpp and src/mln/gfx/uniform_buffer.cpp

use crate::context::Context;
use crate::shaders::shader_defines::MAX_UBO_COUNT_PER_SHADER;
use std::any::Any;
use std::rc::Rc;

/// `gfx::UniformBuffer`. Created only by backends
/// (`Context::create_uniform_buffer`). `update` takes `&self`: the buffer is
/// shared (`UniformBufferPtr`) and GPU writes go through the queue.
pub trait UniformBuffer: Any {
    fn update(&self, data: &[u8]);
    fn get_size(&self) -> usize;
    fn as_any(&self) -> &dyn Any;
    /// Byte distance between per-drawable entries (see `upload_drawable_ubos`); 0 for a single UBO.
    fn element_stride(&self) -> usize {
        0
    }
    /// Records the entry stride `upload_drawable_ubos` packed this buffer with.
    fn set_element_stride(&self, _stride: usize) {}
}

/// Entry alignment of per-drawable UBO arrays: WebGPU's default `minUniformBufferOffsetAlignment`.
pub const DRAWABLE_UBO_ALIGNMENT: usize = 256;

/// Creates or updates `buffer` with one entry per drawable (indexed by `ubo_index`), each at a
/// `DRAWABLE_UBO_ALIGNMENT` stride so a draw binds its entry as a uniform. C++ (Metal, WebGPU)
/// indexes a storage buffer instead, whose per-vertex loads are slow on llvmpipe.
pub fn upload_drawable_ubos<T: bytemuck::Pod>(context: &dyn Context, buffer: &mut Option<UniformBufferPtr>, entries: &[T]) {
    let size = size_of::<T>();
    let stride = size.next_multiple_of(DRAWABLE_UBO_ALIGNMENT);
    let mut bytes = vec![0u8; stride * entries.len()];
    for (chunk, entry) in bytes.chunks_exact_mut(stride).zip(entries) {
        chunk[..size].copy_from_slice(bytemuck::bytes_of(entry));
    }
    match buffer {
        Some(existing) if existing.get_size() >= bytes.len() => existing.update(&bytes),
        _ => *buffer = Some(context.create_uniform_buffer(&bytes, false, true)),
    }
    if let Some(buffer) = buffer {
        buffer.set_element_stride(stride);
    }
}

pub type UniformBufferPtr = Rc<dyn UniformBuffer>;
pub type UniqueUniformBufferArray = Box<UniformBufferArray>;

/// `gfx::UniformBufferArray`: a collection of uniform buffers by id.
///
/// The C++ class is abstract only for `bind(RenderPass&)` and `copy()`; the
/// WebGPU overrides are no-ops (bind groups are built per drawable in
/// `Drawable::draw`), so the port is a plain struct.
#[derive(Clone, Default)]
pub struct UniformBufferArray {
    uniform_buffer_vector: [Option<UniformBufferPtr>; MAX_UBO_COUNT_PER_SHADER],
}

impl UniformBufferArray {
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of maximum allocated elements
    pub fn allocated_size(&self) -> usize {
        self.uniform_buffer_vector.len()
    }

    /// Get an uniform buffer element, or `None` if it doesn't exist.
    pub fn get(&self, id: usize) -> Option<&UniformBufferPtr> {
        self.uniform_buffer_vector.get(id).and_then(Option::as_ref)
    }

    /// Set a new uniform buffer element or replace the existing one.
    pub fn set(&mut self, id: usize, uniform_buffer: Option<UniformBufferPtr>) -> Option<&UniformBufferPtr> {
        debug_assert!(id < self.uniform_buffer_vector.len());
        if id >= self.uniform_buffer_vector.len() {
            return None;
        }
        self.uniform_buffer_vector[id] = uniform_buffer;
        self.uniform_buffer_vector[id].as_ref()
    }

    /// Create and add a new buffer or update an existing one.
    pub fn create_or_update(&mut self, id: usize, data: &[u8], context: &dyn Context, persistent: bool) {
        if let Some(ubo) = self.get(id)
            && ubo.get_size() == data.len()
        {
            ubo.update(data);
        } else {
            self.uniform_buffer_vector[id] = Some(context.create_uniform_buffer(data, persistent, false));
        }
    }

    /// `createOrUpdate(id, const T* data, context)` for a `Pod` UBO struct.
    pub fn create_or_update_pod<T: bytemuck::Pod>(&mut self, id: usize, data: &T, context: &dyn Context) {
        self.create_or_update(id, bytemuck::bytes_of(data), context, false);
    }

    /// The C++ `operator=(const UniformBufferArray&)`: copy every slot.
    pub fn assign_from(&mut self, other: &UniformBufferArray) {
        self.uniform_buffer_vector.clone_from(&other.uniform_buffer_vector);
    }
}
