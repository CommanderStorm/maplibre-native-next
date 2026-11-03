// Port-only: where the thread pools' threads come from.
//
// Natively (and on Emscripten, with pthreads) they are std threads. A
// wasm32-unknown-unknown build with shared memory (`+atomics`, the threaded
// browser build) has no `std::thread::spawn`: the host starts a Web Worker on
// the same module and memory for each thread, through the spawner it
// registers with `set_spawner`, and that worker runs the closure.
// (wasm32-unknown-unknown without atomics has no threads at all; see
// thread_pool.rs.)

#[cfg(not(all(target_family = "wasm", target_os = "unknown", target_feature = "atomics")))]
mod imp {
    pub type JoinHandle = std::thread::JoinHandle<()>;

    pub fn spawn(name: String, f: impl FnOnce() + Send + 'static) -> JoinHandle {
        std::thread::Builder::new().name(name).spawn(f).expect("failed to spawn worker thread")
    }
}

#[cfg(all(target_family = "wasm", target_os = "unknown", target_feature = "atomics"))]
mod imp {
    use std::sync::{Arc, Condvar, Mutex, OnceLock};

    /// Starts a thread named `name` that runs `f` (on a Web Worker that
    /// shares this module's memory).
    pub type Spawner = fn(name: String, f: Box<dyn FnOnce() + Send + 'static>);

    static SPAWNER: OnceLock<Spawner> = OnceLock::new();

    /// Registers the host's spawner; before any scheduler starts.
    pub fn set_spawner(spawner: Spawner) {
        let _ = SPAWNER.set(spawner);
    }

    /// `std::thread::JoinHandle<()>` for a spawned closure.
    pub struct JoinHandle {
        done: Arc<(Mutex<bool>, Condvar)>,
    }

    impl JoinHandle {
        /// Waits for the closure to return (only where blocking is allowed:
        /// not on a browser's main thread).
        pub fn join(self) -> Result<(), ()> {
            let (done, condvar) = &*self.done;
            let mut done = done.lock().unwrap();
            while !*done {
                done = condvar.wait(done).unwrap();
            }
            Ok(())
        }
    }

    pub fn spawn(name: String, f: impl FnOnce() + Send + 'static) -> JoinHandle {
        let done = Arc::new((Mutex::new(false), Condvar::new()));
        let finished = done.clone();
        let spawner = SPAWNER.get().expect("mln_actor::spawn::set_spawner must register a thread spawner first");
        spawner(
            name,
            Box::new(move || {
                f();
                let (done, condvar) = &*finished;
                *done.lock().unwrap() = true;
                condvar.notify_all();
            }),
        );
        JoinHandle { done }
    }
}

pub use imp::*;
