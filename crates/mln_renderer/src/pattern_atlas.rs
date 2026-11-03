// Ports src/mln/renderer/pattern_atlas.hpp and src/mln/renderer/pattern_atlas.cpp
// (the parts an empty atlas needs).
//
// TODO(port): patterns (ShelfPack bins, addPattern/removePattern, the atlas
// image and texture upload) arrive with background-pattern / fill-pattern
// (M3). With no patterns the C++ atlas is a 64x64 ShelfPack.

use mln_util::size::Size;

pub struct PatternAtlas {
    width: u32,
    height: u32,
}

impl Default for PatternAtlas {
    fn default() -> Self {
        PatternAtlas::new()
    }
}

impl PatternAtlas {
    /// `PatternAtlas() : shelfPack(64, 64, shelfPackOptions())`.
    pub fn new() -> Self {
        PatternAtlas { width: 64, height: 64 }
    }

    /// `getPattern(id)`. TODO(port): ImagePosition; always empty for now.
    pub fn get_pattern(&self, _id: &str) -> Option<()> {
        None
    }

    pub fn get_pixel_size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    pub fn is_empty(&self) -> bool {
        true
    }
}
