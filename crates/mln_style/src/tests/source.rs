// Ports the style-side tests of test/style/source.test.cpp (the GeoJSON
// source; the render-source tests are in mln_renderer/tests/source_test.rs).

use crate::observer::SourceObserver;
use crate::source::Source;
use crate::sources::geojson_source::GeoJSONSource;
use crate::sources::tile_source::{UrlOrTileset, VectorSource};
use mln_util::tileset::{Tileset, VectorEncoding};
use mln_actor::RunLoop;
use mln_storage::stub_file_source::{ResponseType, StubFileSource};
use mln_storage::{Kind, Response};
use mln_util::exception::ExceptionPtr;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// `StubStyleObserver` (test/src/mln/test/stub_style_observer.hpp), the
/// SourceObserver half.
#[derive(Default)]
struct StubStyleObserver {
    source_loaded: RefCell<Option<Box<dyn Fn(&dyn Source)>>>,
    source_changed: RefCell<Option<Box<dyn Fn(&dyn Source)>>>,
    source_error: RefCell<Option<Box<dyn Fn(&dyn Source, ExceptionPtr)>>>,
}

impl SourceObserver for StubStyleObserver {
    fn on_source_changed(&self, source: &dyn Source) {
        if let Some(f) = &*self.source_changed.borrow() {
            f(source);
        }
    }

    fn on_source_loaded(&self, source: &dyn Source) {
        if let Some(f) = &*self.source_loaded.borrow() {
            f(source);
        }
    }

    fn on_source_error(&self, source: &dyn Source, error: ExceptionPtr) {
        if let Some(f) = &*self.source_error.borrow() {
            f(source, error);
        }
    }
}

#[test]
fn geojson_source_url_update() {
    let run_loop = RunLoop::new();
    let file_source = StubFileSource::new(ResponseType::Asynchronous);
    file_source.set_response(Kind::Source, |resource| {
        assert_eq!("http://source-url.ext", resource.url);
        let mut response = Response::new();
        response.data = Some(Arc::new(
            br#"{"geometry": {"type": "Point", "coordinates": [1.1, 1.1]}, "type": "Feature", "properties": {}}"#
                .to_vec(),
        ));
        Some(response)
    });

    let observer = Rc::new(StubStyleObserver::default());
    let loaded = Rc::new(RefCell::new(false));
    {
        let loaded = loaded.clone();
        let handle = run_loop.handle();
        *observer.source_loaded.borrow_mut() = Some(Box::new(move |_source| {
            // Should be called (test will hang if it doesn't)
            *loaded.borrow_mut() = true;
            handle.stop();
        }));
    }

    let source = GeoJSONSource::with_default_options("source");
    source.set_observer(Some(observer.clone()));

    // Load initial, so the source state will be loaded=true
    source.load_description(&file_source);

    // Schedule an update. C++ runs this from the loop (`loop.invoke`); the
    // source's callbacks only arrive while the loop runs either way.
    // Update the url
    source.set_url("http://source-url.ext");
    source.load_description(&file_source);

    run_loop.run();
    assert!(*loaded.borrow());
    assert!(source.loaded());
    assert!(source.impl_().get_data().and_then(|d| d.upgrade()).is_some());
}

fn as_vector(source: &dyn Source) -> &VectorSource {
    source.as_any().downcast_ref::<VectorSource>().expect("a VectorSource")
}

#[test]
fn vector_source_set_tiles() {
    let run_loop = RunLoop::new();
    let file_source = StubFileSource::new(ResponseType::Asynchronous);
    let observer = Rc::new(StubStyleObserver::default());
    let changed = Rc::new(RefCell::new(false));
    {
        let changed = changed.clone();
        *observer.source_changed.borrow_mut() = Some(Box::new(move |source| {
            assert_eq!(as_vector(source).get_tiles(), vec!["new".to_owned()]);
            *changed.borrow_mut() = true;
        }));
    }

    let tileset = Tileset {
        tiles: vec!["url".to_owned()],
        ..Default::default()
    };
    let source = VectorSource::new("source", UrlOrTileset::Tileset(tileset), None, None, VectorEncoding::Mapbox);
    source.set_observer(Some(observer.clone()));
    source.set_tiles(&["unused".to_owned()]); // make sure early setTiles does not crash
    source.load_description(&file_source);
    assert_eq!(source.get_tiles(), vec!["url".to_owned()]);
    source.set_tiles(&["new".to_owned()]);
    run_loop.run_once();
    assert!(*changed.borrow());
}

#[test]
fn vector_source_url_set_tiles() {
    let run_loop = RunLoop::new();
    let file_source = StubFileSource::new(ResponseType::Asynchronous);
    file_source.set_response(Kind::Source, |resource| {
        assert_eq!("url", resource.url);
        let mut response = Response::new();
        response.data = Some(Arc::new(br#"{"tiles": ["tiles"]}"#.to_vec()));
        Some(response)
    });

    let observer = Rc::new(StubStyleObserver::default());
    *observer.source_loaded.borrow_mut() = Some(Box::new(|source| {
        assert_eq!(as_vector(source).get_tiles(), vec!["tiles".to_owned()]);
        as_vector(source).set_tiles(&["new".to_owned()]);
    }));
    let count = Rc::new(RefCell::new(0));
    {
        let count = count.clone();
        let handle = run_loop.handle();
        *observer.source_changed.borrow_mut() = Some(Box::new(move |source| {
            let n = *count.borrow();
            *count.borrow_mut() += 1;
            if n > 0 {
                assert_eq!(as_vector(source).get_tiles(), vec!["new".to_owned()]);
                handle.stop();
            }
        }));
    }

    let source = VectorSource::new("source", UrlOrTileset::Url("url".to_owned()), None, None, VectorEncoding::Mapbox);
    source.set_observer(Some(observer.clone()));
    source.set_tiles(&["unused".to_owned()]);
    source.load_description(&file_source);
    run_loop.run();
    assert_eq!(*count.borrow(), 2);
}
