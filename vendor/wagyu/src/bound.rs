// Ports vendor/wagyu/include/mapbox/geometry/wagyu/bound.hpp. Bounds live in
// an arena (`Vec<Bound>`) and `bound_ptr` is an index (NIL = nullptr);
// `current_edge`/`next_edge` iterators are indices into `edges` (len =
// end()).

use crate::config::PolygonType;
use crate::edge::Edge;
use crate::point::{GPoint, NIL, RingPtr};

pub(crate) type BoundPtr = usize;

/// `enum edge_side`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EdgeSide {
    Left = 0,
    Right,
}

/// `bound<int32_t>`.
#[derive(Clone, Debug)]
pub(crate) struct Bound {
    pub edges: Vec<Edge>,
    pub current_edge: usize,
    pub next_edge: usize,
    pub last_point: GPoint,
    pub ring: RingPtr,
    /// the bound who's maximum connects with this bound
    pub maximum_bound: BoundPtr,
    pub current_x: f64,
    pub pos: usize,
    pub winding_count: i32,
    /// winding count of the opposite polytype
    pub winding_count2: i32,
    /// 1 or -1 depending on winding direction - 0 for linestrings
    pub winding_delta: i8,
    pub poly_type: PolygonType,
    /// side only refers to current side of solution poly
    pub side: EdgeSide,
}

impl Bound {
    pub fn new() -> Bound {
        Bound {
            edges: Vec::new(),
            current_edge: 0,
            next_edge: 0,
            last_point: GPoint::new(0, 0),
            ring: NIL,
            maximum_bound: NIL,
            current_x: 0.0,
            pos: 0,
            winding_count: 0,
            winding_count2: 0,
            winding_delta: 0,
            poly_type: PolygonType::Subject,
            side: EdgeSide::Left,
        }
    }

    pub fn cur(&self) -> &Edge {
        &self.edges[self.current_edge]
    }

    pub fn next(&self) -> &Edge {
        &self.edges[self.next_edge]
    }
}
