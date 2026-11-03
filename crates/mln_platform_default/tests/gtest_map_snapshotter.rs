// Ports test/map/map_snapshotter.test.cpp (a module of the gtest_map test
// crate). `MapSnapshotter` (platform/default/src/mln/map/map_snapshotter.cpp)
// is not ported, so the bodies can't be written yet; each test names what
// it checks. Four of the six also need the HTTP test server.

// gtest: MapSnapshotter.setStyleJSON
#[test]
#[ignore = "TODO(port): MapSnapshotter (platform/default map_snapshotter.cpp); see map_snapshotter.test.cpp:33"]
fn set_style_json() {
    // A 32x16 snapshotter with a green background style snapshots without
    // error into a 32x16 image.
}

// gtest: MapSnapshotter.setSize
#[test]
#[ignore = "TODO(port): MapSnapshotter (platform/default map_snapshotter.cpp); see map_snapshotter.test.cpp:59"]
fn set_size() {
    // After setSize({16, 32}) the snapshot is 16x32.
}

// gtest: MapSnapshotter.setStyleURL
#[test]
#[ignore = "TEST_REQUIRES_SERVER; also TODO(port): MapSnapshotter; see map_snapshotter.test.cpp:87"]
fn set_style_url() {
    // http://127.0.0.1:3000/online/style.json snapshots into a 64x32 image.
}

// gtest: MapSnapshotter.cancel
#[test]
#[ignore = "TEST_REQUIRES_SERVER; also TODO(port): MapSnapshotter; see map_snapshotter.test.cpp:105"]
fn cancel() {
    // A cancelled snapshot never calls its callback.
}

// gtest: MapSnapshotter.runtimeStyling
#[test]
#[ignore = "TEST_REQUIRES_SERVER; also TODO(port): MapSnapshotter and MapSnapshotterObserver; see map_snapshotter.test.cpp:123"]
fn runtime_styling() {
    // A fill layer added in onDidFinishLoadingStyle shows in the snapshot
    // (test/fixtures/map/map_snapshotter_overlay, 0.005, 0.1).
}

// gtest: MapSnapshotter.annotation
#[test]
#[ignore = "TEST_REQUIRES_SERVER; also TODO(port): MapSnapshotter and annotations; see map_snapshotter.test.cpp:150"]
fn annotation() {
    // Symbol, fill and line annotations show in the snapshot
    // (test/fixtures/map/map_snapshotter_annotation, 0.005, 0.1).
}
