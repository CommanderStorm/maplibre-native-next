//! Ports the high files of `src/mln/tile` (PLAN.md, M3: they include the
//! renderer, so they live here; `mln_tile` holds the tile data decoding).
//!
//! TODO(port): vector_tile / vector_mvt_tile and the tile loader (M4),
//! raster and raster-dem tiles, custom geometry tiles, MLT tiles.

#[allow(clippy::module_inception)]
pub mod tile;

pub mod geojson_tile;
pub mod geojson_tile_data;
pub mod geometry_tile;
pub mod geometry_tile_worker;
pub mod tile_cache;
pub mod tile_observer;

pub use tile::{Kind, Tile, TileBase, TileNecessity, TileOperation, TileUpdateParameters};
