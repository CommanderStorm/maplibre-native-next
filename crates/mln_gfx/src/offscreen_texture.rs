// Ports src/mln/gfx/offscreen_texture.hpp

use crate::renderable::Renderable;
use crate::texture2d::Texture2DPtr;
use mln_util::image::PremultipliedImage;
use std::any::Any;

/// `gfx::OffscreenTexture`: a renderable backed by a texture.
pub trait OffscreenTexture: Renderable + Any {
    fn is_renderable(&self) -> bool;
    fn read_still_image(&self) -> PremultipliedImage;
    fn get_texture(&self) -> Texture2DPtr;
    fn as_any(&self) -> &dyn Any;
}
