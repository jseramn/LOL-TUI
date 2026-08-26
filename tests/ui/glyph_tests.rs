//! Glyph whitelist nets (viz:R1/S1, viz:R1/S2, design D8).
//!
//! Net 1 (export range): every codepoint exported by `tui_lol::glyphs`
//! must lie in box drawing U+2500–U+257F or block elements/shades
//! U+2580–U+259F — the space classic conhost renders faithfully.
//! Net 2 (purity): anything a Glyph-driven widget draws scans clean
//! against whitelist ∪ printable ASCII.

use std::collections::HashSet;

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;
use ratatui::Terminal;
use tui_lol::glyphs::{ALL_CODEPOINTS, ALL_GLYPHS, Glyph};

/// Box drawing (U+2500–U+257F) ∪ Block Elements/shades (U+2580–U+259F).
fn is_whitelisted(c: char) -> bool {
    ('\u{2500}'..='\u{257F}').contains(&c) || ('\u{2580}'..='\u{259F}').contains(&c)
}

#[test]
fn exports_at_least_one_glyph() {
    // A trivially-empty whitelist would make every other assertion pass
    // vacuously; pin real content first.
    assert!(
        !ALL_CODEPOINTS.is_empty(),
        "the whitelist must actually export glyphs"
    );
    assert!(!ALL_GLYPHS.is_empty(), "the Glyph enum must have variants");
}

#[test]
fn every_exported_codepoint_is_conhost_safe() {
    // viz:R1/S1 — enumerate the whole exported set; each codepoint must sit
    // inside one of the two permitted blocks.
    let offenders: Vec<char> = ALL_CODEPOINTS
        .iter()
        .copied()
        .filter(|c| !is_whitelisted(*c))
        .collect();
    assert!(
        offenders.is_empty(),
        "codepoints outside U+2500-257F/U+2580-259F exported: {offenders:?}"
    );
}

#[test]
fn no_braille_or_sextant_octant_codepoints_are_exported() {
    // Braille lives at U+2800–U+28FF; sextant/octant symbols at
    // U+1FB00–U+1FBFF (Symbols for Legacy Computing). Both render as `?`
    // on classic conhost, so neither may EVER appear in the exports.
    let forbidden: Vec<char> = ALL_CODEPOINTS
        .iter()
        .copied()
        .filter(|c| {
            ('\u{2800}'..='\u{28FF}').contains(c) || ('\u{1FB00}'..='\u{1FBFF}').contains(c)
        })
        .collect();
    assert!(
        forbidden.is_empty(),
        "Braille/sextant-octant codepoints exported: {forbidden:?}"
    );
}

#[test]
fn glyph_enum_symbols_stay_inside_the_exported_set() {
    // Completeness half of the net: a new Glyph variant whose symbol was
    // forgotten in ALL_CODEPOINTS would dodge the range checks above.
    let registered: HashSet<char> = ALL_CODEPOINTS.iter().copied().collect();
    for glyph in ALL_GLYPHS {
        for c in glyph.symbol().chars() {
            assert!(
                registered.contains(&c),
                "Glyph {glyph:?} emits {c:?} which is not registered in ALL_CODEPOINTS"
            );
        }
    }
}

// --- Net 2: rendered-buffer purity (viz:R1/S2) ------------------------------

/// Test-only widget drawn EXCLUSIVELY through whitelist glyphs plus a
/// printable-ASCII label — the smallest real consumer of the D8 contract.
struct GlyphBanner {
    label: &'static str,
}

impl Widget for GlyphBanner {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }
        let left = area.left();
        let right = area.right() - 1;
        let top = area.top();
        let bottom = area.bottom() - 1;
        for x in left..=right {
            buf[(x, top)].set_symbol(Glyph::Horizontal.symbol());
            buf[(x, bottom)].set_symbol(Glyph::Horizontal.symbol());
        }
        for y in top..=bottom {
            buf[(left, y)].set_symbol(Glyph::Vertical.symbol());
            buf[(right, y)].set_symbol(Glyph::Vertical.symbol());
        }
        buf[(left, top)].set_symbol(Glyph::CornerTopLeft.symbol());
        buf[(right, top)].set_symbol(Glyph::CornerTopRight.symbol());
        buf[(left, bottom)].set_symbol(Glyph::CornerBottomLeft.symbol());
        buf[(right, bottom)].set_symbol(Glyph::CornerBottomRight.symbol());
        // One shaded interior cell proves block fills flow through the same
        // whitelist-only path as the frame pieces.
        buf[(left + 1, top + 1)].set_symbol(Glyph::DarkShade.symbol());
        if area.width as usize >= self.label.len() + 2 && area.height >= 4 {
            buf.set_string(left + 2, top + 2, self.label, Style::default());
        }
    }
}

/// Cells inside `area` whose symbol falls outside the permitted space.
/// Permitted = printable ASCII (labels) ∪ the two whitelisted codepoint
/// blocks (viz:R1/S2).
fn buffer_violations(area: Rect, buf: &Buffer) -> Vec<(u16, u16, char)> {
    let mut offenders = Vec::new();
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            for c in buf[(x, y)].symbol().chars() {
                let printable_ascii = c.is_ascii_graphic() || c == ' ';
                let whitelisted = ('\u{2500}'..='\u{257F}').contains(&c)
                    || ('\u{2580}'..='\u{259F}').contains(&c);
                if !(printable_ascii || whitelisted) {
                    offenders.push((x, y, c));
                }
            }
        }
    }
    offenders
}

#[test]
fn purity_scanner_flags_injected_braille() {
    // Detector-sensitivity control: a hand-built buffer carrying one Braille
    // rune MUST be flagged, proving the scan can actually fail.
    let area = Rect::new(0, 0, 4, 1);
    let mut buf = Buffer::empty(area);
    buf[(0, 0)].set_symbol("a");
    buf[(2, 0)].set_symbol("\u{2802}");
    let offenders = buffer_violations(area, &buf);
    assert_eq!(
        offenders,
        vec![(2, 0, '\u{2802}')],
        "the injected Braille rune was not detected (printable 'a' must stay clean)"
    );
}

#[test]
fn glyph_driven_widget_output_is_whitelist_pure() {
    // viz:R1/S2 — render one trivial Glyph-driven widget to a TestBackend
    // and require every cell to hold a whitelisted codepoint or printable
    // ASCII.
    let area = Rect::new(1, 1, 20, 5);
    let mut terminal = Terminal::new(TestBackend::new(24, 8)).unwrap();
    terminal
        .draw(|frame| frame.render_widget(GlyphBanner { label: "CS 195" }, area))
        .unwrap();

    let offenders = buffer_violations(area, terminal.backend().buffer());
    assert!(
        offenders.is_empty(),
        "non-whitelisted cells rendered: {offenders:?}"
    );
}
