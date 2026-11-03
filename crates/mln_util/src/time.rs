// Port-only: the clocks of std::time, from the web-time crate. On
// wasm32-unknown-unknown (the browser build) std's `Instant::now()` and
// `SystemTime::now()` panic, as there is no OS clock; web-time reads
// `performance.now()` and `Date.now()` there and re-exports std::time
// everywhere else.

pub use web_time::{Instant, SystemTime, UNIX_EPOCH};
