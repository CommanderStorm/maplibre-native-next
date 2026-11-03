// Ports include/mln/gfx/rendering_stats.hpp (the counters).
//
// TODO(port): `FeatureInfo`/`NDCBound` (rendered-feature capture) and
// `RenderingStats::toString`/`toJSONString`/`eval` (src/mln/gfx/rendering_stats.cpp)
// arrive with their first user.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderingStats {
    pub encoding_time: f64,
    pub rendering_time: f64,
    pub num_frames: i32,
    pub num_draw_calls: i32,
    pub total_draw_calls: i32,
    pub num_created_textures: i32,
    pub num_active_textures: i32,
    pub num_texture_bindings: i32,
    pub num_texture_updates: i32,
    pub texture_update_bytes: usize,
    pub total_buffers: usize,
    pub total_buffer_objs: usize,
    pub buffer_updates: usize,
    pub buffer_obj_updates: usize,
    pub buffer_update_bytes: usize,
    pub num_buffers: i32,
    pub num_frame_buffers: i32,
    pub num_index_buffers: i32,
    pub index_update_bytes: usize,
    pub num_vertex_buffers: i32,
    pub vertex_update_bytes: usize,
    pub num_uniform_buffers: i32,
    pub num_uniform_updates: i32,
    pub uniform_update_bytes: usize,
    pub mem_textures: i32,
    pub mem_buffers: i32,
    pub mem_index_buffers: i32,
    pub mem_vertex_buffers: i32,
    pub mem_uniform_buffers: i32,
    pub stencil_clears: i32,
    pub stencil_updates: i32,
}

impl RenderingStats {
    pub fn is_zero(&self) -> bool {
        *self == RenderingStats::default()
    }
}
