//! Ports `src/mln/text` (text shaping and glyphs) of the C++ core.
//!
//! Shaping uses HarfRust instead of HarfBuzz, and the ported FreeType subset
//! in `ft` (charmaps, sizing, advances) instead of FreeType; outlines come
//! from skrifa. See PLAN.md section 4.

pub mod freetype;
pub mod ft;
pub mod glyph;
pub mod glyph_pbf;
pub mod harfbuzz;
pub mod harfbuzz_impl;
pub mod hb_ft;
pub mod hb_ucd;
pub mod language_tag;
pub mod tagged_string;
mod hb_ucd_table;
mod rasterize;

pub use freetype::{FreeTypeFace, FreeTypeLibrary};
pub use glyph::{Glyph, GlyphID, GlyphIDType, GlyphMetrics, GlyphRange};
pub use harfbuzz::{HBShapeAdjust, HBShaper};
