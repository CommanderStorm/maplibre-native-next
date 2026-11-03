// Ports src/mln/text/local_glyph_rasterizer.hpp and
// platform/default/src/mln/text/local_glyph_rasterizer.cpp.

use mln_util::font_stack::FontStack;

use crate::glyph::{Glyph, GlyphID};

/// Given a font stack and a glyph ID, platform-specific implementations of
/// LocalGlyphRasterizer will decide which, if any, local fonts to use and
/// then generate a matching glyph object with a greyscale rasterization of
/// the glyph and appropriate metrics. GlyphManager will then use TinySDF to
/// transform the rasterized bitmap into an SDF.
///
/// The shared default codepoint filter is
/// `util::i18n::allowsFixedWidthGlyphGeneration`. Following
/// maplibre-gl-js, platform implementations use 2x bitmaps with 1x metrics
/// via `GlyphMetrics::isDoubleResolution`.
///
/// It is left to platform-specific implementation to decide how best to
/// map a FontStack to a particular rasterization (font, weight, style).
///
/// The default implementation simply refuses to rasterize any glyphs.
pub trait LocalGlyphRasterizer {
    // virtual so that test harness can override platform-specific behavior
    fn can_rasterize_glyph(&self, _font_stack: &FontStack, _glyph_id: GlyphID) -> bool {
        false
    }

    fn rasterize_glyph(&self, _font_stack: &FontStack, _glyph_id: GlyphID) -> Glyph {
        Glyph::default()
    }
}

/// The platform/default `LocalGlyphRasterizer(const std::optional<std::string>&
/// fontFamily)`, which refuses every glyph.
#[derive(Clone, Debug, Default)]
pub struct DefaultLocalGlyphRasterizer {
    _font_family: Option<String>,
}

impl DefaultLocalGlyphRasterizer {
    pub fn new(font_family: Option<String>) -> Self {
        DefaultLocalGlyphRasterizer { _font_family: font_family }
    }
}

impl LocalGlyphRasterizer for DefaultLocalGlyphRasterizer {}
