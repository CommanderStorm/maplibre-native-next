// Ports the tests of test/storage/main_resource_loader.test.cpp that need
// no test server and no write path.
//
// TODO(port): OptionalNonExpired and OptionalExpired (they store with
// `forward`, the write path); the TEST_REQUIRES_SERVER tests (online file
// source).

use super::temp_path;
use crate::database_file_source::DatabaseFileSource;
use crate::file_source::{FileSource, FileSourceType};
use crate::main_resource_loader::MainResourceLoader;
use crate::resource::{Kind, LoadingMethod, Resource};
use crate::resource_options::ResourceOptions;
use crate::response::{Reason, Response};
use crate::{AsyncRequest, ClientOptions};
use mln_actor::RunLoop;
use mln_util::tile_server_options::TileServerOptions;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn optional_not_found() {
    let run_loop = RunLoop::new();
    let fs = MainResourceLoader::new(&ResourceOptions::default(), &ClientOptions::default());

    let optional_resource =
        Resource::with_tile_data(Kind::Unknown, "http://127.0.0.1:3000/test", None, LoadingMethod::CacheOnly);

    let req: Rc<RefCell<Option<Box<dyn AsyncRequest>>>> = Rc::new(RefCell::new(None));
    let result: Rc<RefCell<Option<Response>>> = Rc::new(RefCell::new(None));
    let (r, out, stop) = (req.clone(), result.clone(), run_loop.clone());
    *req.borrow_mut() = Some(fs.request(
        &optional_resource,
        Box::new(move |res: Response| {
            r.borrow_mut().take();
            *out.borrow_mut() = Some(res);
            stop.stop();
        }),
    ));

    run_loop.run();

    let res = result.borrow_mut().take().expect("response");
    let error = res.error.as_ref().expect("error");
    assert_eq!(Reason::NotFound, error.reason);
    assert_eq!("Not found in offline database", error.message);
    assert!(res.data.is_none());
    assert!(res.expires.is_none());
    assert!(!res.must_revalidate);
    assert!(res.modified.is_none());
    assert!(res.etag.is_none());
}

#[test]
fn set_resource_cache_path() {
    let run_loop = RunLoop::new();
    let _resource_loader = MainResourceLoader::new(&ResourceOptions::default(), &ClientOptions::default());
    // C++ creates a separate DatabaseFileSource via static_pointer_cast of the
    // manager's; the Rust manager returns `dyn FileSource`, so the test
    // creates one directly with the same options.
    let dbfs = DatabaseFileSource::new(&ResourceOptions::default(), &ClientOptions::default());
    let stop = run_loop.handle();
    // C++: "./new_offline.db".
    dbfs.set_database_path(&temp_path("new_offline.db"), Some(Box::new(move || stop.stop())));
    run_loop.run();
}

#[test]
fn resource_options() {
    let fs = MainResourceLoader::new(
        &ResourceOptions::default().with_tile_server_options(
            TileServerOptions::new()
                .with_base_url("originalBaseURL")
                .with_uri_scheme_alias("originalAlias"),
        ),
        &ClientOptions::default(),
    );

    fs.set_resource_options(
        ResourceOptions::default().with_tile_server_options(
            TileServerOptions::new()
                .with_base_url("updatedBaseURL")
                .with_uri_scheme_alias("updatedAlias"),
        ),
    );

    let updated_options = fs.get_resource_options().tile_server_options();

    assert_eq!(updated_options.base_url(), "updatedBaseURL");
    assert_eq!(updated_options.uri_scheme_alias(), "updatedAlias");
}

#[test]
fn cache_only_without_database_is_unsupported() {
    // The "Unsupported resource request." branch: asset:// is neither an
    // asset (no asset file source ported yet) nor cacheable.
    let run_loop = RunLoop::new();
    let fs = MainResourceLoader::new(&ResourceOptions::default(), &ClientOptions::default());
    let resource = Resource::with_tile_data(Kind::Unknown, "asset://x.json", None, LoadingMethod::CacheOnly);
    let result: Rc<RefCell<Option<Response>>> = Rc::new(RefCell::new(None));
    let (out, stop) = (result.clone(), run_loop.clone());
    let _req = fs.request(
        &resource,
        Box::new(move |res: Response| {
            *out.borrow_mut() = Some(res);
            stop.stop();
        }),
    );
    run_loop.run();
    let res = result.borrow_mut().take().expect("response");
    assert!(res.no_content);
    assert_eq!("Unsupported resource request.", res.error.unwrap().message);
    let _ = FileSourceType::Asset;
}
