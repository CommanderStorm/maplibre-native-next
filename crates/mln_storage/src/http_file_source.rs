// Ports src/mln/storage/http_file_source.hpp and
// platform/default/src/mln/storage/http_file_source.cpp (curl -> ureq).
//
// C++ drives a curl multi handle from the requesting thread's RunLoop (socket
// watches and a timer). ureq is blocking, so each request runs on one of the
// file source's worker threads, and the result goes back to the requesting
// thread through a mailbox, where `handleResult` runs the callback as in
// C++. Dropping the request cancels it: the mailbox closes, so the callback
// never runs (an in-flight transfer finishes on its worker and is dropped).
//
// Request headers as in C++: `Range` for data ranges, `If-None-Match` from
// the prior etag, else `If-Modified-Since` from the prior modification date,
// user agent "MapLibreNative/1.0", redirects followed. Response headers are
// read in order with curl's header callback rules (case-insensitive names;
// `Last-Modified`/`Expires` through vendor parsedate, `Cache-Control`,
// `ETag`, `Retry-After`, `X-Rate-Limit-Reset`). Status codes map to the same
// Response fields and errors.
//
// Under Emscripten the transfer is an `emscripten_fetch` on the same worker
// threads, which wait for it by suspending through JSPI, so the browser does
// the HTTP: CORS, its cache, compression. It sends no `If-None-Match` /
// `If-Modified-Since` (they would force a CORS preflight; the browser cache
// revalidates instead) and can't set the user agent. See `transport`.
//
// In the wasm-bindgen browser build (wasm32-unknown-unknown) there are no
// threads: `request` starts a `fetch` and awaits it in a future on the
// requesting thread's event loop (wasm-bindgen-futures), which hands the
// response to the same mailbox. Dropping the request aborts the fetch.
// Its requests send the same headers as the Emscripten ones.
//
// Differences (DEVIATIONS.md [glfw]): ureq sends `Accept-Encoding: gzip` and
// decodes gzip (curl: "gzip, deflate"); a connection failure's message is
// ureq's text instead of curl's `curl_easy_strerror: errorbuffer`; dates use
// the vendored parsedate instead of libcurl's `curl_getdate` (they differ
// only after 2037).

use crate::async_request::AsyncRequest;
use crate::file_source::{Callback, FileSource};
use crate::resource::{Kind, Resource};
use crate::resource_options::ResourceOptions;
use crate::response::{Error, Reason, Response};
use mln_actor::{ActorRef, Mailbox, scheduler};
use mln_util::chrono::{Timestamp, parse_timestamp_str};
use mln_util::client_options::ClientOptions;
use mln_util::http_header::{CacheControl, parse_retry_headers};
use mln_util::string::to_string_i64;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// The browser build (wasm32-unknown-unknown): no threads.
macro_rules! cfg_web {
    ($($item:item)*) => { $(#[cfg(all(target_family = "wasm", target_os = "unknown"))] $item)* };
}

/// The part of `HTTPRequest` that lives on the requesting thread.
pub struct HTTPRequestState {
    callback: RefCell<Callback>,
}

impl HTTPRequestState {
    /// The callback half of `handleResult(CURLcode)`.
    fn handle_result(&self, response: Response) {
        // Calling `callback` may result in deleting `this`; the mailbox keeps
        // the state alive while this message runs.
        (self.callback.borrow_mut())(response);
    }
}

/// `HTTPRequest`
struct HTTPRequest {
    mailbox: Arc<Mailbox>,
    cancelled: Arc<AtomicBool>,
    /// The browser build: aborts the fetch.
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    abort: Option<web_sys::AbortController>,
}

impl AsyncRequest for HTTPRequest {}

impl Drop for HTTPRequest {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.mailbox.close();
        #[cfg(all(target_family = "wasm", target_os = "unknown"))]
        if let Some(abort) = &self.abort {
            abort.abort();
        }
    }
}

struct Job {
    resource: Resource,
    reply: ActorRef<HTTPRequestState>,
    cancelled: Arc<AtomicBool>,
}

/// `HTTPFileSource::Impl`
struct Impl {
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    jobs: Mutex<mpsc::Sender<Job>>,
    resource_options: Mutex<ResourceOptions>,
    client_options: Mutex<ClientOptions>,
}

/// `headerMatches(header, buffer, length)`: ASCII-case-insensitive prefix.
fn header_value<'a>(name: &str, header: &str, line: &'a str) -> Option<&'a str> {
    (name.eq_ignore_ascii_case(header)).then_some(line)
}

/// `curl_getdate` (vendor parsedate, -1 when unparsable).
fn getdate(value: &str) -> Timestamp {
    parse_timestamp_str(value)
}

/// What a transport hands back: the status code, the response headers in
/// order, and the body, which only a 200 or 206 reads.
struct Transfer {
    status: i64,
    headers: Vec<(String, String)>,
    body: Result<Vec<u8>, Box<Error>>,
}

/// The worker half of `HTTPRequest`: performs the transfer and builds the
/// response as the header callback and `handleResult(CURLcode)` do.
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
fn perform(transport: &transport::Transport, resource: &Resource) -> Response {
    build_response(transport.transfer(resource), resource)
}

/// The response for a transfer, as the header callback and
/// `handleResult(CURLcode)` build it.
fn build_response(transfer: Result<Transfer, Box<Error>>, resource: &Resource) -> Response {
    // Make sure a response object exists in case we haven't got any headers or content.
    let mut response = Response::new();
    let transfer = match transfer {
        Ok(transfer) => transfer,
        Err(error) => {
            response.error = Some(error);
            return response;
        }
    };

    let mut retry_after: Option<String> = None;
    let mut x_rate_limit_reset: Option<String> = None;
    for (name, value) in &transfer.headers {
        let (name, value) = (name.as_str(), value.as_str());
        if let Some(value) = header_value(name, "last-modified", value) {
            // Always overwrite the modification date; We might already have
            // a value here from the Date header, but this one is more accurate.
            response.modified = Some(getdate(value));
        } else if let Some(value) = header_value(name, "etag", value) {
            response.etag = Some(value.to_owned());
        } else if let Some(value) = header_value(name, "cache-control", value) {
            let cc = CacheControl::parse(value);
            response.expires = cc.to_time_point();
            response.must_revalidate = cc.must_revalidate;
        } else if let Some(value) = header_value(name, "expires", value) {
            response.expires = Some(getdate(value));
        } else if let Some(value) = header_value(name, "retry-after", value) {
            retry_after = Some(value.to_owned());
        } else if let Some(value) = header_value(name, "x-rate-limit-reset", value) {
            x_rate_limit_reset = Some(value.to_owned());
        }
    }

    let response_code = transfer.status;
    if response_code == 200 || response_code == 206 {
        match transfer.body {
            Ok(data) => response.data = Some(Arc::new(data)),
            // curl reports a failed transfer as an error code, not a status.
            Err(error) => response.error = Some(error),
        }
    } else if response_code == 204 || (response_code == 404 && resource.kind == Kind::Tile) {
        response.no_content = true;
    } else if response_code == 304 {
        response.not_modified = true;
    } else if response_code == 404 {
        response.error = Some(Box::new(Error::new(Reason::NotFound, "HTTP status code 404")));
    } else if response_code == 429 {
        response.error = Some(Box::new(Error::with_retry_after(
            Reason::RateLimit,
            "HTTP status code 429",
            parse_retry_headers(&retry_after, &x_rate_limit_reset),
        )));
    } else if (500..600).contains(&response_code) {
        response.error = Some(Box::new(Error::new(
            Reason::Server,
            format!("HTTP status code {}", to_string_i64(response_code)),
        )));
    } else {
        response.error = Some(Box::new(Error::new(
            Reason::Other,
            format!("HTTP status code {}", to_string_i64(response_code)),
        )));
    }
    response
}

/// The `Range` request header for a data range.
fn range_header(resource: &Resource) -> Option<String> {
    resource.data_range.map(|(first, second)| format!("bytes={first}-{second}"))
}

/// curl through ureq: blocking transfers on the worker threads.
#[cfg(not(target_family = "wasm"))]
mod transport {
    use super::{Transfer, range_header};
    use crate::resource::Resource;
    use crate::response::{Error, Reason};
    use mln_util::chrono::rfc1123;

    /// The worker threads of a file source: as many as the online file
    /// source's default maximum of concurrent requests.
    pub const WORKERS: u32 = mln_util::constants::DEFAULT_MAXIMUM_CONCURRENT_REQUESTS;

    #[derive(Clone)]
    pub struct Transport {
        agent: ureq::Agent,
    }

    /// The curl error codes `handleResult` maps to `Reason::Connection`.
    fn is_connection_error(error: &ureq::Error) -> bool {
        matches!(
            error,
            ureq::Error::HostNotFound | ureq::Error::ConnectionFailed | ureq::Error::Timeout(_) | ureq::Error::Io(_)
        )
    }

    fn error(error: ureq::Error) -> Box<Error> {
        // Add human-readable error code
        let reason = if is_connection_error(&error) { Reason::Connection } else { Reason::Other };
        Box::new(Error::new(reason, error.to_string()))
    }

    impl Transport {
        pub fn new() -> Self {
            let agent = ureq::Agent::config_builder()
                .http_status_as_error(false)
                .user_agent("MapLibreNative/1.0")
                .build()
                .into();
            Transport { agent }
        }

        pub fn transfer(&self, resource: &Resource) -> Result<Transfer, Box<Error>> {
            let mut request = self.agent.get(&resource.url);
            if let Some(range) = range_header(resource) {
                request = request.header("Range", range);
            }

            // If there's already a response, set the correct etags/modified headers to
            // make sure we are getting a 304 response if possible. This avoids
            // redownloading unchanged data.
            if let Some(prior_etag) = &resource.prior_etag {
                request = request.header("If-None-Match", prior_etag.as_str());
            } else if let Some(prior_modified) = resource.prior_modified {
                request = request.header("If-Modified-Since", rfc1123(prior_modified));
            }

            let mut http_response = request.call().map_err(error)?;
            let headers = http_response
                .headers()
                .iter()
                .filter_map(|(name, value)| Some((name.as_str().to_owned(), value.to_str().ok()?.to_owned())))
                .collect();
            let status = i64::from(http_response.status().as_u16());
            let body = if status == 200 || status == 206 {
                http_response.body_mut().with_config().limit(u64::MAX).read_to_vec().map_err(error)
            } else {
                Ok(Vec::new())
            };
            Ok(Transfer { status, headers, body })
        }
    }
}

/// The browser's fetch through Emscripten (`-sFETCH=1`, `-sJSPI`). The worker
/// thread waits for the transfer as it waits for a ureq one, but by
/// suspending in `emscripten_sleep`, which lets its event loop deliver the
/// response. (A synchronous XMLHttpRequest from these threads intermittently
/// fails with a NetworkError in Chromium.)
///
/// Chromium also aborts an occasional request from these threads right after
/// it starts (net::ERR_ABORTED), which emscripten_fetch reports through no
/// callback, only through `readyState` reaching DONE. Such a request is tried
/// again a few times before it fails with a connection error.
#[cfg(target_os = "emscripten")]
mod transport {
    use super::{Transfer, range_header};
    use crate::resource::Resource;
    use crate::response::{Error, Reason};
    use std::ffi::{CStr, CString, c_char, c_void};
    use std::ptr::null;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Browsers run a handful of requests per host at a time, and every
    /// worker is a pthread, so fewer than natively.
    pub const WORKERS: u32 = 6;

    const EMSCRIPTEN_FETCH_LOAD_TO_MEMORY: u32 = 1;
    const EMSCRIPTEN_FETCH_REPLACE: u32 = 16;

    /// `emscripten_fetch_attr_t` (emscripten/fetch.h).
    #[repr(C)]
    struct FetchAttr {
        request_method: [c_char; 32],
        user_data: *mut c_void,
        onsuccess: Option<unsafe extern "C" fn(*mut Fetch)>,
        onerror: Option<unsafe extern "C" fn(*mut Fetch)>,
        onprogress: Option<unsafe extern "C" fn(*mut Fetch)>,
        onreadystatechange: Option<unsafe extern "C" fn(*mut Fetch)>,
        attributes: u32,
        timeout_msecs: u32,
        with_credentials: bool,
        destination_path: *const c_char,
        user_name: *const c_char,
        password: *const c_char,
        request_headers: *const *const c_char,
        overridden_mime_type: *const c_char,
        request_data: *const c_char,
        request_data_size: usize,
    }

    /// `emscripten_fetch_t` (emscripten/fetch.h).
    #[repr(C)]
    struct Fetch {
        id: u32,
        user_data: *mut c_void,
        url: *const c_char,
        data: *const c_char,
        num_bytes: u64,
        data_offset: u64,
        total_bytes: u64,
        ready_state: u16,
        status: u16,
        status_text: [c_char; 64],
        attributes: FetchAttr,
        response_url: *const c_char,
    }

    unsafe extern "C" {
        fn emscripten_fetch_attr_init(attr: *mut FetchAttr);
        fn emscripten_fetch(attr: *mut FetchAttr, url: *const c_char) -> *mut Fetch;
        fn emscripten_fetch_close(fetch: *mut Fetch) -> i32;
        fn emscripten_fetch_get_response_headers_length(fetch: *mut Fetch) -> usize;
        fn emscripten_fetch_get_response_headers(fetch: *mut Fetch, dst: *mut c_char, size: usize) -> usize;
        fn emscripten_sleep(ms: u32);
    }

    /// The fetch's `onsuccess` and `onerror`: they run on the requesting
    /// thread, from its event loop, while it sleeps.
    unsafe extern "C" fn finished(fetch: *mut Fetch) {
        // SAFETY: user_data is the requesting thread's flag, which outlives
        // the fetch.
        unsafe { (*(*fetch).user_data.cast::<AtomicBool>()).store(true, Ordering::SeqCst) };
    }

    /// `XMLHttpRequest.DONE`.
    const READY_STATE_DONE: u16 = 4;
    const ATTEMPTS: u32 = 4;

    #[derive(Clone)]
    pub struct Transport;

    /// `Name: value` lines of the browser's raw header block, in order.
    fn parse_headers(raw: &str) -> Vec<(String, String)> {
        raw.split("\r\n")
            .filter_map(|line| {
                let (name, value) = line.split_once(':')?;
                Some((name.trim().to_owned(), value.trim().to_owned()))
            })
            .collect()
    }

    impl Transport {
        pub fn new() -> Self {
            Transport
        }

        pub fn transfer(&self, resource: &Resource) -> Result<Transfer, Box<Error>> {
            for attempt in 1..ATTEMPTS {
                match self.attempt(resource) {
                    Some(result) => return result,
                    // SAFETY: sleeping suspends this thread through JSPI.
                    None => unsafe { emscripten_sleep(50 * attempt) },
                }
            }
            self.attempt(resource).unwrap_or_else(|| {
                Err(Box::new(Error::new(Reason::Connection, format!("the browser aborted the request for {}", resource.url))))
            })
        }

        /// One fetch: `None` when the browser aborted it before it finished.
        fn attempt(&self, resource: &Resource) -> Option<Result<Transfer, Box<Error>>> {
            let connection = |message: String| Box::new(Error::new(Reason::Connection, message));
            let url = match CString::new(resource.url.as_str()) {
                Ok(url) => url,
                Err(_) => return Some(Err(connection("URL contains a NUL byte".into()))),
            };
            let range = range_header(resource).map(|range| CString::new(range).unwrap());
            let range_name = c"Range";
            let headers: Vec<*const c_char> = match &range {
                Some(range) => vec![range_name.as_ptr(), range.as_ptr(), null()],
                None => vec![null()],
            };
            let done = AtomicBool::new(false);
            // SAFETY: emscripten_fetch_attr_init initializes every field; the
            // strings, the header list and `done` outlive the fetch, which
            // finishes before this returns.
            unsafe {
                let mut attr = std::mem::MaybeUninit::<FetchAttr>::uninit();
                emscripten_fetch_attr_init(attr.as_mut_ptr());
                let mut attr = attr.assume_init();
                for (slot, byte) in attr.request_method.iter_mut().zip(b"GET\0") {
                    *slot = *byte as c_char;
                }
                // REPLACE: straight to the network, never to emscripten_fetch's
                // IndexedDB cache.
                attr.attributes = EMSCRIPTEN_FETCH_LOAD_TO_MEMORY | EMSCRIPTEN_FETCH_REPLACE;
                attr.request_headers = headers.as_ptr();
                attr.user_data = (&raw const done).cast_mut().cast();
                attr.onsuccess = Some(finished);
                attr.onerror = Some(finished);
                let fetch = emscripten_fetch(&mut attr, url.as_ptr());
                if fetch.is_null() {
                    return Some(Err(connection(format!("emscripten_fetch failed for {}", resource.url))));
                }
                while !done.load(Ordering::SeqCst) {
                    if (*fetch).ready_state == READY_STATE_DONE && (*fetch).status == 0 {
                        // Aborted: give a late callback one more turn, then
                        // stop waiting.
                        emscripten_sleep(1);
                        if !done.load(Ordering::SeqCst) {
                            emscripten_fetch_close(fetch);
                            return None;
                        }
                    }
                    emscripten_sleep(1);
                }
                let status = i64::from((*fetch).status);
                if status == 0 {
                    // A network error or a CORS rejection: the browser reports
                    // no status.
                    let text = CStr::from_ptr((*fetch).status_text.as_ptr()).to_string_lossy().into_owned();
                    emscripten_fetch_close(fetch);
                    let message = if text.is_empty() { "network error".to_owned() } else { text };
                    return Some(Err(connection(message)));
                }
                let length = emscripten_fetch_get_response_headers_length(fetch);
                let mut raw = vec![0u8; length + 1];
                emscripten_fetch_get_response_headers(fetch, raw.as_mut_ptr().cast(), raw.len());
                raw.truncate(length);
                let headers = parse_headers(&String::from_utf8_lossy(&raw));
                let body = if status == 200 || status == 206 {
                    let data = (*fetch).data.cast::<u8>();
                    let size = (*fetch).num_bytes as usize;
                    Ok(if data.is_null() { Vec::new() } else { std::slice::from_raw_parts(data, size).to_vec() })
                } else {
                    Ok(Vec::new())
                };
                emscripten_fetch_close(fetch);
                Some(Ok(Transfer { status, headers, body }))
            }
        }
    }
}

cfg_web! {
    /// The browser's `fetch` through wasm-bindgen, awaited on the requesting
    /// thread (see the top of the file).
    mod fetch {
        use super::{Transfer, range_header};
        use crate::resource::Resource;
        use crate::response::{Error, Reason};
        use wasm_bindgen::{JsCast, JsValue};
        use wasm_bindgen_futures::JsFuture;

        fn message(value: &JsValue) -> String {
            value
                .dyn_ref::<js_sys::Error>()
                .map(|e| String::from(e.message()))
                .or_else(|| value.as_string())
                .unwrap_or_else(|| format!("{value:?}"))
        }

        /// The global scope's `fetch(request)`, in a window or a worker.
        fn start(request: &web_sys::Request) -> Result<js_sys::Promise, JsValue> {
            let global = js_sys::global();
            if let Some(window) = global.dyn_ref::<web_sys::Window>() {
                return Ok(window.fetch_with_request(request));
            }
            if let Some(worker) = global.dyn_ref::<web_sys::WorkerGlobalScope>() {
                return Ok(worker.fetch_with_request(request));
            }
            Err(JsValue::from_str("no fetch in this global scope"))
        }

        pub async fn transfer(resource: &Resource, signal: &web_sys::AbortSignal) -> Result<Transfer, Box<Error>> {
            let connection = |value: JsValue| Box::new(Error::new(Reason::Connection, message(&value)));
            let init = web_sys::RequestInit::new();
            init.set_method("GET");
            init.set_mode(web_sys::RequestMode::Cors);
            init.set_signal(Some(signal));
            if let Some(range) = range_header(resource) {
                let headers = web_sys::Headers::new().map_err(connection)?;
                headers.set("Range", &range).map_err(connection)?;
                init.set_headers(&headers);
            }
            let request = web_sys::Request::new_with_str_and_init(&resource.url, &init).map_err(connection)?;
            let response: web_sys::Response =
                JsFuture::from(start(&request).map_err(connection)?).await.map_err(connection)?.unchecked_into();

            let status = i64::from(response.status());
            let mut headers = Vec::new();
            if let Ok(Some(entries)) = js_sys::try_iter(response.headers().as_ref()) {
                for entry in entries.flatten() {
                    let entry: js_sys::Array = entry.unchecked_into();
                    if let (Some(name), Some(value)) = (entry.get(0).as_string(), entry.get(1).as_string()) {
                        headers.push((name, value));
                    }
                }
            }
            let body = if status == 200 || status == 206 {
                match response.array_buffer() {
                    Ok(promise) => match JsFuture::from(promise).await {
                        Ok(buffer) => Ok(js_sys::Uint8Array::new(&buffer).to_vec()),
                        Err(error) => Err(connection(error)),
                    },
                    Err(error) => Err(connection(error)),
                }
            } else {
                Ok(Vec::new())
            };
            Ok(Transfer { status, headers, body })
        }
    }
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
fn worker(transport: transport::Transport, jobs: Arc<Mutex<mpsc::Receiver<Job>>>) {
    loop {
        let job = match jobs.lock() {
            Ok(receiver) => receiver.recv(),
            Err(_) => return,
        };
        let Ok(job) = job else {
            return; // the file source is gone
        };
        if job.cancelled.load(Ordering::SeqCst) {
            continue;
        }
        let response = perform(&transport, &job.resource);
        if !job.cancelled.load(Ordering::SeqCst) {
            job.reply.invoke(move |request: &HTTPRequestState| request.handle_result(response));
        }
    }
}

/// `HTTPFileSource`: plain HTTP(S) requests, used by the OnlineFileSource.
pub struct HTTPFileSource {
    impl_: Impl,
}

impl HTTPFileSource {
    /// `HTTPFileSource(const ResourceOptions&, const ClientOptions&)`: the
    /// browser build needs no threads.
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    pub fn new(resource_options: &ResourceOptions, client_options: &ClientOptions) -> Self {
        HTTPFileSource {
            impl_: Impl {
                resource_options: Mutex::new(resource_options.clone()),
                client_options: Mutex::new(client_options.clone()),
            },
        }
    }

    /// `request(const Resource&, Callback)`: fetches on this thread's event
    /// loop; the callback runs on its RunLoop.
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    pub fn request(&self, resource: &Resource, callback: Callback) -> Box<dyn AsyncRequest> {
        let state = Rc::new(HTTPRequestState {
            callback: RefCell::new(callback),
        });
        let mailbox = Mailbox::with_target(&scheduler::get_current(), state as Rc<dyn Any>);
        let cancelled = Arc::new(AtomicBool::new(false));
        let abort = web_sys::AbortController::new().ok();
        let job = Job {
            resource: resource.clone(),
            reply: ActorRef::new(&mailbox),
            cancelled: cancelled.clone(),
        };
        let signal = abort.as_ref().map(|abort| abort.signal());
        wasm_bindgen_futures::spawn_local(async move {
            let signal = match signal {
                Some(signal) => signal,
                None => match web_sys::AbortController::new() {
                    Ok(abort) => abort.signal(),
                    Err(_) => return,
                },
            };
            let transfer = fetch::transfer(&job.resource, &signal).await;
            if job.cancelled.load(Ordering::SeqCst) {
                return;
            }
            let response = build_response(transfer, &job.resource);
            job.reply.invoke(move |request: &HTTPRequestState| request.handle_result(response));
        });
        Box::new(HTTPRequest { mailbox, cancelled, abort })
    }

    /// `HTTPFileSource(const ResourceOptions&, const ClientOptions&)`: starts
    /// the transfer threads (`transport::WORKERS`).
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    pub fn new(resource_options: &ResourceOptions, client_options: &ClientOptions) -> Self {
        let transport = transport::Transport::new();
        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));
        for i in 0..transport::WORKERS {
            let transport = transport.clone();
            let receiver = receiver.clone();
            // Detached: a worker exits when the channel closes, after its
            // current transfer.
            let _ = std::thread::Builder::new()
                .name(format!("HTTPFileSource {i}"))
                .spawn(move || worker(transport, receiver));
        }
        HTTPFileSource {
            impl_: Impl {
                jobs: Mutex::new(sender),
                resource_options: Mutex::new(resource_options.clone()),
                client_options: Mutex::new(client_options.clone()),
            },
        }
    }

    /// `request(const Resource&, Callback)`. The callback runs on the
    /// calling thread's RunLoop.
    #[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
    pub fn request(&self, resource: &Resource, callback: Callback) -> Box<dyn AsyncRequest> {
        let state = Rc::new(HTTPRequestState {
            callback: RefCell::new(callback),
        });
        let mailbox = Mailbox::with_target(&scheduler::get_current(), state as Rc<dyn Any>);
        let cancelled = Arc::new(AtomicBool::new(false));
        let job = Job {
            resource: resource.clone(),
            reply: ActorRef::new(&mailbox),
            cancelled: cancelled.clone(),
        };
        let _ = self.impl_.jobs.lock().unwrap().send(job);
        Box::new(HTTPRequest {
            mailbox,
            cancelled,
            #[cfg(all(target_family = "wasm", target_os = "unknown"))]
            abort: None,
        })
    }
}

impl FileSource for HTTPFileSource {
    fn request(&self, resource: &Resource, callback: Callback) -> Box<dyn AsyncRequest> {
        HTTPFileSource::request(self, resource, callback)
    }

    fn can_request(&self, _resource: &Resource) -> bool {
        true
    }

    fn set_resource_options(&self, options: ResourceOptions) {
        *self.impl_.resource_options.lock().unwrap() = options;
    }

    fn get_resource_options(&self) -> ResourceOptions {
        self.impl_.resource_options.lock().unwrap().clone()
    }

    fn set_client_options(&self, options: ClientOptions) {
        *self.impl_.client_options.lock().unwrap() = options;
    }

    fn get_client_options(&self) -> ClientOptions {
        self.impl_.client_options.lock().unwrap().clone()
    }
}
