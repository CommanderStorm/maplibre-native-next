// Ports test/include/mln/test/stub_file_source.hpp and
// test/src/mln/test/stub_file_source.cpp, the FileSource of the C++ unit
// tests (here for the tests of every crate that needs one).
//
// The asynchronous mode polls the pending requests from an AsyncTask on the
// current RunLoop, re-armed every millisecond while requests are pending
// (C++: a 1 ms repeating `util::Timer`, a TODO(port) of the RunLoop). The
// stub lives on the thread that created it; requests must come from that
// thread.
//
// TODO(port): ResponseType::Manual (`respondToAll`), the style/source/tile/
// glyphs/image responders beyond the generic `set_response`.

use crate::async_request::AsyncRequest;
use crate::file_source::{Callback, FileSource};
use crate::resource::{Kind, Resource};
use crate::resource_options::ResourceOptions;
use crate::response::Response;
use maplibre_native_base::Value;
use mln_actor::{AsyncTask, LocalObject};
use mln_util::client_options::ClientOptions;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::{Rc, Weak};
use std::sync::Mutex;

/// `StubFileSource::ResponseType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponseType {
    Asynchronous,
    Synchronous,
}

/// A responder: `std::function<std::optional<Response>(const Resource&)>`.
pub type Responder = Rc<dyn Fn(&Resource) -> Option<Response>>;

#[derive(Default)]
struct State {
    pending: BTreeMap<u64, (Resource, Responder, Rc<RefCell<Callback>>)>,
    next_id: u64,
    timer: Option<AsyncTask>,
    responders: HashMap<Kind, Responder>,
    properties: HashMap<String, Value>,
}

/// `StubFileSource`.
pub struct StubFileSource {
    type_: ResponseType,
    state: LocalObject<RefCell<State>>,
    resource_options: Mutex<ResourceOptions>,
    client_options: Mutex<ClientOptions>,
}

struct StubFileRequest {
    id: u64,
    state: Weak<RefCell<State>>,
}

impl AsyncRequest for StubFileRequest {}

impl Drop for StubFileRequest {
    /// `~StubFileRequest() { fileSource.remove(this); }`
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            state.borrow_mut().pending.remove(&self.id);
        }
    }
}

fn unexpected(resource: &Resource) -> Option<Response> {
    // C++ throws std::runtime_error("unexpected ... request").
    panic!("unexpected {:?} request: {}", resource.kind, resource.url)
}

impl StubFileSource {
    pub fn new(type_: ResponseType) -> Self {
        let state = Rc::new(RefCell::new(State::default()));
        if type_ == ResponseType::Asynchronous {
            let weak = Rc::downgrade(&state);
            let timer = AsyncTask::new(move || {
                if let Some(state) = weak.upgrade() {
                    Self::poll(&state);
                }
            });
            state.borrow_mut().timer = Some(timer);
        }
        StubFileSource {
            type_,
            state: LocalObject::from_rc(state),
            resource_options: Mutex::new(ResourceOptions::default()),
            client_options: Mutex::new(ClientOptions::default()),
        }
    }

    fn state(&self) -> Rc<RefCell<State>> {
        self.state.get().expect("StubFileSource used from another thread")
    }

    /// Sets the responder for one resource kind (`styleResponse`,
    /// `spriteJSONResponse`, `spriteImageResponse`, ...).
    pub fn set_response(&self, kind: Kind, responder: impl Fn(&Resource) -> Option<Response> + 'static) {
        self.state().borrow_mut().responders.insert(kind, Rc::new(responder));
    }

    /// `response(resource)`.
    fn response(&self, kind: Kind) -> Responder {
        let state = self.state();
        let responder = state.borrow().responders.get(&kind).cloned();
        responder.unwrap_or_else(|| Rc::new(unexpected))
    }

    fn schedule_poll(state: &Rc<RefCell<State>>) {
        if let Some(timer) = &state.borrow().timer {
            timer.send();
        }
    }

    /// The timer callback.
    fn poll(state: &Rc<RefCell<State>>) {
        // Explicit copy to avoid iterator invalidation if ~StubFileRequest
        // gets called within the loop.
        let pending: Vec<(u64, Resource, Responder, Rc<RefCell<Callback>>)> = state
            .borrow()
            .pending
            .iter()
            .map(|(id, (resource, responder, callback))| (*id, resource.clone(), responder.clone(), callback.clone()))
            .collect();
        for (id, resource, responder, callback) in pending {
            if !state.borrow().pending.contains_key(&id) {
                continue;
            }
            if let Some(res) = responder(&resource) {
                // This must be before calling the callback, because it's
                // possible that the callback could deallocate the
                // AsyncRequest itself.
                if res.error.is_none() {
                    state.borrow_mut().pending.remove(&id);
                }

                (callback.borrow_mut())(res);
            }
        }
        if !state.borrow().pending.is_empty() {
            // The next timer tick.
            std::thread::sleep(std::time::Duration::from_millis(1));
            Self::schedule_poll(state);
        }
    }
}

impl FileSource for StubFileSource {
    fn request(&self, resource: &Resource, mut callback: Callback) -> Box<dyn AsyncRequest> {
        let state = self.state();
        let id = {
            let mut s = state.borrow_mut();
            s.next_id += 1;
            s.next_id
        };
        let req = StubFileRequest {
            id,
            state: Rc::downgrade(&state),
        };
        let responder = self.response(resource.kind);
        if self.type_ == ResponseType::Synchronous {
            if let Some(res) = responder(resource) {
                callback(res);
            }
        } else {
            state
                .borrow_mut()
                .pending
                .insert(id, (resource.clone(), responder, Rc::new(RefCell::new(callback))));
            Self::schedule_poll(&state);
        }
        Box::new(req)
    }

    fn can_request(&self, _resource: &Resource) -> bool {
        true
    }

    fn set_property(&self, key: &str, value: &Value) {
        self.state().borrow_mut().properties.insert(key.to_owned(), value.clone());
    }

    fn get_property(&self, key: &str) -> Value {
        self.state().borrow().properties.get(key).cloned().unwrap_or(Value::Null)
    }

    fn set_resource_options(&self, options: ResourceOptions) {
        *self.resource_options.lock().unwrap() = options;
    }

    fn get_resource_options(&self) -> ResourceOptions {
        self.resource_options.lock().unwrap().clone()
    }

    fn set_client_options(&self, options: ClientOptions) {
        *self.client_options.lock().unwrap() = options;
    }

    fn get_client_options(&self) -> ClientOptions {
        self.client_options.lock().unwrap().clone()
    }
}
