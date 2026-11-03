// Ports include/mln/gfx/debug_group.hpp

/// Something that debug groups can be pushed to (`CommandEncoder`,
/// `RenderPass`, `UploadPass`).
pub trait DebugGroupTarget {
    fn push_debug_group(&self, name: &str);
    fn pop_debug_group(&self);
}

/// `gfx::DebugGroup<T>`: pushes on creation, pops on drop.
pub struct DebugGroup<'a> {
    target: Option<&'a dyn DebugGroupTarget>,
}

impl<'a> DebugGroup<'a> {
    pub fn new(target: &'a dyn DebugGroupTarget, name: &str) -> Self {
        target.push_debug_group(name);
        DebugGroup { target: Some(target) }
    }
}

impl Drop for DebugGroup<'_> {
    fn drop(&mut self) {
        if let Some(target) = self.target.take() {
            target.pop_debug_group();
        }
    }
}
