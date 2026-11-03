// Ports the core cases of test/storage/http_file_source.test.cpp and
// test/storage/online_file_source.test.cpp (TEST_REQUIRES_SERVER) against a
// small in-process HTTP server instead of the C++ node test server: 200 with
// caching headers, 304 revalidation, 404 (and 404 for tiles = no content),
// 500, connection errors, cancellation, and the online file source's
// request path.

use crate::file_source::FileSource;
use crate::http_file_source::HTTPFileSource;
use crate::online_file_source::OnlineFileSource;
use crate::resource::{Kind, Resource};
use crate::resource_options::ResourceOptions;
use crate::response::{Reason, Response};
use crate::{AsyncRequest, ClientOptions};
use mln_actor::RunLoop;
use mln_util::chrono::{Timestamp, now};
use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;

/// Serves `GET <path>` with fixed responses, `Connection: close`.
fn start_server() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            std::thread::spawn(move || serve(stream));
        }
    });
    port
}

fn serve(stream: TcpStream) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    let path = request_line.split_whitespace().nth(1).unwrap_or("/").to_owned();
    let mut if_none_match = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line == "\r\n" || line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("if-none-match")
        {
            if_none_match = Some(value.trim().to_owned());
        }
    }
    let (status, headers, body): (&str, &str, &str) = match path.as_str() {
        "/test" => (
            "200 OK",
            "ETag: \"snowfall\"\r\nCache-Control: max-age=120\r\nLast-Modified: Sun, 06 Nov 1994 08:49:37 GMT\r\n",
            "Hello World!",
        ),
        "/revalidate-etag" if if_none_match.as_deref() == Some("\"snowfall\"") => ("304 Not Modified", "", ""),
        "/revalidate-etag" => ("200 OK", "ETag: \"snowfall\"\r\n", "Response"),
        "/expires" => ("200 OK", "Expires: Wed, 21 Oct 2015 07:28:00 GMT\r\n", "Expired"),
        "/no-content" => ("204 No Content", "", ""),
        "/server-error" => ("500 Internal Server Error", "", "Server Error!"),
        "/rate-limit" => ("429 Too Many Requests", "Retry-After: 30\r\n", ""),
        _ => ("404 Not Found", "", "Not Found!"),
    };
    let mut stream = stream;
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

/// Requests `resource` from `fs` and runs the loop until the first response.
fn fetch(fs: &dyn FileSource, resource: Resource) -> Response {
    let run_loop = RunLoop::new();
    let req: Rc<RefCell<Option<Box<dyn AsyncRequest>>>> = Rc::new(RefCell::new(None));
    let result: Rc<RefCell<Option<Response>>> = Rc::new(RefCell::new(None));
    let (r, out, stop) = (req.clone(), result.clone(), run_loop.clone());
    *req.borrow_mut() = Some(fs.request(
        &resource,
        Box::new(move |res: Response| {
            r.borrow_mut().take();
            *out.borrow_mut() = Some(res);
            stop.stop();
        }),
    ));
    run_loop.run();
    let response = result.borrow_mut().take().expect("response");
    response
}

fn http() -> HTTPFileSource {
    HTTPFileSource::new(&ResourceOptions::default(), &ClientOptions::default())
}

#[test]
fn http_200() {
    let port = start_server();
    let res = fetch(&http(), Resource::new(Kind::Unknown, format!("http://127.0.0.1:{port}/test")));
    assert!(res.error.is_none(), "{:?}", res.error);
    assert_eq!(res.data.as_deref().map(Vec::as_slice), Some(&b"Hello World!"[..]));
    assert_eq!(res.etag.as_deref(), Some("\"snowfall\""));
    assert_eq!(res.modified, Some(Timestamp::from_secs(784111777)));
    let expires = res.expires.expect("expires").secs_since_epoch();
    let now = now().secs_since_epoch();
    assert!((now + 115..=now + 121).contains(&expires));
    assert!(!res.must_revalidate);
}

#[test]
fn http_expires_header() {
    let port = start_server();
    let res = fetch(&http(), Resource::new(Kind::Unknown, format!("http://127.0.0.1:{port}/expires")));
    assert_eq!(res.expires, Some(Timestamp::from_secs(1445412480)));
}

#[test]
fn http_304_with_prior_etag() {
    let port = start_server();
    let mut resource = Resource::new(Kind::Unknown, format!("http://127.0.0.1:{port}/revalidate-etag"));
    resource.prior_etag = Some("\"snowfall\"".to_owned());
    let res = fetch(&http(), resource);
    assert!(res.error.is_none());
    assert!(res.not_modified);
    assert!(res.data.is_none());
}

#[test]
fn http_404() {
    let port = start_server();
    let res = fetch(&http(), Resource::new(Kind::Unknown, format!("http://127.0.0.1:{port}/doesnotexist")));
    let error = res.error.expect("error");
    assert_eq!(error.reason, Reason::NotFound);
    assert_eq!(error.message, "HTTP status code 404");
    assert!(res.data.is_none());
}

#[test]
fn http_tile_404_is_no_content() {
    let port = start_server();
    let res = fetch(&http(), Resource::new(Kind::Tile, format!("http://127.0.0.1:{port}/doesnotexist")));
    assert!(res.error.is_none());
    assert!(res.no_content);
    assert!(res.data.is_none());
}

#[test]
fn http_204() {
    let port = start_server();
    let res = fetch(&http(), Resource::new(Kind::Unknown, format!("http://127.0.0.1:{port}/no-content")));
    assert!(res.error.is_none());
    assert!(res.no_content);
}

#[test]
fn http_500() {
    let port = start_server();
    let res = fetch(&http(), Resource::new(Kind::Unknown, format!("http://127.0.0.1:{port}/server-error")));
    let error = res.error.expect("error");
    assert_eq!(error.reason, Reason::Server);
    assert_eq!(error.message, "HTTP status code 500");
}

#[test]
fn http_429_retry_after() {
    let port = start_server();
    let res = fetch(&http(), Resource::new(Kind::Unknown, format!("http://127.0.0.1:{port}/rate-limit")));
    let error = res.error.expect("error");
    assert_eq!(error.reason, Reason::RateLimit);
    let retry_after = error.retry_after.expect("retry after").secs_since_epoch();
    let now = now().secs_since_epoch();
    assert!((now + 25..=now + 31).contains(&retry_after));
}

#[test]
fn http_connection_error() {
    // Nothing listens on the port of a dropped listener.
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let res = fetch(&http(), Resource::new(Kind::Unknown, format!("http://127.0.0.1:{port}/")));
    assert_eq!(res.error.expect("error").reason, Reason::Connection);
}

#[test]
fn http_cancel() {
    let run_loop = RunLoop::new();
    let port = start_server();
    let fs = http();
    let called = Rc::new(RefCell::new(false));
    let c = called.clone();
    let req = fs.request(
        &Resource::new(Kind::Unknown, format!("http://127.0.0.1:{port}/test")),
        Box::new(move |_| *c.borrow_mut() = true),
    );
    drop(req);
    std::thread::sleep(std::time::Duration::from_millis(200));
    run_loop.run_once();
    assert!(!*called.borrow());
}

#[test]
fn online_file_source_200() {
    let port = start_server();
    let fs = OnlineFileSource::new(&ResourceOptions::default(), &ClientOptions::default());
    let res = fetch(&fs, Resource::new(Kind::Style, format!("http://127.0.0.1:{port}/test")));
    assert!(res.error.is_none());
    assert_eq!(res.data.as_deref().map(Vec::as_slice), Some(&b"Hello World!"[..]));
}

#[test]
fn online_file_source_offline_status() {
    use maplibre_native_base::Value;
    let port = start_server();
    let fs = OnlineFileSource::new(&ResourceOptions::default(), &ClientOptions::default());
    fs.set_property(crate::online_file_source::ONLINE_STATUS_KEY, &Value::Bool(false));
    let res = fetch(&fs, Resource::new(Kind::Unknown, format!("http://127.0.0.1:{port}/test")));
    let error = res.error.expect("error");
    assert_eq!(error.reason, Reason::Connection);
    assert_eq!(error.message, "Online connectivity is disabled.");
}
