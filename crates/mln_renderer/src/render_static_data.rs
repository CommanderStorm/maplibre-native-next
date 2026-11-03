// Ports src/mln/renderer/render_static_data.hpp and src/mln/renderer/render_static_data.cpp
// (the static geometry the background layer uses).

use mln_gfx::shaders::segment::SegmentBase;
use mln_gfx::ShaderRegistry;
use mln_util::constants;

pub struct RenderStaticData {
    // TODO(port): tileVertexBuffer / quadTriangleIndexBuffer (uploaded for
    // stencil clipping masks), depthRenderbuffer.
    pub has_3d: bool,
    pub uploaded: std::cell::Cell<bool>,
    pub shaders: Box<ShaderRegistry>,
    pub clipping_mask_segments: Vec<SegmentBase>,
}

impl RenderStaticData {
    pub fn new(shaders: Box<ShaderRegistry>) -> Self {
        RenderStaticData {
            has_3d: false,
            uploaded: std::cell::Cell::new(false),
            shaders,
            clipping_mask_segments: Self::tile_triangle_segments(),
        }
    }

    /// `upload(gfx::UploadPass&)`. Takes `&self` (the flag is a Cell): the
    /// renderer calls it while PaintParameters borrows the static data.
    pub fn upload(&self, _upload_pass: &dyn mln_gfx::UploadPass) {
        if !self.uploaded.get() {
            // these are still used by stencil buffer rendering
            // TODO(port): tileVertexBuffer / quadTriangleIndexBuffer.
            self.uploaded.set(true);
        }
    }

    /// `tileVertices()`: `PositionOnlyLayoutAttributes` (`short2 a_pos`).
    pub fn tile_vertices() -> Vec<[i16; 2]> {
        let extent = constants::EXTENT as i16;
        vec![[0, 0], [extent, 0], [0, extent], [extent, extent]]
    }

    /// `quadTriangleIndices()`.
    pub fn quad_triangle_indices() -> Vec<u16> {
        vec![0, 1, 2, 1, 2, 3]
    }

    /// `tileTriangleSegments()`.
    pub fn tile_triangle_segments() -> Vec<SegmentBase> {
        vec![SegmentBase::with_lengths(0, 0, 4, 6)]
    }
}
