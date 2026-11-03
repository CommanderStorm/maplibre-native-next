// Ported gtests (test/storage/*.test.cpp) and test helpers.

mod database_file_source;
mod main_resource_loader;
mod offline_database;

use mln_util::logging::{Event, EventSeverity, Log, Observer};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// A path in the runfiles tree of the C++ repository (`maplibre+`).
pub fn cpp_path(relative: &str) -> PathBuf {
    let base = std::env::var("TEST_SRCDIR")
        .or_else(|_| std::env::var("RUNFILES_DIR"))
        .expect("TEST_SRCDIR");
    PathBuf::from(base).join("maplibre+").join(relative)
}

/// A fresh path in the test's temporary directory (C++ tests write next to
/// the fixtures in the source tree; the port never writes there).
pub fn temp_path(name: &str) -> String {
    let dir = std::env::var("TEST_TMPDIR").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned());
    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    format!("{dir}/{n}-{name}")
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub severity: EventSeverity,
    pub event: Event,
    pub code: i64,
    pub msg: String,
}

struct Recorder(Arc<Mutex<Vec<(Message, bool)>>>);

impl Observer for Recorder {
    fn on_record(&self, severity: EventSeverity, event: Event, code: i64, msg: &str) -> bool {
        self.0.lock().unwrap().push((
            Message {
                severity,
                event,
                code,
                msg: msg.to_owned(),
            },
            false,
        ));
        true
    }
}

/// `FixtureLog` (test/src/mln/test/fixture_log_observer.cpp) for the
/// calling thread.
pub struct FixtureLog {
    messages: Arc<Mutex<Vec<(Message, bool)>>>,
}

impl FixtureLog {
    pub fn new() -> Self {
        let messages = Arc::new(Mutex::new(Vec::new()));
        Log::set_thread_observer(Some(Box::new(Recorder(messages.clone()))));
        FixtureLog { messages }
    }

    /// `count(message, substring)`: marks the matches checked.
    pub fn count(&self, message: &Message, substring: bool) -> usize {
        let mut messages = self.messages.lock().unwrap();
        let mut count = 0;
        for (m, checked) in messages.iter_mut() {
            if !*checked
                && m.severity == message.severity
                && m.event == message.event
                && m.code == message.code
                && (if substring { m.msg.contains(&message.msg) } else { m.msg == message.msg })
            {
                *checked = true;
                count += 1;
            }
        }
        count
    }

    pub fn unchecked_count(&self) -> usize {
        self.messages.lock().unwrap().iter().filter(|(_, checked)| !checked).count()
    }

    pub fn unchecked(&self) -> Vec<Message> {
        self.messages
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, checked)| !checked)
            .map(|(m, _)| m.clone())
            .collect()
    }
}

impl Drop for FixtureLog {
    fn drop(&mut self) {
        Log::set_thread_observer(None);
    }
}

pub fn error(code: i32, msg: &str) -> Message {
    Message {
        severity: EventSeverity::Error,
        event: Event::Database,
        code: code as i64,
        msg: msg.to_owned(),
    }
}

pub fn warning(code: i64, msg: &str) -> Message {
    Message {
        severity: EventSeverity::Warning,
        event: Event::Database,
        code,
        msg: msg.to_owned(),
    }
}
