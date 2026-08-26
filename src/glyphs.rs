//! Glyph whitelist — sole source of every non-ASCII symbol this UI draws.
//!
//! Owning decision: design **D8**. Enforcement is static and free at
//! render time: the UI may reference only the exported constants and the
//! closed [`Glyph`] enum defined here; runtime glyph construction must not
//! occur anywhere in the crate. The permitted codepoint space is box
//! drawing (U+2500–U+257F) union block elements and shades (U+2580–U+259F)
//! — the set classic Windows conhost renders faithfully. Braille
//! (U+2800–U+28FF) and sextant/octant codepoints are forbidden.
//!
//! Two offline nets guard this contract in `tests/ui/glyph_tests.rs`:
//! an export-range enumeration and a rendered-buffer purity scan.
//!
//! Implemented by Phase 1 (tasks 1.1–1.3).

/// Box drawing: light horizontal (`─`, U+2500).
pub const BOX_LIGHT_HORIZONTAL: char = '\u{2500}';
/// Box drawing: light vertical (`│`, U+2502).
pub const BOX_LIGHT_VERTICAL: char = '\u{2502}';
/// Box drawing: light down-and-right (`┌`, U+250C).
pub const BOX_LIGHT_DOWN_AND_RIGHT: char = '\u{250C}';
/// Box drawing: light down-and-left (`┐`, U+2510).
pub const BOX_LIGHT_DOWN_AND_LEFT: char = '\u{2510}';
/// Box drawing: light up-and-right (`└`, U+2514).
pub const BOX_LIGHT_UP_AND_RIGHT: char = '\u{2514}';
/// Box drawing: light up-and-left (`┘`, U+2518).
pub const BOX_LIGHT_UP_AND_LEFT: char = '\u{2518}';

/// Block Elements: full block (`█`, U+2588) — solid bar fill.
pub const BLOCK_FULL: char = '\u{2588}';
/// Block Elements: dark shade (`▓`, U+2593) — filled inventory cell.
pub const BLOCK_DARK_SHADE: char = '\u{2593}';
/// Block Elements: medium shade (`▒`, U+2592).
pub const BLOCK_MEDIUM_SHADE: char = '\u{2592}';
/// Block Elements: light shade (`░`, U+2591) — empty inventory cell.
pub const BLOCK_LIGHT_SHADE: char = '\u{2591}';

/// Sparkline ramp from shortest to tallest bar (U+2581–U+2588), index 0
/// being the lowest level. Every entry draws only whitelisted codepoints.
pub const SPARKLINE_LEVELS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

/// The closed set of drawable symbols. Variants are the ONLY constructors:
/// widget code names a variant, so no call path can synthesize an unlisted
/// rune at runtime (design D8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Horizontal,
    Vertical,
    CornerTopLeft,
    CornerTopRight,
    CornerBottomLeft,
    CornerBottomRight,
    FullBlock,
    DarkShade,
    MediumShade,
    LightShade,
}

impl Glyph {
    /// The symbol this variant draws. `const` so whitelisted strings can be
    /// embedded in constants without runtime work.
    pub const fn symbol(self) -> &'static str {
        match self {
            Glyph::Horizontal => "\u{2500}",
            Glyph::Vertical => "\u{2502}",
            Glyph::CornerTopLeft => "\u{250C}",
            Glyph::CornerTopRight => "\u{2510}",
            Glyph::CornerBottomLeft => "\u{2514}",
            Glyph::CornerBottomRight => "\u{2518}",
            Glyph::FullBlock => "\u{2588}",
            Glyph::DarkShade => "\u{2593}",
            Glyph::MediumShade => "\u{2592}",
            Glyph::LightShade => "\u{2591}",
        }
    }
}

/// Every distinct codepoint exported by this module — named constants AND
/// the characters inside multi-char exports such as [`SPARKLINE_LEVELS`].
/// The enumeration net in `tests/ui/glyph_tests.rs` asserts each one sits
/// inside the permitted blocks.
pub const ALL_CODEPOINTS: &[char] = &[
    BOX_LIGHT_HORIZONTAL,
    BOX_LIGHT_VERTICAL,
    BOX_LIGHT_DOWN_AND_RIGHT,
    BOX_LIGHT_DOWN_AND_LEFT,
    BOX_LIGHT_UP_AND_RIGHT,
    BOX_LIGHT_UP_AND_LEFT,
    '\u{2581}',
    '\u{2582}',
    '\u{2583}',
    '\u{2584}',
    '\u{2585}',
    '\u{2586}',
    '\u{2587}',
    BLOCK_FULL,
    BLOCK_DARK_SHADE,
    BLOCK_MEDIUM_SHADE,
    BLOCK_LIGHT_SHADE,
];

/// All [`Glyph`] variants, for the completeness half of the export net.
pub const ALL_GLYPHS: &[Glyph] = &[
    Glyph::Horizontal,
    Glyph::Vertical,
    Glyph::CornerTopLeft,
    Glyph::CornerTopRight,
    Glyph::CornerBottomLeft,
    Glyph::CornerBottomRight,
    Glyph::FullBlock,
    Glyph::DarkShade,
    Glyph::MediumShade,
    Glyph::LightShade,
];
