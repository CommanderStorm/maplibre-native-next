//! The bucket types: the vertex and index data a tile's layers draw.

pub(crate) mod circle_bucket;
pub(crate) mod debug_bucket; // layers: debug
pub(crate) mod fill_bucket;
pub(crate) mod fill_extrusion_bucket;
pub(crate) mod heatmap_bucket; // dem-heatmap
pub(crate) mod hillshade_bucket; // dem-heatmap
pub(crate) mod line_bucket;
pub(crate) mod raster_bucket;
pub(crate) mod symbol_bucket;

pub(crate) use circle_bucket::CircleBucket;
pub(crate) use fill_bucket::FillBucket;
pub(crate) use fill_extrusion_bucket::FillExtrusionBucket;
pub(crate) use line_bucket::LineBucket;
pub(crate) use raster_bucket::RasterBucket;
