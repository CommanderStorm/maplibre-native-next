//! Ports platform/default/src/mln/storage/offline_download.cpp
//! (`OfflineDownload`, which downloads an offline region's resources into
//! the OfflineDatabase) and the canonicalize* half of
//! src/mln/util/mapbox.cpp it uses. It parses styles and covers regions with
//! tiles, so it sits above mln_style and mln_renderer.
// Needs OfflineDatabase, which has no SQLite in the browser.
#![cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]

pub mod mapbox;
pub mod offline_download;

pub use offline_download::OfflineDownload;

#[cfg(test)]
mod tests;
