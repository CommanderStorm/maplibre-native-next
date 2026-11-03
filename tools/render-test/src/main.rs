// Ports platform/default/src/mln/render-test/main.cpp.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    std::process::exit(render_test::run_render_tests(&args, factory()));
}

#[cfg(feature = "map")]
fn factory() -> Option<render_test::runner::ImplFactory> {
    Some(render_test_map::factory())
}

#[cfg(not(feature = "map"))]
fn factory() -> Option<render_test::runner::ImplFactory> {
    None
}
