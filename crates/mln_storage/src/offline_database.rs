// Ports platform/default/include/mln/storage/offline_database.hpp and
// platform/default/src/mln/storage/offline_database.cpp: opening (incl.
// schema creation and migrations), the ambient cache read path (`get`,
// `getResource`, `getTile`, `hasResource`, `hasTile`), read-only reopening
// and the error handling around them.
//
// C++ throws and catches `mapbox::sqlite::Exception`, `util::IOException`
// and `std::runtime_error`; here they are the `DatabaseError` variants,
// returned through `Result` and handled by `handle_error` as in C++.
//
// TODO(port): the write path (`put`, `putInternal`, `putResource`,
// `putTile`, eviction, ambient cache size), offline regions (`listRegions`,
// `createRegion`, ..., `mergeDatabase`, `markUsedResources`), `pack`,
// `invalidateAmbientCache`, `clearAmbientCache`, the Mapbox tile count limit.

use crate::offline_schema::OFFLINE_DATABASE_SCHEMA;
use crate::resource::{Kind, Resource, TileData};
use crate::response::Response;
use crate::sqlite3::{
    self, Database, Exception, OpenFlag, extended_result_code, get_bool, get_optional_bytes, get_optional_i64,
    get_optional_string, result_code,
};
use mln_platform_default_image::compression::decompress;
use mln_util::chrono::{Timestamp, now};
use mln_util::constants::DEFAULT_MAX_CACHE_SIZE;
use mln_util::logging::{Event, EventSeverity, Log};
use mln_util::tile_server_options::TileServerOptions;
use rusqlite::params;
use std::sync::Arc;

/// `util::mapbox::DEFAULT_OFFLINE_TILE_COUNT_LIMIT`.
pub const DEFAULT_OFFLINE_TILE_COUNT_LIMIT: u64 = 6000;

/// The exceptions the C++ code throws and `handleError` catches.
#[derive(Clone, Debug)]
pub enum DatabaseError {
    Sqlite(Exception),
    /// `util::IOException { code }` with its `what()`.
    Io { code: i32, message: String },
    /// `std::runtime_error`.
    Runtime(String),
}

impl From<Exception> for DatabaseError {
    fn from(e: Exception) -> Self {
        DatabaseError::Sqlite(e)
    }
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(e: rusqlite::Error) -> Self {
        DatabaseError::Sqlite(e.into())
    }
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::Sqlite(e) => f.write_str(&e.message),
            DatabaseError::Io { message, .. } => f.write_str(message),
            DatabaseError::Runtime(message) => f.write_str(message),
        }
    }
}

type DbResult<T> = Result<T, DatabaseError>;

/// `util::deleteFile` (platform/default/src/mln/util/io.cpp).
fn delete_file(filename: &str) -> DbResult<()> {
    match std::fs::remove_file(filename) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => {
            let code = e.raw_os_error().unwrap_or(0);
            // IOException(err, msg): msg + ": " + std::generic_category().message(err)
            let what = std::io::Error::from_raw_os_error(code).to_string();
            let what = what.split(" (os error").next().unwrap_or(&what).to_owned();
            Err(DatabaseError::Io {
                code,
                message: format!("Could not delete file {filename}: {what}"),
            })
        }
    }
}

/// `OfflineDatabase`.
pub struct OfflineDatabase {
    path: String,
    db: Option<Database>,

    maximum_ambient_cache_size: u64,
    offline_mapbox_tile_count_limit: u64,

    tile_server_options: TileServerOptions,

    autopack: bool,
    read_only: bool,
}

impl OfflineDatabase {
    /// Single-point transformer for offline storage keys. For sources where
    /// the URL is not enough to guarantee uniqueness (eg. PMTiles where all
    /// requests resolve to the same URL), this function creates a synthetic
    /// key based on the requested data range.
    pub fn cache_key(resource: &Resource) -> String {
        let Some((first, second)) = resource.data_range else {
            return resource.url.clone();
        };
        let sep = if resource.url.contains('?') { '&' } else { '?' };
        format!("{}{}_mlnRange={}-{}", resource.url, sep, first, second)
    }

    /// `OfflineDatabase(std::string path, const TileServerOptions&)`.
    pub fn new(path: impl Into<String>, options: &TileServerOptions) -> Self {
        let mut db = OfflineDatabase {
            path: path.into(),
            db: None,
            maximum_ambient_cache_size: DEFAULT_MAX_CACHE_SIZE,
            offline_mapbox_tile_count_limit: DEFAULT_OFFLINE_TILE_COUNT_LIMIT,
            tile_server_options: options.clone(),
            autopack: true,
            read_only: false,
        };
        if let Err(e) = db.initialize() {
            db.handle_error(&e, "open database");
        }
        // Assume that we can't open the database right now and work with an
        // empty database object.
        db
    }

    fn initialize(&mut self) -> DbResult<()> {
        assert!(self.db.is_none());

        if self.read_only {
            let db = Database::open(&self.path, OpenFlag::ReadOnly)?;
            self.db = Some(db);
            let db = self.db.as_ref().unwrap();
            db.set_busy_timeout(i64::MAX)?;
            db.exec("PRAGMA foreign_keys = ON")?;
            return Ok(());
        }

        let db = Database::open(&self.path, OpenFlag::ReadWriteCreate)?;
        self.db = Some(db);
        {
            let db = self.db.as_ref().unwrap();
            db.set_busy_timeout(i64::MAX)?;
            db.exec("PRAGMA foreign_keys = ON")?;
        }

        let user_version = self.get_pragma_i64("PRAGMA user_version")?;
        match user_version {
            0 | 1 => {
                // Newly created database, or old cache-only database; remove
                // old table if it exists.
                self.remove_old_cache_table()?;
                self.create_schema()?;
                Ok(())
            }
            2..=6 => {
                if user_version == 2 {
                    self.migrate_to_version3()?;
                }
                // 3: Removed migration, see below.
                if user_version <= 4 {
                    self.migrate_to_version5()?;
                }
                if user_version <= 5 {
                    self.migrate_to_version6()?;
                }
                // 6: Happy path; we're done
                Ok(())
            }
            _ => {
                // Downgrade: delete the database and try to reinitialize.
                self.remove_existing()?;
                self.initialize()
            }
        }
    }

    /// `changePath`.
    pub fn change_path(&mut self, path: &str) {
        Log::info(Event::Database, "Changing the database path.");
        self.cleanup();
        self.path = path.to_owned();
        if let Err(e) = self.initialize() {
            // C++ lets this propagate to the caller (the database thread).
            self.handle_error(&e, "open database");
        }
    }

    fn cleanup(&mut self) {
        // Deleting these SQLite objects may result in exceptions (rusqlite
        // closes on drop and ignores the result).
        self.db = None;
    }

    fn disabled(&mut self) -> bool {
        if self.maximum_ambient_cache_size != 0 {
            return false;
        }

        // `auto regions = listRegions(); return !regions || regions.value().empty();`
        // TODO(port): listRegions (offline regions); counting them gives the
        // same answer, except that a region whose definition fails to parse
        // makes C++ listRegions fail (disabled).
        let count = (|| -> DbResult<i64> {
            let db = self.get_db()?;
            let mut stmt = db.conn().prepare_cached("SELECT COUNT(*) FROM regions")?;
            Ok(stmt.query_row([], |row| row.get::<_, i64>(0))?)
        })();
        match count {
            Ok(count) => count == 0,
            Err(e) => {
                self.handle_error(&e, "list regions");
                true
            }
        }
    }

    fn handle_error(&mut self, error: &DatabaseError, action: &str) {
        match error {
            DatabaseError::Sqlite(ex) => {
                if ex.code == result_code::NOT_A_DB
                    || ex.code == result_code::CORRUPT
                    || (ex.code == result_code::READ_ONLY
                        && ex.extended_code == extended_result_code::READ_ONLY_DB_MOVED)
                {
                    // The database was corruped, moved away, or deleted.
                    // We're going to start fresh with a clean slate for the
                    // next operation.
                    Log::record(
                        EventSeverity::Error,
                        Event::Database,
                        ex.code as i64,
                        &format!("Can't {action}: {}", ex.message),
                    );
                    if let Err(io_ex) = self.remove_existing() {
                        self.handle_error(&io_ex, action);
                    }
                } else {
                    // We treat the error as temporary, and pretend we have an
                    // inaccessible DB.
                    Log::record(
                        EventSeverity::Warning,
                        Event::Database,
                        ex.code as i64,
                        &format!("Can't {action}: {}", ex.message),
                    );
                }
            }
            DatabaseError::Io { code, message } => {
                // We failed to delete the database file.
                Log::record(
                    EventSeverity::Error,
                    Event::Database,
                    *code as i64,
                    &format!("Can't {action}: {message}"),
                );
            }
            DatabaseError::Runtime(message) => {
                Log::error(Event::Database, format!("Can't {action}: {message}"));
            }
        }
    }

    fn remove_existing(&mut self) -> DbResult<()> {
        Log::warning(Event::Database, "Removing existing incompatible offline database");

        let filename = match &self.db {
            Some(db) => db.get_filename(),
            None => self.path.clone(),
        };
        self.db = None;

        if !filename.is_empty() {
            delete_file(&filename)?;
        }
        Ok(())
    }

    fn remove_old_cache_table(&mut self) -> DbResult<()> {
        self.check_flags()?;
        self.get_db()?.exec("DROP TABLE IF EXISTS http_cache")?;
        if self.autopack {
            self.vacuum()?;
        }
        Ok(())
    }

    fn create_schema(&mut self) -> DbResult<()> {
        self.check_flags()?;

        self.vacuum()?;
        let db = self.get_db()?;
        db.exec("PRAGMA journal_mode = DELETE")?;
        db.exec("PRAGMA synchronous = FULL")?;
        // mapbox::sqlite::Transaction transaction(*db);
        db.exec("BEGIN DEFERRED TRANSACTION")?;
        let result = db.exec(OFFLINE_DATABASE_SCHEMA).and_then(|()| db.exec("PRAGMA user_version = 6"));
        match result {
            Ok(()) => db.exec("COMMIT TRANSACTION")?,
            Err(e) => {
                let _ = db.exec("ROLLBACK TRANSACTION");
                return Err(e.into());
            }
        }
        Ok(())
    }

    fn migrate_to_version3(&mut self) -> DbResult<()> {
        self.check_flags()?;
        self.vacuum()?;
        self.get_db()?.exec("PRAGMA user_version = 3")?;
        Ok(())
    }

    // Schema version 4 was WAL journal + NORMAL sync. It was reverted during
    // pre-release development and the migration was removed entirely to
    // avoid potential conflicts from quickly (and needlessly) switching
    // journal and sync modes.
    //
    // See: https://github.com/mapbox/mapbox-gl-native/pull/6320

    fn migrate_to_version5(&mut self) -> DbResult<()> {
        self.check_flags()?;
        let db = self.get_db()?;
        db.exec("PRAGMA journal_mode = DELETE")?;
        db.exec("PRAGMA synchronous = FULL")?;
        db.exec("PRAGMA user_version = 5")?;
        Ok(())
    }

    fn migrate_to_version6(&mut self) -> DbResult<()> {
        self.check_flags()?;
        let db = self.get_db()?;
        db.exec("BEGIN DEFERRED TRANSACTION")?;
        let result = db
            .exec("ALTER TABLE resources ADD COLUMN must_revalidate INTEGER NOT NULL DEFAULT 0")
            .and_then(|()| db.exec("ALTER TABLE tiles ADD COLUMN must_revalidate INTEGER NOT NULL DEFAULT 0"))
            .and_then(|()| db.exec("PRAGMA user_version = 6"));
        match result {
            Ok(()) => db.exec("COMMIT TRANSACTION")?,
            Err(e) => {
                let _ = db.exec("ROLLBACK TRANSACTION");
                return Err(e.into());
            }
        }
        Ok(())
    }

    fn vacuum(&mut self) -> DbResult<()> {
        self.check_flags()?;
        if self.get_pragma_i64("PRAGMA auto_vacuum")? != 2
        /*INCREMENTAL*/
        {
            let db = self.get_db()?;
            db.exec("PRAGMA auto_vacuum = INCREMENTAL")?;
            db.exec("VACUUM")?;
        } else {
            self.get_db()?.exec("PRAGMA incremental_vacuum")?;
        }
        Ok(())
    }

    fn check_flags(&self) -> DbResult<()> {
        if self.read_only {
            return Err(DatabaseError::Runtime("Cannot modify database in read-only mode".to_owned()));
        }
        Ok(())
    }

    /// `getStatement`'s `if (!db) initialize();`.
    fn get_db(&mut self) -> DbResult<&Database> {
        if self.db.is_none() {
            self.initialize()?;
        }
        Ok(self.db.as_ref().expect("initialized database"))
    }

    /// `getPragma<int64_t>(sql)`.
    fn get_pragma_i64(&mut self, sql: &str) -> DbResult<i64> {
        let db = self.get_db()?;
        let mut stmt = db.conn().prepare_cached(sql)?;
        let mut rows = stmt.query([])?;
        match rows.next()? {
            Some(row) => Ok(sqlite3::get_i64(row, 0)?),
            None => Ok(0),
        }
    }

    /// `get(const Resource&)`.
    pub fn get(&mut self, resource: &Resource) -> Option<Response> {
        let result = (|| -> DbResult<Option<Response>> {
            if self.disabled() {
                return Ok(None);
            }

            let result = self.get_internal(resource)?;
            Ok(result.map(|(response, _size)| response))
        })();
        match result {
            Ok(response) => response,
            Err(e) => {
                self.handle_error(&e, "read resource");
                None
            }
        }
    }

    fn get_internal(&mut self, resource: &Resource) -> DbResult<Option<(Response, u64)>> {
        if resource.kind == Kind::Tile {
            let tile_data = resource.tile_data.as_ref().expect("tile resource without tile data");
            self.get_tile(tile_data)
        } else {
            self.get_resource(resource)
        }
    }

    /// `hasInternal` (exposed for the ported tests).
    pub fn has_internal(&mut self, resource: &Resource) -> DbResult<Option<i64>> {
        if resource.kind == Kind::Tile {
            let tile_data = resource.tile_data.as_ref().expect("tile resource without tile data");
            self.has_tile(tile_data)
        } else {
            self.has_resource(resource)
        }
    }

    /// `put(const Resource&, const Response&)`: (inserted, stored size).
    pub fn put(&mut self, _resource: &Resource, _response: &Response) -> (bool, u64) {
        if self.read_only {
            return (false, 0);
        }
        // TODO(port): the write path (putInternal, eviction, ambient cache
        // size). Nothing calls it on the cache-only read path.
        (false, 0)
    }

    fn update_accessed(&mut self, sql: &str, bind: &dyn Fn(&mut rusqlite::CachedStatement<'_>, Timestamp) -> rusqlite::Result<usize>) -> DbResult<()> {
        // Update accessed timestamp used for LRU eviction.
        if self.read_only {
            return Ok(());
        }
        let result = (|| -> DbResult<()> {
            let db = self.get_db()?;
            let mut stmt = db.conn().prepare_cached(sql)?;
            bind(&mut stmt, now())?;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(DatabaseError::Sqlite(ex)) => {
                if ex.code == result_code::NOT_A_DB || ex.code == result_code::CORRUPT {
                    return Err(DatabaseError::Sqlite(ex));
                }
                // If we don't have any indication that the database is
                // corrupt, continue as usual.
                Log::record(
                    EventSeverity::Warning,
                    Event::Database,
                    ex.code as i64,
                    &format!("Can't update timestamp: {}", ex.message),
                );
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Reads `etag, expires, must_revalidate, modified, data, compressed` of
    /// the first row, as getResource/getTile do.
    fn read_response(row: &rusqlite::Row<'_>) -> DbResult<(Response, u64)> {
        let mut response = Response::new();
        let mut size = 0u64;

        response.etag = get_optional_string(row, 0)?;
        response.expires = get_optional_i64(row, 1)?.map(Timestamp::from_secs);
        response.must_revalidate = get_bool(row, 2)?;
        response.modified = get_optional_i64(row, 3)?.map(Timestamp::from_secs);

        let data = get_optional_bytes(row, 4)?;
        match data {
            None => response.no_content = true,
            Some(data) => {
                if get_bool(row, 5)? {
                    let decompressed = decompress(&data).map_err(DatabaseError::Runtime)?;
                    response.data = Some(Arc::new(decompressed));
                } else {
                    response.data = Some(Arc::new(data.clone()));
                }
                size = data.len() as u64;
            }
        }

        Ok((response, size))
    }

    fn get_resource(&mut self, resource: &Resource) -> DbResult<Option<(Response, u64)>> {
        let key = Self::cache_key(resource);
        self.update_accessed("UPDATE resources SET accessed = ?1 WHERE url = ?2", &|stmt, now| {
            stmt.execute(params![now.secs_since_epoch(), key])
        })?;

        let db = self.get_db()?;
        let mut stmt = db.conn().prepare_cached(
            //        0      1            2            3       4      5
            "SELECT etag, expires, must_revalidate, modified, data, compressed \
             FROM resources \
             WHERE url = ?",
        )?;
        let mut rows = stmt.query(params![key])?;
        match rows.next()? {
            None => Ok(None),
            Some(row) => Self::read_response(row).map(Some),
        }
    }

    fn has_resource(&mut self, resource: &Resource) -> DbResult<Option<i64>> {
        let key = Self::cache_key(resource);
        let db = self.get_db()?;
        let mut stmt = db.conn().prepare_cached("SELECT length(data) FROM resources WHERE url = ?")?;
        let mut rows = stmt.query(params![key])?;
        match rows.next()? {
            None => Ok(None),
            Some(row) => Ok(get_optional_i64(row, 0)?),
        }
    }

    fn get_tile(&mut self, tile: &TileData) -> DbResult<Option<(Response, u64)>> {
        self.update_accessed(
            "UPDATE tiles \
             SET accessed       = ?1 \
             WHERE url_template = ?2 \
               AND pixel_ratio  = ?3 \
               AND x            = ?4 \
               AND y            = ?5 \
               AND z            = ?6 ",
            &|stmt, now| {
                stmt.execute(params![
                    now.secs_since_epoch(),
                    tile.url_template,
                    tile.pixel_ratio,
                    tile.x,
                    tile.y,
                    tile.z
                ])
            },
        )?;

        let db = self.get_db()?;
        let mut stmt = db.conn().prepare_cached(
            //        0      1           2,            3,      4,      5
            "SELECT etag, expires, must_revalidate, modified, data, compressed \
             FROM tiles \
             WHERE url_template = ?1 \
               AND pixel_ratio  = ?2 \
               AND x            = ?3 \
               AND y            = ?4 \
               AND z            = ?5 ",
        )?;
        let mut rows = stmt.query(params![tile.url_template, tile.pixel_ratio, tile.x, tile.y, tile.z])?;
        match rows.next()? {
            None => Ok(None),
            Some(row) => Self::read_response(row).map(Some),
        }
    }

    fn has_tile(&mut self, tile: &TileData) -> DbResult<Option<i64>> {
        let db = self.get_db()?;
        let mut stmt = db.conn().prepare_cached(
            "SELECT length(data) \
             FROM tiles \
             WHERE url_template = ?1 \
               AND pixel_ratio  = ?2 \
               AND x            = ?3 \
               AND y            = ?4 \
               AND z            = ?5 ",
        )?;
        let mut rows = stmt.query(params![tile.url_template, tile.pixel_ratio, tile.x, tile.y, tile.z])?;
        match rows.next()? {
            None => Ok(None),
            Some(row) => Ok(get_optional_i64(row, 0)?),
        }
    }

    /// `runPackDatabaseAutomatically`.
    pub fn run_pack_database_automatically(&mut self, autopack: bool) {
        self.autopack = autopack;
    }

    /// `setOfflineMapboxTileCountLimit`.
    pub fn set_offline_mapbox_tile_count_limit(&mut self, limit: u64) {
        self.offline_mapbox_tile_count_limit = limit;
    }

    /// `getOfflineMapboxTileCountLimit`.
    pub fn get_offline_mapbox_tile_count_limit(&self) -> u64 {
        self.offline_mapbox_tile_count_limit
    }

    pub fn tile_server_options(&self) -> &TileServerOptions {
        &self.tile_server_options
    }

    /// `reopenDatabaseReadOnly(bool)`.
    pub fn reopen_database_read_only(&mut self, read_only: bool) {
        if self.read_only == read_only {
            return;
        }
        self.cleanup();
        self.read_only = read_only;
        if let Err(e) = self.initialize() {
            self.handle_error(&e, "reopen database read-only");
        }
    }

    /// `resetDatabase()`: C++ returns the exception (or null).
    pub fn reset_database(&mut self) -> Option<DatabaseError> {
        let result = self.remove_existing().and_then(|()| self.initialize());
        match result {
            Ok(()) => None,
            Err(e) => {
                self.handle_error(&e, "reset database");
                Some(e)
            }
        }
    }
}
