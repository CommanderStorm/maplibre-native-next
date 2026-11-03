// Ports include/mln/gfx/depth_mode.hpp (non-OpenGL backends: no `range`)

use crate::types::{DepthFunctionType, DepthMaskType};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DepthMode {
    pub func: DepthFunctionType,
    pub mask: DepthMaskType,
}

impl DepthMode {
    pub fn disabled() -> DepthMode {
        DepthMode { func: DepthFunctionType::Always, mask: DepthMaskType::ReadOnly }
    }
}
