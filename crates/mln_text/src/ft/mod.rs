//! The subset of FreeType 2.13 (../maplibre-native/vendor/freetype) that the
//! C++ core's text code relies on, ported for exact FreeType semantics on
//! top of the font data (see each file's header).

pub mod calc;
pub mod cmap;
pub mod face;
pub mod grays;
