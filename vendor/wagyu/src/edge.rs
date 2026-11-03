// Ports vendor/wagyu/include/mapbox/geometry/wagyu/edge.hpp.

use crate::point::GPoint;
use crate::util::value_is_zero;

/// `edge<int32_t>`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Edge {
    pub bot: GPoint,
    pub top: GPoint,
    pub dx: f64,
}

impl Edge {
    /// `edge(current, next_pt)`.
    pub fn new(current: GPoint, next_pt: GPoint) -> Edge {
        let mut bot = current;
        let mut top = current;
        if current.y >= next_pt.y {
            top = next_pt;
        } else {
            bot = next_pt;
        }
        let dy = f64::from(top.y.wrapping_sub(bot.y));
        let dx = if value_is_zero(dy) { f64::INFINITY } else { f64::from(top.x.wrapping_sub(bot.x)) / dy };
        Edge { bot, top, dx }
    }
}

/// `slopes_equal(edge, edge)`.
pub(crate) fn slopes_equal(e1: &Edge, e2: &Edge) -> bool {
    i64::from(e1.top.y.wrapping_sub(e1.bot.y)) * i64::from(e2.top.x.wrapping_sub(e2.bot.x))
        == i64::from(e1.top.x.wrapping_sub(e1.bot.x)) * i64::from(e2.top.y.wrapping_sub(e2.bot.y))
}

/// `is_horizontal`: `std::isinf(e.dx)`.
pub(crate) fn is_horizontal(e: &Edge) -> bool {
    e.dx.is_infinite()
}

/// `get_current_x`.
pub(crate) fn get_current_x(edge: &Edge, current_y: i32) -> f64 {
    if current_y == edge.top.y {
        f64::from(edge.top.x)
    } else {
        f64::from(edge.bot.x) + edge.dx * f64::from(current_y.wrapping_sub(edge.bot.y))
    }
}
