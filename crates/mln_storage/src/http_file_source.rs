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
use mln_util::chrono::{Timestamp, parse_timestamp_str, rfc1123};
use mln_util::client_options::ClientOptions;
use mln_util::constants::DEFAULT_MAXIMUM_CONCURRENT_REQUESTS;
use mln_util::http_header::{CacheControl, parse_retry_headers};
use mln_util::string::to_string_i64;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};

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
}

impl AsyncRequest for HTTPRequest {}

impl Drop for HTTPRequest {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.mailbox.close();
    }
}

struct Job {
    resource: Resource,
    reply: ActorRef<HTTPRequestState>,
    cancelled: Arc<AtomicBool>,
}

/// `HTTPFileSource::Impl`
struct Impl {
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

/// The curl error codes `handleResult` maps to `Reason::Connection`.
fn is_connection_error(error: &ureq::Error) -> bool {
    matches!(
        error,
        ureq::Error::HostNotFound | ureq::Error::ConnectionFailed | ureq::Error::Timeout(_) | ureq::Error::Io(_)
    )
}

/// The worker half of `HTTPRequest`: performs the transfer and builds the
/// response as the header callback and `handleResult(CURLcode)` do.
fn perform(agent: &ureq::Agent, resource: &Resource) -> Response {
    let mut request = agent.get(&resource.url);
    if let Some((first, second)) = resource.data_range {
        request = request.header("Range", format!("bytes={first}-{second}"));
    }

    // If there's already a response, set the correct etags/modified headers to
    // make sure we are getting a 304 response if possible. This avoids
    // redownloading unchanged data.
    if let Some(prior_etag) = &resource.prior_etag {
        request = request.header("If-None-Match", prior_etag.as_str());
    } else if let Some(prior_modified) = resource.prior_modified {
        request = request.header("If-Modified-Since", rfc1123(prior_modified));
    }

    // Make sure a response object exists in case we haven't got any headers or content.
    let mut response = Response::new();

    let mut http_response = match request.call() {
        Ok(http_response) => http_response,
        Err(error) => {
            // Add human-readable error code
            let reason = if is_connection_error(&error) { Reason::Connection } else { Reason::Other };
            response.error = Some(Box::new(Error::new(reason, error.to_string())));
            return response;
        }
    };

    let mut retry_after: Option<String> = None;
    let mut x_rate_limit_reset: Option<String> = None;
    for (name, value) in http_response.headers() {
        let Ok(value) = value.to_str() else {
            continue;
        };
        let name = name.as_str();
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

    let response_code = i64::from(http_response.status().as_u16());
    if response_code == 200 || response_code == 206 {
        match http_response.body_mut().with_config().limit(u64::MAX).read_to_vec() {
            Ok(data) => response.data = Some(Arc::new(data)),
            // curl reports a failed transfer as an error code, not a status.
            Err(error) => {
                let reason = if is_connection_error(&error) { Reason::Connection } else { Reason::Other };
                response.error = Some(Box::new(Error::new(reason, error.to_string())));
            }
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

fn worker(agent: ureq::Agent, jobs: Arc<Mutex<mpsc::Receiver<Job>>>) {
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
        let response = perform(&agent, &job.resource);
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
    /// `HTTPFileSource(const ResourceOptions&, const ClientOptions&)`: starts
    /// the transfer threads (as many as the online file source's default
    /// maximum of concurrent requests).
    pub fn new(resource_options: &ResourceOptions, client_options: &ClientOptions) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .user_agent("MapLibreNative/1.0")
            .build()
            .into();
        let (sender, receiver) = mpsc::channel::<Job>();
        let receiver = Arc::new(Mutex::new(receiver));
        for i in 0..DEFAULT_MAXIMUM_CONCURRENT_REQUESTS {
            let agent = agent.clone();
            let receiver = receiver.clone();
            // Detached: a worker exits when the channel closes, after its
            // current transfer.
            let _ = std::thread::Builder::new()
                .name(format!("HTTPFileSource {i}"))
                .spawn(move || worker(agent, receiver));
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
        Box::new(HTTPRequest { mailbox, cancelled })
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
