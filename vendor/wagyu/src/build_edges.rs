// Ports vendor/wagyu/include/mapbox/geometry/wagyu/build_edges.hpp (T1 = T2 =
// int32_t). The reverse iterator `itr_rev` is kept as its base() index `rb`
// (it dereferences path[rb - 1]; rend() is rb == 0).

use crate::edge::{Edge, slopes_equal};
use crate::point::GPoint;
use crate::util::gp_slopes_equal3;

/// `point_2_is_between_point_1_and_point_3`.
fn point_2_is_between_point_1_and_point_3(pt1: GPoint, pt2: GPoint, pt3: GPoint) -> bool {
    if (pt1 == pt3) || (pt1 == pt2) || (pt3 == pt2) {
        false
    } else if pt1.x != pt3.x {
        (pt2.x > pt1.x) == (pt2.x < pt3.x)
    } else {
        (pt2.y > pt1.y) == (pt2.y < pt3.y)
    }
}

/// `build_edge_list`.
pub(crate) fn build_edge_list(path_geometry: &[GPoint], edges: &mut Vec<Edge>) -> bool {
    if path_geometry.len() < 3 {
        return false;
    }

    // As this is a loop, we need to first go backwards from end to try and find
    // the proper starting point for the iterators before the beginning
    let mut rb = path_geometry.len();
    let mut itr = 0usize;
    let mut pt1 = path_geometry[rb - 1];
    let mut pt2 = path_geometry[itr];

    // Find next non repeated point going backwards from
    // end for pt1
    while pt1 == pt2 {
        rb -= 1;
        if rb == 0 {
            return false;
        }
        pt1 = path_geometry[rb - 1];
    }
    itr += 1;
    let mut pt3 = path_geometry[itr];
    let mut itr_last = rb;
    let mut front_pt = GPoint::new(0, 0);
    let mut back_pt = GPoint::new(0, 0);
    loop {
        if pt3 == pt2 {
            // Duplicate point advance itr, but do not
            // advance other points
            if itr == itr_last {
                break;
            }
            itr += 1;
            if itr == itr_last {
                if edges.is_empty() {
                    break;
                }
                pt3 = front_pt;
            } else {
                pt3 = path_geometry[itr];
            }
            continue;
        }

        // Now check if slopes are equal between two segments - either
        // a spike or a collinear point - if so drop point number 2.
        if gp_slopes_equal3(pt1, pt2, pt3) {
            // We need to reconsider previously added points
            // because the point it was using was found to be collinear
            // or a spike
            pt2 = pt1;
            if !edges.is_empty() {
                edges.pop(); // remove previous edge (pt1)
            }
            if let Some(back) = edges.last() {
                let back_top = back.top;
                if back_pt.x == back_top.x && back_pt.y == back_top.y {
                    pt1 = back.bot;
                } else {
                    pt1 = back_top;
                }
                back_pt = pt1;
            } else {
                // If this occurs we must look to the back of the
                // ring for new points.
                while path_geometry[rb - 1] == pt2 {
                    rb -= 1;
                    if itr + 1 == rb {
                        return false;
                    }
                }
                pt1 = path_geometry[rb - 1];
                itr_last = rb;
            }
            continue;
        }

        if edges.is_empty() {
            front_pt = pt2;
        }
        edges.push(Edge::new(pt2, pt3));
        back_pt = pt2;
        if itr == itr_last {
            break;
        }
        pt1 = pt2;
        pt2 = pt3;
        itr += 1;
        if itr == itr_last {
            if edges.is_empty() {
                break;
            }
            pt3 = front_pt;
        } else {
            pt3 = path_geometry[itr];
        }
    }

    let mut modified;
    loop {
        modified = false;
        if edges.len() < 3 {
            return false;
        }
        let last = edges.len() - 1;
        let f = edges[0];
        let b = edges[last];
        if slopes_equal(&f, &b) {
            if f.bot == b.top {
                if f.top == b.bot {
                    edges.pop();
                    edges.remove(0);
                } else {
                    edges[0].bot = b.bot;
                    edges.pop();
                }
                modified = true;
            } else if f.top == b.bot {
                edges[0].top = b.top;
                edges.pop();
                modified = true;
            } else if f.top == b.top && f.bot == b.bot {
                edges.pop();
                edges.remove(0);
                modified = true;
            } else if f.top == b.top {
                if point_2_is_between_point_1_and_point_3(f.top, f.bot, b.bot) {
                    edges[last].top = f.bot;
                    edges.remove(0);
                } else {
                    edges[0].top = b.bot;
                    edges.pop();
                }
                modified = true;
            } else if f.bot == b.bot {
                if point_2_is_between_point_1_and_point_3(f.bot, f.top, b.top) {
                    edges[last].bot = f.top;
                    edges.remove(0);
                } else {
                    edges[0].bot = b.top;
                    edges.pop();
                }
                modified = true;
            }
        }
        if !modified {
            break;
        }
    }

    true
}
