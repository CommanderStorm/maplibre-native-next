//! Sprites: the sprite loader, which requests the sprite JSON and image
//! through a `FileSource` and parses them on the thread pool, and the sprite
//! parser.

pub(crate) mod sprite_loader;
pub(crate) mod sprite_loader_observer;
pub mod sprite_parser;

pub use sprite_loader::SpriteLoader;
pub use sprite_loader_observer::SpriteLoaderObserver;
pub use sprite_parser::parse_sprite;
pub(crate) use sprite_parser::{create_style_image, parse_sprite_images};

#[cfg(test)]
mod tests;
