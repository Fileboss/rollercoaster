//! Backend-independent display list for the conceptual diagram.
//!
//! Geometry comes from the document (entity rectangles, association centres
//! and radii). Texts of entities, associations and links are laid out as
//! Looping does (`CDrawTexte` layout 0x1400e9670, drawing 0x1400e5a20), with
//! GDI metrics of Carlito ([`crate::text`]); the other objects' texts are
//! approximated.

use loo_format::{Document, DrawObj, ObjKind, Rect, Texte};
use serde::Serialize;

use crate::mcd::{object_name, ObjId};
use crate::text::Font;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Color(pub u8, pub u8, pub u8);

impl Color {
    pub const BLACK: Color = Color(0, 0, 0);
    pub const WHITE: Color = Color(255, 255, 255);

    /// Win32 `COLORREF` (0x00BBGGRR).
    pub fn from_colorref(c: u32) -> Color {
        Color((c & 0xff) as u8, ((c >> 8) & 0xff) as u8, ((c >> 16) & 0xff) as u8)
    }

    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct P {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct R {
    pub min: P,
    pub max: P,
}

impl R {
    pub fn from_rect(r: &Rect) -> R {
        R {
            min: P { x: r.left.min(r.right) as f32, y: r.top.min(r.bottom) as f32 },
            max: P { x: r.left.max(r.right) as f32, y: r.top.max(r.bottom) as f32 },
        }
    }
    pub fn center(&self) -> P {
        P { x: (self.min.x + self.max.x) / 2.0, y: (self.min.y + self.max.y) / 2.0 }
    }
    pub fn contains(&self, p: P) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
    pub fn union(&self, o: &R) -> R {
        R {
            min: P { x: self.min.x.min(o.min.x), y: self.min.y.min(o.min.y) },
            max: P { x: self.max.x.max(o.max.x), y: self.max.y.max(o.max.y) },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum Align {
    Left,
    Center,
    Right,
}

impl Align {
    /// Left edge of a text `w` wide anchored at `x`.
    pub fn left(self, x: f32, w: f32) -> f32 {
        match self {
            Align::Left => x,
            Align::Center => x - w / 2.0,
            Align::Right => x - w,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Shape {
    Rect {
        r: R,
        fill: Option<Color>,
        stroke: Option<Color>,
        radius: f32,
    },
    /// Box with a title band (entities, associations): the band of height
    /// `header` uses `header_fill`, the rest `body_fill`.
    Box {
        r: R,
        radius: f32,
        header: f32,
        header_fill: Color,
        body_fill: Color,
        stroke: Color,
        /// dotted outline (`PS_DOT`)
        dashed: bool,
    },
    Ellipse {
        c: P,
        rx: f32,
        ry: f32,
        fill: Option<Color>,
        stroke: Color,
    },
    Line {
        points: Vec<P>,
        stroke: Color,
        dashed: bool,
        /// pen width (1 for the MCD objects)
        width: f32,
    },
    /// Filled polygon (arrow heads).
    Polygon {
        points: Vec<P>,
        fill: Color,
    },
    /// Shape of an illustration (rectangle, ellipse, text frame), with its
    /// own pen width and style; `radius` rounds the corners of a rectangle.
    Figure {
        r: R,
        ellipse: bool,
        radius: f32,
        fill: Option<Color>,
        stroke: Option<Color>,
        width: f32,
        dashed: bool,
    },
    /// One line of text: `pos.y` is the top of its cell (`tmHeight` high),
    /// `pos.x` its left edge or its centre according to `align`. A
    /// `background` fills the cell (GDI opaque mode).
    Text {
        pos: P,
        text: String,
        font: Font,
        color: Color,
        align: Align,
        background: Option<Color>,
    },
    /// Drop shadow: a filled (rounded) rectangle without outline.
    Shadow {
        r: R,
        radius: f32,
        color: Color,
    },
    Image {
        r: R,
        path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Item {
    /// Object the item belongs to (for hit testing / selection).
    pub obj: ObjId,
    /// `CDrawTexte` a text item comes from (0 otherwise): dragging it moves
    /// the text (`+0x458/+0x45c`).
    pub text: ObjId,
    pub shape: Shape,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Scene {
    pub items: Vec<Item>,
}

/// Outline of the part of a rounded rectangle above `r.min.y + band`,
/// as a convex polygon (clockwise from the bottom-left of the band).
pub fn rounded_band(r: &R, radius: f32, band: f32) -> Vec<P> {
    let w = r.max.x - r.min.x;
    let h = r.max.y - r.min.y;
    let rad = radius.min(w / 2.0).min(h / 2.0).max(0.0);
    let band = band.min(h);
    // horizontal inset of the outline at depth `d` below the top
    let inset = |d: f32| {
        if rad <= 0.0 || d >= rad {
            0.0
        } else {
            let dy = rad - d;
            rad - (rad * rad - dy * dy).max(0.0).sqrt()
        }
    };
    let steps = 12;
    let mut left = Vec::new();
    for i in 0..=steps {
        let d = band * (1.0 - i as f32 / steps as f32);
        left.push(P { x: r.min.x + inset(d), y: r.min.y + d });
    }
    let mut pts = left.clone();
    for p in left.iter().rev() {
        pts.push(P { x: r.max.x - (p.x - r.min.x), y: p.y });
    }
    pts
}

/// Document font at `off` (see `options::FIELDS`).
fn doc_font(doc: &Document, off: usize) -> Font {
    Font::from_logfont(&doc.settings.logfont(off))
}

/// Font of a text: its own `LOGFONTW` (`+0x394`) when the flag before it
/// (`+0x390`) is 0, else the document font of its kind.
fn text_font(doc: &Document, t: &Texte, doc_off: usize) -> Font {
    let b = &t.font_blocks[0];
    if b.len() >= 96 && i32::from_le_bytes([b[0], b[1], b[2], b[3]]) == 0 {
        Font::from_logfont(&loo_format::LogFont::parse(&b[4..96]))
    } else {
        doc_font(doc, doc_off)
    }
}

/// Text colour at `off` (-1: black).
fn text_color(doc: &Document, off: usize) -> Color {
    doc.settings.color(off).map_or(Color::BLACK, Color::from_colorref)
}

fn texte<'a>(doc: &'a Document, o: &DrawObj, slot: usize) -> Option<&'a Texte> {
    doc.object(o.base.links.texts[slot]).and_then(DrawObj::as_texte)
}

/// Size of the rectangle of a text of an entity or association as Looping
/// computes it (0x1400e9670): widest line + 2, lines × line height; slot 0
/// is the name, slot 1 the attributes (identifiers and, per « Usage du
/// gras », UNIQUE or NOT NULL ones in bold).
pub fn text_extent(doc: &Document, owner: ObjId, slot: usize) -> Option<(i32, i32)> {
    let o = doc.object(owner)?;
    let (name_off, attr_off) = match o.kind {
        ObjKind::Entite(_) => (0x40c, 0x5d0),
        ObjKind::Relation(_) => (0x468, 0x574),
        _ => return None,
    };
    let t = texte(doc, o, slot)?;
    if slot == 0 {
        let font = text_font(doc, t, name_off);
        let lines = name_lines(t);
        let w = lines.iter().map(|l| font.width(l)).max().unwrap_or(0);
        return Some((w + 2, font.height() * lines.len() as i32));
    }
    let font = text_font(doc, t, attr_off);
    let bold_use = doc.settings.i32(0x7ec);
    let rows: Vec<i32> = t
        .params
        .iter()
        .filter(|p| p.role == 0)
        .map(|p| {
            let bold = p.f8 != 0 || (p.f8c != 0 && bold_use == 0) || (p.f88 != 0 && bold_use == 1);
            font.with_bold(font.bold || bold).width(&p.name)
        })
        .collect();
    let w = rows.iter().copied().max().unwrap_or(0);
    Some((if rows.is_empty() { 0 } else { w + 2 }, font.height() * rows.len() as i32))
}

/// Lines of a name text (`+0x468`), without the trailing empty ones.
fn name_lines(t: &Texte) -> Vec<&str> {
    let n = t.lines.iter().rposition(|l| !l.is_empty()).map_or(0, |i| i + 1);
    t.lines[..n].iter().map(String::as_str).collect()
}

fn text_item(obj: ObjId, x: i32, top: i32, text: &str, font: Font, color: Color, align: Align) -> Item {
    Item {
        obj,
        text: 0,
        shape: Shape::Text {
            pos: P { x: x as f32, y: top as f32 },
            text: text.to_string(),
            font,
            color,
            align,
            background: None,
        },
    }
}

/// Marks the items pushed since `start` as drawn from the text `text`.
fn mark(s: &mut Scene, start: usize, text: ObjId) {
    for it in &mut s.items[start..] {
        if matches!(it.shape, Shape::Text { .. }) {
            it.text = text;
        }
    }
}

/// Name text centred on (`ax`, `ay`) (alignment 6/6): its rectangle is
/// (width + 2) × (lines × height), moved by `+0x458/+0x45c`; each line is
/// drawn with `TA_CENTER` at the middle of that rectangle.
fn centered_name(o: &DrawObj, t: &Texte, font: Font, color: Color, (ax, ay): (i32, i32), s: &mut Scene) {
    let lines = name_lines(t);
    let w = lines.iter().map(|l| font.width(l)).max().unwrap_or(0);
    let lh = font.height();
    let left = ax + t.f458 - w / 2;
    let top = ay + t.f45c - lh * lines.len() as i32 / 2;
    let x = left + (w + 2) / 2;
    for (k, l) in lines.iter().enumerate() {
        s.items.push(text_item(o.id(), x, top + lh * k as i32, l, font, color, Align::Center));
    }
}

/// The « Types données » column of an entity or association: the SQL type
/// of each attribute for the document's DBMS (`+0x52c`), in the « type »
/// font, `column` from the left of the names (10 after the longest one).
#[derive(Clone)]
struct Types {
    font: Font,
    color: Color,
    column: i32,
    rows: Vec<(i32, String)>,
}

impl Types {
    fn of(doc: &Document, t: &Texte, names: Font, font_off: usize, color_off: usize) -> Types {
        let dbms = crate::Dbms::from_index(doc.settings.i32(0x52c));
        let font = doc_font(doc, font_off);
        let attrs: Vec<&loo_format::Parametre> = t.params.iter().filter(|p| p.role == 0).collect();
        let column = attrs.iter().map(|p| names.with_bold(true).width(&p.name)).max().unwrap_or(0) + 10;
        let rows = attrs
            .iter()
            .map(|p| {
                let text = crate::DataType::from_param(p).sql(dbms, false);
                (font.width(&text), text)
            })
            .collect();
        Types { font, color: text_color(doc, color_off), column, rows }
    }

    /// Width of the widest type.
    fn widest(&self) -> i32 {
        self.rows.iter().map(|r| r.0).max().unwrap_or(0)
    }
}

/// Attribute rows from (`left`, `top`): one row per parameter of role 0,
/// `TA_BOTTOM` at `top + row × height`; identifiers in bold, underlined two
/// pixels above the bottom of their row, and in bold too the UNIQUE (`+0x8c`)
/// or NOT NULL (`+0x88`) attributes according to « Usage du gras » (`+0x7ec`).
#[allow(clippy::too_many_arguments)]
fn attribute_rows(
    doc: &Document,
    o: &DrawObj,
    t: &Texte,
    font: Font,
    color: Color,
    (left, top): (i32, i32),
    types: Option<Types>,
    s: &mut Scene,
) {
    let rh = font.height();
    if let Some(ty) = types {
        for (i, (_, text)) in ty.rows.iter().enumerate() {
            let bottom = top + rh * (i as i32 + 1);
            s.items.push(text_item(
                o.id(),
                left + ty.column,
                bottom - ty.font.height(),
                text,
                ty.font,
                ty.color,
                Align::Left,
            ));
        }
    }
    let bold_use = doc.settings.i32(0x7ec);
    for (i, p) in t.params.iter().filter(|p| p.role == 0).enumerate() {
        let bottom = top + rh * (i as i32 + 1);
        let bold = p.f8 != 0 || (p.f8c != 0 && bold_use == 0) || (p.f88 != 0 && bold_use == 1);
        let f = font.with_bold(font.bold || bold);
        s.items.push(text_item(o.id(), left, bottom - f.height(), &p.name, f, color, Align::Left));
        if p.f8 != 0 {
            let y = (bottom - 2) as f32;
            s.items.push(Item {
                obj: o.id(),
                text: 0,
                shape: Shape::Line {
                    points: vec![P { x: left as f32, y }, P { x: (left + f.width(&p.name)) as f32, y }],
                    stroke: color,
                    dashed: false,
                    width: 1.0,
                },
            });
        }
    }
}

fn fill_of(o: &DrawObj) -> Option<Color> {
    let b = o.base.styles[0].brush;
    if b.style == 1 {
        None
    } else {
        Some(Color::from_colorref(b.color))
    }
}

/// Fill of an object that may follow the document colours (style `b`):
/// default background by class (0x1400b6270).
fn default_fill(doc: &Document, o: &DrawObj) -> Option<Color> {
    let own = fill_of(o);
    if o.base.styles[0].b == 0 || own.is_none() {
        return own;
    }
    let off = match &o.kind {
        ObjKind::Heritage(_) => 0x7b0,
        ObjKind::Contrainte(_) => 0x7b4,
        ObjKind::Cif(_) => 0x7b8,
        ObjKind::Regle(_) => 0x7bc,
        _ => return own,
    };
    Some(doc_color(doc, off))
}

fn stroke_of(o: &DrawObj) -> Color {
    Color::from_colorref(o.base.styles[0].pen.color)
}

/// Bounding box of an object as drawn (used for links and hit testing).
pub fn bounds(doc: &Document, o: &DrawObj) -> Option<R> {
    match &o.kind {
        ObjKind::Entite(e) => Some(R::from_rect(&e.rect)),
        ObjKind::Relation(r) => {
            let (rx, ry) = relation_radii(doc, o, r);
            let c = P { x: r.center.x as f32, y: r.center.y as f32 };
            Some(R { min: P { x: c.x - rx, y: c.y - ry }, max: P { x: c.x + rx, y: c.y + ry } })
        }
        ObjKind::Image(i) => Some(R::from_rect(&i.rect)),
        ObjKind::Regle(x) => Some(R::from_rect(&x.rect)),
        ObjKind::Ellipse(x) | ObjKind::Rectangle(x) => Some(R::from_rect(&x.rect)),
        ObjKind::Cif(x) => Some(R::from_rect(&x.rect)),
        ObjKind::Contrainte(x) => Some(R::from_rect(&x.rect)),
        ObjKind::Heritage(x) => Some(R::from_rect(&x.rect)),
        ObjKind::Titre(x) => Some(R::from_rect(&x.rect)),
        ObjKind::Prof(x) => Some(R::from_rect(&x.rect)),
        ObjKind::Crayon(c) => {
            let pts = crayon_points(c);
            let f = *pts.first()?;
            Some(pts.iter().fold(R { min: f, max: f }, |a, p| a.union(&R { min: *p, max: *p })))
        }
        _ => None,
    }
}

fn doc_color(doc: &Document, off: usize) -> Color {
    Color::from_colorref(doc.settings.u32(off))
}

/// Drop shadows, drawn before everything (pass with `param_4 != 0` of the
/// drawing functions; depth set by 0x1400b4990): entities `+0x550` deep in
/// `+0x554`, offset by depth + 1; associations `+0x558` in `+0x55c`, offset
/// by depth.
fn shadows(doc: &Document, d: &Display, s: &mut Scene) {
    for o in doc.objects.iter().filter(|o| d.shows(doc, o)) {
        let (r, radius, depth, color) = match &o.kind {
            ObjKind::Entite(e) if o.class == "CDrawEntite" => {
                let depth = doc.settings.i32(0x550);
                (entity_box(&e.rect, entity_types(doc, d, o).as_ref()), 0.0, depth + 1, 0x554)
            }
            // `+0x3b0` deep in `+0x3b4`, when filled (0x1400d3fb0)
            ObjKind::Image(i) if o.base.f308[2] != 0 && i.f3a8[2] > 0 => {
                let r = R::from_rect(&i.rect);
                let d = (i.f3a8[2] + 1) as f32;
                s.items.push(Item {
                    obj: o.id(),
                    text: 0,
                    shape: Shape::Shadow {
                        // the export's extent stops at rect + depth + 1 (one pixel short
                        // of the border's right and bottom lines)
                        r: R { min: P { x: r.min.x + d, y: r.min.y + d }, max: P { x: r.max.x + d, y: r.max.y + d } },
                        radius: 0.0,
                        color: Color::from_colorref(i.f3a8[3] as u32),
                    },
                });
                continue;
            }
            // `+0x3a8` deep in `+0x3ac`, when filled (0x1400c3850), offset by depth + 1
            ObjKind::Rectangle(x) | ObjKind::Ellipse(x) if o.base.f308[2] != 0 && x.f3a8 > 0 => {
                let ellipse = matches!(o.kind, ObjKind::Ellipse(_));
                let d = (x.f3a8 + 1) as f32;
                let r = R::from_rect(&x.rect);
                s.items.push(Item {
                    obj: o.id(),
                    text: 0,
                    shape: Shape::Figure {
                        r: R { min: P { x: r.min.x + d, y: r.min.y + d }, max: P { x: r.max.x + d, y: r.max.y + d } },
                        ellipse,
                        radius: if ellipse { 0.0 } else { x.f3b0.max(0) as f32 / 2.0 },
                        fill: Some(Color::from_colorref(x.f3ac as u32)),
                        stroke: None,
                        width: 1.0,
                        dashed: false,
                    },
                });
                continue;
            }
            // rule: the pentagon, as the associations' shadow
            ObjKind::Regle(x) => {
                let depth = doc.settings.i32(0x558);
                if depth > 0 {
                    let dd = (depth + 1) as f32;
                    let r = R::from_rect(&x.rect);
                    let moved =
                        R { min: P { x: r.min.x + dd, y: r.min.y + dd }, max: P { x: r.max.x + dd, y: r.max.y + dd } };
                    s.items.push(Item {
                        obj: o.id(),
                        text: 0,
                        shape: Shape::Polygon { points: pentagon(&moved), fill: doc_color(doc, 0x55c) },
                    });
                }
                continue;
            }
            // inheritance: the triangle, `+0x558` deep in `+0x55c` (as the associations)
            ObjKind::Heritage(x) => {
                let depth = doc.settings.i32(0x558);
                if depth > 0 {
                    let dd = (depth + 1) as f32;
                    let r = R::from_rect(&x.rect);
                    let moved =
                        R { min: P { x: r.min.x + dd, y: r.min.y + dd }, max: P { x: r.max.x + dd, y: r.max.y + dd } };
                    s.items.push(Item {
                        obj: o.id(),
                        text: 0,
                        shape: Shape::Polygon { points: triangle(&moved), fill: doc_color(doc, 0x55c) },
                    });
                }
                continue;
            }
            // text frame: `+0x3b0` deep (0x1400f0990), or the document's
            ObjKind::Titre(t) => {
                let st = titre_style(doc, o, t);
                if st.shadow > 0 && st.filled {
                    let d = (st.shadow + 1) as f32;
                    let r = R::from_rect(&t.rect);
                    s.items.push(Item {
                        obj: o.id(),
                        text: 0,
                        shape: Shape::Shadow {
                            r: R {
                                min: P { x: r.min.x + d, y: r.min.y + d },
                                max: P { x: r.max.x + d, y: r.max.y + d },
                            },
                            radius: st.radius.max(0) as f32 / 2.0,
                            color: st.shadow_color,
                        },
                    });
                }
                continue;
            }
            ObjKind::Relation(r) if !relation_is_circle(doc, o) => {
                let (rx, ry) = relation_radii(doc, o, r);
                let c = P { x: r.center.x as f32, y: r.center.y as f32 };
                let radius = (doc.settings.i32(0x534).min(doc.settings.i32(0x538)) as f32 / 2.0).min(rx).min(ry);
                let b = R { min: P { x: c.x - rx, y: c.y - ry }, max: P { x: c.x + rx, y: c.y + ry } };
                (b, radius, doc.settings.i32(0x558), 0x55c)
            }
            _ => continue,
        };
        let depth_set = match &o.kind {
            ObjKind::Entite(_) => doc.settings.i32(0x550),
            _ => doc.settings.i32(0x558),
        };
        if depth_set < 1 {
            continue;
        }
        let d = depth as f32;
        s.items.push(Item {
            obj: o.id(),
            text: 0,
            shape: Shape::Shadow {
                r: R { min: P { x: r.min.x + d, y: r.min.y + d }, max: P { x: r.max.x + d, y: r.max.y + d } },
                radius,
                color: doc_color(doc, color),
            },
        });
    }
}

/// True when one leg of the association has a maximum cardinality of 1
/// (`CDrawRelation` vtable +0x3e0, 0x1400e1eb0): it then generates no table.
pub fn relation_is_functional(doc: &Document, rel: &DrawObj) -> bool {
    if let ObjKind::Relation(r) = &rel.kind {
        if r.f3ac != 0 {
            return false;
        }
    }
    let id = rel.id();
    doc.objects.iter().filter(|o| o.class == "CDrawArc" && o.base.links.prev == 0).any(|first| {
        let mut last = first;
        while let Some(n) = doc.object(last.base.links.next) {
            last = n;
        }
        if first.base.links.owner != id && last.base.links.owner != id {
            return false;
        }
        doc.object(first.base.links.texts[1])
            .and_then(DrawObj::as_texte)
            .and_then(|t| t.params.iter().find(|p| p.role == 1))
            .is_some_and(|p| match p.cardinality {
                0 | 1 => true,
                4 => p.card_max.trim() == "1",
                _ => false,
            })
    })
}

/// Fill colours (title band, body) as chosen by the colour getters at
/// vtable +0x228/+0x230 (0x1400b6260, 0x1400b6530): an object whose style
/// flag is set uses the document palette, otherwise its own brushes.
fn box_colors(doc: &Document, o: &DrawObj) -> (Color, Color) {
    let st = &o.base.styles[0];
    let (brush1, brush2) = if st.b == 0 {
        (Color::from_colorref(st.brush.color), Color::from_colorref(st.brush2.color))
    } else {
        match &o.kind {
            ObjKind::Relation(_) if relation_is_functional(doc, o) => (doc_color(doc, 0xa08), doc_color(doc, 0x398)),
            ObjKind::Relation(_) => (doc_color(doc, 0xa0c), doc_color(doc, 0x800)),
            // entity of an external model (vtable +0x410)
            ObjKind::Entite(e) if e.is_external() => (doc_color(doc, 0x804), doc_color(doc, 0x804)),
            ObjKind::Entite(_) => (doc_color(doc, 0xa04), doc_color(doc, 0x394)),
            _ => (Color::from_colorref(st.brush.color), Color::from_colorref(st.brush2.color)),
        }
    };
    match &o.kind {
        // entities: band = brush 1; associations: band = brush 2 (0x1400c4540, 0x1400ddf80)
        ObjKind::Relation(_) => (brush2, brush1),
        _ => (brush1, brush2),
    }
}

fn pen_color(doc: &Document, o: &DrawObj) -> Color {
    let st = &o.base.styles[0];
    if st.a == 0 {
        return Color::from_colorref(st.pen.color);
    }
    match &o.kind {
        ObjKind::Relation(_) => doc_color(doc, 0x388),
        ObjKind::Entite(_) => doc_color(doc, 0x384),
        ObjKind::Arc(_) => doc_color(doc, 0x38c),
        _ => Color::from_colorref(st.pen.color),
    }
}

/// Simplified association: a small circle (`+0x7c0 == 2` and functional).
fn relation_is_circle(doc: &Document, o: &DrawObj) -> bool {
    doc.settings.i32(0x7c0) == 2 && relation_is_functional(doc, o)
}

/// Half-width and half-height of the association box (0x398 / 0x3a0).
fn relation_radii(doc: &Document, o: &DrawObj, r: &loo_format::Relation) -> (f32, f32) {
    if relation_is_circle(doc, o) {
        let c = r.f39c.max(4) as f32;
        return (c, c);
    }
    // drawn as stored (0x1400ddf80), checked against Looping's image export
    (r.f398.max(1) as f32, r.f3a0.max(1) as f32)
}

/// Height of the association title band: `+0xa14` (or the half-height
/// when below 5), at least half the corner height `+0x538` (0x1400ddf80).
fn relation_header(doc: &Document, r: &loo_format::Relation) -> i32 {
    let a14 = doc.settings.i32(0xa14);
    let band = if a14 < 5 { r.f3a0 } else { a14 };
    band.max(doc.settings.i32(0x538) / 2)
}

/// Point where the segment from the centre of `r` towards `to` leaves `r`.
fn clip_rect(r: &R, to: P) -> P {
    let c = r.center();
    let (dx, dy) = (to.x - c.x, to.y - c.y);
    if dx == 0.0 && dy == 0.0 {
        return c;
    }
    let hw = (r.max.x - r.min.x) / 2.0;
    let hh = (r.max.y - r.min.y) / 2.0;
    let t = (hw / dx.abs()).min(hh / dy.abs());
    P { x: c.x + dx * t.min(1.0), y: c.y + dy * t.min(1.0) }
}

fn clip_ellipse(c: P, rx: f32, ry: f32, to: P) -> P {
    let (dx, dy) = (to.x - c.x, to.y - c.y);
    let d = ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt();
    if d == 0.0 {
        return c;
    }
    P { x: c.x + dx / d, y: c.y + dy / d }
}

/// Anchor of a link end on object `o`, heading towards `to`.
fn anchor(doc: &Document, o: &DrawObj, to: P) -> Option<P> {
    match &o.kind {
        ObjKind::Relation(r) if relation_is_circle(doc, o) => {
            let (rx, ry) = relation_radii(doc, o, r);
            Some(clip_ellipse(P { x: r.center.x as f32, y: r.center.y as f32 }, rx, ry, to))
        }
        ObjKind::Ellipse(x) => {
            let b = R::from_rect(&x.rect);
            Some(clip_ellipse(b.center(), (b.max.x - b.min.x) / 2.0, (b.max.y - b.min.y) / 2.0, to))
        }
        _ => bounds(doc, o).map(|b| clip_rect(&b, to)),
    }
}

fn center_of(doc: &Document, id: ObjId) -> Option<P> {
    let o = doc.object(id)?;
    bounds(doc, o).map(|b| b.center())
}

/// What a window shows (ribbon « Affichage » and « Vue »): attributes
/// (`CLoopingView+0x3ec`), data types (`+0x3f0`) and illustrations
/// (`+0x3e8`), which Looping keeps in the document at `+0x540`, `+0x544`
/// and `+0x748`; active views (`+0x130`, bit 0: every object).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Display {
    pub attributes: bool,
    pub types: bool,
    pub illustrations: bool,
    pub views: u32,
}

impl Display {
    /// As the document was saved, all objects shown.
    pub fn of(doc: &Document) -> Display {
        Display {
            attributes: doc.settings.i32(0x540) != 0,
            types: doc.settings.i32(0x544) != 0,
            illustrations: doc.settings.i32(0x748) != 0,
            views: 1,
        }
    }

    /// Whether `o` is drawn: an illustration only with « Illustrations »
    /// (0x1400c3cf0; a sub-model's thumbnail always, 0x1400d3df0); with
    /// views, only the objects of an active view (0x1400b41f0: a document
    /// with views, a mask without bit 0, `CDrawObj+0x338 & mask == 0` hides).
    pub fn shows(&self, doc: &Document, o: &DrawObj) -> bool {
        let illustration = match &o.kind {
            ObjKind::Image(i) => i.f3c0 == 0,
            ObjKind::Rectangle(_)
            | ObjKind::Ellipse(_)
            | ObjKind::Titre(_)
            | ObjKind::Crayon(_)
            | ObjKind::Trait(_) => true,
            _ => o.class == "CDrawArcObjetLibre",
        };
        if illustration && !self.illustrations {
            return false;
        }
        doc.views.is_empty() || self.views & 1 != 0 || (o.base.f338 as u32) & self.views != 0
    }

    /// `shows` for a link or free line: its segments and both ends.
    fn shows_chain(&self, doc: &Document, first: &DrawObj) -> bool {
        if !self.shows(doc, first) {
            return false;
        }
        let mut cur = Some(first);
        let mut n = 0;
        while let Some(seg) = cur {
            if let Some(owner) = doc.object(seg.base.links.owner) {
                if !self.shows(doc, owner) {
                    return false;
                }
            }
            n += 1;
            cur = doc.object(seg.base.links.next).filter(|_| n < 1000);
        }
        true
    }
}

pub fn build(doc: &Document) -> Scene {
    build_with(doc, &Display::of(doc))
}

pub fn build_with(doc: &Document, d: &Display) -> Scene {
    let mut s = Scene::default();
    shadows(doc, d, &mut s);
    // in document order, as Looping draws: a leg goes from centre to centre
    // and is hidden by the boxes drawn after it
    for o in &doc.objects {
        let chained = o.class.starts_with("CDrawArc") || matches!(o.kind, ObjKind::Trait(_));
        if if chained { o.base.links.prev == 0 && !d.shows_chain(doc, o) } else { !d.shows(doc, o) } {
            continue;
        }
        if o.class.starts_with("CDrawArc") {
            if o.base.links.prev == 0 {
                link(doc, o, &mut s);
            }
            continue;
        }
        match &o.kind {
            ObjKind::Entite(e) if o.class == "CDrawEntite" => entity(doc, d, o, &e.rect, &mut s),
            ObjKind::Relation(r) => relation(doc, d, o, r, &mut s),
            ObjKind::Image(i) => image(doc, o, i, &mut s),
            ObjKind::Rectangle(x) => s.items.push(figure(o, x, false)),
            ObjKind::Ellipse(x) => s.items.push(figure(o, x, true)),
            ObjKind::Regle(x) => regle(doc, o, x, &mut s),
            ObjKind::Titre(x) => titre(doc, o, x, &mut s),
            ObjKind::Prof(x) => boxed(doc, o, &x.rect, 0.0, &mut s),
            ObjKind::Cif(x) => symbol(doc, o, &x.rect, "CIF", &mut s),
            // constraint type (`CUnParametre+0x110` of the symbol's parameter)
            ObjKind::Contrainte(x) => {
                let label = ["X", "T", "XT", "S", "I"].get(symbol_param(doc, o) as usize).copied().unwrap_or("");
                symbol(doc, o, &x.rect, label, &mut s)
            }
            ObjKind::Heritage(x) => {
                let label = ["", "X", "T", "XT"].get(symbol_param(doc, o) as usize).copied().unwrap_or("");
                heritage(doc, o, &x.rect, label, &mut s)
            }
            ObjKind::Trait(_) if o.base.links.prev == 0 => free_line(doc, o, &mut s),
            // pen width `+0x464`, style `+0x468`
            ObjKind::Crayon(c) => s.items.push(Item {
                obj: o.id(),
                text: 0,
                shape: Shape::Line {
                    points: crayon_points(c),
                    stroke: stroke_of(o),
                    dashed: c.f468 != 0,
                    width: c.f464.max(1) as f32,
                },
            }),
            _ => {}
        }
    }
    s
}

/// Entity (0x1400c4540): title band of `+0x530`, outline and band line.
/// Types column of an entity, when shown.
fn entity_types(doc: &Document, d: &Display, o: &DrawObj) -> Option<Types> {
    let tx = texte(doc, o, 1).filter(|_| d.attributes && d.types)?;
    Some(Types::of(doc, tx, text_font(doc, tx, 0x5d0), 0x6e4, 0x56c))
}

/// Box of an entity as drawn: with the types, widened on both sides by the
/// widest type and 15 (the borders of Looping's `c49` exactly; the stored
/// rectangle is unchanged).
fn entity_box(rect: &Rect, types: Option<&Types>) -> R {
    let mut r = R::from_rect(rect);
    if let Some(ty) = types {
        let extra = (ty.widest() + 15) as f32;
        r.min.x -= (extra / 2.0).floor();
        r.max.x += extra - (extra / 2.0).floor();
    }
    r
}

fn entity(doc: &Document, d: &Display, o: &DrawObj, rect: &Rect, s: &mut Scene) {
    let id = o.id();
    let attrs = texte(doc, o, 1).filter(|_| d.attributes);
    let types = entity_types(doc, d, o);
    let r = entity_box(rect, types.as_ref());
    let (l, t, rr, b) = (r.min.x as i32, r.min.y as i32, r.max.x as i32, r.max.y as i32);
    let hb = (t + doc.settings.i32(0x530)).min(b);
    let (header_fill, body_fill) = box_colors(doc, o);
    let stroke = pen_color(doc, o);
    s.items.push(Item {
        obj: id,
        text: 0,
        shape: Shape::Box {
            r,
            radius: 0.0,
            header: (hb - t) as f32,
            header_fill,
            body_fill,
            stroke,
            // entity of an external model: dotted pen (0x1400b48f0)
            dashed: matches!(&o.kind, ObjKind::Entite(e) if e.is_external()),
        },
    });
    // name: centred in the band (0x1400c59c0, slot 0)
    if let Some(tx) = texte(doc, o, 0) {
        let font = text_font(doc, tx, 0x40c);
        let start = s.items.len();
        centered_name(o, tx, font, text_color(doc, 0x3a4), ((l + rr) / 2, (t + hb) / 2), s);
        mark(s, start, o.base.links.texts[0]);
    }
    // attributes: left-aligned, `doc+0x308` (5) from the left edge, below the band
    if let Some(tx) = attrs {
        let font = text_font(doc, tx, 0x5d0);
        let start = s.items.len();
        attribute_rows(doc, o, tx, font, text_color(doc, 0x560), (l + 5 + tx.f458, hb + 1 + tx.f45c), types, s);
        mark(s, start, o.base.links.texts[1]);
    }
}

fn relation(doc: &Document, d: &Display, o: &DrawObj, r: &loo_format::Relation, s: &mut Scene) {
    let id = o.id();
    let (rx, ry) = relation_radii(doc, o, r);
    let c = P { x: r.center.x as f32, y: r.center.y as f32 };
    let (header_fill, body_fill) = box_colors(doc, o);
    let stroke = pen_color(doc, o);
    let title = doc_font(doc, 0x468);
    if relation_is_circle(doc, o) {
        s.items.push(Item { obj: id, text: 0, shape: Shape::Ellipse { c, rx, ry, fill: Some(body_fill), stroke } });
        let name = object_name(doc, o);
        let top = (c.y - ry) as i32 - title.height() - 2;
        s.items.push(text_item(id, c.x as i32, top, &name, title, text_color(doc, 0x3a8), Align::Center));
        return;
    }
    let (cx, cy, hh) = (r.center.x, r.center.y, ry as i32);
    let b = R { min: P { x: c.x - rx, y: c.y - ry }, max: P { x: c.x + rx, y: c.y + ry } };
    let header = relation_header(doc, r);
    // RoundRect with an ellipse of (0x534, 0x538)
    let radius = (doc.settings.i32(0x534).min(doc.settings.i32(0x538)) as f32 / 2.0).min(rx).min(ry);
    s.items.push(Item {
        obj: id,
        text: 0,
        shape: Shape::Box {
            r: b,
            radius,
            header: (header as f32).min(2.0 * ry),
            header_fill,
            body_fill,
            stroke,
            dashed: false,
        },
    });
    // bottom of the band (0x1400e0710)
    let band = (cy - hh + header).min(cy + hh);
    if let Some(tx) = texte(doc, o, 0) {
        let font = text_font(doc, tx, 0x468);
        let start = s.items.len();
        centered_name(o, tx, font, text_color(doc, 0x3a8), (cx, (band - hh + cy) / 2), s);
        mark(s, start, o.base.links.texts[0]);
    }
    // attributes: left-aligned, 2 × `doc+0x308` from the left edge
    if let Some(tx) = texte(doc, o, 1).filter(|_| d.attributes) {
        let font = text_font(doc, tx, 0x574);
        let types = d.types.then(|| Types::of(doc, tx, font, 0x688, 0x570));
        let start = s.items.len();
        attribute_rows(
            doc,
            o,
            tx,
            font,
            text_color(doc, 0x564),
            (cx - rx as i32 + 10 + tx.f458, band + 1 + tx.f45c),
            types,
            s,
        );
        mark(s, start, o.base.links.texts[1]);
    }
}

fn link(doc: &Document, first: &DrawObj, s: &mut Scene) {
    let mut chain = vec![first];
    while let Some(n) = doc.object(chain.last().unwrap().base.links.next) {
        if chain.len() > 1000 {
            break;
        }
        chain.push(n);
    }
    // a link has at least two segments (one per end)
    if chain.len() < 2 {
        return;
    }
    let a_id = first.base.links.owner;
    let b_id = chain.last().unwrap().base.links.owner;
    let (Some(a), Some(b)) = (doc.object(a_id), doc.object(b_id)) else {
        return;
    };
    let (Some(ca), Some(cb)) = (center_of(doc, a_id), center_of(doc, b_id)) else {
        return;
    };
    // intermediate segments without owner are break points
    let bends: Vec<P> = chain[1..chain.len().saturating_sub(1)]
        .iter()
        .filter(|seg| seg.base.links.owner == 0)
        .filter_map(|seg| match &seg.kind {
            ObjKind::Arc(arc) => Some(P { x: arc.anchor.offset.x as f32, y: arc.anchor.offset.y as f32 }),
            _ => None,
        })
        .collect();
    let toward_a = bends.first().copied().unwrap_or(cb);
    let toward_b = bends.last().copied().unwrap_or(ca);
    let (Some(pa), Some(pb)) = (anchor(doc, a, toward_a), anchor(doc, b, toward_b)) else {
        return;
    };
    // dotted (PS_DOT, 0x14009a130 argument): rule links, a CIF's link to its
    // association, a constraint's link to an entity (as Looping draws `c28`,
    // `c45`); the others solid
    let to_kind = |want: fn(&ObjKind) -> bool| want(&a.kind) || want(&b.kind);
    let dashed = match first.class.as_str() {
        "CDrawArcRegle" => true,
        "CDrawArcCif" => to_kind(|k| matches!(k, ObjKind::Relation(_))),
        "CDrawArcContrainte" => to_kind(|k| matches!(k, ObjKind::Entite(_))),
        _ => false,
    };
    let mut points = vec![pa];
    points.extend(bends.iter().copied());
    points.push(pb);
    // drawn between the attach points (0x14009ceb0: the owner's, 0x1400c5910
    // for an entity: the centre of its rectangle), not clipped; an
    // illustration, often unfilled, is left at its border
    let attach = |o: &DrawObj, c: P, clipped: P| match &o.kind {
        ObjKind::Entite(e) => {
            P { x: ((e.rect.left + e.rect.right) / 2) as f32, y: ((e.rect.top + e.rect.bottom) / 2) as f32 }
        }
        ObjKind::Relation(r) => P { x: r.center.x as f32, y: r.center.y as f32 },
        ObjKind::Rectangle(_) | ObjKind::Ellipse(_) | ObjKind::Image(_) | ObjKind::Titre(_) => clipped,
        _ => c,
    };
    // « Flèche pour DF »: the leg ends at the entity's border, under an arrow
    let df = df_arrow(doc, first, a, b);
    let mut line = vec![if df && matches!(a.kind, ObjKind::Entite(_)) { pa } else { attach(a, ca, pa) }];
    line.extend(bends.iter().copied());
    line.push(if df && matches!(b.kind, ObjKind::Entite(_)) { pb } else { attach(b, cb, pb) });
    if df {
        let (tip, from) = if matches!(a.kind, ObjKind::Entite(_)) { (pa, toward_a) } else { (pb, toward_b) };
        let size = 9.0 + 3.0 * doc.settings.i32(0x9f8).clamp(0, 4) as f32;
        s.items.push(Item {
            obj: first.id(),
            text: 0,
            shape: Shape::Polygon { points: arrow_triangle(from, tip, size), fill: Color::BLACK },
        });
    }
    // filled arrow toward the supertype (inheritance), the targeted entity
    // (CIF) or, for an inclusion constraint, its target (0x1400a30a0,
    // 0x1400a1540, 0x1400a2aa0), at the end that is not the symbol
    let flag = |role: i32| {
        doc.object(first.base.links.texts[1])
            .and_then(DrawObj::as_texte)
            .and_then(|t| t.params.iter().find(|p| p.role == role))
            .is_some_and(|p| p.f114 != 0)
    };
    let symbol_kind = |k: &ObjKind| matches!(k, ObjKind::Heritage(_) | ObjKind::Cif(_) | ObjKind::Contrainte(_));
    let arrow = match first.class.as_str() {
        "CDrawArcHeritage" | "CDrawArcHeritageUML" => flag(3),
        "CDrawArcCif" => flag(4),
        "CDrawArcContrainte" => {
            flag(4) && [a, b].iter().any(|o| matches!(o.kind, ObjKind::Contrainte(_)) && symbol_param(doc, o) == 4)
        }
        _ => false,
    };
    if arrow {
        // the leg stops at the border, under the arrow
        if symbol_kind(&a.kind) {
            *line.last_mut().unwrap() = pb;
        } else {
            line[0] = pa;
        }
        let (tip, from) = if symbol_kind(&a.kind) { (pb, toward_b) } else { (pa, toward_a) };
        let size = 9.0 + 3.0 * doc.settings.i32(0x9f8).clamp(0, 4) as f32;
        s.items.push(Item {
            obj: first.id(),
            text: 0,
            shape: Shape::Polygon { points: arrow_triangle(from, tip, size), fill: Color::BLACK },
        });
    }
    // cardinality (0x14009cf30, MCD view): centred on the middle of the
    // first segment from the entity, or of the next one when that segment
    // is at most `doc+0x320` (40) long; drawn transparent, the leg being
    // interrupted under it
    let caption = link_caption(doc, first, &points);
    let label = cardinality(doc, first, matches!(a.kind, ObjKind::Entite(_)), points);
    let pieces = match &label {
        Some((_, r)) => cut_out(&line, r),
        None => vec![line],
    };
    for points in pieces {
        s.items.push(Item {
            obj: first.id(),
            text: 0,
            shape: Shape::Line { points, stroke: Color::BLACK, dashed, width: 1.0 },
        });
    }
    if let Some((item, r)) = label {
        s.items.push(item);
        if let Some(role) = role_text(doc, first) {
            let font = doc_font(doc, 0x994);
            let cx = ((r.min.x + r.max.x) / 2.0) as i32;
            s.items.push(text_item(first.id(), cx, r.max.y as i32, &role, font, text_color(doc, 0x9f4), Align::Center));
        }
    }
    s.items.extend(caption);
}

/// Label of a link to a symbol (`+0x120` of its parameter: role 3
/// inheritance criterion, 4 CIF or constraint, 5 rule, 6 illustrations),
/// centred on the middle of its first
/// segment (as in Looping's export of `TP 1`), in the font « Liens — Police
/// autres textes » (`+0x994`).
fn link_caption(doc: &Document, first: &DrawObj, points: &[P]) -> Option<Item> {
    let role = match first.class.as_str() {
        "CDrawArcHeritage" => 3,
        "CDrawArcCif" | "CDrawArcContrainte" => 4,
        "CDrawArcRegle" => 5,
        "CDrawArcObjetLibre" => 6,
        _ => return None,
    };
    let text = texte(doc, first, 1)?.params.iter().find(|p| p.role == role).map(|p| p.s120.as_str())?;
    if text.trim().is_empty() || points.len() < 2 {
        return None;
    }
    let (a, b) = (points[0], points[1]);
    let font = doc_font(doc, 0x994);
    let (x, y) = (((a.x + b.x) / 2.0) as i32, ((a.y + b.y) / 2.0) as i32);
    Some(text_item(first.id(), x, y - font.height() / 2, text, font, text_color(doc, 0x9f4), Align::Center))
}

/// Cardinality text of a leg and its rectangle; `points` go from the first
/// owner to the last, clipped to both boxes.
/// Role of a leg as shown: `+0xf0` of its role parameter, unless `+0x12c`
/// (stored inverted) hides it.
fn role_text(doc: &Document, first: &DrawObj) -> Option<String> {
    let t = texte(doc, first, 1)?;
    let p = t.params.iter().find(|p| p.role == 2)?;
    (p.n12c == 0 && !p.sf0.trim().is_empty()).then(|| p.sf0.clone())
}

/// With a role, the cardinality goes up by a line of the role's font and
/// the role under it (as Looping draws `c02` and `c03`).
fn cardinality(doc: &Document, first: &DrawObj, entity_first: bool, mut pts: Vec<P>) -> Option<(Item, R)> {
    let card = texte(doc, first, 1)?;
    let p = card.params.iter().find(|p| p.role == 1)?;
    if !entity_first {
        pts.reverse();
    }
    let len = |i: usize| ((pts[i + 1].x - pts[i].x).powi(2) + (pts[i + 1].y - pts[i].y).powi(2)).sqrt();
    let seg = if pts.len() > 2 && len(0) <= 40.0 { 1 } else { 0 };
    let (p1, p2) = ((pts[seg].x as i32, pts[seg].y as i32), (pts[seg + 1].x as i32, pts[seg + 1].y as i32));
    let (mx, my) = ((p1.0 + p2.0) / 2, (p1.1 + p2.1) / 2);
    let mut font = text_font(doc, card, 0x62c);
    // relative identification (`+0x84`): notation `+0xa00`
    let text = match (p.cardinality, p.f84 != 0, doc.settings.i32(0xa00)) {
        (1, true, 0) => "1,1(R)".to_string(),
        (1, true, 1) => "(1,1)".to_string(),
        (1, true, _) => {
            font.underline = true;
            "1,1".to_string()
        }
        _ => p.cardinality_text(),
    };
    let w = font.width(&text);
    let h = font.height();
    let below = if role_text(doc, first).is_some() { doc_font(doc, 0x994).height() } else { 0 };
    let left = mx + card.f458 - w / 2;
    let top = my + card.f45c - h / 2 - below;
    let rect = R { min: P { x: left as f32, y: top as f32 }, max: P { x: (left + w + 2) as f32, y: (top + h) as f32 } };
    let item = Item {
        obj: first.id(),
        text: first.base.links.texts[1],
        shape: Shape::Text {
            pos: P { x: (left + (w + 2) / 2) as f32, y: top as f32 },
            text,
            font,
            color: text_color(doc, 0x568),
            align: Align::Center,
            background: None,
        },
    };
    Some((item, rect))
}

/// Parts of a polyline outside `r`.
fn cut_out(points: &[P], r: &R) -> Vec<Vec<P>> {
    let mut out: Vec<Vec<P>> = vec![Vec::new()];
    let lerp = |a: P, b: P, t: f32| P { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t };
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        // Liang-Barsky: part of the segment inside the rectangle
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let (mut t0, mut t1) = (0.0f32, 1.0f32);
        let mut inside = true;
        for (p, q) in [(-dx, a.x - r.min.x), (dx, r.max.x - a.x), (-dy, a.y - r.min.y), (dy, r.max.y - a.y)] {
            if p == 0.0 {
                if q < 0.0 {
                    inside = false;
                }
            } else {
                let t = q / p;
                if p < 0.0 {
                    t0 = t0.max(t);
                } else {
                    t1 = t1.min(t);
                }
            }
        }
        let cur = out.last_mut().unwrap();
        if cur.is_empty() {
            cur.push(a);
        }
        if !inside || t0 >= t1 {
            cur.push(b);
            continue;
        }
        if t0 > 0.0 {
            cur.push(lerp(a, b, t0));
        }
        let mut next = Vec::new();
        if t1 < 1.0 {
            next.push(lerp(a, b, t1));
            next.push(b);
        }
        out.push(next);
    }
    out.into_iter().filter(|l| l.len() >= 2).collect()
}

/// Free line / arrow (`CDrawTrait` / `CDrawFleche`, drawn by 0x1400f11e0):
/// a chain of segments, one point per segment (its anchor `+0x390`, or the
/// object it is attached to); dashed when `+0x3ac` is set. The arrow
/// points to the end of the chain.
fn free_line(doc: &Document, first: &DrawObj, s: &mut Scene) {
    let mut chain = vec![first];
    while let Some(n) = doc.object(chain.last().unwrap().base.links.next) {
        if chain.len() > 1000 {
            break;
        }
        chain.push(n);
    }
    let free: Vec<Option<P>> = chain
        .iter()
        .map(|seg| match &seg.kind {
            ObjKind::Trait(t) if seg.base.links.owner == 0 => {
                Some(P { x: t.anchor.offset.x as f32, y: t.anchor.offset.y as f32 })
            }
            _ => None,
        })
        .collect();
    let mut points = Vec::new();
    for (i, seg) in chain.iter().enumerate() {
        let p = match free[i] {
            Some(p) => Some(p),
            None => {
                // attached end: clipped toward the neighbouring free point
                let toward = free.get(i + 1).or(i.checked_sub(1).and_then(|j| free.get(j))).copied().flatten();
                doc.object(seg.base.links.owner).and_then(|o| match toward {
                    Some(t) => anchor(doc, o, t),
                    None => bounds(doc, o).map(|b| b.center()),
                })
            }
        };
        points.extend(p);
    }
    if points.len() < 2 {
        return;
    }
    // pen width `+0x3a8`, style `+0x3ac`
    let (width, dashed) = match &first.kind {
        ObjKind::Trait(t) => (t.f3a8.max(1) as f32, t.f3ac != 0),
        _ => (1.0, false),
    };
    let stroke = stroke_of(first);
    if first.class == "CDrawFleche" {
        let n = points.len();
        s.items.push(Item {
            obj: first.id(),
            text: 0,
            shape: Shape::Line { points: arrow_head(points[n - 2], points[n - 1]), stroke, dashed: false, width: 1.0 },
        });
    }
    s.items.push(Item { obj: first.id(), text: 0, shape: Shape::Line { points, stroke, dashed, width } });
}

/// Points of a free-hand stroke: offsets from its origin.
pub fn crayon_points(c: &loo_format::Crayon) -> Vec<P> {
    c.points.iter().map(|p| P { x: (c.origin.x + p.x) as f32, y: (c.origin.y + p.y) as f32 }).collect()
}

/// Rectangle (0x1400d7e60) or ellipse (0x1400c2d10) of the illustrations:
/// filled when `+0x310`, outlined when `+0x314`, pen width `+0x3a0` and
/// style `+0x3a4`; the rectangle's corners are rounded by `+0x3b0` (the
/// ellipse size of `RoundRect`).
fn figure(o: &DrawObj, x: &loo_format::Shape, ellipse: bool) -> Item {
    let [_, _, filled, outlined] = o.base.f308;
    Item {
        obj: o.id(),
        text: 0,
        shape: Shape::Figure {
            r: R::from_rect(&x.rect),
            ellipse,
            radius: if ellipse { 0.0 } else { x.f3b0.max(0) as f32 / 2.0 },
            fill: if filled != 0 { fill_of(o) } else { None },
            stroke: (outlined != 0).then(|| stroke_of(o)),
            width: x.f3a0.max(1) as f32,
            dashed: x.f3a4 != 0,
        },
    }
}

/// Resolved look of a text frame (`CDrawTitre`): each property is the
/// object's own or, when its « Par défaut » flag is set, the document's
/// (section « Textes » of the property grid).
pub struct TitreStyle {
    pub filled: bool,
    pub outlined: bool,
    pub fill: Color,
    pub stroke: Color,
    pub width: i32,
    pub radius: i32,
    pub shadow: i32,
    pub shadow_color: Color,
}

pub fn titre_style(doc: &Document, o: &DrawObj, t: &loo_format::Titre) -> TitreStyle {
    let set = |off: usize| doc.settings.i32(off);
    let pick = |default: i32, off: usize, own: i32| if default != 0 { set(off) } else { own };
    let [fill_default, outline_default, filled, outlined] = o.base.f308;
    let st = &o.base.styles[0];
    TitreStyle {
        filled: pick(fill_default, 0x740, filled) != 0,
        outlined: pick(outline_default, 0x744, outlined) != 0,
        fill: Color::from_colorref(if st.b != 0 { doc.settings.u32(0x390) } else { st.brush.color }),
        stroke: Color::from_colorref(if st.a != 0 { doc.settings.u32(0x380) } else { st.pen.color }),
        width: pick(t.f3ac[3], 0x370, t.f3ac[0]),
        radius: pick(t.f3a4[0], 0x7c8, t.f3a4[1]),
        shadow: pick(t.f3ac[4], 0x548, t.f3ac[1]),
        shadow_color: Color::from_colorref(pick(t.f3ac[5], 0x54c, t.f3ac[2]) as u32),
    }
}

/// Text frame (« Texte » tool, `CDrawTitre`, 0x1400ef8c0): a rounded
/// rectangle around the lines of its slot-0 text, aligned horizontally by
/// `+0x318` (`TA_LEFT` 0, `TA_CENTER` 6, `TA_RIGHT` 2) and vertically by
/// `+0x390` (top 0, middle 6, bottom 8), in the font « Textes » (`+0x3b0`)
/// unless the text has its own.
fn titre(doc: &Document, o: &DrawObj, t: &loo_format::Titre, s: &mut Scene) {
    let st = titre_style(doc, o, t);
    let r = R::from_rect(&t.rect);
    s.items.push(Item {
        obj: o.id(),
        text: 0,
        shape: Shape::Figure {
            r,
            ellipse: false,
            radius: st.radius.max(0) as f32 / 2.0,
            fill: st.filled.then_some(st.fill),
            stroke: st.outlined.then_some(st.stroke),
            width: st.width.max(1) as f32,
            dashed: false,
        },
    });
    let Some(text) = texte(doc, o, 0) else { return };
    let font = text_font(doc, text, 0x3b0);
    let lines = name_lines(text);
    let h = font.height() * lines.len() as i32;
    let (l, top, rr, b) = (r.min.x as i32, r.min.y as i32, r.max.x as i32, r.max.y as i32);
    let margin = 4;
    let y0 = match t.f390 {
        6 => (top + b - h) / 2,
        8 => b - margin - h,
        _ => top + margin,
    };
    let (x, align) = match o.base.f318 {
        6 => ((l + rr) / 2, Align::Center),
        2 => (rr - margin, Align::Right),
        _ => (l + margin, Align::Left),
    };
    for (i, line) in lines.iter().enumerate() {
        s.items.push(text_item(o.id(), x, y0 + i as i32 * font.height(), line, font, text_color(doc, 0x3a0), align));
    }
}

fn boxed(doc: &Document, o: &DrawObj, rect: &Rect, radius: f32, s: &mut Scene) {
    let r = R::from_rect(rect);
    s.items.push(Item {
        obj: o.id(),
        text: 0,
        shape: Shape::Rect { r, fill: default_fill(doc, o), stroke: Some(stroke_of(o)), radius },
    });
    // approximate layout: one line after the other from the top left
    let font = doc_font(doc, 0x994);
    let mut y = r.min.y as i32 + 4;
    for t in o.base.links.texts.iter().filter_map(|&id| doc.object(id)).filter_map(DrawObj::as_texte) {
        let f = font.with_bold(t.slot == 0);
        for line in t.lines.iter().chain(t.params.iter().map(|p| &p.name)) {
            if line.trim().is_empty() || y + f.height() > r.max.y as i32 {
                continue;
            }
            s.items.push(text_item(o.id(), r.min.x as i32 + 6, y, line, f, Color::BLACK, Align::Left));
            y += f.height();
        }
    }
}

/// Image (0x1400d2d70): Looping draws its bitmap, or when the file cannot
/// be read, its background (`+0x310` set) with a border and the message
/// below in the document font `+0x3b0`. Frontends that can load the file
/// paint it over the placeholder ([`Shape::Image`]).
fn image(doc: &Document, o: &DrawObj, i: &loo_format::Image, s: &mut Scene) {
    let r = R::from_rect(&i.rect);
    // Rectangle(left, top, right + 2, bottom + 2)
    let outer = R { min: r.min, max: P { x: r.max.x + 1.0, y: r.max.y + 1.0 } };
    if o.base.f308[2] != 0 {
        s.items.push(Item {
            obj: o.id(),
            text: 0,
            shape: Shape::Rect { r: outer, fill: fill_of(o), stroke: None, radius: 0.0 },
        });
    }
    s.items.push(Item {
        obj: o.id(),
        text: 0,
        shape: Shape::Rect { r: outer, fill: None, stroke: Some(stroke_of(o)), radius: 0.0 },
    });
    let font = doc_font(doc, 0x3b0);
    let (l, t, rr, b) = (r.min.x as i32, r.min.y as i32, r.max.x as i32, r.max.y as i32);
    s.items.push(text_item(o.id(), (l + rr) / 2, (t + b) / 2 - 10, IMAGE_MISSING, font, Color::BLACK, Align::Center));
    s.items.push(Item { obj: o.id(), text: 0, shape: Shape::Image { r, path: i.path.clone() } });
}

const IMAGE_MISSING: &str = "Veuillez sélectionner une image";

/// `CUnParametre+0x110` of the first parameter of an object's slot-1 text.
fn symbol_param(doc: &Document, o: &DrawObj) -> i32 {
    doc.object(o.base.links.texts[1]).and_then(DrawObj::as_texte).and_then(|t| t.params.first()).map_or(0, |p| p.f110)
}

/// Notations « Flèche pour DF » and « Asso simplifiée » (`+0x7c0` = 1, 2;
/// 0x14009bc60): in an association with a leg of maximum cardinality 1, an
/// arrow (size `+0x9f8`) ends every other leg at its entity (as Looping
/// draws `c05` and `c13`). The leg that keeps no arrow is the first 1,1
/// leg, else the first one of maximum 1.
fn df_arrow(doc: &Document, first: &DrawObj, a: &DrawObj, b: &DrawObj) -> bool {
    if first.class != "CDrawArc" || !matches!(doc.settings.i32(0x7c0), 1 | 2) {
        return false;
    }
    let rel = match (&a.kind, &b.kind) {
        (_, ObjKind::Relation(_)) => b,
        (ObjKind::Relation(_), _) => a,
        _ => return false,
    };
    if !relation_is_functional(doc, rel) {
        return false;
    }
    let card = |leg: &DrawObj| {
        texte(doc, leg, 1).and_then(|t| t.params.iter().find(|p| p.role == 1)).map(|p| {
            let max_one = match p.cardinality {
                0 | 1 => true,
                4 => p.card_max.trim() == "1",
                _ => false,
            };
            (p.cardinality == 1, max_one)
        })
    };
    let legs: Vec<&DrawObj> = doc
        .objects
        .iter()
        .filter(|o| o.class == "CDrawArc" && o.base.links.prev == 0)
        .filter(|o| {
            let mut last = *o;
            while let Some(n) = doc.object(last.base.links.next) {
                last = n;
            }
            o.base.links.owner == rel.id() || last.base.links.owner == rel.id()
        })
        .collect();
    let child = legs
        .iter()
        .find(|l| card(l).is_some_and(|c| c.0))
        .or_else(|| legs.iter().find(|l| card(l).is_some_and(|c| c.1)))
        .map(|l| l.id());
    child.is_some_and(|c| c != first.id())
}

/// Filled arrow head of length `size` ending at `tip`, pointing away from `from`.
fn arrow_triangle(from: P, tip: P, size: f32) -> Vec<P> {
    let (dx, dy) = (tip.x - from.x, tip.y - from.y);
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let (ux, uy) = (dx / len, dy / len);
    let half = size * 0.4;
    let base = P { x: tip.x - ux * size, y: tip.y - uy * size };
    vec![P { x: base.x - uy * half, y: base.y + ux * half }, tip, P { x: base.x + uy * half, y: base.y - ux * half }]
}

/// Two strokes ending at `tip`, pointing away from `from`.
fn arrow_head(from: P, tip: P) -> Vec<P> {
    let (dx, dy) = (tip.x - from.x, tip.y - from.y);
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let (ux, uy) = (dx / len, dy / len);
    let (back, half) = (10.0, 5.0);
    let base = P { x: tip.x - ux * back, y: tip.y - uy * back };
    vec![P { x: base.x - uy * half, y: base.y + ux * half }, tip, P { x: base.x + uy * half, y: base.y - ux * half }]
}

/// Width of the constraint letter of an inheritance (0 for other objects).
pub fn heritage_letter_width(doc: &Document, id: ObjId) -> i32 {
    let Some(o) = doc.object(id).filter(|o| matches!(o.kind, ObjKind::Heritage(_))) else { return 0 };
    let label = ["", "X", "T", "XT"].get(symbol_param(doc, o) as usize).copied().unwrap_or("");
    doc_font(doc, 0x994).width(label)
}

/// Size of the folded corner of a rule (`CLoopingDoc+0x304`, always 10).
const FOLD: f32 = 10.0;

/// Pentagon of a rule (0x1400d8ad0): its top-right corner cut by `FOLD`.
fn pentagon(r: &R) -> Vec<P> {
    vec![
        r.min,
        P { x: r.max.x - FOLD, y: r.min.y },
        P { x: r.max.x, y: r.min.y + FOLD },
        r.max,
        P { x: r.min.x, y: r.max.y },
    ]
}

/// Texts of a rule, top to bottom: the name (bold, underlined, centred),
/// the description lines (slot-1 parameters, when `+0x3a0`), a rule and
/// the script lines (slot-1 `+0x4a0`, when `+0x3a4`). Returns the lines
/// with their font, and the size they need (as Looping grows a rule:
/// widest + 23, height + 5, measured on `c45`).
struct RuleText {
    title: (String, Font),
    description: Vec<String>,
    body: Font,
    script: Vec<String>,
    small: Font,
}

impl RuleText {
    fn of(doc: &Document, o: &DrawObj, x: &loo_format::Regle) -> RuleText {
        let title = texte(doc, o, 0).map(|t| name_lines(t).join(" ")).unwrap_or_default();
        let t1 = texte(doc, o, 1);
        let description = if x.f3a0 != 0 {
            t1.map(|t| t.params.iter().map(|p| p.name.clone()).collect()).unwrap_or_default()
        } else {
            Vec::new()
        };
        let script = if x.f3a4 != 0 {
            t1.map(|t| t.lines2.iter().filter(|l| !l.trim().is_empty()).cloned().collect()).unwrap_or_default()
        } else {
            Vec::new()
        };
        // a rule showing nothing else has a plain title (`c32`)
        let mut head = doc_font(doc, 0x40c).with_bold(true);
        head.underline = true;
        if x.f3a0 == 0 && x.f3a4 == 0 {
            head = doc_font(doc, 0x5d0);
        }
        RuleText { title: (title, head), description, body: doc_font(doc, 0x5d0), script, small: doc_font(doc, 0x994) }
    }

    fn widest_description(&self) -> i32 {
        self.description.iter().map(|l| self.body.width(l)).max().unwrap_or(0)
    }

    /// Width and height the texts need inside the pentagon.
    fn size(&self) -> (i32, i32) {
        let w = self
            .title
            .1
            .width(&self.title.0)
            .max(self.widest_description())
            .max(self.script.iter().map(|l| self.small.width(l)).max().unwrap_or(0));
        let mut h = 4 + self.title.1.height() + self.body.height() * self.description.len() as i32;
        if !self.script.is_empty() {
            h += 6 + self.small.height() * self.script.len() as i32;
        }
        (w + 23, h + 5)
    }
}

/// Size a rule needs for its texts (see [`RuleText`]).
pub fn rule_extent(doc: &Document, id: ObjId) -> Option<(i32, i32)> {
    let o = doc.object(id)?;
    let ObjKind::Regle(x) = &o.kind else { return None };
    Some(RuleText::of(doc, o, x).size())
}

/// Rule (0x1400d8ad0): pentagon with its folded corner in `0xc4c4c4`.
fn regle(doc: &Document, o: &DrawObj, x: &loo_format::Regle, s: &mut Scene) {
    let r = R::from_rect(&x.rect);
    let stroke = stroke_of(o);
    let line = |points: Vec<P>| Shape::Line { points, stroke, dashed: false, width: 1.0 };
    if let Some(fill) = default_fill(doc, o) {
        s.items.push(Item { obj: o.id(), text: 0, shape: Shape::Polygon { points: pentagon(&r), fill } });
    }
    let fold = vec![
        P { x: r.max.x - FOLD, y: r.min.y },
        P { x: r.max.x, y: r.min.y + FOLD },
        P { x: r.max.x - FOLD, y: r.min.y + FOLD },
    ];
    s.items.push(Item {
        obj: o.id(),
        text: 0,
        shape: Shape::Polygon { points: fold.clone(), fill: Color(0xc4, 0xc4, 0xc4) },
    });
    let mut outline = pentagon(&r);
    outline.push(r.min);
    s.items.push(Item { obj: o.id(), text: 0, shape: line(outline) });
    let mut f = fold;
    f.push(f[0]);
    s.items.push(Item { obj: o.id(), text: 0, shape: line(f) });
    let t = RuleText::of(doc, o, x);
    let (l, cx) = (r.min.x as i32 + 5, ((r.min.x + r.max.x) / 2.0) as i32);
    let mut y = r.min.y as i32 + 4;
    let (title, head) = &t.title;
    if !title.is_empty() {
        let grey = x.f3a0 == 0 && x.f3a4 == 0;
        let color = if grey { Color(0x80, 0x80, 0x80) } else { Color::BLACK };
        s.items.push(text_item(o.id(), cx, y, title, *head, color, Align::Center));
    }
    y += head.height();
    for d in &t.description {
        s.items.push(text_item(o.id(), l, y, d, t.body, Color::BLACK, Align::Left));
        y += t.body.height();
    }
    if !t.script.is_empty() {
        y += 3;
        // the rule under the description, when there is one
        if !t.description.is_empty() {
            let w = t.widest_description().max(t.script.iter().map(|l| t.small.width(l)).max().unwrap_or(0));
            s.items.push(Item {
                obj: o.id(),
                text: 0,
                shape: line(vec![P { x: l as f32, y: y as f32 }, P { x: (l + w) as f32, y: y as f32 }]),
            });
        }
        y += 3;
        for sc in &t.script {
            s.items.push(text_item(o.id(), l, y, sc, t.small, Color::BLACK, Align::Left));
            y += t.small.height();
        }
    }
}

/// Triangle of an inheritance in the MCD view (0x1400cfe30): apex at the
/// middle of the top of `+0x41c`, base along its bottom.
fn triangle(r: &R) -> Vec<P> {
    vec![P { x: (r.min.x + r.max.x) / 2.0, y: r.min.y }, P { x: r.max.x, y: r.max.y }, P { x: r.min.x, y: r.max.y }]
}

/// Inheritance (0x1400cfe30): a filled triangle, its outline, and the
/// letter of its constraint (X, T, XT) low in the middle.
fn heritage(doc: &Document, o: &DrawObj, rect: &Rect, label: &str, s: &mut Scene) {
    let r = R::from_rect(rect);
    let pts = triangle(&r);
    if let Some(fill) = default_fill(doc, o) {
        s.items.push(Item { obj: o.id(), text: 0, shape: Shape::Polygon { points: pts.clone(), fill } });
    }
    let mut outline = pts;
    outline.push(outline[0]);
    s.items.push(Item {
        obj: o.id(),
        text: 0,
        shape: Shape::Line { points: outline, stroke: stroke_of(o), dashed: false, width: 1.0 },
    });
    if !label.is_empty() {
        let font = doc_font(doc, 0x994);
        let (cx, y) = ((r.min.x + r.max.x) / 2.0, r.min.y + (r.max.y - r.min.y) * 2.0 / 3.0);
        s.items.push(text_item(
            o.id(),
            cx as i32,
            y as i32 - font.height() / 2,
            label,
            font,
            Color::BLACK,
            Align::Center,
        ));
    }
}

fn symbol(doc: &Document, o: &DrawObj, rect: &Rect, label: &str, s: &mut Scene) {
    let r = R::from_rect(rect);
    s.items.push(Item {
        obj: o.id(),
        text: 0,
        shape: Shape::Ellipse {
            c: r.center(),
            rx: (r.max.x - r.min.x) / 2.0,
            ry: (r.max.y - r.min.y) / 2.0,
            fill: default_fill(doc, o),
            stroke: stroke_of(o),
        },
    });
    if !label.is_empty() {
        let font = doc_font(doc, 0x994);
        let c = r.center();
        s.items.push(text_item(
            o.id(),
            c.x as i32,
            c.y as i32 - font.height() / 2,
            label,
            font,
            Color::BLACK,
            Align::Center,
        ));
    }
}

impl Scene {
    pub fn bounds(&self) -> Option<R> {
        let mut acc: Option<R> = None;
        for it in &self.items {
            let b = match &it.shape {
                Shape::Rect { r, .. } | Shape::Image { r, .. } | Shape::Box { r, .. } | Shape::Figure { r, .. } => *r,
                Shape::Ellipse { c, rx, ry, .. } => {
                    R { min: P { x: c.x - rx, y: c.y - ry }, max: P { x: c.x + rx, y: c.y + ry } }
                }
                Shape::Line { points, .. } | Shape::Polygon { points, .. } => {
                    let Some(f) = points.first() else { continue };
                    points.iter().fold(R { min: *f, max: *f }, |a, p| a.union(&R { min: *p, max: *p }))
                }
                Shape::Text { pos, text, font, align, .. } => {
                    let w = font.width(text) as f32;
                    let x = align.left(pos.x, w);
                    R { min: P { x, y: pos.y }, max: P { x: x + w, y: pos.y + font.height() as f32 } }
                }
                Shape::Shadow { r, .. } => *r,
            };
            acc = Some(acc.map_or(b, |a| a.union(&b)));
        }
        acc
    }

    /// The diagram as a PNG file (the SVG of [`Scene::to_svg`] rasterised,
    /// with the embedded Carlito), as Looping's « Modèle graphique au format
    /// image » (33136).
    #[cfg(feature = "png")]
    pub fn to_png(&self) -> Result<Vec<u8>, String> {
        use resvg::{tiny_skia, usvg};
        let mut opt = usvg::Options::default();
        opt.fontdb_mut().load_font_data(crate::text::CARLITO_REGULAR.to_vec());
        opt.fontdb_mut().load_font_data(crate::text::CARLITO_BOLD.to_vec());
        opt.fontdb_mut().load_system_fonts();
        let tree = usvg::Tree::from_str(&self.to_svg(), &opt).map_err(|e| e.to_string())?;
        let size = tree.size().to_int_size();
        let mut pix = tiny_skia::Pixmap::new(size.width(), size.height()).ok_or("image vide")?;
        resvg::render(&tree, tiny_skia::Transform::default(), &mut pix.as_mut());
        pix.encode_png().map_err(|e| e.to_string())
    }

    pub fn to_svg(&self) -> String {
        let b = self.bounds().unwrap_or(R { min: P { x: 0.0, y: 0.0 }, max: P { x: 100.0, y: 100.0 } });
        // margin of Looping's image export
        let pad = 5.0;
        // the export starts at the page origin at most: objects at negative
        // coordinates are cut
        let (x0, y0) = ((b.min.x - pad).max(0.0), (b.min.y - pad).max(0.0));
        let (w, h) = (b.max.x + pad - x0, b.max.y + pad - y0);
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{x0} {y0} {w} {h}\" width=\"{w}\" height=\"{h}\" font-family=\"Carlito, Calibri, sans-serif\">\n<rect x=\"{x0}\" y=\"{y0}\" width=\"{w}\" height=\"{h}\" fill=\"white\"/>\n"
        );
        for it in &self.items {
            match &it.shape {
                Shape::Rect { r, fill, stroke, radius } => {
                    let (w, h) = (r.max.x - r.min.x, r.max.y - r.min.y);
                    if let Some(f) = fill {
                        out.push_str(&format!(
                            "<rect x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{h}\" rx=\"{radius}\" fill=\"{}\"/>\n",
                            r.min.x,
                            r.min.y,
                            f.hex()
                        ));
                    }
                    if let Some(c) = stroke {
                        out.push_str(&format!(
                            "<rect x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{h}\" rx=\"{radius}\" fill=\"none\" stroke=\"{}\"/>\n",
                            r.min.x + 0.5,
                            r.min.y + 0.5,
                            c.hex()
                        ));
                    }
                }
                Shape::Box { r, radius, header, header_fill, body_fill, stroke, dashed } => {
                    let dash = if *dashed { " stroke-dasharray=\"1 2\"" } else { "" };
                    let (x, y, w, h) = (r.min.x, r.min.y, r.max.x - r.min.x, r.max.y - r.min.y);
                    let rad = radius.min(w / 2.0).min(h / 2.0);
                    let hb = header.min(h);
                    out.push_str(&format!(
                        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{rad}\" fill=\"{}\"/>\n",
                        body_fill.hex()
                    ));
                    let band: Vec<String> =
                        rounded_band(r, *radius, hb).iter().map(|p| format!("{},{}", p.x, p.y)).collect();
                    out.push_str(&format!("<polygon points=\"{}\" fill=\"{}\"/>\n", band.join(" "), header_fill.hex()));
                    // GDI outlines: one pixel wide, from the left/top pixel to
                    // the right/bottom one included (pixel centres at +0.5)
                    out.push_str(&format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{w}\" height=\"{h}\" rx=\"{rad}\" fill=\"none\" stroke=\"{}\"{dash}/>\n<line x1=\"{x}\" y1=\"{yb}\" x2=\"{xr}\" y2=\"{yb}\" stroke=\"{}\"{dash}/>\n",
                        x + 0.5,
                        y + 0.5,
                        stroke.hex(),
                        stroke.hex(),
                        yb = y + hb + 0.5,
                        xr = x + w
                    ));
                }
                Shape::Ellipse { c, rx, ry, fill, stroke } => out.push_str(&format!(
                    "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{rx}\" ry=\"{ry}\" fill=\"{}\" stroke=\"{}\"/>\n",
                    c.x,
                    c.y,
                    fill.map_or("none".into(), |c| c.hex()),
                    stroke.hex()
                )),
                Shape::Line { points, stroke, dashed, width } => {
                    let pts: Vec<String> = points.iter().map(|p| format!("{},{}", p.x + 0.5, p.y + 0.5)).collect();
                    out.push_str(&format!(
                        "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\"{}{}/>\n",
                        pts.join(" "),
                        stroke.hex(),
                        if *dashed { " stroke-dasharray=\"1 2\"" } else { "" },
                        if *width != 1.0 { format!(" stroke-width=\"{width}\"") } else { String::new() }
                    ));
                }
                Shape::Polygon { points, fill } => {
                    let pts: Vec<String> = points.iter().map(|p| format!("{},{}", p.x, p.y)).collect();
                    out.push_str(&format!("<polygon points=\"{}\" fill=\"{}\"/>\n", pts.join(" "), fill.hex()));
                }
                Shape::Figure { r, ellipse, radius, fill, stroke, width, dashed } => {
                    let (w, h) = (r.max.x - r.min.x, r.max.y - r.min.y);
                    let paint = format!(
                        "fill=\"{}\" stroke=\"{}\" stroke-width=\"{width}\"{}",
                        fill.map_or("none".into(), |c| c.hex()),
                        stroke.map_or("none".into(), |c| c.hex()),
                        if *dashed { " stroke-dasharray=\"4 3\"" } else { "" }
                    );
                    if *ellipse {
                        out.push_str(&format!(
                            "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" {paint}/>\n",
                            r.min.x + w / 2.0,
                            r.min.y + h / 2.0,
                            w / 2.0,
                            h / 2.0
                        ));
                    } else {
                        // the pen is inside the frame (`PS_INSIDEFRAME`)
                        let i = if stroke.is_some() { width / 2.0 } else { 0.0 };
                        out.push_str(&format!(
                            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{radius}\" {paint}/>\n",
                            r.min.x + i,
                            r.min.y + i,
                            (w - 2.0 * i).max(0.0),
                            (h - 2.0 * i).max(0.0)
                        ));
                    }
                }
                Shape::Text { pos, text, font, color, align, background } => {
                    if let Some(bg) = background {
                        let w = font.width(text) as f32;
                        let x = if *align == Align::Center { pos.x - (w / 2.0).floor() } else { align.left(pos.x, w) };
                        out.push_str(&format!(
                            "<rect x=\"{x}\" y=\"{}\" width=\"{w}\" height=\"{}\" fill=\"{}\"/>\n",
                            pos.y,
                            font.height(),
                            bg.hex()
                        ));
                    }
                    out.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"{}\"{}{}{} fill=\"{}\">{}</text>\n",
                        pos.x,
                        pos.y + font.ascent() as f32,
                        font.size,
                        match align {
                            Align::Center => "middle",
                            Align::Right => "end",
                            Align::Left => "start",
                        },
                        if font.bold { " font-weight=\"bold\"" } else { "" },
                        if font.italic { " font-style=\"italic\"" } else { "" },
                        if font.underline { " text-decoration=\"underline\"" } else { "" },
                        color.hex(),
                        xml_escape(text)
                    ))
                }
                Shape::Shadow { r, radius, color } => out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{radius}\" fill=\"{}\"/>\n",
                    r.min.x,
                    r.min.y,
                    r.max.x - r.min.x,
                    r.max.y - r.min.y,
                    color.hex()
                )),
                // the image file is not read: Looping's placeholder only
                Shape::Image { .. } => {}
            }
        }
        out.push_str("</svg>\n");
        out
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(all(test, feature = "png"))]
mod png_tests {
    use super::*;

    #[test]
    fn png_export_has_the_size_of_the_svg() {
        let mut doc = Document::default();
        crate::edit::add_entity(&mut doc, "Client", loo_format::Point { x: 0, y: 0 });
        crate::edit::fit_all(&mut doc);
        let png = build(&doc).to_png().unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        // IHDR: width and height, big endian
        let (w, h) =
            (u32::from_be_bytes(png[16..20].try_into().unwrap()), u32::from_be_bytes(png[20..24].try_into().unwrap()));
        let svg = build(&doc).to_svg();
        assert!(svg.contains(&format!("width=\"{w}\"")) && svg.contains(&format!("height=\"{h}\"")), "{w}×{h}");
    }
}
