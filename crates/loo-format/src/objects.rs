//! Drawing objects (`CDrawObj` hierarchy) and their serializers.
//!
//! Every serializer mirrors the corresponding `Serialize` method of
//! Looping.exe 4.1 (x64). Field comments give the member offset in the
//! original object, which is what `docs/FORMAT.md` refers to.

use serde::Serialize;

use crate::archive::{Archive, Result};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        ar.i32(&mut self.left)?;
        ar.i32(&mut self.top)?;
        ar.i32(&mut self.right)?;
        ar.i32(&mut self.bottom)
    }
    pub fn width(&self) -> i32 {
        self.right - self.left
    }
    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        ar.i32(&mut self.x)?;
        ar.i32(&mut self.y)
    }
}

/// Win32 `LOGPEN` (16 bytes on disk).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Pen {
    pub style: u32,
    pub width: Point,
    /// `COLORREF` (0x00BBGGRR)
    pub color: u32,
}

impl Default for Pen {
    fn default() -> Self {
        Pen { style: 6, width: Point { x: 1, y: 1 }, color: 0 }
    }
}

impl Pen {
    pub fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        ar.u32(&mut self.style)?;
        self.width.ser(ar)?;
        ar.u32(&mut self.color)
    }
}

/// Win32 `LOGBRUSH`, stored as three 32-bit values (hatch truncated).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Brush {
    pub style: u32,
    /// `COLORREF` (0x00BBGGRR)
    pub color: u32,
    pub hatch: u32,
}

impl Brush {
    pub fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        ar.u32(&mut self.style)?;
        ar.u32(&mut self.color)?;
        ar.u32(&mut self.hatch)
    }
}

/// `COleDateTime` as serialized by MFC: status then `DATE`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct OleDateTime {
    pub status: i32,
    /// Days since 1899-12-30.
    pub date: f64,
}

impl OleDateTime {
    pub fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        ar.i32(&mut self.status)?;
        ar.f64(&mut self.date)
    }
}

/// Per-notation style block. A `CDrawObj` carries three of them; the third
/// (offsets 0xE0/0xF4/0xF8...) was added in format 0x17, the extra brush in 0x22.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ViewStyle {
    pub a: u16,
    pub b: u16,
    pub c: u16,
    pub pen: Pen,
    pub brush: Brush,
    pub brush2: Brush,
}

/// Object type code stored at `CDrawObj+0x140` (set by each constructor;
/// distinct from the class code returned by vtable `+0x238`).
pub mod kind {
    pub const RELATION: u32 = 1;
    /// UML class of an association (owned by its `CDrawRelation`).
    pub const ENTITE_UML: u32 = 2;
    pub const ENTITE: u32 = 4;
    pub const IMAGE: u32 = 8;
    pub const CIF: u32 = 0x10;
    pub const ARC: u32 = 0x20;
    pub const TEXTE: u32 = 0x40;
    pub const TITRE: u32 = 0x80;
    pub const HERITAGE: u32 = 0x100;
    pub const ARC_HERITAGE: u32 = 0x200;
    pub const ARC_HERITAGE_UML: u32 = 0x400;
    pub const CONTRAINTE: u32 = 0x800;
    pub const ARC_CONTRAINTE: u32 = 0x1000;
    pub const ARC_CIF: u32 = 0x2000;
    pub const REGLE: u32 = 0x4000;
    pub const ARC_REGLE: u32 = 0x8000;
    pub const TRAIT: u32 = 0x10000;
    pub const FLECHE: u32 = 0x20000;
    pub const RECTANGLE: u32 = 0x40000;
    pub const ELLIPSE: u32 = 0x80000;
    pub const ARC_OBJET_LIBRE: u32 = 0x100000;
    pub const PROF: u32 = 0x200000;
    pub const CRAYON: u32 = 0x400000;
}

/// Fields shared by every drawing object (`CDrawObj::Serialize`, 0x1400b2980).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct DrawBase {
    /// 0x140 — type code, see [`kind`].
    pub kind: u32,
    pub styles: [ViewStyle; 3],
    /// 0x10 — document-unique id, used by the link pass.
    pub id: i32,
    /// 0x308..0x314
    pub f308: [i32; 4],
    /// 0x318
    pub f318: i32,
    /// 0x338 (format >= 0x2d)
    pub f338: i32,
    /// Second pass (`CDrawObj` vtable slot 3, 0x1400b3840): ids of
    /// text0 (0x148), text1 (0x150), text2 (0x158), previous segment (0x360),
    /// next segment (0x368) and owner (0x370).
    pub links: Links,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Links {
    pub texts: [i32; 3],
    pub prev: i32,
    pub next: i32,
    pub owner: i32,
}

impl DrawBase {
    pub fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        let v = ar.version;
        let loading = ar.is_loading();
        if loading && v < 7 {
            let mut k = 0u16;
            ar.u16(&mut k)?;
            self.kind = k as u32;
        } else {
            ar.u32(&mut self.kind)?;
        }
        let [s0, s1, s2] = &mut self.styles;
        ar.u16(&mut s0.a)?;
        ar.u16(&mut s0.b)?;
        if loading && v < 0x17 {
            s1.a = s0.a;
            s2.a = s0.a;
            s1.b = s0.b;
            s2.b = s0.b;
        } else {
            ar.u16(&mut s1.a)?;
            ar.u16(&mut s1.b)?;
            ar.u16(&mut s2.a)?;
            ar.u16(&mut s2.b)?;
        }
        s0.pen.ser(ar)?;
        s0.brush.ser(ar)?;
        if loading && v < 0x17 {
            s1.pen = s0.pen;
            s1.brush = s0.brush;
            s2.pen = s0.pen;
            s2.brush = s0.brush;
        } else {
            s1.pen.ser(ar)?;
            s1.brush.ser(ar)?;
            s2.pen.ser(ar)?;
            s2.brush.ser(ar)?;
        }
        ar.i32(&mut self.id)?;
        for x in &mut self.f308 {
            ar.i32(x)?;
        }
        if loading && v < 2 {
            self.f318 = 6;
        } else {
            ar.i32(&mut self.f318)?;
        }
        if loading && v < 0x22 {
            for s in [&mut *s0, &mut *s1, &mut *s2] {
                s.c = s.b;
                s.brush2 = s.brush;
            }
        } else {
            ar.u16(&mut s0.c)?;
            ar.u16(&mut s1.c)?;
            ar.u16(&mut s2.c)?;
            s0.brush2.ser(ar)?;
            s1.brush2.ser(ar)?;
            s2.brush2.ser(ar)?;
        }
        if !loading || v >= 0x2d {
            ar.i32(&mut self.f338)?;
        }
        Ok(())
    }

    pub fn ser_links(&mut self, ar: &mut Archive) -> Result<()> {
        let l = &mut self.links;
        for t in &mut l.texts {
            ar.i32(t)?;
        }
        ar.i32(&mut l.prev)?;
        ar.i32(&mut l.next)?;
        ar.i32(&mut l.owner)
    }
}

/// `CRattach` (0x1401039e0): anchor of an arc end.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Rattach {
    pub mode: u16,
    pub offset: Point,
    pub reserved: i32,
}

impl Rattach {
    pub fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        ar.u16(&mut self.mode)?;
        self.offset.ser(ar)?;
        ar.i32(&mut self.reserved)
    }
}

// ------------------------------------------------------------------ classes

/// `CDrawArc` and its subclasses (one segment of a link).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Arc {
    /// 0x3c8..0x3d0 (format > 0x26)
    pub f3c8: [i32; 3],
    /// 0x3d8 (format > 0x26)
    pub label: String,
    /// 0x398, serialized in the link pass.
    pub anchor: Rattach,
}

impl Arc {
    fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        if !ar.is_loading() || ar.version > 0x26 {
            for x in &mut self.f3c8 {
                ar.i32(x)?;
            }
            ar.string(&mut self.label)?;
        }
        Ok(())
    }
}

/// `CDrawEntite` / `CDrawEntiteUML` (0x1400c4210).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Entite {
    /// bit 0 -> 0x4f0, bit 1 -> 0x4f4
    pub flags: u16,
    /// 0x4d0
    pub rect: Rect,
}

impl Entite {
    fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        ar.u16(&mut self.flags)?;
        self.rect.ser(ar)
    }
    /// Fictive entity: no table generated in the MLD.
    pub fn is_fictive(&self) -> bool {
        self.flags & 1 != 0
    }

    /// "Classe d'entités appartenant à un modèle externe": no table, but
    /// its key still migrates.
    pub fn is_external(&self) -> bool {
        self.flags & 2 != 0
    }
}

/// `CDrawRelation` (association, 0x1400ddba0).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Relation {
    /// 0x390 — centre of the association symbol.
    pub center: Point,
    /// 0x398
    pub f398: i32,
    /// 0x3a0
    pub f3a0: i32,
    /// 0x3a4 (format >= 3)
    pub f3a4: Point,
    /// 0x3ac (format > 0x18)
    pub f3ac: i32,
    /// 0x39c (format > 0x1d)
    pub f39c: i32,
    pub reserved: [i32; 2],
    pub reserved_str: String,
}

impl Relation {
    fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        let v = ar.version;
        let all = !ar.is_loading();
        self.center.ser(ar)?;
        ar.i32(&mut self.f398)?;
        ar.i32(&mut self.f3a0)?;
        if all || v >= 3 {
            self.f3a4.ser(ar)?;
        }
        if all || v > 0x18 {
            ar.i32(&mut self.f3ac)?;
        }
        if all || v > 0x1d {
            ar.i32(&mut self.f39c)?;
            ar.i32(&mut self.reserved[0])?;
            ar.i32(&mut self.reserved[1])?;
            ar.string(&mut self.reserved_str)?;
        }
        Ok(())
    }
}

/// `CUnParametre` (0x1400a6800): one line of a text block. For entities and
/// associations this is an attribute ("rubrique"); for arc texts it holds
/// the cardinality.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Parametre {
    pub f8: i32,
    /// 0x10 — attribute name
    pub name: String,
    pub f20: i32,
    pub f24: i32,
    // format > 0xf
    pub f28: i32,
    pub s48: String,
    pub s50: String,
    pub s58: String,
    pub f60: i32,
    pub s18: String,
    pub s118: String,
    // format > 0x13
    pub s138: String,
    pub f140: i32,
    pub f144: i32,
    /// format > 0x27
    pub f14c: i32,
    /// format > 0x19
    pub f148: i32,
    pub f2c: [i32; 4],
    pub f84: i32,
    pub f88: i32,
    /// format > 0xd
    pub f8c: i32,
    pub f90: i32,
    pub s98: String,
    pub fa0: u64,
    pub fa8: i32,
    /// 0xb0 — creation date
    pub created: OleDateTime,
    /// 0xc0 — modification date
    pub modified: OleDateTime,
    pub fd0: i32,
    pub f64: i32,
    /// 0x68..0x78 (format >= 0x24, else copies of f64)
    pub f68: [i32; 5],
    pub f7c: i32,
    pub f80: i32,
    /// 0xd4 — line role: 1 = cardinality, 2 = role/label
    pub role: i32,
    /// 0xd8 — cardinality: 0 "0,1", 1 "1,1", 2 "0,n", 3 "1,n", 4 custom
    pub cardinality: i32,
    /// 0xe0 / 0xe8 — custom cardinality min / max
    pub card_min: String,
    pub card_max: String,
    pub sf0: String,
    /// raw value stored for 0x12c (inverted flag, format >= 0x11)
    pub n12c: i32,
    /// raw value stored for 0x130 (inverted flag, format > 0x11)
    pub n130: i32,
    /// format >= 0x1f
    pub f134: i32,
    pub sf8: String,
    /// format >= 4
    pub s100: String,
    pub f108: i32,
    /// format >= 5
    pub f10c: i32,
    /// format >= 6 (default 3)
    pub f110: i32,
    pub f114: i32,
    pub s120: String,
    /// format >= 7
    pub s40: String,
    /// format >= 0xb
    pub f128: i32,
}

impl Parametre {
    pub fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        let v = ar.version;
        let all = !ar.is_loading();
        ar.i32(&mut self.f8)?;
        ar.string(&mut self.name)?;
        ar.i32(&mut self.f20)?;
        ar.i32(&mut self.f24)?;
        if all || v > 0xf {
            ar.i32(&mut self.f28)?;
            ar.string(&mut self.s48)?;
            ar.string(&mut self.s50)?;
            ar.string(&mut self.s58)?;
            ar.i32(&mut self.f60)?;
            ar.string(&mut self.s18)?;
            ar.string(&mut self.s118)?;
        }
        if all || v > 0x13 {
            ar.string(&mut self.s138)?;
            ar.i32(&mut self.f140)?;
            ar.i32(&mut self.f144)?;
        }
        if all || v > 0x27 {
            ar.i32(&mut self.f14c)?;
        }
        if all || v > 0x19 {
            ar.i32(&mut self.f148)?;
        }
        for x in &mut self.f2c {
            ar.i32(x)?;
        }
        ar.i32(&mut self.f84)?;
        ar.i32(&mut self.f88)?;
        if all || v > 0xd {
            ar.i32(&mut self.f8c)?;
        }
        ar.i32(&mut self.f90)?;
        ar.string(&mut self.s98)?;
        ar.u64(&mut self.fa0)?;
        ar.i32(&mut self.fa8)?;
        self.created.ser(ar)?;
        self.modified.ser(ar)?;
        ar.i32(&mut self.fd0)?;
        ar.i32(&mut self.f64)?;
        if all || v >= 0x24 {
            for x in &mut self.f68 {
                ar.i32(x)?;
            }
        } else {
            self.f68 = [self.f64; 5];
        }
        ar.i32(&mut self.f7c)?;
        ar.i32(&mut self.f80)?;
        ar.i32(&mut self.role)?;
        ar.i32(&mut self.cardinality)?;
        ar.string(&mut self.card_min)?;
        ar.string(&mut self.card_max)?;
        ar.string(&mut self.sf0)?;
        if all || v >= 0x11 {
            ar.i32(&mut self.n12c)?;
        }
        if all || v > 0x11 {
            ar.i32(&mut self.n130)?;
        }
        if all || v >= 0x1f {
            ar.i32(&mut self.f134)?;
        }
        ar.string(&mut self.sf8)?;
        if all || v >= 4 {
            ar.string(&mut self.s100)?;
            ar.i32(&mut self.f108)?;
        }
        if all || v >= 5 {
            ar.i32(&mut self.f10c)?;
        }
        if all || v >= 6 {
            ar.i32(&mut self.f110)?;
            ar.i32(&mut self.f114)?;
            ar.string(&mut self.s120)?;
        } else {
            self.f110 = 3;
        }
        if all || v >= 7 {
            ar.string(&mut self.s40)?;
        }
        if all || v >= 0xb {
            ar.i32(&mut self.f128)?;
        }
        Ok(())
    }

    /// Text shown for a cardinality line (0x14009dea0), ignoring the
    /// relative-identification suffix.
    pub fn cardinality_text(&self) -> String {
        match self.cardinality {
            0 => "0,1".into(),
            1 => "1,1".into(),
            3 => "1,n".into(),
            4 => {
                if self.card_min.is_empty() {
                    self.card_max.clone()
                } else if self.card_max.is_empty() {
                    self.card_min.clone()
                } else {
                    format!("{},{}", self.card_min, self.card_max)
                }
            }
            _ => "0,n".into(),
        }
    }
}

/// `CDrawTexte` (0x1400e52c0). Entities, associations and arcs own up to
/// three of these (name, attribute list, extra).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Texte {
    pub f458: i32,
    pub f45c: i32,
    /// 0x468 — `CStringList`, first line is the displayed name.
    pub lines: Vec<String>,
    /// 0x4a0 (format > 0xd)
    pub lines2: Vec<String>,
    /// 0x4d8 (format > 0xd)
    pub f4d8: i32,
    /// 0x5f8 (format > 0xf)
    pub s5f8: String,
    /// 0x460 (format > 0x29)
    pub f460: i32,
    pub reserved: [i32; 2],
    /// 0x390 + LOGFONTW at 0x394 (and a trailing int), twice.
    #[serde(skip)]
    pub font_blocks: [Vec<u8>; 4],
    /// 0x4dc — position of this text in its owner (0 name, 1 body, 2 extra).
    pub slot: u16,
    /// 0x4e0 — `CPtrListParametre`
    pub params: Vec<Parametre>,
}

impl Default for Texte {
    fn default() -> Self {
        Texte {
            f458: 0,
            f45c: 0,
            lines: vec![String::new()],
            lines2: vec![String::new()],
            f4d8: 0,
            s5f8: String::new(),
            f460: 0,
            reserved: [0; 2],
            // the int before each LOGFONTW (+0x390) is 1 in Looping's files:
            // the text uses the document font, not its own (zeroed) one
            font_blocks: [flagged_font_block(), vec![0; 0x5c], flagged_font_block(), vec![0; 0x5c]],
            slot: 0,
            params: Vec::new(),
        }
    }
}

fn flagged_font_block() -> Vec<u8> {
    let mut b = vec![0; 100];
    b[0] = 1;
    b
}

impl Texte {
    fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        let v = ar.version;
        let all = !ar.is_loading();
        ar.i32(&mut self.f458)?;
        ar.i32(&mut self.f45c)?;
        ar.string_list(&mut self.lines)?;
        if all || v > 0xd {
            ar.string_list(&mut self.lines2)?;
            ar.i32(&mut self.f4d8)?;
        }
        if all || v > 0xf {
            ar.string(&mut self.s5f8)?;
        }
        if all || v > 0x29 {
            ar.i32(&mut self.f460)?;
            ar.i32(&mut self.reserved[0])?;
            ar.i32(&mut self.reserved[1])?;
        }
        for (i, len) in [100usize, 0x5c, 100, 0x5c].into_iter().enumerate() {
            self.font_blocks[i].resize(len, 0);
            ar.bytes(&mut self.font_blocks[i])?;
        }
        ar.u16(&mut self.slot)?;
        let mut n = self.params.len() as i32;
        ar.i32(&mut n)?;
        if ar.is_loading() {
            self.params.clear();
            for _ in 0..n.max(0) {
                let mut p = Parametre::default();
                p.ser(ar)?;
                self.params.push(p);
            }
        } else {
            for p in &mut self.params {
                p.ser(ar)?;
            }
        }
        Ok(())
    }

    pub fn first_line(&self) -> &str {
        self.lines.first().map(String::as_str).unwrap_or("")
    }

    /// Decoded `LOGFONTW` of the first font block.
    pub fn font(&self) -> LogFont {
        LogFont::parse(&self.font_blocks[1])
    }
}

/// Subset of `LOGFONTW` that matters for rendering.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct LogFont {
    pub height: i32,
    pub weight: i32,
    pub italic: bool,
    pub underline: bool,
    pub face: String,
}

impl LogFont {
    pub fn parse(b: &[u8]) -> Self {
        if b.len() < 92 {
            return LogFont::default();
        }
        let i = |o: usize| i32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        let units: Vec<u16> =
            b[28..92].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
        LogFont {
            height: i(0),
            weight: i(16),
            italic: b[20] != 0,
            underline: b[21] != 0,
            face: String::from_utf16_lossy(&units),
        }
    }
}

/// `CDrawImage` (0x1400d28a0).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Image {
    pub reserved: u16,
    /// 0x398
    pub rect: Rect,
    /// 0x390 — image file path (relative to the document)
    pub path: String,
    /// 0x3a8 border width, 0x3ac border style, 0x3b0 shadow depth, 0x3b4
    /// shadow colour (format > 0x2d; constructor 0x1400d2250 defaults)
    pub f3a8: [i32; 4],
    /// 0x3b8 (format > 0x2e)
    pub s3b8: String,
    pub f3c0: i32,
}

impl Default for Image {
    fn default() -> Self {
        Image {
            reserved: 0,
            rect: Rect::default(),
            path: String::new(),
            f3a8: [1, 0, 5, 0xc0c0c0],
            s3b8: String::new(),
            f3c0: 0,
        }
    }
}

impl Image {
    fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        let v = ar.version;
        let all = !ar.is_loading();
        ar.u16(&mut self.reserved)?;
        self.rect.ser(ar)?;
        ar.string(&mut self.path)?;
        if all || v > 0x2d {
            for x in &mut self.f3a8 {
                ar.i32(x)?;
            }
        }
        if all || v > 0x2e {
            ar.string(&mut self.s3b8)?;
            ar.i32(&mut self.f3c0)?;
        }
        Ok(())
    }
}

/// `CDrawCif` (functional integrity constraint, 0x1400b8b00).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Cif {
    pub reserved: u16,
    pub center: Point,
    pub f398: i32,
    pub rect: Rect,
    /// format > 0x14
    pub f3ac: [i32; 2],
}

impl Cif {
    fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        ar.u16(&mut self.reserved)?;
        self.center.ser(ar)?;
        ar.i32(&mut self.f398)?;
        self.rect.ser(ar)?;
        if !ar.is_loading() || ar.version > 0x14 {
            ar.i32(&mut self.f3ac[0])?;
            ar.i32(&mut self.f3ac[1])?;
        }
        Ok(())
    }
}

/// `CDrawContrainte` (inter-association constraint, 0x1400bd8b0).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Contrainte {
    pub reserved: u16,
    pub center: Point,
    pub f3e0: i32,
    pub rect: Rect,
}

impl Contrainte {
    fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        ar.u16(&mut self.reserved)?;
        self.center.ser(ar)?;
        ar.i32(&mut self.f3e0)?;
        self.rect.ser(ar)
    }
}

/// `CDrawCrayon` (free-hand stroke, 0x1400c1630).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Crayon {
    pub f460: i32,
    pub f464: i32,
    pub f468: i32,
    pub origin: Point,
    pub points: Vec<Point>,
}

impl Crayon {
    fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        ar.i32(&mut self.f460)?;
        ar.i32(&mut self.f464)?;
        ar.i32(&mut self.f468)?;
        self.origin.ser(ar)?;
        let mut n = self.points.len() as i32;
        ar.i32(&mut n)?;
        if ar.is_loading() {
            self.points = vec![Point::default(); n.max(0) as usize];
        }
        for p in &mut self.points {
            p.ser(ar)?;
        }
        Ok(())
    }
}

/// `CDrawEllipse` (0x1400c2850) and `CDrawRectangle` (0x1400d7960) share
/// this layout; the rectangle stores one more int (0x3b0).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Shape {
    pub reserved: u16,
    pub rect: Rect,
    pub f3a0: i32,
    pub f3a4: i32,
    /// rectangle only
    pub f3b0: i32,
    pub f3a8: i32,
    pub f3ac: i32,
}

impl Shape {
    fn ser(&mut self, ar: &mut Archive, rectangle: bool) -> Result<()> {
        ar.u16(&mut self.reserved)?;
        self.rect.ser(ar)?;
        ar.i32(&mut self.f3a0)?;
        ar.i32(&mut self.f3a4)?;
        ar.skip_i32()?;
        if rectangle {
            ar.i32(&mut self.f3b0)?;
        }
        ar.i32(&mut self.f3a8)?;
        ar.i32(&mut self.f3ac)
    }
}

/// `CDrawTrait` / `CDrawFleche` (free line / arrow, 0x1400f0da0).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Trait {
    pub f3a8: i32,
    pub f3ac: i32,
    /// format > 7
    pub f3b0: i32,
    pub f3b4: i32,
    /// format > 0xb
    pub f3b8: i32,
    /// 0x390, serialized in the link pass.
    pub anchor: Rattach,
}

impl Trait {
    fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        let v = ar.version;
        let all = !ar.is_loading();
        ar.i32(&mut self.f3a8)?;
        ar.i32(&mut self.f3ac)?;
        if all || v > 7 {
            ar.i32(&mut self.f3b0)?;
            ar.i32(&mut self.f3b4)?;
        }
        if all || v > 0xb {
            ar.i32(&mut self.f3b8)?;
        }
        Ok(())
    }
}

/// `CDrawHeritage` (inheritance symbol, 0x1400cf940).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Heritage {
    pub reserved: u16,
    pub rect: Rect,
    pub rect2: Rect,
    /// format > 0x32
    pub f418: i32,
}

impl Heritage {
    fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        ar.u16(&mut self.reserved)?;
        self.rect.ser(ar)?;
        self.rect2.ser(ar)?;
        if !ar.is_loading() || ar.version > 0x32 {
            ar.i32(&mut self.f418)?;
        }
        Ok(())
    }
}

/// `CDrawProf` (0x1400d5d60).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Prof {
    pub reserved: u16,
    pub rect: Rect,
    pub p3b0: Point,
    pub f3b8: i32,
    /// format > 0x1a
    pub f3bc: i32,
}

impl Prof {
    fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        ar.u16(&mut self.reserved)?;
        self.rect.ser(ar)?;
        self.p3b0.ser(ar)?;
        ar.i32(&mut self.f3b8)?;
        if !ar.is_loading() || ar.version > 0x1a {
            ar.i32(&mut self.f3bc)?;
        }
        Ok(())
    }
}

/// `CDrawRegle` (business rule note, 0x1400d8710).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Regle {
    /// raw flag; 0x3ac = (raw == 0)
    pub raw_flag: u16,
    pub rect: Rect,
    pub f3a0: i32,
    /// format > 0xd
    pub f3a4: i32,
    /// format > 0x25
    pub f3a8: i32,
}

impl Regle {
    fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        let v = ar.version;
        let all = !ar.is_loading();
        ar.u16(&mut self.raw_flag)?;
        self.rect.ser(ar)?;
        ar.i32(&mut self.f3a0)?;
        if all || v > 0xd {
            ar.i32(&mut self.f3a4)?;
        }
        if all || v > 0x25 {
            ar.i32(&mut self.f3a8)?;
        }
        Ok(())
    }
}

/// `CDrawTitre` (diagram title block, 0x1400ef320).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Titre {
    pub f390: u16,
    pub rect: Rect,
    /// format > 8
    pub f3a4: [i32; 2],
    /// format > 9
    pub f3ac: [i32; 6],
}

impl Titre {
    fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        let v = ar.version;
        let all = !ar.is_loading();
        ar.u16(&mut self.f390)?;
        self.rect.ser(ar)?;
        if all || v > 8 {
            for x in &mut self.f3a4 {
                ar.i32(x)?;
            }
        }
        if all || v > 9 {
            for x in &mut self.f3ac {
                ar.i32(x)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum ObjKind {
    Arc(Arc),
    Entite(Entite),
    Relation(Relation),
    Texte(Texte),
    Image(Image),
    Cif(Cif),
    Contrainte(Contrainte),
    Crayon(Crayon),
    Ellipse(Shape),
    Rectangle(Shape),
    Trait(Trait),
    Heritage(Heritage),
    Prof(Prof),
    Regle(Regle),
    Titre(Titre),
}

/// A drawing object as stored in the document's `CObList`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DrawObj {
    /// MFC class name (`CDrawEntite`, `CDrawArcHeritage`, ...).
    pub class: String,
    pub base: DrawBase,
    pub kind: ObjKind,
}

/// Every class name Looping 4.1 registers with `IMPLEMENT_SERIAL` (schema 0).
pub const CLASSES: &[&str] = &[
    "CDrawArc",
    "CDrawArcCif",
    "CDrawArcContrainte",
    "CDrawArcHeritage",
    "CDrawArcHeritageUML",
    "CDrawArcObjetLibre",
    "CDrawArcRegle",
    "CDrawCif",
    "CDrawContrainte",
    "CDrawCrayon",
    "CDrawEllipse",
    "CDrawEntite",
    "CDrawEntiteUML",
    "CDrawFleche",
    "CDrawHeritage",
    "CDrawImage",
    "CDrawProf",
    "CDrawRectangle",
    "CDrawRegle",
    "CDrawRelation",
    "CDrawTexte",
    "CDrawTitre",
    "CDrawTrait",
];

impl DrawObj {
    pub fn empty(class: &str) -> Option<Self> {
        let kind = match class {
            c if c.starts_with("CDrawArc") => ObjKind::Arc(Arc::default()),
            "CDrawEntite" | "CDrawEntiteUML" => ObjKind::Entite(Entite::default()),
            "CDrawRelation" => ObjKind::Relation(Relation::default()),
            "CDrawTexte" => ObjKind::Texte(Texte::default()),
            "CDrawImage" => ObjKind::Image(Image::default()),
            "CDrawCif" => ObjKind::Cif(Cif::default()),
            "CDrawContrainte" => ObjKind::Contrainte(Contrainte::default()),
            "CDrawCrayon" => ObjKind::Crayon(Crayon::default()),
            "CDrawEllipse" => ObjKind::Ellipse(Shape::default()),
            "CDrawRectangle" => ObjKind::Rectangle(Shape::default()),
            "CDrawTrait" | "CDrawFleche" => ObjKind::Trait(Trait::default()),
            "CDrawHeritage" => ObjKind::Heritage(Heritage::default()),
            "CDrawProf" => ObjKind::Prof(Prof::default()),
            "CDrawRegle" => ObjKind::Regle(Regle::default()),
            "CDrawTitre" => ObjKind::Titre(Titre::default()),
            _ => return None,
        };
        Some(DrawObj { class: class.to_owned(), base: DrawBase::default(), kind })
    }

    /// First pass: `Serialize` (vtable slot 2).
    pub fn ser(&mut self, ar: &mut Archive) -> Result<()> {
        self.base.ser(ar)?;
        match &mut self.kind {
            ObjKind::Arc(x) => x.ser(ar),
            ObjKind::Entite(x) => x.ser(ar),
            ObjKind::Relation(x) => x.ser(ar),
            ObjKind::Texte(x) => x.ser(ar),
            ObjKind::Image(x) => x.ser(ar),
            ObjKind::Cif(x) => x.ser(ar),
            ObjKind::Contrainte(x) => x.ser(ar),
            ObjKind::Crayon(x) => x.ser(ar),
            ObjKind::Ellipse(x) => x.ser(ar, false),
            ObjKind::Rectangle(x) => x.ser(ar, true),
            ObjKind::Trait(x) => x.ser(ar),
            ObjKind::Heritage(x) => x.ser(ar),
            ObjKind::Prof(x) => x.ser(ar),
            ObjKind::Regle(x) => x.ser(ar),
            ObjKind::Titre(x) => x.ser(ar),
        }
    }

    /// Second pass: vtable slot 3.
    pub fn ser_links(&mut self, ar: &mut Archive) -> Result<()> {
        self.base.ser_links(ar)?;
        match &mut self.kind {
            ObjKind::Arc(a) => a.anchor.ser(ar),
            ObjKind::Trait(t) => t.anchor.ser(ar),
            _ => Ok(()),
        }
    }

    pub fn id(&self) -> i32 {
        self.base.id
    }

    pub fn as_texte(&self) -> Option<&Texte> {
        match &self.kind {
            ObjKind::Texte(t) => Some(t),
            _ => None,
        }
    }
}
