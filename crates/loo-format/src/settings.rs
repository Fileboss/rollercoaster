//! The document settings block: 0x88c bytes copied verbatim from
//! `CLoopingDoc+0x36c` (fonts, colours, MLD/SQL generation options...).
//!
//! Offsets in this module are `CLoopingDoc` offsets, as in the decompiled
//! code; [`Settings::BASE`] is subtracted to index the block.

use serde::{Serialize, Serializer};

#[derive(Clone, PartialEq)]
pub struct Settings {
    raw: Vec<u8>,
}

impl std::fmt::Debug for Settings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Settings").field("version", &self.version()).finish_non_exhaustive()
    }
}

impl Serialize for Settings {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("Settings", 3)?;
        st.serialize_field("version", &self.version())?;
        st.serialize_field("dpi", &self.i32(0x7e4))?;
        st.serialize_field("fk_mask", &self.fk_mask())?;
        st.end()
    }
}

/// Settings of a new document, as Looping 4.1 creates it (« Nouveau »,
/// answering « Non » to « récupérer les propriétés d'un modèle existant ? »;
/// saved under Wine by `tools/wine/newdoc.py`, format 0x33).
static DEFAULT_BLOCK: &[u8; Settings::LEN] = include_bytes!("../assets/default_settings.bin");

impl Settings {
    pub const BASE: usize = 0x36c;
    pub const LEN: usize = 0x88c;

    pub fn new_default() -> Self {
        Settings { raw: DEFAULT_BLOCK.to_vec() }
    }

    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    pub fn raw_mut(&mut self) -> &mut [u8] {
        &mut self.raw
    }

    fn idx(off: usize) -> usize {
        assert!((Self::BASE..Self::BASE + Self::LEN).contains(&off), "offset {off:#x} outside settings");
        off - Self::BASE
    }

    pub fn i32(&self, off: usize) -> i32 {
        let i = Self::idx(off);
        i32::from_le_bytes(self.raw[i..i + 4].try_into().unwrap())
    }

    pub fn u32(&self, off: usize) -> u32 {
        self.i32(off) as u32
    }

    pub fn set_i32(&mut self, off: usize, v: i32) {
        let i = Self::idx(off);
        self.raw[i..i + 4].copy_from_slice(&v.to_le_bytes());
    }

    pub fn set_u32(&mut self, off: usize, v: u32) {
        self.set_i32(off, v as i32)
    }

    fn set_u64(&mut self, off: usize, v: u64) {
        let i = Self::idx(off);
        self.raw[i..i + 8].copy_from_slice(&v.to_le_bytes());
    }

    fn copy(&mut self, dst: usize, src: usize, len: usize) {
        let (d, s) = (Self::idx(dst), Self::idx(src));
        self.raw.copy_within(s..s + len, d);
    }

    /// Null-terminated UTF-16 string of at most `cap` code units.
    pub fn wstr(&self, off: usize, cap: usize) -> String {
        let i = Self::idx(off);
        let units: Vec<u16> = self.raw[i..i + cap * 2]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|&u| u != 0)
            .collect();
        String::from_utf16_lossy(&units)
    }

    /// Writes a NUL-terminated UTF-16 string of at most `cap` units.
    pub fn set_wstr(&mut self, off: usize, s: &str, cap: usize) {
        let i = Self::idx(off);
        let mut units: Vec<u16> = s.encode_utf16().take(cap - 1).collect();
        units.resize(cap, 0);
        for (k, u) in units.into_iter().enumerate() {
            self.raw[i + 2 * k..i + 2 * k + 2].copy_from_slice(&u.to_le_bytes());
        }
    }

    /// `LOGFONTW` (0x5c bytes) at `off`.
    pub fn logfont(&self, off: usize) -> crate::LogFont {
        let i = Self::idx(off);
        crate::LogFont::parse(&self.raw[i..i + 0x5c])
    }

    /// COLORREF at `off`; Looping stores -1 for "Par défaut".
    pub fn color(&self, off: usize) -> Option<u32> {
        Some(self.u32(off)).filter(|&c| c != u32::MAX)
    }

    pub fn version(&self) -> i32 {
        self.i32(0x36c)
    }

    pub fn set_version(&mut self, v: i32) {
        self.set_i32(0x36c, v)
    }

    /// Defaults Looping applies right after reading an older block
    /// (0x140074c20, between the header and the string tables).
    pub fn upgrade_from(&mut self, v: i32) {
        if v < 0x18 {
            if v < 0x16 {
                if v < 0x13 {
                    if v < 0xf {
                        if v < 0xd {
                            self.set_i32(0x7e8, 1);
                        }
                        self.set_i32(0x7f0, 1);
                    }
                    self.set_u64(0x7f8, 0);
                }
                self.set_u32(0x800, 0xfff700);
            }
            self.set_u32(0x804, 0xffffff);
        }
        if v < 0x21 {
            if v < 0x20 {
                if v < 0x1c {
                    self.set_u64(0x808, 1);
                    self.set_i32(0x810, 0);
                    self.set_u64(0x814, 1);
                    self.set_u64(0x81c, 0);
                    self.set_i32(0x824, 0);
                    self.set_u64(0x828, 1);
                    self.set_u64(0x830, 0);
                    self.set_u64(0x838, 0);
                    self.set_u64(0x840, 1);
                    self.set_i32(0x848, 0);
                    self.set_u64(0x84c, 0xffffff);
                    self.set_i32(0x854, 0);
                    self.set_u64(0xa90, 1);
                    self.set_i32(0xa98, 0);
                    self.set_u64(0xa9c, 0xffffff);
                    self.set_u64(0x864, 1);
                    self.set_i32(0x86c, 0);
                    self.set_u64(0x870, 0xffffff);
                }
                self.set_i32(0x88c, 1);
                self.copy(0x890, 0x468, 0x5c);
                for (off, val) in [(0x8ec, 0), (0x8f0, 6), (0x8f4, 6), (0x8f8, 1), (0x8fc, 1), (0x900, 1), (0x904, 1)] {
                    self.set_i32(off, val);
                }
                self.set_u64(0x908, 1);
                self.set_u64(0x910, 1);
                for (off, val) in [
                    (0x918, 1),
                    (0x91c, 5),
                    (0x920, 1),
                    (0x924, 0xc0c0c0),
                    (0x928, 1),
                    (0x92c, 1),
                    (0x930, 1),
                    (0x934, 0xf0f0f0),
                ] {
                    self.set_i32(off, val);
                }
            }
            // LOGFONTW at 0x994: Calibri 10pt (MulDiv(-10, 96, 72) on a 96 dpi screen)
            self.set_wstr(0x9b0, "Calibri", 32);
            self.set_i32(0x9ac, 0x22020004);
            self.set_i32(0x994, -13);
            self.copy(0x938, 0x4c4, 0x5c);
        }
        if v < 0x23 {
            self.set_u32(0xa04, 0x80dfff);
            let c = self.i32(0x398);
            self.set_i32(0xa08, c);
            self.set_u64(0xa0c, 0xffd700);
            self.set_i32(0xa14, 0x1e);
        }
        let half = self.i32(0x538) / 2;
        if self.i32(0xa14) < half {
            self.set_i32(0xa14, half);
        }
        if v < 0x25 {
            self.set_i32(0xa18, 0);
        }
        if self.wstr(0x7cc, 16).trim_matches(' ').is_empty() {
            self.set_wstr(0x7cc, "[FK]", 16);
        }
        if v < 0x29 {
            self.set_i32(0xa7c, 1);
            self.set_i32(0xa80, 0);
        }
        if v < 0x2a {
            self.set_i32(0xa84, 1);
        }
        if v < 0x2b {
            self.set_i32(0xa88, 100);
            self.set_u32(0xa8c, 0xd2d2d2);
        }
        if v < 0x32 {
            if v < 0x30 {
                self.set_u32(0xad0, 0x867a7a28);
                self.set_u32(0xad4, 0x7d7d3c49);
                self.set_u32(0xad8, 0x3c4998);
            }
            self.copy(0xae0, 0x74c, 0x5c);
        }
        let m = self.i32(0x524);
        if self.i32(0xa88) < m {
            self.set_i32(0xa88, m);
        }
        if self.i32(0xadc) == 0 {
            self.set_i32(0xadc, 0xff);
        }
    }

    /// Colour defaults applied after the string tables.
    pub fn fix_after_strings(&mut self) {
        for (off, val) in [
            (0x7ac, 0xf0ffff),
            (0x7b0, 0xc0c0),
            (0x7b4, 0xc0c0),
            (0x7b8, 0xff),
            (0x7bc, 0xe0e0),
            (0x884, 0xc0c0c0),
            (0xa68, 0xff0000),
        ] {
            if self.i32(off) == 0 {
                self.set_i32(off, val);
            }
        }
    }

    /// Range checks performed at the end of loading.
    pub fn clamp(&mut self) {
        for (off, max) in [(0x53c, 1), (0xa00, 2), (0xa20, 1), (0xa18, 2), (0x7ec, 1), (0x7f4, 2), (0xb40, 2)] {
            if self.u32(off) > max {
                self.set_i32(off, 0);
            }
        }
        let k = self.u32(0x52c);
        if k > 8 && k != 0x14 {
            self.set_i32(0x52c, 0);
        }
        let dpi = self.i32(0x7e4).clamp(96, 1200);
        self.set_i32(0x7e4, dpi);
        let z = self.i32(0x7e8).clamp(0, 9);
        self.set_i32(0x7e8, z);
    }

    /// "Nom clé étrangère": mask of migrated key names, `[FK]` (the
    /// referenced column) and `[Classe]` (the referenced entity). Looping
    /// keeps its first 10 characters at `+0x7cc` and the rest at `+0xa52`
    /// (0x140079fd0).
    pub fn fk_mask(&self) -> String {
        self.wstr(0x7cc, 12) + &self.wstr(0xa52, 13)
    }

    pub fn set_fk_mask(&mut self, mask: &str) {
        let mask = if mask.trim_matches(' ').is_empty() { "[FK]" } else { mask };
        let split = mask.char_indices().nth(10).map_or(mask.len(), |(i, _)| i);
        self.set_wstr(0x7cc, &mask[..split], 12);
        self.set_wstr(0xa52, &mask[split..], 13);
    }
}
