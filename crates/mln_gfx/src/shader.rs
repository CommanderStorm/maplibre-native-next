// Ports include/mln/gfx/shader.hpp

use std::any::Any;
use std::rc::Rc;

/// `gfx::Shader`: base of all programs across backends; registered with a
/// `ShaderRegistry`. `to<T>()` becomes `as_any().downcast_ref::<T>()`.
pub trait Shader: Any {
    /// Get the type name of this shader
    fn type_name(&self) -> &'static str;
    fn as_any(&self) -> &dyn Any;
}

pub type ShaderPtr = Rc<dyn Shader>;
