// Ports maplibre-native-ffi src/handles/handle_table.cpp in spirit: every
// object the C API hands out is a nonzero 64-bit handle into one
// process-wide table, so a stale or foreign handle is an error, never a
// dangling pointer.

use crate::delivery::DeliveryQueue;
use crate::map::MapShared;
use crate::runtime::RuntimeShared;
use crate::session::{FrameBatch, SessionShared};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

#[derive(Clone)]
pub enum Object {
    Runtime(Arc<RuntimeShared>),
    Map(Arc<MapShared>),
    Session(Arc<SessionShared>),
    Queue(Arc<DeliveryQueue>),
    // Held so the batch's results stay valid until its release.
    #[allow(dead_code)]
    FrameBatch(Arc<FrameBatch>),
    OffscreenDevice,
}

static NEXT: AtomicU64 = AtomicU64::new(1);
static TABLE: LazyLock<Mutex<HashMap<u64, Object>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// A fresh handle, not yet in the table.
pub fn reserve() -> u64 {
    NEXT.fetch_add(1, Ordering::Relaxed)
}

pub fn insert(object: Object) -> u64 {
    let handle = reserve();
    insert_at(handle, object);
    handle
}

pub fn insert_at(handle: u64, object: Object) {
    TABLE.lock().unwrap().insert(handle, object);
}

pub fn get(handle: u64) -> Option<Object> {
    if handle == 0 {
        return None;
    }
    TABLE.lock().unwrap().get(&handle).cloned()
}

pub fn remove(handle: u64) -> Option<Object> {
    TABLE.lock().unwrap().remove(&handle)
}

pub fn runtime(handle: u64) -> Option<Arc<RuntimeShared>> {
    match get(handle) {
        Some(Object::Runtime(runtime)) => Some(runtime),
        _ => None,
    }
}

pub fn map(handle: u64) -> Option<Arc<MapShared>> {
    match get(handle) {
        Some(Object::Map(map)) => Some(map),
        _ => None,
    }
}

pub fn session(handle: u64) -> Option<Arc<SessionShared>> {
    match get(handle) {
        Some(Object::Session(session)) => Some(session),
        _ => None,
    }
}
