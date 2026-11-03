// Ports platform/default/include/mln/storage/sqlite3.hpp and
// platform/default/src/mln/storage/sqlite3.cpp (`mapbox::sqlite`) over
// rusqlite (the same bundled SQLite engine).
//
// `Statement`/`Query` map to rusqlite's cached statements; the typed
// `Query::get<T>` overloads are the `get_*` functions below, which apply
// SQLite's own column conversions (`sqlite3_column_int64` of a text value
// parses it, `sqlite3_column_blob` of a number formats it, ...).
//
// TODO(port): `setTempPath`, `Transaction` (write path), the debug
// `SQLITE_CONFIG_LOG` hook.

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags, Row};

/// `mapbox::sqlite::ResultCode` values (`SQLITE_*`).
pub mod result_code {
    pub const OK: i32 = 0;
    pub const ERROR: i32 = 1;
    pub const READ_ONLY: i32 = 8;
    pub const CORRUPT: i32 = 11;
    pub const CANT_OPEN: i32 = 14;
    pub const NOT_A_DB: i32 = 26;
}

/// `mapbox::sqlite::ExtendedResultCode` values (`err >> 8`).
pub mod extended_result_code {
    pub const UNKNOWN: i32 = 0;
    /// `SQLITE_READONLY_DBMOVED >> 8`.
    pub const READ_ONLY_DB_MOVED: i32 = 4;
}

/// `mapbox::sqlite::OpenFlag`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenFlag {
    ReadOnly = 0x0000_0001,
    ReadWriteCreate = 0x0000_0006,
}

/// `mapbox::sqlite::Exception`: `code` is the (extended) result code as C++
/// stores it (`static_cast<ResultCode>(err)`), `extended_code` is `err >> 8`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exception {
    pub code: i32,
    pub extended_code: i32,
    pub message: String,
}

impl Exception {
    pub fn new(err: i32, message: impl Into<String>) -> Self {
        Exception {
            code: err,
            extended_code: err >> 8,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for Exception {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<rusqlite::Error> for Exception {
    fn from(e: rusqlite::Error) -> Self {
        match &e {
            rusqlite::Error::SqliteFailure(error, message) => Exception::new(
                error.extended_code,
                message.clone().unwrap_or_else(|| error.to_string()),
            ),
            _ => Exception::new(result_code::ERROR, e.to_string()),
        }
    }
}

/// `mapbox::sqlite::Database`.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// `Database::open(filename, flags)` (`sqlite3_open_v2(filename, flags |
    /// SQLITE_OPEN_URI)`, extended result codes on).
    pub fn open(filename: &str, flags: OpenFlag) -> Result<Database, Exception> {
        let flags = OpenFlags::from_bits_retain(flags as i32) | OpenFlags::SQLITE_OPEN_URI;
        let conn = Connection::open_with_flags(filename, flags)?;
        Ok(Database { conn })
    }

    /// `getFilename()`.
    pub fn get_filename(&self) -> String {
        self.conn.path().unwrap_or("").to_owned()
    }

    /// `setBusyTimeout(std::chrono::milliseconds)` (clamped to `int`).
    pub fn set_busy_timeout(&self, timeout_ms: i64) -> Result<(), Exception> {
        let ms = timeout_ms.clamp(0, i32::MAX as i64) as u64;
        self.conn.busy_timeout(std::time::Duration::from_millis(ms))?;
        Ok(())
    }

    /// `exec(sql)`.
    pub fn exec(&self, sql: &str) -> Result<(), Exception> {
        self.conn.execute_batch(sql)?;
        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

/// `sqlite3_column_int64`.
pub fn get_i64(row: &Row<'_>, offset: usize) -> Result<i64, Exception> {
    Ok(match row.get_ref(offset)? {
        ValueRef::Null => 0,
        ValueRef::Integer(i) => i,
        ValueRef::Real(r) => real_to_i64(r),
        ValueRef::Text(t) | ValueRef::Blob(t) => text_to_i64(t),
    })
}

/// `sqlite3_column_int` (the low 32 bits of the 64-bit value).
pub fn get_i32(row: &Row<'_>, offset: usize) -> Result<i32, Exception> {
    Ok(get_i64(row, offset)? as i32)
}

/// `Query::get<bool>` (`sqlite3_column_int(...) != 0`... C++ converts the
/// `int` to `bool`).
pub fn get_bool(row: &Row<'_>, offset: usize) -> Result<bool, Exception> {
    Ok(get_i32(row, offset)? != 0)
}

/// `Query::get<std::optional<int64_t>>`.
pub fn get_optional_i64(row: &Row<'_>, offset: usize) -> Result<Option<i64>, Exception> {
    if matches!(row.get_ref(offset)?, ValueRef::Null) {
        return Ok(None);
    }
    get_i64(row, offset).map(Some)
}

/// `Query::get<std::string>` (`sqlite3_column_blob`): the bytes of a text or
/// blob value, a number's text form.
pub fn get_bytes(row: &Row<'_>, offset: usize) -> Result<Vec<u8>, Exception> {
    Ok(match row.get_ref(offset)? {
        ValueRef::Null => Vec::new(),
        ValueRef::Integer(i) => i.to_string().into_bytes(),
        // TODO(port): SQLite formats reals with "%!.15g".
        ValueRef::Real(r) => format!("{r}").into_bytes(),
        ValueRef::Text(t) | ValueRef::Blob(t) => t.to_vec(),
    })
}

/// `Query::get<std::optional<std::string>>`, as bytes.
pub fn get_optional_bytes(row: &Row<'_>, offset: usize) -> Result<Option<Vec<u8>>, Exception> {
    if matches!(row.get_ref(offset)?, ValueRef::Null) {
        return Ok(None);
    }
    get_bytes(row, offset).map(Some)
}

/// `Query::get<std::optional<std::string>>` for text columns (lossy for
/// invalid UTF-8, which C++ would keep).
pub fn get_optional_string(row: &Row<'_>, offset: usize) -> Result<Option<String>, Exception> {
    Ok(get_optional_bytes(row, offset)?.map(|bytes| match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    }))
}

/// SQLite's REAL to INTEGER conversion (`sqlite3RealToI64`: saturating).
fn real_to_i64(r: f64) -> i64 {
    if r.is_nan() {
        0
    } else if r <= i64::MIN as f64 {
        i64::MIN
    } else if r >= i64::MAX as f64 {
        i64::MAX
    } else {
        r as i64
    }
}

/// SQLite's TEXT to INTEGER conversion for `sqlite3_column_int64`: the
/// longest integer prefix after leading spaces, saturating.
fn text_to_i64(t: &[u8]) -> i64 {
    let mut i = 0;
    while i < t.len() && t[i].is_ascii_whitespace() {
        i += 1;
    }
    let mut negative = false;
    if i < t.len() && (t[i] == b'-' || t[i] == b'+') {
        negative = t[i] == b'-';
        i += 1;
    }
    let mut value: i128 = 0;
    while i < t.len() && t[i].is_ascii_digit() {
        value = (value * 10 + (t[i] - b'0') as i128).min(i64::MAX as i128 + 1);
        i += 1;
    }
    if negative {
        value = -value;
    }
    value.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}
