//! Draw a font's outline glyphs with the system's own text rasterizer (e.g. DirectWrite on
//! Windows) instead of egui's, for the fonts that ask for it ([`FontTweak::system_rasterizer`]).
//!
//! Only the pixels change: shaping, kerning, advances and line metrics stay egui's, so the
//! layout is the same either way. The application installs the rasterizer (epaint has no
//! platform code) before it loads its fonts.
//!
//! [`FontTweak::system_rasterizer`]: crate::text::FontTweak::system_rasterizer

use alloc::{sync::Arc, vec::Vec};

/// One glyph to rasterize, from a font file the application gave egui.
pub struct SystemGlyphRequest<'a> {
    /// The whole font file (a `.ttf`, or a collection with [`Self::index`]).
    pub font_data: &'a [u8],

    /// Which face of a collection (`.ttc`); 0 for a single font.
    pub index: u32,

    /// The glyph in that face.
    pub glyph_id: u32,

    /// The size, in physical pixels per em.
    pub size_px: f32,

    /// The glyph origin's offset from the pixel grid, in physical pixels (x right, y down).
    pub offset_px: emath::Vec2,
}

/// A glyph's coverage, as the system drew it.
pub struct SystemGlyph {
    /// Width and height, in physical pixels.
    pub size: [usize; 2],

    /// From the glyph origin (on the baseline) to the bitmap's top-left, in physical pixels
    /// (y down: usually negative).
    pub offset_px: [i32; 2],

    /// One coverage byte per pixel, row by row (`size[0] * size[1]` of them).
    pub coverage: Vec<u8>,
}

/// The callback: `None` lets egui draw that glyph itself.
pub type SystemRasterizeFn =
    dyn for<'a> Fn(&SystemGlyphRequest<'a>) -> Option<SystemGlyph> + Send + Sync;

static RASTERIZER: parking_lot::RwLock<Option<Arc<SystemRasterizeFn>>> =
    parking_lot::RwLock::new(None);

/// Install (or with `None`, remove) the system rasterizer for every [`Fonts`](crate::text::Fonts)
/// in the process. Glyphs already in an atlas stay as they were drawn: install it before the
/// fonts are loaded (or load them again).
pub fn set_system_rasterizer(rasterizer: Option<Arc<SystemRasterizeFn>>) {
    *RASTERIZER.write() = rasterizer;
}

pub(crate) fn system_rasterizer() -> Option<Arc<SystemRasterizeFn>> {
    RASTERIZER.read().clone()
}
