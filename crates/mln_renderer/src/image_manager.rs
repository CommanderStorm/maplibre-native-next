// Ports src/mln/renderer/image_manager.hpp and src/mln/renderer/image_manager.cpp
// (the loaded state; images, requestors and the missing-image protocol
// arrive with sprites and symbols).

#[derive(Default)]
pub struct ImageManager {
    loaded: bool,
}

impl ImageManager {
    pub fn new() -> Self {
        ImageManager { loaded: false }
    }

    pub fn set_loaded(&mut self, loaded: bool) {
        if self.loaded == loaded {
            return;
        }

        self.loaded = loaded;

        if self.loaded {
            // TODO(port): notify the requestors (symbol layouts).
        }
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    // TODO(port): addImage, updateImage, removeImage, getImage,
    // notifyIfMissingImageAdded, reduceMemoryUseIfCacheSizeExceedsLimit,
    // clear.
}
