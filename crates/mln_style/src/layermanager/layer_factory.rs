// Ports include/mln/layermanager/layer_factory.hpp +
// src/mln/layermanager/layer_factory.cpp (style half).

use crate::layer::{Layer, LayerTypeInfo};
use mln_conversion::Convertible;

pub trait LayerFactory: Send + Sync {
    /// Returns the layer type data.
    fn get_type_info(&self) -> &'static LayerTypeInfo;

    /// Returns a new Layer instance on success call; returns `None` otherwise.
    fn create_layer(&self, id: &str, value: &Convertible<'_>) -> Option<Box<dyn Layer>>;

    // TODO(port): createRenderLayer / createBucket / createLayout live with
    // the renderer (see the module comment).
}

/// `LayerFactory::getSource` (protected helper).
pub fn get_source(value: &Convertible<'_>) -> Option<String> {
    let source_value = value.object_member("source")?;
    source_value.to_string()
}
