//! Ports the parts of GCC's libstdc++ (the C++ standard library the C++ core
//! is built with on Linux: GCC 16, `bits/stl_algo.h`, `bits/stl_heap.h`,
//! `bits/stl_queue.h`) whose results the C++ standard leaves unspecified,
//! or whose results depend on the algorithm when the comparator is not a
//! strict weak ordering:
//!
//! - [`sort`] / [`sort_by`]: `std::sort`, libstdc++'s introsort. Not stable:
//!   the order of elements with equal keys is whatever introsort produces.
//! - [`stable_sort_by`]: `std::stable_sort` (adaptive merge sort with a
//!   temporary buffer). For a strict weak ordering every stable sort gives
//!   the same result, but wagyu sorts with ULP-based "almost equal"
//!   comparators that are not transitive, so the merge order matters.
//! - [`make_heap`], [`push_heap`], [`pop_heap`], [`sort_heap`] and
//!   [`PriorityQueue`] (`std::priority_queue`): which of several equal
//!   elements is on top depends on the heap algorithm.
//!
//! - [`cvt`]: `static_cast<Int>(double)` as clang emits it on x86-64 (the
//!   libraries convert projected doubles to tile integers this way, and
//!   out-of-range values differ from Rust's saturating `as`).
//!
//! Comparators are C++ "less" predicates (`comp(a, b)` = `a < b`). Iterators
//! are indices into the slice. Where libstdc++'s unguarded loops would run
//! off the range (only possible when `comp` is not a strict weak ordering in
//! a way that makes even the C++ read out of bounds, which is UB), indexing
//! panics here.

pub mod cvt;
mod heap;
mod sort;
mod stable_sort;

pub use heap::{PriorityQueue, make_heap, pop_heap, push_heap, sort_heap};
pub use sort::{sort, sort_by};
pub use stable_sort::stable_sort_by;

#[cfg(test)]
mod tests;
