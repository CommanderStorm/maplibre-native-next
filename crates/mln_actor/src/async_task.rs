// Ports include/mln/util/async_task.hpp and
// platform/default/src/mln/util/async_task.cpp: a coalescing wake-up whose
// callback runs on the RunLoop of the thread that created it.

use crate::run_loop::{AsyncHandle, RunLoop};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// `util::AsyncTask`.
pub struct AsyncTask {
    handle: Rc<AsyncHandle>,
}

impl AsyncTask {
    /// `AsyncTask(std::function<void()>&&)`, on the current thread's RunLoop.
    pub fn new(task: impl FnMut() + 'static) -> Self {
        let run_loop = RunLoop::get();
        let handle = Rc::new(AsyncHandle {
            pending: Arc::new(AtomicBool::new(false)),
            task: RefCell::new(Box::new(task)),
            core: run_loop.core(),
        });
        run_loop.register(&handle);
        AsyncTask { handle }
    }

    /// `send()`: `uv_async_send` does the call coalescing (and wakes a
    /// blocking `RunLoop::run`).
    pub fn send(&self) {
        self.handle.pending.store(true, Ordering::SeqCst);
        self.handle.core.wake();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn coalesces() {
        let run_loop = RunLoop::new();
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        let task = AsyncTask::new(move || c.set(c.get() + 1));
        task.send();
        task.send();
        run_loop.run_once();
        assert_eq!(count.get(), 1);
        run_loop.run_once();
        assert_eq!(count.get(), 1);
        task.send();
        run_loop.run_once();
        assert_eq!(count.get(), 2);
        drop(task);
        run_loop.run_once();
    }
}
