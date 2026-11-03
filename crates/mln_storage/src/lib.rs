//! Ports `src/mln/storage`, `include/mln/storage` and the storage part of
//! `platform/default` that the cache-only path needs: `Resource`,
//! `Response`, `FileSource`, `FileSourceRequest`, `ResourceOptions`,
//! `FileSourceManager`, `MainResourceLoader`, `DatabaseFileSource` and the
//! read path of `OfflineDatabase`.

pub mod async_request;
pub mod database_file_source;
pub mod file_source;
pub mod file_source_manager;
pub mod file_source_request;
pub mod main_resource_loader;
pub mod offline_database;
pub mod offline_schema;
pub mod resource;
pub mod resource_options;
pub mod response;
pub mod sqlite3;
pub mod stub_file_source;

pub use async_request::AsyncRequest;
pub use database_file_source::DatabaseFileSource;
pub use file_source::{Callback, FileSource, FileSourceType, READ_ONLY_MODE_KEY};
pub use file_source_manager::{FileSourceFactory, FileSourceManager};
pub use file_source_request::{FileSourceRequest, FileSourceRequestState};
pub use main_resource_loader::MainResourceLoader;
pub use mln_util::client_options::ClientOptions;
pub use offline_database::OfflineDatabase;
pub use resource::{Kind, LoadingMethod, Resource, StoragePolicy, TileData};
pub use resource_options::ResourceOptions;
pub use response::{Data, Response};

#[cfg(test)]
mod tests;
