// Ports src/mln/renderer/buckets/circle_bucket.hpp and circle_bucket.cpp
//
// The vertex/index vectors are plain `Vec`s here (the bucket is built on the
// worker thread and shared with `Arc`); the render layer copies them into
// the shared gfx vectors its drawables reference.

use crate::bucket::{Bucket, BucketBase};
use crate::paint_property_binder::PaintPropertyBinder;
use crate::render_layer::RenderLayer;
use mln_gfx::shaders::segment::SegmentBase;
use mln_gfx::UploadPass;
use mln_map_types::MapMode;
use mln_style::layer_properties::{get_evaluated, LayerProperties};
use mln_style::layers::circle_layer_properties::*;
use mln_style::properties::Property;
use mln_util::immutable::Immutable;
use mln_util::Color;
use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

crate::paint_property_binders!(
    /// `CircleBinders = PaintPropertyBinders<CirclePaintProperties::DataDrivenProperties>`.
    pub struct CircleBinders for CirclePaintProperties::PossiblyEvaluated {
        circle_blur: CircleBlur => PaintPropertyBinder<f32>,
        circle_color: CircleColor => PaintPropertyBinder<Color>,
        circle_opacity: CircleOpacity => PaintPropertyBinder<f32>,
        circle_radius: CircleRadius => PaintPropertyBinder<f32>,
        circle_stroke_color: CircleStrokeColor => PaintPropertyBinder<Color>,
        circle_stroke_opacity: CircleStrokeOpacity => PaintPropertyBinder<f32>,
        circle_stroke_width: CircleStrokeWidth => PaintPropertyBinder<f32>,
    }
);

/// `CircleLayoutVertex = gfx::Vertex<TypeList<attributes::pos>>`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
#[repr(C)]
pub struct CircleLayoutVertex {
    pub a1: [i16; 2],
}

pub struct CircleBucket {
    pub base: BucketBase,
    pub vertices: Vec<CircleLayoutVertex>,
    /// `gfx::IndexVector<gfx::Triangles>`.
    pub triangles: Vec<u16>,
    pub segments: Vec<SegmentBase>,
    pub paint_property_binders: BTreeMap<String, CircleBinders>,
    pub mode: MapMode,
}

impl CircleBucket {
    /// `CircleBucket(layerPaintProperties, mode, zoom)`.
    pub fn new(
        layer_paint_properties: &BTreeMap<String, Immutable<dyn LayerProperties>>,
        mode: MapMode,
        zoom: f32,
    ) -> Self {
        let mut paint_property_binders = BTreeMap::new();
        for (id, properties) in layer_paint_properties {
            paint_property_binders
                .entry(id.clone())
                .or_insert_with(|| CircleBinders::new(get_evaluated::<CircleLayerProperties>(properties), zoom));
        }
        CircleBucket {
            base: BucketBase::default(),
            vertices: Vec::new(),
            triangles: Vec::new(),
            segments: Vec::new(),
            paint_property_binders,
            mode,
        }
    }

    /// `vertex(Point<int16_t> p, float ex, float ey)`: `p.x * 2` is an
    /// `int`, plus a `float`, converted to `int16_t` (truncation).
    pub fn vertex(p: mln_tile_id::geometry_tile_data::GeometryCoordinate, ex: f32, ey: f32) -> CircleLayoutVertex {
        CircleLayoutVertex {
            a1: [
                ((p.x as i32 * 2) as f32 + ((ex + 1.0) / 2.0)) as i32 as i16,
                ((p.y as i32 * 2) as f32 + ((ey + 1.0) / 2.0)) as i32 as i16,
            ],
        }
    }
}

/// The anonymous-namespace `get<Property>(evaluated, id, binders)` of
/// circle_bucket.cpp.
fn get_radius_or_width(
    evaluated_constant: Option<f32>,
    default_value: f32,
    statistics_max: Option<Option<f32>>,
) -> f32 {
    match statistics_max {
        Some(Some(max)) => max,
        _ => evaluated_constant.unwrap_or(default_value),
    }
}

impl Bucket for CircleBucket {
    fn base(&self) -> &BucketBase {
        &self.base
    }
    fn base_mut(&mut self) -> &mut BucketBase {
        &mut self.base
    }

    fn upload(&self, _upload_pass: &dyn UploadPass) {
        self.base.uploaded.store(true, Ordering::SeqCst);
    }

    fn has_data(&self) -> bool {
        !self.segments.is_empty()
    }

    fn get_query_radius(&self, layer: &dyn RenderLayer) -> f32 {
        let evaluated = get_evaluated::<CircleLayerProperties>(&layer.base().evaluated_properties);
        let binders = self.paint_property_binders.get(layer.get_id());
        let radius = get_radius_or_width(
            evaluated.get::<CircleRadius>().constant(),
            CircleRadius::default_value(),
            binders.map(|b| b.circle_radius.statistics.max()),
        );
        let stroke = get_radius_or_width(
            evaluated.get::<CircleStrokeWidth>().constant(),
            CircleStrokeWidth::default_value(),
            binders.map(|b| b.circle_stroke_width.statistics.max()),
        );
        let translate = evaluated.get::<CircleTranslate>();
        // util::length(a, b) = std::sqrt(a * a + b * b), in float
        radius + stroke + (translate[0] * translate[0] + translate[1] * translate[1]).sqrt()
    }

    // TODO(port): update(FeatureStates, ...) (feature state; the bucket is
    // shared immutably once built).
}
