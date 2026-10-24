//! Text metrics as Looping gets them from GDI.
//!
//! Documents use Calibri, which is not free; Carlito (embedded, SIL OFL) has
//! the same metrics. GDI's numbers are reproduced from the font tables:
//! cell height = round(usWinAscent·h/upm) + round(usWinDescent·h/upm) and
//! per-glyph advances rounded to whole pixels, `h` being the em size in
//! pixels (`-lfHeight`). Checked against `GetTextExtentPoint32W` under Wine
//! (`tools/wine/metrics.py`): heights all equal, 99 % of the advances.

use std::sync::OnceLock;

use loo_format::LogFont;
use serde::Serialize;

pub const CARLITO_REGULAR: &[u8] = include_bytes!("../assets/fonts/Carlito-Regular.ttf");
pub const CARLITO_BOLD: &[u8] = include_bytes!("../assets/fonts/Carlito-Bold.ttf");

struct Face {
    upm: f32,
    win_ascent: f32,
    win_descent: f32,
    face: ttf_parser::Face<'static>,
}

impl Face {
    fn load(data: &'static [u8]) -> Face {
        let face = ttf_parser::Face::parse(data, 0).expect("embedded font");
        let os2 = face.tables().os2.expect("OS/2 table");
        Face {
            upm: face.units_per_em() as f32,
            win_ascent: os2.windows_ascender() as f32,
            win_descent: os2.windows_descender().unsigned_abs() as f32,
            face,
        }
    }
}

fn face(bold: bool) -> &'static Face {
    static REGULAR: OnceLock<Face> = OnceLock::new();
    static BOLD: OnceLock<Face> = OnceLock::new();
    if bold {
        BOLD.get_or_init(|| Face::load(CARLITO_BOLD))
    } else {
        REGULAR.get_or_init(|| Face::load(CARLITO_REGULAR))
    }
}

/// A GDI font as selected in a device context: em size in pixels and weight.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Font {
    pub size: i32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Font {
    /// Font Looping creates from a stored `LOGFONTW` at 100 % zoom and 96 dpi
    /// (0x140012dc0): `lfHeight · 96/72 · 0.78 · 100/96`, truncated.
    pub fn from_logfont(lf: &LogFont) -> Font {
        let pt = 96.0f32 * -(lf.height as f32) / 72.0;
        let h = (-pt * 0.78f32 * 100.0 / 96.0) as i32;
        let size = if h < 0 {
            -h
        } else {
            // positive lfHeight is a cell height
            let f = face(lf.weight >= 600);
            ((h as f32) * f.upm / (f.win_ascent + f.win_descent)).round() as i32
        };
        Font { size: size.max(1), bold: lf.weight >= 600, italic: lf.italic, underline: lf.underline }
    }

    pub fn with_bold(self, bold: bool) -> Font {
        Font { bold, ..self }
    }

    fn scale(&self, v: f32) -> i32 {
        (v * self.size as f32 / face(self.bold).upm).round() as i32
    }

    /// `tmHeight`.
    pub fn height(&self) -> i32 {
        let f = face(self.bold);
        self.scale(f.win_ascent) + self.scale(f.win_descent)
    }

    /// `tmAscent`.
    pub fn ascent(&self) -> i32 {
        self.scale(face(self.bold).win_ascent)
    }

    fn advance(&self, c: char) -> i32 {
        let f = face(self.bold);
        let gid = f.face.glyph_index(c).unwrap_or(ttf_parser::GlyphId(0));
        let adv = f.face.glyph_hor_advance(gid).unwrap_or(0) as f32;
        (adv * self.size as f32 / f.upm + 0.5) as i32
    }

    /// `GetTextExtentPoint32W(s).cx`.
    pub fn width(&self, s: &str) -> i32 {
        s.chars().map(|c| self.advance(c)).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calibri(height: i32, weight: i32) -> LogFont {
        LogFont { height, weight, italic: false, underline: false, face: "Calibri".into() }
    }

    /// Values read from GDI under Wine (Calibri replaced by Carlito).
    #[test]
    fn gdi_metrics() {
        let f = Font { size: 13, bold: false, italic: false, underline: false };
        assert_eq!((f.height(), f.ascent()), (15, 12));
        let f = Font { size: 11, bold: true, italic: false, underline: false };
        assert_eq!((f.height(), f.ascent()), (13, 10));
        // the Looping scaling of -12 (9 pt) and -15
        assert_eq!(Font::from_logfont(&calibri(-12, 400)).size, 13);
        assert_eq!(Font::from_logfont(&calibri(-15, 700)).size, 16);
    }
}
