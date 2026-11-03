// Ports the tests of test/storage/offline_database.test.cpp that do not
// need the write path (put, regions, eviction; TODO(port) with it). Tests
// run on copies in TEST_TMPDIR. Where a C++ test continues with put/get
// round trips or region deletion, the ported part stops before that (noted
// per test).

use super::{FixtureLog, cpp_path, error, temp_path, warning};
use crate::offline_database::OfflineDatabase;
use crate::resource::{Kind, LoadingMethod, Resource};
use crate::sqlite3::{Database, OpenFlag, result_code};
use mln_util::tile_server_options::TileServerOptions;
use mln_util::tileset::Scheme;

fn tile_server_options() -> TileServerOptions {
    // C++: TileServerOptions::MapTilerConfiguration() (TODO(port)); the read
    // path doesn't use it.
    TileServerOptions::default_configuration()
}

fn fixture_resource() -> Resource {
    Resource::new(Kind::Style, "maptiler://test")
}

fn fixture_tile() -> Resource {
    Resource::tile("maptiler://test", 1.0, 0, 0, 0, Scheme::XYZ, LoadingMethod::All)
}

fn copy_fixture(name: &str) -> String {
    let path = temp_path(name);
    std::fs::copy(cpp_path(&format!("test/fixtures/offline_database/{name}")), &path).unwrap();
    path
}

fn pragma_i64(path: &str, sql: &str) -> i64 {
    let db = Database::open(path, OpenFlag::ReadOnly).unwrap();
    db.conn().query_row(sql, [], |row| row.get::<_, i64>(0)).unwrap()
}

fn pragma_string(path: &str, sql: &str) -> String {
    let db = Database::open(path, OpenFlag::ReadOnly).unwrap();
    db.conn().query_row(sql, [], |row| row.get::<_, String>(0)).unwrap()
}

fn database_table_columns(path: &str, name: &str) -> Vec<String> {
    let db = Database::open(path, OpenFlag::ReadOnly).unwrap();
    let mut stmt = db.conn().prepare(&format!("pragma table_info({name})")).unwrap();
    stmt.query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .map(|c| c.unwrap())
        .collect()
}

const TILES_COLUMNS: [&str; 13] = [
    "id",
    "url_template",
    "pixel_ratio",
    "z",
    "x",
    "y",
    "expires",
    "modified",
    "etag",
    "data",
    "compressed",
    "accessed",
    "must_revalidate",
];
const RESOURCES_COLUMNS: [&str; 10] = [
    "id",
    "url",
    "kind",
    "expires",
    "modified",
    "etag",
    "data",
    "compressed",
    "accessed",
    "must_revalidate",
];

fn removing() -> super::Message {
    warning(-1, "Removing existing incompatible offline database")
}

#[test]
fn create() {
    let log = FixtureLog::new();
    let filename = temp_path("offline.db");
    let mut db = OfflineDatabase::new(&filename, &tile_server_options());
    assert!(db.get(&Resource::new(Kind::Unknown, "maptiler://test")).is_none());

    assert_eq!(0, log.unchecked_count(), "{:?}", log.unchecked());
}

#[test]
fn schema_version() {
    let log = FixtureLog::new();
    let filename = temp_path("offline.db");

    {
        let db = Database::open(&filename, OpenFlag::ReadWriteCreate).unwrap();
        db.set_busy_timeout(1000).unwrap();
        db.exec("PRAGMA user_version = 1").unwrap();
    }

    {
        let _db = OfflineDatabase::new(&filename, &tile_server_options());
    }

    assert_eq!(6, pragma_i64(&filename, "pragma user_version"));

    // TODO(port): the put/get round trip (write path).
    let mut db = OfflineDatabase::new(&filename, &tile_server_options());
    for res in [fixture_resource(), fixture_tile()] {
        assert!(db.get(&res).is_none());
    }

    assert_eq!(0, log.unchecked_count(), "{:?}", log.unchecked());
}

#[test]
fn invalid() {
    let log = FixtureLog::new();
    let filename = temp_path("offline.db");
    std::fs::write(&filename, "this is an invalid file").unwrap();

    let _db = OfflineDatabase::new(&filename, &tile_server_options());
    // Checking two possibilities for the error string because it apparently
    // changes between SQLite versions.
    assert_eq!(
        1,
        log.count(
            &error(result_code::NOT_A_DB, "Can't open database: file is encrypted or is not a database"),
            true
        ) + log.count(&error(result_code::NOT_A_DB, "Can't open database: file is not a database"), true)
    );
    assert_eq!(1, log.count(&removing(), false));
    // TODO(port): the put/get round trip (write path).
}

#[test]
fn cache_key() {
    let bare = Resource::new(Kind::Source, "https://example.com/x.pmtiles");
    assert_eq!("https://example.com/x.pmtiles", OfflineDatabase::cache_key(&bare));

    let mut ranged = Resource::new(Kind::Source, "https://example.com/x.pmtiles");
    ranged.data_range = Some((0, 126));
    assert_eq!("https://example.com/x.pmtiles?_mlnRange=0-126", OfflineDatabase::cache_key(&ranged));

    let mut ranged_with_query = Resource::new(Kind::Source, "https://example.com/x.pmtiles?token=abc");
    ranged_with_query.data_range = Some((200, 399));
    assert_eq!(
        "https://example.com/x.pmtiles?token=abc&_mlnRange=200-399",
        OfflineDatabase::cache_key(&ranged_with_query)
    );
}

// TODO(port): MigrateFromV2Schema compares page counts after deleting its
// regions (regions API).

#[test]
fn migrate_from_v3_schema() {
    let log = FixtureLog::new();
    let filename = copy_fixture("v3.db");
    {
        let _db = OfflineDatabase::new(&filename, &tile_server_options());
        // TODO(port): setMaximumAmbientCacheSize(0) + deleting the regions.
    }
    assert_eq!(6, pragma_i64(&filename, "pragma user_version"));
    assert_eq!(0, log.unchecked_count(), "{:?}", log.unchecked());
}

#[test]
fn migrate_from_v4_schema() {
    let log = FixtureLog::new();
    let filename = copy_fixture("v4.db");
    for suffix in ["-shm", "-wal"] {
        let _ = std::fs::copy(
            cpp_path(&format!("test/fixtures/offline_database/v4.db{suffix}")),
            format!("{filename}{suffix}"),
        );
    }
    {
        let _db = OfflineDatabase::new(&filename, &tile_server_options());
        // TODO(port): setMaximumAmbientCacheSize(0) + deleting the regions.
    }
    assert_eq!(6, pragma_i64(&filename, "pragma user_version"));
    // Journal mode should be DELETE after migration to v5.
    assert_eq!("delete", pragma_string(&filename, "pragma journal_mode"));
    // Synchronous setting should be FULL (2) after migration to v5.
    assert_eq!(2, pragma_i64(&filename, "pragma synchronous"));
    assert_eq!(0, log.unchecked_count(), "{:?}", log.unchecked());
}

#[test]
fn migrate_from_v5_schema() {
    let log = FixtureLog::new();
    let filename = copy_fixture("v5.db");
    {
        let _db = OfflineDatabase::new(&filename, &tile_server_options());
        // TODO(port): setMaximumAmbientCacheSize(0) + deleting the regions.
    }
    assert_eq!(6, pragma_i64(&filename, "pragma user_version"));
    assert_eq!(TILES_COLUMNS.to_vec(), database_table_columns(&filename, "tiles"));
    assert_eq!(RESOURCES_COLUMNS.to_vec(), database_table_columns(&filename, "resources"));
    assert_eq!(0, log.unchecked_count(), "{:?}", log.unchecked());
}

#[test]
fn downgrade_schema() {
    // v999.db is a v999 database, it should be deleted and recreated with
    // the current schema.
    let log = FixtureLog::new();
    let filename = copy_fixture("v999.db");
    {
        let _db = OfflineDatabase::new(&filename, &tile_server_options());
        // TODO(port): setMaximumAmbientCacheSize(0).
    }
    assert_eq!(6, pragma_i64(&filename, "pragma user_version"));
    assert_eq!(TILES_COLUMNS.to_vec(), database_table_columns(&filename, "tiles"));
    assert_eq!(RESOURCES_COLUMNS.to_vec(), database_table_columns(&filename, "resources"));
    assert_eq!(1, log.count(&removing(), false));
    assert_eq!(0, log.unchecked_count(), "{:?}", log.unchecked());
}

#[test]
fn corrupt_database_on_open() {
    let log = FixtureLog::new();
    let filename = copy_fixture("corrupt-immediate.db");

    // This database is corrupt in a way that will prevent opening the
    // database.
    let _db = OfflineDatabase::new(&filename, &tile_server_options());
    assert_eq!(
        1,
        log.count(&error(result_code::CORRUPT, "Can't open database: database disk image is malformed"), true)
    );
    assert_eq!(1, log.count(&removing(), false));
    assert_eq!(0, log.unchecked_count(), "{:?}", log.unchecked());
    // TODO(port): the put/get round trip (write path).
}

#[test]
fn corrupt_database_on_query() {
    let log = FixtureLog::new();
    let filename = copy_fixture("corrupt-delayed.db");

    // This database is corrupt in a way that won't manifest itself until we
    // start querying it, so just opening it will not cause an error.
    let mut db = OfflineDatabase::new(&filename, &tile_server_options());

    // Just opening this corrupt database should not have produced an error
    // yet, since PRAGMA user_version still succeeds with this database.
    assert_eq!(0, log.unchecked_count(), "{:?}", log.unchecked());

    // The first request fails because the database is corrupt and has to be
    // recreated.
    assert!(db.get(&fixture_tile()).is_none());
    assert_eq!(
        1,
        log.count(&error(result_code::CORRUPT, "Can't read resource: database disk image is malformed"), true)
    );
    assert_eq!(1, log.count(&removing(), false));
    assert_eq!(0, log.unchecked_count(), "{:?}", log.unchecked());
    // TODO(port): the put/get round trip (write path).
}

#[test]
fn change_path() {
    let new_path = temp_path("test.db");
    let mut db = OfflineDatabase::new(":memory:", &tile_server_options());
    db.change_path(&new_path);
    Database::open(&new_path, OpenFlag::ReadOnly).unwrap();
    std::fs::remove_file(&new_path).unwrap();
}

#[test]
fn reset_database() {
    let log = FixtureLog::new();
    let filename = copy_fixture("satellite_test.db");

    let mut db = OfflineDatabase::new(&filename, &tile_server_options());
    let result = db.reset_database();
    assert!(result.is_none());

    // C++: db.listRegions().value().size() (TODO(port)).
    assert_eq!(0, pragma_i64(&filename, "SELECT COUNT(*) FROM regions"));
    assert_eq!(1, log.count(&removing(), false));
    assert_eq!(0, log.unchecked_count(), "{:?}", log.unchecked());
}

#[test]
fn reset_in_memory_database() {
    for path in [":memory:", "file::memory:", "file::memory:?cache=shared", "file:reset-test?mode=memory"] {
        let log = FixtureLog::new();
        // C++ writes a decoy file at the relative path, which the reset must
        // not delete. Skipped where the working directory is read-only.
        let decoy = std::fs::write(path, "unrelated file").is_ok();

        let mut db = OfflineDatabase::new(path, &tile_server_options());
        assert!(db.reset_database().is_none(), "{path}");
        if decoy {
            assert_eq!("unrelated file", std::fs::read_to_string(path).unwrap(), "{path}");
            let _ = std::fs::remove_file(path);
        }
        assert_eq!(1, log.count(&removing(), false), "{path}");
        assert_eq!(0, log.unchecked_count(), "{path}: {:?}", log.unchecked());
    }
}

#[test]
fn put_resource_read_only_mode() {
    let log = FixtureLog::new();
    let mut db = OfflineDatabase::new(":memory:", &tile_server_options());

    let resource = Resource::new(Kind::Style, "http://example.com/");
    let mut response = crate::response::Response::new();
    response.data = Some(std::sync::Arc::new(b"success".to_vec()));

    // In read-only mode put() is a no-op
    db.reopen_database_read_only(true);
    let failed_put_result = db.put(&resource, &response);
    assert!(!failed_put_result.0);
    assert_eq!(0, failed_put_result.1);

    // TODO(port): put() with read-only mode disabled (write path).
    db.reopen_database_read_only(false);

    assert_eq!(0, log.unchecked_count(), "{:?}", log.unchecked());
}
