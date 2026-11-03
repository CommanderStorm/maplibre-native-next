// Ports include/mln/gfx/drawable_builder.hpp, src/mln/gfx/drawable_builder.cpp,
// src/mln/gfx/drawable_builder_impl.hpp and the non-polyline parts of
// src/mln/gfx/drawable_builder_impl.cpp
//
// Same split as `Drawable`: `DrawableBuilderBase` holds the C++ data members
// (including the private `Impl`), the trait has the virtuals
// (`create_drawable`, `create_segment`, `init`) and provided methods for the
// rest.
//
// TODO(port): addPolyline / addWideVectorPolylineLocal/Global and the
// polyline/wide-vector setup in flush (Impl::Mode::Polylines/WideVector*),
// with gfx/polyline_generator, arrive with line layers (M3).

use crate::color_mode::ColorMode;
use crate::context::Context;
use crate::cull_face_mode::CullFaceMode;
use crate::draw_mode::DrawMode;
use crate::drawable::{DrawPriority, Textures, UniqueDrawSegment, UniqueDrawable};
use crate::drawable_tweaker::DrawableTweakerPtr;
use crate::gfx_types::{AttributeDataType, DrawModeType};
use crate::index_vector::{IndexVectorBase, IndexVectorBasePtr};
use crate::renderer::render_pass::RenderPass;
use crate::shaders::segment::SegmentBase;
use crate::shaders::shader_program_base::ShaderProgramBasePtr;
use crate::texture2d::Texture2DPtr;
use crate::types::DepthMaskType;
use crate::vertex_attribute::VertexAttributeArrayPtr;
use crate::vertex_vector::VertexVector;
use maplibre_native_base::Point;
use mln_util::identity::SimpleIdentity;
use std::cell::RefCell;
use std::rc::Rc;

/// `DrawableBuilder::Impl::Mode`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// building primitive drawables. Not implemented
    Primitives,
    /// building drawables for thick polylines
    Polylines,
    /// building drawables for thick polylines using wide vectors in local coordinates
    WideVectorLocal,
    /// building drawables for thick polylines using wide vectors in global coordinates
    WideVectorGlobal,
    /// building custom drawables.
    Custom,
}

/// `DrawableBuilder::Impl::VT = gfx::detail::VertexType<AttributeType<int16_t, 2>>`
pub type VT = [i16; 2];

/// `DrawableBuilder::Impl`
pub struct DrawableBuilderImpl {
    pub vertices: VertexVector<VT>,
    pub raw_vertices: Vec<u8>,
    pub raw_vertices_count: usize,
    pub build_indexes: Vec<u16>,
    pub shared_indexes: Option<IndexVectorBasePtr>,
    pub segments: Vec<UniqueDrawSegment>,
    /// C++ initializes this to `static_cast<AttributeDataType>(-1)` = 255 =
    /// `Invalid`.
    pub raw_vertices_type: AttributeDataType,
    pub color_mode: ColorMode,
    pub cull_face_mode: CullFaceMode,
    mode: Mode,
    // TODO(port): polylineVertices, polylineIndexes, wideVectorVertices,
    // wideVectorInstanceData, wideVectorIndexes.
}

impl Default for DrawableBuilderImpl {
    fn default() -> Self {
        DrawableBuilderImpl {
            vertices: VertexVector::new(),
            raw_vertices: Vec::new(),
            raw_vertices_count: 0,
            build_indexes: Vec::new(),
            shared_indexes: None,
            segments: Vec::new(),
            raw_vertices_type: AttributeDataType::Invalid,
            color_mode: ColorMode::disabled(),
            cull_face_mode: CullFaceMode::disabled(),
            mode: Mode::Custom,
        }
    }
}

impl DrawableBuilderImpl {
    pub fn check_and_set_mode(&mut self, target: Mode) -> bool {
        if target != self.mode && self.vertex_count() != 0 {
            // C++ logs "DrawableBuilder mode mismatch. Target is <target>,
            // current is <mode>" and asserts.
            // TODO(port): mln::Log once a logging port exists.
            debug_assert!(false, "DrawableBuilder mode mismatch. Target is {target:?}, current is {:?}", self.mode);
            return false;
        }
        self.mode = target;
        true
    }

    pub fn get_mode(&self) -> Mode {
        self.mode
    }

    /// C++ `bool setMode(Mode value) { return mode == value; }` - a quirk: it
    /// does not set anything. Ported as is.
    pub fn set_mode(&self, value: Mode) -> bool {
        self.mode == value
    }

    pub fn vertex_count(&self) -> usize {
        self.raw_vertices_count.max(self.vertices.elements())
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.raw_vertices.clear();
        self.raw_vertices_count = 0;
        self.build_indexes.clear();
        self.segments.clear();
    }
}

/// The data members of `gfx::DrawableBuilder`.
pub struct DrawableBuilderBase {
    pub name: String,
    pub drawable_name: String,
    pub vertex_attr_id: usize,
    pub render_pass: RenderPass,
    pub enabled: bool,
    pub enable_color: bool,
    pub enable_stencil: bool,
    pub enable_depth: bool,
    pub is_3d: bool,
    pub line_width: f32,
    pub draw_priority: DrawPriority,
    pub sub_layer_index: i32,
    pub depth_type: DepthMaskType,
    pub shader: Option<ShaderProgramBasePtr>,
    pub current_drawable: Option<UniqueDrawable>,
    pub drawables: Vec<UniqueDrawable>,
    pub textures: Textures,
    pub tweakers: Vec<DrawableTweakerPtr>,
    pub vertex_attrs: Option<VertexAttributeArrayPtr>,
    pub instance_attrs: Option<VertexAttributeArrayPtr>,
    pub origin: Option<Point<f64>>,
    pub impl_: DrawableBuilderImpl,
}

impl DrawableBuilderBase {
    pub fn new(name: String) -> Self {
        DrawableBuilderBase {
            name,
            drawable_name: String::new(),
            vertex_attr_id: 0,
            render_pass: RenderPass::Opaque,
            enabled: true,
            enable_color: true,
            enable_stencil: false,
            enable_depth: true,
            is_3d: false,
            line_width: 1.0,
            draw_priority: 0,
            sub_layer_index: 0,
            depth_type: DepthMaskType::ReadOnly,
            shader: None,
            current_drawable: None,
            drawables: Vec::new(),
            textures: Default::default(),
            tweakers: Vec::new(),
            vertex_attrs: None,
            instance_attrs: None,
            origin: None,
            impl_: DrawableBuilderImpl::default(),
        }
    }
}

pub trait DrawableBuilder {
    fn base(&self) -> &DrawableBuilderBase;
    fn base_mut(&mut self) -> &mut DrawableBuilderBase;

    /// Create an instance of the appropriate drawable type
    fn create_drawable(&self) -> UniqueDrawable;
    /// Create a segment wrapper
    fn create_segment(&self, mode: DrawMode, segment: SegmentBase) -> UniqueDrawSegment;
    /// Setup the SDK-specific aspects after all the values are present
    fn init(&mut self);

    /// Get the drawable we're currently working on, if any
    fn get_current_drawable(&mut self, create_if_none: bool) -> Option<&mut UniqueDrawable> {
        if self.base().current_drawable.is_none() && create_if_none {
            let drawable = self.create_drawable();
            self.base_mut().current_drawable = Some(drawable);
        }
        self.base_mut().current_drawable.as_mut()
    }

    /// Close the current drawable, using a new one for any further work
    fn flush(&mut self, context: &dyn Context) {
        if self.cur_vertex_count() != 0 {
            match self.base().impl_.get_mode() {
                Mode::Polylines | Mode::WideVectorLocal | Mode::WideVectorGlobal => {
                    // TODO(port): impl->setupForPolylines / setupForWideVectors
                }
                _ => {}
            }

            let new_vertex_attributes = if self.base().vertex_attrs.is_none() {
                Some(context.create_vertex_attribute_array())
            } else {
                None
            };
            self.get_current_drawable(true);
            let base = self.base_mut();
            let draw = base.current_drawable.as_mut().expect("current drawable");
            draw.set_enabled(base.enabled);
            draw.set_line_width(base.line_width as i32);
            draw.set_enable_color(base.enable_color);
            draw.set_enable_stencil(base.enable_stencil);
            draw.set_render_pass(base.render_pass);
            draw.set_draw_priority(base.draw_priority);
            draw.set_sub_layer_index(base.sub_layer_index);
            draw.set_enable_depth(base.enable_depth);
            draw.set_depth_type(base.depth_type);
            draw.set_is_3d(base.is_3d);
            draw.set_color_mode(&base.impl_.color_mode);
            draw.set_cull_face_mode(&base.impl_.cull_face_mode);
            draw.set_shader(base.shader.clone());
            draw.set_textures(base.textures.clone());
            draw.set_tweakers(base.tweakers.clone());
            draw.set_origin(base.origin);
            if let Some(attrs) = &base.vertex_attrs {
                draw.set_vertex_attributes(Some(attrs.clone()));
            } else {
                draw.set_vertex_attributes(new_vertex_attributes);
            }
            if let Some(attrs) = &base.instance_attrs {
                draw.set_instance_attributes(Some(attrs.clone()));
            }

            self.init();

            let base = self.base_mut();
            // reset mode (quirk: C++ `setMode` only compares, see DrawableBuilderImpl::set_mode)
            base.impl_.set_mode(Mode::Custom);
            // reset origin
            base.origin = None;
        }

        let base = self.base_mut();
        if let Some(drawable) = base.current_drawable.take() {
            base.drawables.push(drawable);
        }
    }

    /// Get all the completed drawables
    fn get_drawables(&self) -> &Vec<UniqueDrawable> {
        &self.base().drawables
    }

    /// Get all the completed drawables and release ownership
    fn clear_drawables(&mut self) -> Vec<UniqueDrawable> {
        std::mem::take(&mut self.base_mut().drawables)
    }

    /// Get the ID of the drawable we're currently working on, if any
    fn get_drawable_id(&self) -> SimpleIdentity {
        self.base().current_drawable.as_ref().map_or(SimpleIdentity::EMPTY, |d| d.get_id())
    }

    fn get_enabled(&self) -> bool {
        self.base().enabled
    }
    fn set_enabled(&mut self, value: bool) {
        self.base_mut().enabled = value;
    }
    fn get_render_pass(&self) -> RenderPass {
        self.base().render_pass
    }
    fn set_render_pass(&mut self, value: RenderPass) {
        self.base_mut().render_pass = value;
    }
    fn get_draw_priority(&self) -> DrawPriority {
        self.base().draw_priority
    }
    fn set_draw_priority(&mut self, value: DrawPriority) {
        let base = self.base_mut();
        base.draw_priority = value;
        if let Some(d) = base.current_drawable.as_mut() {
            d.set_draw_priority(value);
        }
    }
    fn get_sub_layer_index(&self) -> i32 {
        self.base().sub_layer_index
    }
    fn set_sub_layer_index(&mut self, value: i32) {
        self.base_mut().sub_layer_index = value;
    }
    fn get_enable_depth(&self) -> bool {
        self.base().enable_depth
    }
    fn set_enable_depth(&mut self, value: bool) {
        self.base_mut().enable_depth = value;
    }
    fn get_depth_type(&self) -> DepthMaskType {
        self.base().depth_type
    }
    fn set_depth_type(&mut self, value: DepthMaskType) {
        self.base_mut().depth_type = value;
    }
    fn get_is_3d(&self) -> bool {
        self.base().is_3d
    }
    fn set_is_3d(&mut self, value: bool) {
        self.base_mut().is_3d = value;
    }
    /// Set the draw priority on all drawables including those already generated
    fn reset_draw_priority(&mut self, value: DrawPriority) {
        self.set_draw_priority(value);
        for drawable in &mut self.base_mut().drawables {
            drawable.set_draw_priority(value);
        }
    }
    fn get_line_width(&self) -> f32 {
        self.base().line_width
    }
    fn set_line_width(&mut self, value: f32) {
        self.base_mut().line_width = value;
    }
    fn get_enable_color(&self) -> bool {
        self.base().enable_color
    }
    fn set_enable_color(&mut self, value: bool) {
        self.base_mut().enable_color = value;
    }
    fn get_enable_stencil(&self) -> bool {
        self.base().enable_stencil
    }
    fn set_enable_stencil(&mut self, value: bool) {
        self.base_mut().enable_stencil = value;
    }
    fn get_color_mode(&self) -> &ColorMode {
        &self.base().impl_.color_mode
    }
    fn set_color_mode(&mut self, value: &ColorMode) {
        self.base_mut().impl_.color_mode = *value;
    }
    fn get_cull_face_mode(&self) -> &CullFaceMode {
        &self.base().impl_.cull_face_mode
    }
    fn set_cull_face_mode(&mut self, value: &CullFaceMode) {
        self.base_mut().impl_.cull_face_mode = *value;
    }
    fn get_shader(&self) -> &Option<ShaderProgramBasePtr> {
        &self.base().shader
    }
    fn set_shader(&mut self, value: Option<ShaderProgramBasePtr>) {
        self.base_mut().shader = value;
    }
    /// Set the name given to new drawables
    fn set_drawable_name(&mut self, value: String) {
        self.base_mut().drawable_name = value;
    }
    /// The attribute id for vertex/position attribute
    fn set_vertex_attr_id(&mut self, id: usize) {
        self.base_mut().vertex_attr_id = id;
    }
    fn get_texture(&self, id: usize) -> Option<&Texture2DPtr> {
        self.base().textures.get(id).and_then(Option::as_ref)
    }
    fn set_texture(&mut self, texture: Option<Texture2DPtr>, id: usize) {
        debug_assert!(id < self.base().textures.len());
        if id >= self.base().textures.len() {
            return;
        }
        self.base_mut().textures[id] = texture;
    }
    fn add_tweaker(&mut self, value: DrawableTweakerPtr) {
        self.base_mut().tweakers.push(value);
    }
    fn clear_tweakers(&mut self) {
        self.base_mut().tweakers.clear();
    }
    fn get_vertex_attributes(&self) -> Option<&VertexAttributeArrayPtr> {
        self.base().vertex_attrs.as_ref()
    }
    fn set_vertex_attributes(&mut self, attrs: Option<VertexAttributeArrayPtr>) {
        self.base_mut().vertex_attrs = attrs;
    }
    fn get_instance_attributes(&self) -> Option<&VertexAttributeArrayPtr> {
        self.base().instance_attrs.as_ref()
    }
    fn set_instance_attributes(&mut self, attrs: Option<VertexAttributeArrayPtr>) {
        self.base_mut().instance_attrs = attrs;
    }

    /// Add some vertex elements, returns the index of the first one added
    fn add_vertices(&mut self, vertices: &[[i16; 2]], vertex_offset: usize, vertex_length: usize) -> usize {
        let impl_ = &mut self.base_mut().impl_;
        let base_index = impl_.vertices.elements();
        for x in &vertices[vertex_offset..vertex_offset + vertex_length] {
            impl_.vertices.emplace_back(*x);
        }
        base_index
    }

    /// Provide raw data for vertices. Incompatible with adding primitives
    fn set_raw_vertices(&mut self, data: Vec<u8>, count: usize, type_: AttributeDataType) {
        let impl_ = &mut self.base_mut().impl_;
        impl_.raw_vertices = data;
        impl_.raw_vertices_count = count;
        impl_.raw_vertices_type = type_;
    }

    /// Set indexes and segments
    fn set_segments(&mut self, mode: DrawMode, indexes: Vec<u16>, segments: &[SegmentBase]) {
        self.set_segments_shared(mode, Some(Rc::new(RefCell::new(IndexVectorBase::new(indexes)))), segments);
    }

    /// Set shared indices and segments
    fn set_segments_shared(&mut self, mode: DrawMode, indexes: Option<IndexVectorBasePtr>, segments: &[SegmentBase]) {
        let Some(indexes) = indexes.filter(|i| i.borrow().elements() != 0) else {
            debug_assert!(false, "Missing segment indexes");
            return;
        };
        #[cfg(debug_assertions)]
        let vertex_count = self.cur_vertex_count();
        self.base_mut().impl_.shared_indexes = Some(indexes);
        for seg in segments {
            #[cfg(debug_assertions)]
            {
                if mode.type_ == DrawModeType::Triangles {
                    debug_assert!(seg.index_length % 3 == 0);
                } else if mode.type_ == DrawModeType::Lines {
                    debug_assert!(seg.index_length % 2 == 0);
                }
                let shared = self.base().impl_.shared_indexes.as_ref().unwrap().borrow();
                debug_assert!(vertex_count == 0 || seg.vertex_offset + seg.vertex_length <= vertex_count);
                debug_assert!(seg.index_offset + seg.index_length <= shared.elements());
                if vertex_count != 0 {
                    for j in 0..seg.index_length {
                        debug_assert!((shared.vector()[seg.index_offset + j] as usize) < vertex_count);
                    }
                }
            }
            let draw_segment = self.create_segment(mode, seg.copy());
            self.base_mut().impl_.segments.push(draw_segment);
        }
    }

    /// Add a triangle
    fn add_triangle(&mut self, x0: i16, y0: i16, x1: i16, y1: i16, x2: i16, y2: i16) {
        // check and set the current mode
        if !self.base_mut().impl_.check_and_set_mode(Mode::Primitives) {
            return;
        }
        let needs_segment = self.base().impl_.segments.is_empty();
        let new_segment =
            if needs_segment { Some(self.create_segment(DrawMode::Triangles(), SegmentBase::new(0, 0))) } else { None };
        let impl_ = &mut self.base_mut().impl_;
        let n = impl_.vertices.elements() as u16;
        impl_.vertices.emplace_back([x0, y0]);
        impl_.vertices.emplace_back([x1, y1]);
        impl_.vertices.emplace_back([x2, y2]);
        impl_.build_indexes.extend_from_slice(&[n, n.wrapping_add(1), n.wrapping_add(2)]);
        if let Some(segment) = new_segment {
            impl_.segments.push(segment);
        }
        let segment = impl_.segments.last_mut().unwrap().get_segment_mut();
        segment.vertex_length += 3;
        segment.index_length += 3;
    }

    /// Add another triangle based on the previous two points
    fn append_triangle(&mut self, x0: i16, y0: i16) {
        // check and set the current mode
        if !self.base_mut().impl_.check_and_set_mode(Mode::Primitives) {
            return;
        }
        let impl_ = &mut self.base_mut().impl_;
        let n = impl_.vertices.elements() as u16;
        impl_.vertices.emplace_back([x0, y0]);
        impl_.build_indexes.extend_from_slice(&[n.wrapping_sub(2), n.wrapping_sub(1), n]);
        debug_assert!(!impl_.segments.is_empty());
        let segment = impl_.segments.last_mut().unwrap().get_segment_mut();
        segment.vertex_length += 1;
        segment.index_length += 3;
    }

    /// Add a rectangle consisting of two triangles
    fn add_quad(&mut self, x0: i16, y0: i16, x1: i16, y1: i16) {
        // check and set the current mode
        if !self.base_mut().impl_.check_and_set_mode(Mode::Primitives) {
            return;
        }
        self.add_triangle(x0, y0, x1, y0, x0, y1);
        self.append_triangle(x1, y1);
    }

    /// Add lines based on existing vertices
    fn add_lines(&mut self, indexes: &[u16], index_offset: usize, index_length: usize, base_index: usize) {
        if index_length == 0 {
            return;
        }
        let line_width = self.base().line_width;
        let segment =
            self.create_segment(DrawMode::Lines(line_width), SegmentBase::with_lengths(0, indexes.len(), 0, index_length));
        let impl_ = &mut self.base_mut().impl_;
        impl_.segments.push(segment);
        if impl_.build_indexes.is_empty() {
            impl_.build_indexes.reserve(index_length);
        }
        for i in &indexes[index_offset..index_offset + index_length] {
            impl_.build_indexes.push((*i as usize + base_index) as u16);
        }
    }

    /// Add triangles based on existing vertices
    fn add_triangles(&mut self, indexes: &[u16], index_offset: usize, index_length: usize, base_index: usize) {
        if index_length == 0 {
            return;
        }
        let (build_len, vertex_len) = {
            let impl_ = &self.base().impl_;
            (impl_.build_indexes.len(), impl_.vertices.elements())
        };
        let segment =
            self.create_segment(DrawMode::Triangles(), SegmentBase::with_lengths(0, build_len, vertex_len, index_length));
        let impl_ = &mut self.base_mut().impl_;
        impl_.segments.push(segment);
        if impl_.build_indexes.is_empty() {
            impl_.build_indexes.reserve(index_length);
        }
        for i in &indexes[index_offset..index_offset + index_length] {
            impl_.build_indexes.push((*i as usize + base_index) as u16);
        }
    }

    /// return the current vertex count
    fn cur_vertex_count(&self) -> usize {
        self.base().impl_.vertex_count()
    }

    fn empty(&self) -> bool {
        self.base().drawables.is_empty() && self.cur_vertex_count() == 0
    }
}

pub type UniqueDrawableBuilder = Box<dyn DrawableBuilder>;
