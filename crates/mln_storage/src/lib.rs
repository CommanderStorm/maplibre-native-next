//! Ports `src/mln/storage`, `include/mln/storage` and the storage part of
//! `platform/default`: `Resource`, `Response`, `FileSource`,
//! `FileSourceRequest`, `ResourceOptions`, `ResourceTransform`,
//! `FileSourceManager`, `MainResourceLoader`, the database, online/HTTP,
//! asset, local file, MBTiles and PMTiles file sources, `OfflineDatabase`
//! and the offline region definitions. `OfflineDownload` needs the style
//! parser and is in //crates/mln_storage_offline.
//!
//! In the browser (wasm32-unknown-unknown) there is no SQLite: the database
//! and MBTiles file sources and `OfflineDatabase` are left out, and the
//! browser's HTTP cache does the caching.

pub mod asset_file_source;
pub mod async_request;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub mod database_file_source;
pub mod file_source;
pub mod file_source_manager;
pub mod file_source_request;
pub mod http_file_source;
pub mod http_timeout;
pub mod local_file_request;
pub mod local_file_source;
pub mod main_resource_loader;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub mod mbtiles_file_source;
pub mod merge_sideloaded;
pub mod network_status;
pub mod offline;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub mod offline_database;
pub mod offline_schema;
pub mod online_file_source;
pub mod pmtiles_file_source;
pub mod resource;
pub mod resource_options;
pub mod resource_transform;
pub mod response;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub mod sqlite3;
pub mod stub_file_source;
pub(crate) mod tilejson_document;

pub use asset_file_source::AssetFileSource;
pub use async_request::AsyncRequest;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use database_file_source::DatabaseFileSource;
pub use file_source::{Callback, FileSource, FileSourceType, READ_ONLY_MODE_KEY};
pub use file_source_manager::{FileSourceFactory, FileSourceManager};
pub use file_source_request::{FileSourceRequest, FileSourceRequestState};
pub use local_file_source::LocalFileSource;
pub use main_resource_loader::MainResourceLoader;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use mbtiles_file_source::MBTilesFileSource;
pub use online_file_source::OnlineFileSource;
pub use pmtiles_file_source::PMTilesFileSource;
pub use mln_util::client_options::ClientOptions;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use offline_database::{MapboxTileLimitExceededException, OfflineDatabase};
pub use resource::{Kind, LoadingMethod, Resource, StoragePolicy, TileData};
pub use resource_options::ResourceOptions;
pub use response::{Data, Response};

#[cfg(test)]
mod tests;
