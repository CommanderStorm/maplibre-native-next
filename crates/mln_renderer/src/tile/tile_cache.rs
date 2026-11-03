// Ports src/mln/tile/tile_cache.hpp and src/mln/tile/tile_cache.cpp.
//
// Deviation: C++ destroys released tiles on the thread pool
// (`deferPendingReleases` schedules the destruction and the destructor waits
// for it). Rust tiles live on the render thread (their state is `Rc`), so
// the released tiles are cancelled as in C++ and dropped on the render
// thread in `defer_pending_releases`.

use crate::tile::tile::Tile;
use mln_actor::TaggedScheduler;
use mln_tile_id::OverscaledTileID;
use std::collections::BTreeMap;

/// `TileCache`.
pub struct TileCache {
    tiles: BTreeMap<OverscaledTileID, Box<dyn Tile>>,
    /// `std::list<OverscaledTileID>`, oldest first.
    ordered_keys: Vec<OverscaledTileID>,
    _thread_pool: TaggedScheduler,
    pending_releases: Vec<Box<dyn Tile>>,
    size: usize,
}

impl TileCache {
    /// `TileCache(const TaggedScheduler&, size_t size = 0)`.
    pub fn new(thread_pool: TaggedScheduler, size: usize) -> Self {
        TileCache {
            tiles: BTreeMap::new(),
            ordered_keys: Vec::new(),
            _thread_pool: thread_pool,
            pending_releases: Vec::new(),
            size,
        }
    }

    /// `std::list::remove(key)`: removes every occurrence.
    fn remove_key(&mut self, key: &OverscaledTileID) {
        self.ordered_keys.retain(|k| k != key);
    }

    /// Change the maximum size of the cache.
    pub fn set_size(&mut self, size: usize) {
        self.size = size;

        while self.ordered_keys.len() > self.size {
            let key = self.ordered_keys[0];
            self.remove_key(&key);

            if let Some(tile) = self.tiles.remove(&key) {
                self.deferred_release(tile);
            }
        }

        debug_assert!(self.ordered_keys.len() <= self.size);
    }

    /// Get the maximum size
    pub fn get_max_size(&self) -> usize {
        self.size
    }

    /// Add a new tile with the given ID. If a tile with the same ID is
    /// already present, it will be retained and the new one will be
    /// discarded.
    pub fn add(&mut self, key: OverscaledTileID, tile: Box<dyn Tile>) {
        if !tile.is_renderable() || self.size == 0 {
            self.deferred_release(tile);
            return;
        }

        if let std::collections::btree_map::Entry::Vacant(e) = self.tiles.entry(key) {
            // inserted
            e.insert(tile);
        } else {
            // already present
            // remove existing tile key to move it to the end
            self.remove_key(&key);
            // release the newly-provided item
            self.deferred_release(tile);
        }

        // (re-)insert tile key as newest
        self.ordered_keys.push(key);

        // purge oldest key/tile if necessary
        if self.ordered_keys.len() > self.size {
            let oldest = self.ordered_keys[0];
            if let Some(tile) = self.pop(&oldest) {
                self.deferred_release(tile);
            }
        }

        debug_assert!(self.ordered_keys.len() <= self.size);
    }

    pub fn pop(&mut self, key: &OverscaledTileID) -> Option<Box<dyn Tile>> {
        let tile = self.tiles.remove(key);
        if tile.is_some() {
            self.remove_key(key);
            debug_assert!(tile.as_ref().is_some_and(|t| t.is_renderable()));
        }
        tile
    }

    pub fn get(&self, key: &OverscaledTileID) -> Option<&dyn Tile> {
        self.tiles.get(key).map(|t| t.as_ref())
    }

    pub fn has(&self, key: &OverscaledTileID) -> bool {
        self.tiles.contains_key(key)
    }

    pub fn clear(&mut self) {
        let tiles = std::mem::take(&mut self.tiles);
        for (_, tile) in tiles {
            self.deferred_release(tile);
        }
        self.ordered_keys.clear();
    }

    /// Set aside a tile to be destroyed later, without blocking
    pub fn deferred_release(&mut self, tile: Box<dyn Tile>) {
        tile.cancel();
        self.pending_releases.push(tile);
    }

    /// Schedule any accumulated deferred tiles to be destroyed (see the
    /// file comment: here they are dropped right away).
    pub fn defer_pending_releases(&mut self) {
        self.pending_releases.clear();
    }

    /// The cached tile ids (oldest first), for tests.
    pub fn ordered_keys(&self) -> &[OverscaledTileID] {
        &self.ordered_keys
    }
}

impl Drop for TileCache {
    fn drop(&mut self) {
        self.clear();
        self.pending_releases.clear();
    }
}
