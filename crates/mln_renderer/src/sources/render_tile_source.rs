// Ports src/mln/renderer/sources/render_tile_source.hpp and
// src/mln/renderer/sources/render_tile_source.cpp (RenderTileSource and
// TileSourceRenderItem).
//
// TODO(port): RenderTileSetSource (vector/raster sources, M4), the debug
// drawables of TileSourceRenderItem (debug overlays), queries and feature
// state.

use crate::render_source::{RenderSourceBase, RenderTiles, SourcePrepareParameters};
use crate::render_tile::RenderTile;
use crate::render_tree::RenderItem;
use crate::render_pass::RenderPass;
use crate::tile::tile::Tile;
use crate::tile_pyramid::TilePyramid;
use maplibre_native_base::Point;
use mln_actor::TaggedScheduler;
use mln_gfx::UploadPass;
use mln_style::source;
use mln_tile_id::UnwrappedTileID;
use mln_util::immutable::Immutable;
use std::cell::RefCell;
use std::rc::Rc;

/// `TileSourceRenderItem`.
pub struct TileSourceRenderItem {
    render_tiles: Rc<Vec<RenderTile>>,
    name: String,
}

impl TileSourceRenderItem {
    pub fn new(render_tiles: Rc<Vec<RenderTile>>, name: String) -> Self {
        TileSourceRenderItem { render_tiles, name }
    }
}

impl RenderItem for TileSourceRenderItem {
    fn upload(&self, upload_pass: &dyn UploadPass) {
        for tile in self.render_tiles.iter() {
            tile.upload(upload_pass);
        }
    }

    fn has_render_pass(&self, _pass: RenderPass) -> bool {
        false
    }

    fn get_name(&self) -> &str {
        &self.name
    }

    // TODO(port): updateDebugDrawables (tile borders, parse status,
    // timestamps).
}

/// `util::rotate(Point<float>, float)`.
fn rotate(a: Point<f32>, angle: f32) -> Point<f32> {
    let cos = angle.cos();
    let sin = angle.sin();
    Point::new(cos * a.x - sin * a.y, sin * a.x + cos * a.y)
}

/// `RenderTileSource`: base class for render sources that provide render
/// tiles.
pub struct RenderTileSource {
    pub base: RenderSourceBase,
    pub tile_pyramid: TilePyramid,
    pub render_tiles: Rc<Vec<RenderTile>>,
    filtered_render_tiles: RefCell<Option<RenderTiles>>,
    render_tiles_sorted_by_y: RefCell<Option<RenderTiles>>,

    bearing: f32,
    // layers: feature state
    pub feature_state: crate::source_state::SourceFeatureState,
}

impl RenderTileSource {
    /// `RenderTileSource(Immutable<style::Source::Impl>, const TaggedScheduler&)`.
    pub fn new(impl_: Immutable<dyn source::Impl>, thread_pool: &TaggedScheduler) -> Self {
        let base = RenderSourceBase::new(impl_);
        let mut tile_pyramid = TilePyramid::new(thread_pool.clone());
        tile_pyramid.set_observer(base.tile_observer());
        RenderTileSource {
            base,
            tile_pyramid,
            render_tiles: Rc::new(Vec::new()),
            filtered_render_tiles: RefCell::new(None),
            render_tiles_sorted_by_y: RefCell::new(None),
            bearing: 0.0,
            feature_state: crate::source_state::SourceFeatureState::new(),
        }
    }

    // layers: feature state
    /// `setFeatureState(sourceLayerID, featureID, state)`.
    pub fn set_feature_state(
        &mut self,
        source_layer_id: Option<&str>,
        feature_id: &str,
        state: &mln_style::expression::expression::FeatureState,
    ) {
        self.feature_state.update_state(source_layer_id, feature_id, state);
    }

    /// `getFeatureState(state, sourceLayerID, featureID)`.
    pub fn get_feature_state(
        &self,
        source_layer_id: Option<&str>,
        feature_id: &str,
    ) -> mln_style::expression::expression::FeatureState {
        self.feature_state.get_state(source_layer_id, feature_id)
    }

    /// `removeFeatureState(sourceLayerID, featureID, stateKey)`.
    pub fn remove_feature_state(&mut self, source_layer_id: Option<&str>, feature_id: Option<&str>, state_key: Option<&str>) {
        self.feature_state.remove_state(source_layer_id, feature_id, state_key);
    }

    pub fn is_loaded(&self) -> bool {
        self.tile_pyramid.is_loaded()
    }

    pub fn create_render_item(&mut self) -> Box<dyn RenderItem> {
        Box::new(TileSourceRenderItem::new(
            self.render_tiles.clone(),
            self.base.base_impl.id.clone(),
        ))
    }

    pub fn prepare(&mut self, parameters: &SourcePrepareParameters<'_>) {
        self.bearing = parameters.transform.state.get_bearing() as f32;
        *self.filtered_render_tiles.borrow_mut() = None;
        *self.render_tiles_sorted_by_y.borrow_mut() = None;
        let mut tiles: Vec<RenderTile> = Vec::with_capacity(self.tile_pyramid.rendered_tile_count());
        for (id, tile) in self.tile_pyramid.get_rendered_tiles() {
            let mut render_tile = RenderTile::new(*id, tile);
            render_tile.prepare(tile, parameters);
            tiles.push(render_tile);
        }
        // layers: feature state
        // featureState.coalesceChanges(*tiles): RenderTile::setFeatureState
        // is tile.setFeatureState, applied to the pyramid's tiles directly.
        if let Some(changes) = self.feature_state.coalesce_changes() {
            for (_, tile) in self.tile_pyramid.get_rendered_tiles() {
                tile.set_feature_state(&changes);
            }
        }
        self.render_tiles = Rc::new(tiles);
    }

    pub fn update_fading_tiles(&mut self) {
        self.tile_pyramid.update_fading_tiles();
    }

    pub fn has_fading_tiles(&self) -> bool {
        self.tile_pyramid.has_fading_tiles()
    }

    pub fn get_render_tiles(&self) -> Option<RenderTiles> {
        let mut filtered = self.filtered_render_tiles.borrow_mut();
        if filtered.is_none() {
            let indices: Vec<usize> = self
                .render_tiles
                .iter()
                .enumerate()
                .filter(|(_, render_tile)| !render_tile.hold_for_fade())
                .map(|(i, _)| i)
                .collect();
            *filtered = Some(RenderTiles::new(self.render_tiles.clone(), indices));
        }
        filtered.clone()
    }

    pub fn get_render_tiles_sorted_by_y_position(&self) -> Option<RenderTiles> {
        let mut sorted = self.render_tiles_sorted_by_y.borrow_mut();
        if sorted.is_none() {
            let source_bearing = self.bearing;
            let tiles = &self.render_tiles;
            let comp = |a: &usize, b: &usize| -> bool {
                let a = &tiles[*a];
                let b = &tiles[*b];
                let pa = Point::new(a.id.canonical.x as f32, a.id.canonical.y as f32);
                let pb = Point::new(b.id.canonical.x as f32, b.id.canonical.y as f32);

                let par = rotate(pa, source_bearing);
                let pbr = rotate(pb, source_bearing);

                // std::tie(b.z, par.y, par.x) < std::tie(a.z, pbr.y, pbr.x)
                let (l0, l1, l2) = (b.id.canonical.z, par.y, par.x);
                let (r0, r1, r2) = (a.id.canonical.z, pbr.y, pbr.x);
                if l0 != r0 {
                    return l0 < r0;
                }
                if l1 < r1 {
                    return true;
                }
                if r1 < l1 {
                    return false;
                }
                l2 < r2
            };
            let mut indices: Vec<usize> = (0..tiles.len()).collect();
            libstdcxx::sort_by(&mut indices, comp);
            *sorted = Some(RenderTiles::new(self.render_tiles.clone(), indices));
        }
        sorted.clone()
    }

    pub fn get_rendered_tile(&self, tile_id: &UnwrappedTileID) -> Option<&dyn Tile> {
        self.tile_pyramid.get_rendered_tile(tile_id)
    }

    pub fn get_raw_render_tiles(&self) -> Rc<Vec<RenderTile>> {
        self.render_tiles.clone()
    }

    pub fn set_cache_enabled(&mut self, enable: bool) {
        self.tile_pyramid.set_cache_enabled(enable);
    }

    pub fn reduce_memory_use(&mut self) {
        self.tile_pyramid.reduce_memory_use();
    }

    pub fn dump_debug_logs(&self) {
        self.tile_pyramid.dump_debug_logs();
    }
}
