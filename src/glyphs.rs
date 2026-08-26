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
//! Implemented by Phase 1 (tasks 1.1–1.3); populated in task 1.2.
