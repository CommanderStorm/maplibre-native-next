//! Small helpers for the C++ idioms that have no direct Rust spelling.

use std::any::Any;
use std::sync::Arc;

/// Upcasts to `Any`, for the C++ `static_cast` downcasts of `Layer::Impl`,
/// `LayerProperties` and `Layer` (blanket-implemented).
pub trait AsAny: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
}

impl<T: Any + Send + Sync> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn into_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
}
