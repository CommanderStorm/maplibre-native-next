// Ports include/mln/gfx/context_observer.hpp

use crate::backend::Type as BackendType;
use crate::shaders::shader_source::BuiltIn;

/// `gfx::ContextObserver`. `onRenderError(std::exception_ptr)` takes the
/// error message (the port has no exceptions).
pub trait ContextObserver {
    fn on_pre_compile_shader(&self, _shader: BuiltIn, _backend: BackendType, _defines: &str) {}
    fn on_post_compile_shader(&self, _shader: BuiltIn, _backend: BackendType, _defines: &str) {}
    fn on_shader_compile_failed(&self, _shader: BuiltIn, _backend: BackendType, _defines: &str) {}
    fn on_render_error(&self, _error: &str) {}
}

/// `Context::nullObserver`
pub struct NullContextObserver;

impl ContextObserver for NullContextObserver {}
