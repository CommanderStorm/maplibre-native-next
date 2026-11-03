// Ports test/storage/database_file_source.test.cpp.
//
// TODO(port): VolatileResource (needs `forward` to store, the write path).

use crate::database_file_source::DatabaseFileSource;
use crate::file_source::{FileSource, FileSourceType};
use crate::file_source_manager::FileSourceManager;
use crate::resource::{Kind, LoadingMethod, Resource, StoragePolicy};
use crate::resource_options::ResourceOptions;
use crate::response::{Reason, Response};
use crate::{AsyncRequest, ClientOptions};
use mln_actor::RunLoop;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

fn dbfs() -> Arc<dyn FileSource> {
    FileSourceManager::get()
        .get_file_source(FileSourceType::Database, &ResourceOptions::default(), &ClientOptions::default())
        .expect("database file source")
}

#[test]
fn pause_resume() {
    let run_loop = RunLoop::new();

    let dbfs = dbfs();
    dbfs.pause();

    let res = Resource::with_tile_data(Kind::Unknown, "http://127.0.0.1:3000/test", None, LoadingMethod::CacheOnly);
    let stop = run_loop.clone();
    let _req = dbfs.request(&res, Box::new(move |_: Response| stop.stop()));

    // C++: util::Timer resumeTimer.start(5ms, ...) (timers are a TODO(port)
    // of the RunLoop); a thread resumes instead.
    let resumer = {
        let dbfs = dbfs.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(5));
            dbfs.resume();
        })
    };

    run_loop.run();
    resumer.join().unwrap();
}

#[test]
fn volatile_resource_is_not_stored() {
    let run_loop = RunLoop::new();

    let dbfs = dbfs();

    let mut resource = Resource::with_tile_data(
        Kind::Unknown,
        "http://127.0.0.1:3000/volatile-not-stored",
        None,
        LoadingMethod::CacheOnly,
    );
    resource.storage_policy = StoragePolicy::Volatile;
    let mut response = Response::new();
    response.data = Some(Arc::new(b"Volatile value".to_vec()));

    // Forwarding a volatile response must not write it to the database, so
    // a regular request for the same resource finds nothing.
    dbfs.forward(&resource, &response, None);
    resource.storage_policy = StoragePolicy::Permanent;
    let req: Rc<RefCell<Option<Box<dyn AsyncRequest>>>> = Rc::new(RefCell::new(None));
    let result: Rc<RefCell<Option<Response>>> = Rc::new(RefCell::new(None));
    let (r, out, stop) = (req.clone(), result.clone(), run_loop.clone());
    *req.borrow_mut() = Some(dbfs.request(
        &resource,
        Box::new(move |res: Response| {
            r.borrow_mut().take();
            stop.stop();
            *out.borrow_mut() = Some(res);
        }),
    ));
    run_loop.run();

    let res = result.borrow_mut().take().expect("response");
    assert!(res.no_content);
    let error = res.error.expect("error");
    assert_eq!(Reason::NotFound, error.reason);
}

#[test]
fn can_request() {
    let dbfs = DatabaseFileSource::new(&ResourceOptions::default(), &ClientOptions::default());
    let cache_only = |url: &str| Resource::with_tile_data(Kind::Unknown, url, None, LoadingMethod::CacheOnly);
    assert!(dbfs.can_request(&cache_only("http://example.com")));
    assert!(!dbfs.can_request(&cache_only("asset://example.json")));
    assert!(!dbfs.can_request(&cache_only("file://example.json")));
    assert!(!dbfs.can_request(&Resource::with_tile_data(
        Kind::Unknown,
        "http://example.com",
        None,
        LoadingMethod::NetworkOnly
    )));
}
