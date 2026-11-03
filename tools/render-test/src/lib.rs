//! Ports `render-test/` (the C++ render-test runner) for the Rust core.
//!
//! The runner core here does not depend on the map crates: the map is
//! reached through [`runner::TestRunnerImpl`], implemented over the real map
//! in `//tools/render-test:render_test_map`, and `--self-check` runs the full
//! comparison and reporting path with each test's expected image as the
//! rendered one.

pub mod file_source;
pub mod filesystem;
pub mod log;
pub mod manifest_parser;
pub mod metadata;
pub mod parser;
pub mod render_test;
pub mod runner;
pub mod std_random;
pub mod test_runner_common;
pub mod watchdog;

pub use render_test::run_render_tests;

#[cfg(test)]
mod tests;
