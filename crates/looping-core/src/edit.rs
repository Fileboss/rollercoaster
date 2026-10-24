//! Document mutations that keep Looping's object structure intact
//! (owned texts, arc chains, unique ids), so edited files open in Looping.

use loo_format::{
    Arc, Brush, Document, DrawBase, DrawObj, Entite, ObjKind, OleDateTime, Parametre, Point, Rattach, Rect, Relation,
    Texte, Trait, ViewStyle,
};

use crate::mcd::ObjId;

const ENTITY_FILL: u32 = 0x80ffff;
const RELATION_FILL: u32 = 0xffff00;
const DEFAULT_FILL: u32 = 0xc0c0c0;

pub fn next_id(doc: &Document) -> ObjId {
    doc.objects.iter().map(|o| o.base.id).max().unwrap_or(0) + 1
}

fn obj_mut(doc: &mut Document, id: ObjId) -> Option<&mut DrawObj> {
    if id == 0 {
        return None;
    }
    doc.objects.iter_mut().find(|o| o.base.id == id)
}

pub fn texte_mut(doc: &mut Document, owner: ObjId, slot: usize) -> Option<&mut Texte> {
    let tid = doc.object(owner)?.base.links.texts[slot];
    match &mut obj_mut(doc, tid)?.kind {
        ObjKind::Texte(t) => Some(t),
        _ => None,
    }
}

fn base(kind: u32, id: ObjId, fill: u32) -> DrawBase {
    let style = ViewStyle {
        a: 1,
        b: 1,
        c: 1,
        pen: Default::default(),
        brush: Brush { style: 0, color: fill, hatch: 0 },
        brush2: Brush { style: 0, color: fill, hatch: 0 },
    };
    DrawBase { kind, styles: [style; 3], id, f308: [1; 4], f318: 6, ..Default::default() }
}

/// Font blocks reused from an existing text so new texts match the document.
fn font_template(doc: &Document) -> Option<[Vec<u8>; 4]> {
    doc.objects.iter().find_map(|o| o.as_texte()).map(|t| t.font_blocks.clone())
}

fn new_texte(doc: &Document, id: ObjId, owner: ObjId, slot: u16, line: &str) -> DrawObj {
    let mut t = Texte { slot, lines: vec![line.to_string()], lines2: Vec::new(), ..Default::default() };
    if let Some(f) = font_template(doc) {
        t.font_blocks = f;
    }
    let mut b = base(loo_format::kind::TEXTE, id, DEFAULT_FILL);
    b.links.owner = owner;
    DrawObj { class: "CDrawTexte".into(), base: b, kind: ObjKind::Texte(t) }
}

/// Days since 1899-12-30 for "now" (OLE automation date).
fn ole_now() -> OleDateTime {
    let secs =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
    OleDateTime { status: 0, date: 25569.0 + secs / 86400.0 }
}

/// A new attribute with Looping's defaults: VARCHAR(50), DECIMAL(15,2).
pub fn new_parametre(name: &str) -> Parametre {
    let now = ole_now();
    Parametre {
        name: name.to_string(),
        f24: 1,
        f2c: [0, 2, 1, 0],
        created: now,
        modified: now,
        f64: 50,
        f68: [50; 5],
        f7c: 15,
        f80: 2,
        f110: 3,
        f14c: 1,
        ..Default::default()
    }
}

/// Appends an object and its three texts; returns the object id.
fn add_with_texts(doc: &mut Document, mut obj: DrawObj, name: &str) -> ObjId {
    let id = next_id(doc);
    obj.base.id = id;
    obj.base.links.texts = [id + 1, id + 2, id + 3];
    let texts =
        [new_texte(doc, id + 1, id, 0, name), new_texte(doc, id + 2, id, 1, ""), new_texte(doc, id + 3, id, 2, "")];
    doc.objects.push(obj);
    doc.objects.extend(texts);
    id
}

/// Name of a new entity's identifier, from the document template
/// (`+0x7fc`: `Id_[Classe]`, `[Classe]_Id`, `Id[Classe]`, `[Classe]Id`,
/// 10 = custom text at `+0xa28`).
pub fn identifier_name(doc: &Document, class: &str) -> String {
    let template = match doc.settings.i32(0x7fc) {
        1 => "[Classe]_Id".to_string(),
        2 => "Id[Classe]".to_string(),
        3 => "[Classe]Id".to_string(),
        10 => doc.settings.wstr(0xa28, 16),
        _ => "Id_[Classe]".to_string(),
    };
    template.replace("[Classe]", class)
}

pub fn add_entity(doc: &mut Document, name: &str, at: Point) -> ObjId {
    let obj = DrawObj {
        class: "CDrawEntite".into(),
        base: base(loo_format::kind::ENTITE, 0, ENTITY_FILL),
        kind: ObjKind::Entite(Entite {
            flags: 0,
            rect: Rect { left: at.x, top: at.y, right: at.x + 160, bottom: at.y + 90 },
        }),
    };
    let id = add_with_texts(doc, obj, name);
    let p = auto_identifier(doc, name);
    if let Some(t) = texte_mut(doc, id, 1) {
        t.params.push(p);
    }
    id
}

pub fn add_association(doc: &mut Document, name: &str, at: Point) -> ObjId {
    let obj = DrawObj {
        class: "CDrawRelation".into(),
        base: base(loo_format::kind::RELATION, 0, RELATION_FILL),
        kind: ObjKind::Relation(Relation {
            center: at,
            f398: 50,
            f3a0: 30,
            f3a4: Point { x: -50, y: 50 },
            f39c: 10,
            ..Default::default()
        }),
    };
    add_with_texts(doc, obj, name)
}

/// Creates a link entity <-> association made of two arc segments, as
/// Looping does. `cardinality` uses the stored code (0 "0,1" .. 3 "1,n").
pub fn add_link(doc: &mut Document, entity: ObjId, association: ObjId, cardinality: i32) -> ObjId {
    let first = add_arc(doc, "CDrawArc", loo_format::kind::ARC, entity, association);
    if let Some(t) = texte_mut(doc, first, 1) {
        let mut p = new_parametre("");
        p.role = 1;
        p.cardinality = cardinality;
        t.params.push(p);
    }
    first
}

/// An inheritance symbol (`CDrawHeritage`, 0x1400cf940); `constraint`
/// (stored in its parameter, `CUnParametre+0x110`): 0 none, 1 exclusion,
/// 2 totality, 3 partition.
pub fn add_heritage(doc: &mut Document, at: Point, constraint: i32) -> ObjId {
    let obj = DrawObj {
        class: "CDrawHeritage".into(),
        base: base(loo_format::kind::HERITAGE, 0, RELATION_FILL),
        kind: ObjKind::Heritage(loo_format::Heritage {
            rect: Rect { left: at.x - 20, top: at.y - 20, right: at.x + 20, bottom: at.y + 20 },
            rect2: Rect { left: at.x - 20, top: at.y - 20, right: at.x + 20, bottom: at.y + 20 },
            ..Default::default()
        }),
    };
    let id = add_with_texts(doc, obj, "");
    let mut p = new_parametre("");
    p.f110 = constraint;
    if let Some(t) = texte_mut(doc, id, 1) {
        t.params.push(p);
    }
    id
}

/// Links `entity` to an inheritance symbol (`CDrawArcHeritage`); its
/// parameter (role 3) marks the supertype (`CUnParametre+0x114`).
pub fn add_heritage_link(doc: &mut Document, entity: ObjId, heritage: ObjId, supertype: bool) -> ObjId {
    let first = add_arc(doc, "CDrawArcHeritage", loo_format::kind::ARC_HERITAGE, entity, heritage);
    if let Some(t) = texte_mut(doc, first, 1) {
        let mut p = new_parametre("");
        p.role = 3;
        p.f114 = supertype as i32;
        t.params.push(p);
    }
    first
}

/// A functional integrity constraint symbol (`CDrawCif`, 0x1400b8b00).
pub fn add_cif(doc: &mut Document, at: Point) -> ObjId {
    let obj = DrawObj {
        class: "CDrawCif".into(),
        base: base(loo_format::kind::CIF, 0, DEFAULT_FILL),
        kind: ObjKind::Cif(loo_format::Cif {
            center: at,
            f398: 20,
            rect: Rect { left: at.x - 20, top: at.y - 20, right: at.x + 20, bottom: at.y + 20 },
            ..Default::default()
        }),
    };
    add_with_texts(doc, obj, "CIF")
}

/// Links a CIF (`CDrawArcCif`) to its one association or to an entity;
/// its parameter (role 4) marks the targeted entity (`CUnParametre+0x114`).
pub fn add_cif_link(doc: &mut Document, from: ObjId, cif: ObjId, target: bool) -> ObjId {
    let first = add_arc(doc, "CDrawArcCif", loo_format::kind::ARC_CIF, from, cif);
    if let Some(t) = texte_mut(doc, first, 1) {
        let mut p = new_parametre("");
        p.role = 4;
        p.f114 = target as i32;
        t.params.push(p);
    }
    first
}

/// A management rule (`CDrawRegle`): `description` lines (parameter names),
/// SQL `script` put before or after the tables (`CDrawRegle+0x3ac`);
/// `flags` are `CDrawRegle+0x3a0/+0x3a4/+0x3a8`.
pub fn add_regle(
    doc: &mut Document,
    name: &str,
    at: Point,
    description: &[&str],
    script: &[&str],
    after: bool,
    flags: [i32; 3],
) -> ObjId {
    let obj = DrawObj {
        class: "CDrawRegle".into(),
        base: base(loo_format::kind::REGLE, 0, DEFAULT_FILL),
        kind: ObjKind::Regle(loo_format::Regle {
            raw_flag: if after { 0 } else { 1 },
            rect: Rect { left: at.x, top: at.y, right: at.x + 160, bottom: at.y + 60 },
            f3a0: flags[0],
            f3a4: flags[1],
            f3a8: flags[2],
        }),
    };
    let id = add_with_texts(doc, obj, name);
    if let Some(t) = texte_mut(doc, id, 1) {
        t.params = description.iter().map(|l| new_parametre(l)).collect();
    }
    set_script(doc, id, script, 0);
    id
}

/// An inter-association constraint (`CDrawContrainte`); `kind`
/// (`CUnParametre+0x110`): exclusion, totality, partition, simultaneity, inclusion.
pub fn add_contrainte(doc: &mut Document, name: &str, at: Point, kind: i32) -> ObjId {
    let obj = DrawObj {
        class: "CDrawContrainte".into(),
        base: base(loo_format::kind::CONTRAINTE, 0, DEFAULT_FILL),
        kind: ObjKind::Contrainte(loo_format::Contrainte {
            center: at,
            f3e0: 20,
            rect: Rect { left: at.x - 20, top: at.y - 20, right: at.x + 20, bottom: at.y + 20 },
            ..Default::default()
        }),
    };
    let id = add_with_texts(doc, obj, name);
    let mut p = new_parametre("");
    p.f110 = kind;
    if let Some(t) = texte_mut(doc, id, 1) {
        t.params.push(p);
    }
    id
}

/// Links an association (or an entity) to an inter-association constraint
/// (`CDrawArcContrainte`).
pub fn add_contrainte_link(doc: &mut Document, from: ObjId, contrainte: ObjId) -> ObjId {
    add_arc(doc, "CDrawArcContrainte", loo_format::kind::ARC_CONTRAINTE, from, contrainte)
}

/// Links any object to a management rule (`CDrawArcRegle`).
pub fn add_regle_link(doc: &mut Document, from: ObjId, regle: ObjId) -> ObjId {
    add_arc(doc, "CDrawArcRegle", loo_format::kind::ARC_REGLE, from, regle)
}

/// Identifier named after the document template (`+0x7fc`), as « Clé auto »
/// of Looping's entity dialog: a counter, first in the list.
pub fn auto_identifier(doc: &Document, entity_name: &str) -> Parametre {
    let mut p = new_parametre(&identifier_name(doc, entity_name));
    p.f8 = 1;
    p.f20 = 1;
    p.f2c[0] = 4;
    p
}

/// "Script SQL associé" of a rule, constraint or inheritance: lines of its
/// slot-1 text (`CDrawTexte+0x4a0`), ordered by `position` (`+0x4d8`).
pub fn set_script(doc: &mut Document, owner: ObjId, script: &[&str], position: i32) {
    if let Some(t) = texte_mut(doc, owner, 1) {
        t.lines2 = script.iter().map(|l| l.to_string()).collect();
        t.f4d8 = position;
    }
}

/// Two arc segments of class `class`, from `from` to `to`; returns the first.
fn add_arc(doc: &mut Document, class: &str, kind: u32, from: ObjId, to: ObjId) -> ObjId {
    let n = doc.objects.iter().filter(|o| o.class == class && o.base.links.prev == 0).count() + 1;
    let arc = |owner: ObjId| DrawObj {
        class: class.into(),
        base: {
            let mut b = base(kind, 0, DEFAULT_FILL);
            b.f338 = -1;
            b.links.owner = owner;
            b
        },
        kind: ObjKind::Arc(Arc { anchor: Rattach { mode: 1, ..Default::default() }, ..Default::default() }),
    };
    let first = add_with_texts(doc, arc(from), &format!("Lien {n}"));
    let second = add_with_texts(doc, arc(to), "???");
    obj_mut(doc, first).unwrap().base.links.next = second;
    obj_mut(doc, second).unwrap().base.links.prev = first;
    first
}

// ------------------------------------------------------------ illustrations
//
// Each illustration takes the values last chosen in its dialog, kept in the
// document (constructors 0x1400d76d0, 0x1400c25d0, 0x1400d2590, 0x1400eef30,
// 0x1400f0b00, 0x1400cec70; dialogs' « OK », see `remember_style`).

/// Illustration base: pen colour `pen`, fill colour `fill` (both brushes of
/// the three views), style flags `a`/`b`/`c` (« Par défaut » of the pen and
/// the fills, `+0xd8..+0xf8`), `f308` = `+0x308..+0x314`.
fn illustration_base(kind: u32, pen: u32, fill: u32, defaults: [u16; 3], f308: [i32; 4]) -> DrawBase {
    let [a, b, c] = defaults;
    let style = ViewStyle {
        a,
        b,
        c,
        pen: loo_format::Pen { color: pen, ..Default::default() },
        brush: Brush { style: 0, color: fill, hatch: 0 },
        brush2: Brush { style: 0, color: fill, hatch: 0 },
    };
    DrawBase { kind, styles: [style; 3], f308, f318: 6, ..Default::default() }
}

/// Name of a new illustration as Looping gives it (0x14011b130): `prefix`,
/// a space and a counter per class.
fn illustration_name(doc: &Document, class: &str, prefix: &str) -> String {
    let n = doc.objects.iter().filter(|o| o.class == class).count() + 1;
    format!("{prefix} {n}")
}

fn normalized(r: Rect) -> Rect {
    Rect {
        left: r.left.min(r.right),
        top: r.top.min(r.bottom),
        right: r.left.max(r.right),
        bottom: r.top.max(r.bottom),
    }
}

/// Rectangle (`CDrawRectangle`): pen width `+0x3a0`, style `+0x3a4`, shadow
/// `+0x3a8` in `+0x3ac`, rounded corners `+0x3b0`, from `+0x840..+0x860`.
pub fn add_rectangle(doc: &mut Document, r: Rect) -> ObjId {
    let s = &doc.settings;
    let obj = DrawObj {
        class: "CDrawRectangle".into(),
        base: illustration_base(
            loo_format::kind::RECTANGLE,
            s.u32(0x854),
            s.u32(0x860),
            [0; 3],
            [0, 0, s.i32(0x85c), s.i32(0x858)],
        ),
        kind: ObjKind::Rectangle(loo_format::Shape {
            rect: normalized(r),
            f3a0: s.i32(0x840),
            f3a4: s.i32(0x844),
            f3a8: s.i32(0x848),
            f3ac: s.i32(0x84c),
            f3b0: s.i32(0x850),
            ..Default::default()
        }),
    };
    let name = illustration_name(doc, "CDrawRectangle", "Texte");
    add_with_texts(doc, obj, &name)
}

/// Ellipse (`CDrawEllipse`), from `+0x864..+0x880`.
pub fn add_ellipse(doc: &mut Document, r: Rect) -> ObjId {
    let s = &doc.settings;
    let obj = DrawObj {
        class: "CDrawEllipse".into(),
        base: illustration_base(
            loo_format::kind::ELLIPSE,
            s.u32(0x874),
            s.u32(0x880),
            [0; 3],
            [0, 0, s.i32(0x87c), s.i32(0x878)],
        ),
        kind: ObjKind::Ellipse(loo_format::Shape {
            rect: normalized(r),
            f3a0: s.i32(0x864),
            f3a4: s.i32(0x868),
            f3a8: s.i32(0x86c),
            f3ac: s.i32(0x870),
            ..Default::default()
        }),
    };
    let name = illustration_name(doc, "CDrawEllipse", "Texte");
    add_with_texts(doc, obj, &name)
}

/// Text frame of the « Texte » tool (`CDrawTitre`): `lines` in its slot-0
/// text; alignments `+0x318` / `+0x390` and the frame's properties with
/// their « Par défaut » flags, from `+0x8f0..+0x934`.
pub fn add_texte(doc: &mut Document, r: Rect, lines: &[&str]) -> ObjId {
    let s = &doc.settings;
    let mut b = illustration_base(
        loo_format::kind::TITRE,
        s.u32(0x90c),
        s.u32(0x934),
        [s.i32(0x908) as u16, s.i32(0x930) as u16, s.i32(0x930) as u16],
        [s.i32(0x928), s.i32(0x8f8), s.i32(0x92c), s.i32(0x8fc)],
    );
    b.f318 = s.i32(0x8f0);
    let obj = DrawObj {
        class: "CDrawTitre".into(),
        base: b,
        kind: ObjKind::Titre(loo_format::Titre {
            f390: s.i32(0x8f4) as u16,
            rect: normalized(r),
            f3a4: [s.i32(0x910), s.i32(0x914)],
            f3ac: [s.i32(0x904), s.i32(0x91c), s.i32(0x924), s.i32(0x900), s.i32(0x918), s.i32(0x920)],
        }),
    };
    let id = add_with_texts(doc, obj, "");
    if let Some(t) = texte_mut(doc, id, 0) {
        t.lines = lines.iter().map(|l| l.to_string()).collect();
    }
    id
}

/// Image (`CDrawImage`): file `path` (`+0x390`), or with `vignette` a
/// sub-model (`+0x3c0`) whose model is `+0x3b8`; from `+0xa90..+0xaac`.
pub fn add_image(doc: &mut Document, r: Rect, vignette: bool) -> ObjId {
    let s = &doc.settings;
    let obj = DrawObj {
        class: "CDrawImage".into(),
        base: illustration_base(
            loo_format::kind::IMAGE,
            s.u32(0xaa0),
            s.u32(0xaac),
            [0; 3],
            [0, 0, s.i32(0xaa8), s.i32(0xaa4)],
        ),
        kind: ObjKind::Image(loo_format::Image {
            rect: normalized(r),
            f3a8: [s.i32(0xa90), s.i32(0xa94), s.i32(0xa98), s.i32(0xa9c)],
            f3c0: vignette as i32,
            ..Default::default()
        }),
    };
    let name = illustration_name(doc, "CDrawImage", "");
    add_with_texts(doc, obj, &name)
}

/// Free-hand stroke (`CDrawCrayon`, 0x14011d830): no texts; origin at the
/// first point, points relative to it; pen width `+0x464`, style `+0x468`
/// and colour from `+0x808..+0x810`.
pub fn add_crayon(doc: &mut Document, points: &[Point]) -> ObjId {
    let Some(&origin) = points.first() else { return 0 };
    let s = &doc.settings;
    let mut b = illustration_base(loo_format::kind::CRAYON, s.u32(0x810), 0xffffff, [0; 3], [0; 4]);
    b.id = next_id(doc);
    let obj = DrawObj {
        class: "CDrawCrayon".into(),
        base: b,
        kind: ObjKind::Crayon(loo_format::Crayon {
            f464: s.i32(0x808),
            f468: s.i32(0x80c),
            origin,
            points: points.iter().map(|p| Point { x: p.x - origin.x, y: p.y - origin.y }).collect(),
            ..Default::default()
        }),
    };
    let id = obj.base.id;
    doc.objects.push(obj);
    id
}

/// Link between illustrations (`CDrawArcObjetLibre`): made by the « Lien »
/// tool when one end is a rectangle, an ellipse, an image or a text frame.
pub fn add_illustration_link(doc: &mut Document, from: ObjId, to: ObjId) -> ObjId {
    add_arc(doc, "CDrawArcObjetLibre", loo_format::kind::ARC_OBJET_LIBRE, from, to)
}

/// Grid step of the magnetism (« Grille » 33212, « Demi-grille » 33213,
/// « Sans » 33214): `CLoopingDoc+0x334`, halved by `+0x340`; `None`
/// without magnetism (`+0x33c` = 0).
pub fn grid_step(doc: &Document) -> Option<i32> {
    let step = doc.f334.max(2);
    (doc.f33c != 0).then_some(if doc.f340 != 0 { step / 2 } else { step })
}

/// Puts an object on the grid as Looping does after a move (0x14005bdf0):
/// each coordinate of an entity's rectangle, an association's centre, the
/// rectangle of the other objects, rounded to the nearest multiple.
pub fn snap(doc: &mut Document, id: ObjId, step: i32) {
    let near = |v: &mut i32| {
        let rest = *v % step;
        *v -= rest;
        if rest > step / 2 {
            *v += step;
        }
    };
    let rect = |r: &mut Rect| {
        for v in [&mut r.left, &mut r.top, &mut r.right, &mut r.bottom] {
            near(v);
        }
    };
    let Some(o) = obj_mut(doc, id) else { return };
    match &mut o.kind {
        ObjKind::Entite(e) => rect(&mut e.rect),
        ObjKind::Relation(r) => {
            near(&mut r.center.x);
            near(&mut r.center.y);
        }
        ObjKind::Image(x) => rect(&mut x.rect),
        ObjKind::Regle(x) => rect(&mut x.rect),
        ObjKind::Ellipse(x) | ObjKind::Rectangle(x) => rect(&mut x.rect),
        ObjKind::Titre(x) => rect(&mut x.rect),
        _ => {}
    }
}

/// Bit of view `v` (`CUneVueUrbanisme+8`) in an object's views (`+0x338`).
pub fn view_bit(v: &loo_format::View) -> u32 {
    1u32.checked_shl(v.f8 as u32).unwrap_or(0)
}

/// Adds or removes object `id` (and the other segments of a link) from the
/// views of `mask`.
pub fn set_views(doc: &mut Document, id: ObjId, mask: u32, member: bool) {
    let ids = match doc.object(id) {
        Some(o) if o.class.starts_with("CDrawArc") || matches!(o.kind, ObjKind::Trait(_)) => {
            chain_ids(doc, first_segment(doc, id))
        }
        _ => vec![id],
    };
    for i in ids {
        if let Some(o) = obj_mut(doc, i) {
            let v = o.base.f338 as u32;
            o.base.f338 = if member { v | mask } else { v & !mask } as i32;
        }
    }
}

/// A new view (« Urbanisation »): the first free bit from 1 (Looping offers
/// 21 entries, bits 1 to 20 in the objects' dialogs).
pub fn add_view(doc: &mut Document, name: &str) -> Option<i32> {
    let bit = (1..=20).find(|b| !doc.views.iter().any(|v| v.f8 == *b))?;
    doc.views.push(loo_format::View { f8: bit, name: name.to_string() });
    Some(bit)
}

/// Removes a view and its bit from every object.
pub fn remove_view(doc: &mut Document, bit: i32) {
    doc.views.retain(|v| v.f8 != bit);
    let mask = 1u32.checked_shl(bit as u32).unwrap_or(0);
    // links made for every view (-1) stay so
    for o in doc.objects.iter_mut().filter(|o| o.base.f338 != -1) {
        o.base.f338 = ((o.base.f338 as u32) & !mask) as i32;
    }
}

/// Gives every segment of the free line starting at `first` the look of
/// that segment (pen, width, style, arrow), as the dialogs of the line and
/// the arrow do.
pub fn sync_free_line(doc: &mut Document, first: ObjId) {
    let Some(src) = doc.object(first).cloned() else { return };
    let ObjKind::Trait(t) = &src.kind else { return };
    for seg in chain_ids(doc, first).into_iter().skip(1) {
        if let Some(o) = obj_mut(doc, seg) {
            o.base.styles = src.base.styles;
            if let ObjKind::Trait(x) = &mut o.kind {
                (x.f3a8, x.f3ac, x.f3b0, x.f3b4, x.f3b8) = (t.f3a8, t.f3ac, t.f3b0, t.f3b4, t.f3b8);
            }
        }
    }
}

/// Keeps the look of illustration `id` as the one of the next objects of
/// its kind, as the « OK » of its dialog does (0x140036eb0, 0x140027330,
/// 0x140028e40, 0x1400440c0, 0x140026bf0, 0x140044940, 0x140027d70).
pub fn remember_style(doc: &mut Document, id: ObjId) {
    let Some(o) = doc.object(id).cloned() else { return };
    let st = o.base.styles[0];
    let [_, _, filled, outlined] = o.base.f308;
    let s = &mut doc.settings;
    let mut put = |pairs: &[(usize, i32)]| {
        for &(off, v) in pairs {
            s.set_i32(off, v);
        }
    };
    let (pen, fill) = (st.pen.color as i32, st.brush.color as i32);
    match (&o.kind, o.class.as_str()) {
        (ObjKind::Rectangle(x), _) => put(&[
            (0x840, x.f3a0),
            (0x844, x.f3a4),
            (0x848, x.f3a8),
            (0x84c, x.f3ac),
            (0x850, x.f3b0),
            (0x854, pen),
            (0x858, outlined),
            (0x85c, filled),
            (0x860, fill),
        ]),
        (ObjKind::Ellipse(x), _) => put(&[
            (0x864, x.f3a0),
            (0x868, x.f3a4),
            (0x86c, x.f3a8),
            (0x870, x.f3ac),
            (0x874, pen),
            (0x878, outlined),
            (0x87c, filled),
            (0x880, fill),
        ]),
        (ObjKind::Image(x), _) => put(&[
            (0xa90, x.f3a8[0]),
            (0xa94, x.f3a8[1]),
            (0xa98, x.f3a8[2]),
            (0xa9c, x.f3a8[3]),
            (0xaa0, pen),
            (0xaa4, outlined),
            (0xaa8, filled),
            (0xaac, fill),
        ]),
        (ObjKind::Titre(t), _) => put(&[
            (0x8f0, o.base.f318),
            (0x8f4, t.f390 as i32),
            (0x8f8, o.base.f308[1]),
            (0x8fc, outlined),
            (0x900, t.f3ac[3]),
            (0x904, t.f3ac[0]),
            (0x908, st.a as i32),
            (0x90c, pen),
            (0x910, t.f3a4[0]),
            (0x914, t.f3a4[1]),
            (0x918, t.f3ac[4]),
            (0x91c, t.f3ac[1]),
            (0x920, t.f3ac[5]),
            (0x924, t.f3ac[2]),
            (0x928, o.base.f308[0]),
            (0x92c, filled),
            (0x930, st.b as i32),
            (0x934, fill),
        ]),
        (ObjKind::Crayon(c), _) => put(&[(0x808, c.f464), (0x80c, c.f468), (0x810, pen)]),
        (ObjKind::Trait(t), "CDrawTrait") => put(&[(0x814, t.f3a8), (0x818, t.f3ac), (0x820, t.f3b8), (0x824, pen)]),
        (ObjKind::Trait(t), _) => {
            put(&[(0x828, t.f3a8), (0x82c, t.f3ac), (0x830, t.f3b0), (0x834, t.f3b4), (0x838, t.f3b8), (0x83c, pen)])
        }
        _ => {}
    }
}

/// Free line (`CDrawTrait`) or arrow (`CDrawFleche`, pointing to the last
/// point): one segment per point, chained like arcs, each anchored at its
/// point (`+0x390`). Pen width `+0x3a8`, style `+0x3ac`, arrow size and
/// shape `+0x3b0` / `+0x3b4`, smoothing `+0x3b8` and colour from
/// `+0x814..+0x824` (line) or `+0x828..+0x83c` (arrow); a width of 0 (an
/// empty settings block) gives 1.
pub fn add_free_line(doc: &mut Document, points: &[Point], arrow: bool) -> ObjId {
    let (class, kind, off) = if arrow {
        ("CDrawFleche", loo_format::kind::FLECHE, 0x828)
    } else {
        ("CDrawTrait", loo_format::kind::TRAIT, 0x814)
    };
    let s = &doc.settings;
    let width = s.i32(off).max(1);
    let style = s.i32(off + 4);
    let (size, shape) = if arrow { (s.i32(0x830), s.i32(0x834)) } else { (0, 0) };
    let smoothing = s.i32(if arrow { 0x838 } else { 0x820 });
    let pen = s.u32(if arrow { 0x83c } else { 0x824 });
    let mut ids: Vec<ObjId> = Vec::new();
    for &p in points {
        let seg = DrawObj {
            class: class.into(),
            base: {
                let mut b = base(kind, 0, DEFAULT_FILL);
                b.f338 = -1;
                for st in &mut b.styles {
                    st.pen.color = pen;
                }
                b
            },
            kind: ObjKind::Trait(Trait {
                f3a8: width,
                f3ac: style,
                f3b0: size,
                f3b4: shape,
                f3b8: smoothing,
                anchor: Rattach { mode: 1, offset: p, ..Default::default() },
            }),
        };
        ids.push(add_with_texts(doc, seg, ""));
    }
    for w in ids.windows(2) {
        obj_mut(doc, w[0]).unwrap().base.links.next = w[1];
        obj_mut(doc, w[1]).unwrap().base.links.prev = w[0];
    }
    ids.first().copied().unwrap_or(0)
}

pub fn set_cardinality(doc: &mut Document, arc: ObjId, cardinality: i32) {
    if let Some(t) = texte_mut(doc, arc, 1) {
        match t.params.iter_mut().find(|p| p.role == 1) {
            Some(p) => p.cardinality = cardinality,
            None => {
                let mut p = new_parametre("");
                p.role = 1;
                p.cardinality = cardinality;
                t.params.push(p);
            }
        }
    }
}

pub fn rename(doc: &mut Document, id: ObjId, name: &str) {
    if let Some(t) = texte_mut(doc, id, 0) {
        if t.lines.is_empty() {
            t.lines.push(String::new());
        }
        t.lines[0] = name.to_string();
    }
}

pub fn translate(doc: &mut Document, id: ObjId, dx: i32, dy: i32) {
    let shift = |r: &mut Rect| {
        r.left += dx;
        r.right += dx;
        r.top += dy;
        r.bottom += dy;
    };
    // a free line moves with its points (segments without owner)
    if is_free_line(doc, id) {
        for seg in chain_ids(doc, first_segment(doc, id)) {
            if let Some(o) = obj_mut(doc, seg).filter(|o| o.base.links.owner == 0) {
                if let ObjKind::Trait(t) = &mut o.kind {
                    t.anchor.offset.x += dx;
                    t.anchor.offset.y += dy;
                }
            }
        }
        return;
    }
    let Some(o) = obj_mut(doc, id) else { return };
    match &mut o.kind {
        ObjKind::Crayon(c) => {
            c.origin.x += dx;
            c.origin.y += dy;
        }
        ObjKind::Entite(e) => shift(&mut e.rect),
        ObjKind::Relation(r) => {
            r.center.x += dx;
            r.center.y += dy;
        }
        ObjKind::Image(x) => shift(&mut x.rect),
        ObjKind::Regle(x) => shift(&mut x.rect),
        ObjKind::Ellipse(x) | ObjKind::Rectangle(x) => shift(&mut x.rect),
        ObjKind::Cif(x) => {
            shift(&mut x.rect);
            x.center.x += dx;
            x.center.y += dy;
        }
        ObjKind::Contrainte(x) => {
            shift(&mut x.rect);
            x.center.x += dx;
            x.center.y += dy;
        }
        ObjKind::Heritage(x) => {
            shift(&mut x.rect);
            shift(&mut x.rect2);
        }
        ObjKind::Titre(x) => shift(&mut x.rect),
        ObjKind::Prof(x) => shift(&mut x.rect),
        ObjKind::Arc(a) if a.anchor.mode == 0 => {
            a.anchor.offset.x += dx;
            a.anchor.offset.y += dy;
        }
        _ => {}
    }
}

/// Removes an object, its texts, and every link chain touching it.
pub fn delete(doc: &mut Document, id: ObjId) {
    let mut doomed: Vec<ObjId> = vec![id];
    // whole chain when deleting an arc or a free line
    if let Some(o) = doc.object(id) {
        if o.class.starts_with("CDrawArc") || matches!(o.kind, ObjKind::Trait(_)) {
            let mut first = o;
            while let Some(p) = doc.object(first.base.links.prev) {
                first = p;
            }
            doomed = chain_ids(doc, first.base.id);
        }
    }
    // chains attached to the object
    for o in &doc.objects {
        if o.class.starts_with("CDrawArc") && o.base.links.prev == 0 {
            let chain = chain_ids(doc, o.base.id);
            if chain.iter().any(|&a| doc.object(a).is_some_and(|x| x.base.links.owner == id)) {
                doomed.extend(chain);
            }
        }
    }
    let mut all = doomed.clone();
    for d in &doomed {
        if let Some(o) = doc.object(*d) {
            all.extend(o.base.links.texts.iter().filter(|&&t| t != 0));
        }
        // dependent objects (e.g. the UML class of an association)
        all.extend(
            doc.objects
                .iter()
                .filter(|o| o.base.links.owner == *d && !o.class.starts_with("CDrawArc"))
                .map(|o| o.base.id),
        );
    }
    let extra: Vec<ObjId> =
        all.iter().filter_map(|&d| doc.object(d)).flat_map(|o| o.base.links.texts).filter(|&t| t != 0).collect();
    all.extend(extra);
    doc.objects.retain(|o| !all.contains(&o.base.id));
}

/// A segment of a free line or arrow (`CDrawTrait` / `CDrawFleche`).
pub fn is_free_line(doc: &Document, id: ObjId) -> bool {
    matches!(doc.object(id).map(|o| &o.kind), Some(ObjKind::Trait(_)))
}

/// Segments of the chain starting at `first` (a link and its break points).
pub fn chain_ids(doc: &Document, first: ObjId) -> Vec<ObjId> {
    let mut v = vec![first];
    while let Some(n) = doc.object(*v.last().unwrap()).map(|o| o.base.links.next).filter(|&n| n != 0) {
        if v.contains(&n) {
            break;
        }
        v.push(n);
    }
    v
}

/// Grows an entity or an association so that its name and attributes fit,
/// as Looping does at opening and after each change (vtable `+0x1d8`:
/// entity 0x1400c5e60, association 0x1400e03b0); never shrinks.
/// Entity: width at least `+0x520`, the name + 15, the attributes + 5;
/// height at least `+0x524` and, with « Hauteur toutes rubriques »
/// (`+0xa84`), the attributes + 5 + the band `+0x530` (else up to the
/// « Hauteur auto maximale » `+0xa88`). Association: half-width for the
/// name + 25 and the attributes + 5 (+ 10), half-height for the attributes
/// plus 5 and the band (`+0xa14`, at least half `+0x538`). Rules and
/// inheritances: see `render::rule_extent` and below.
pub fn fit_box(doc: &mut Document, id: ObjId) {
    let ext = |slot| crate::render::text_extent(doc, id, slot);
    let (name, attrs) = (ext(0), ext(1));
    let rule = crate::render::rule_extent(doc, id);
    let letter = crate::render::heritage_letter_width(doc, id);
    let s = &doc.settings;
    let (min_w, min_h, all_rows, max_auto) = (s.i32(0x520), s.i32(0x524), s.i32(0xa84), s.i32(0xa88));
    let (entity_band, assoc_band, corner) = (s.i32(0x530), s.i32(0xa14), s.i32(0x538));
    let Some(o) = obj_mut(doc, id) else { return };
    match &mut o.kind {
        ObjKind::Entite(e) => {
            let r = &mut e.rect;
            let mut need_w = min_w;
            let mut need_h = min_h;
            if let Some((w, h)) = name {
                need_w = need_w.max(w + 15);
                need_h = need_h.max(h);
            }
            if let Some((w, h)) = attrs {
                need_w = need_w.max(w + 5);
                let full = h + 5 + entity_band;
                need_h = need_h.max(if all_rows != 0 { full } else { full.min(max_auto) });
            }
            r.right = r.right.max(r.left + need_w);
            r.bottom = r.bottom.max(r.top + need_h);
        }
        // inheritance (0x1400d0db0): at least 60 × 30, twice its letter + 15 wide
        ObjKind::Heritage(x) => {
            let r = &mut x.rect;
            r.right = r.right.max(r.left + 60.max(letter * 2 + 15));
            r.bottom = r.bottom.max(r.top + 30);
        }
        ObjKind::Regle(x) => {
            if let Some((w, h)) = rule {
                x.rect.right = x.rect.right.max(x.rect.left + w);
                x.rect.bottom = x.rect.bottom.max(x.rect.top + h);
            }
        }
        ObjKind::Relation(x) => {
            let band = if assoc_band > 4 { assoc_band } else { x.f3a0 }.max(corner / 2);
            x.f3a0 = x.f3a0.max(band);
            if let Some((w, h)) = name {
                if 2 * x.f398 < w + 25 {
                    x.f398 = w / 2 + 12;
                }
                if 2 * x.f3a0 < h {
                    x.f3a0 = h / 2 + 1;
                }
            }
            if let Some((w, h)) = attrs.filter(|&(_, h)| h > 0) {
                if 2 * x.f398 < w + 5 {
                    x.f398 = (w + 5) / 2 + 10;
                }
                let need = h + 5 + band;
                if 2 * x.f3a0 < need {
                    x.f3a0 = need / 2 + 1;
                }
            }
        }
        _ => {}
    }
}

/// `fit_box` on every entity, association, rule and inheritance.
pub fn fit_all(doc: &mut Document) {
    let ids: Vec<ObjId> = doc
        .objects
        .iter()
        .filter(|o| matches!(o.class.as_str(), "CDrawEntite" | "CDrawRelation" | "CDrawRegle" | "CDrawHeritage"))
        .map(|o| o.base.id)
        .collect();
    for id in ids {
        fit_box(doc, id);
    }
}

/// Moves a text from the place its owner gives it (`CDrawTexte+0x458/+0x45c`).
pub fn move_text(doc: &mut Document, text: ObjId, dx: i32, dy: i32) {
    if let Some(ObjKind::Texte(t)) = obj_mut(doc, text).map(|o| &mut o.kind) {
        t.f458 += dx;
        t.f45c += dy;
    }
}

/// First segment of the link containing segment `id`.
pub fn first_segment(doc: &Document, id: ObjId) -> ObjId {
    let mut cur = id;
    let mut seen = Vec::new();
    while let Some(p) = doc.object(cur).map(|o| o.base.links.prev).filter(|&p| p != 0 && !seen.contains(&p)) {
        seen.push(cur);
        cur = p;
    }
    cur
}

/// Rectangle of an object as stored (entity, association box, symbol…).
pub fn object_rect(doc: &Document, id: ObjId) -> Option<Rect> {
    let o = doc.object(id)?;
    let b = crate::render::bounds(doc, o)?;
    Some(Rect { left: b.min.x as i32, top: b.min.y as i32, right: b.max.x as i32, bottom: b.max.y as i32 })
}

/// Gives an object the rectangle `r` (resize handles): an association keeps
/// its centre and half-sizes (`+0x390`, `+0x398`, `+0x3a0`), a symbol its
/// centre.
pub fn resize(doc: &mut Document, id: ObjId, r: Rect) {
    let r = Rect {
        left: r.left.min(r.right),
        top: r.top.min(r.bottom),
        right: r.left.max(r.right),
        bottom: r.top.max(r.bottom),
    };
    let centre = Point { x: (r.left + r.right) / 2, y: (r.top + r.bottom) / 2 };
    let Some(o) = obj_mut(doc, id) else { return };
    match &mut o.kind {
        ObjKind::Entite(e) => e.rect = r,
        ObjKind::Relation(x) => {
            x.center = centre;
            x.f398 = ((r.right - r.left) / 2).max(10);
            x.f3a0 = ((r.bottom - r.top) / 2).max(10);
        }
        ObjKind::Image(x) => x.rect = r,
        ObjKind::Regle(x) => x.rect = r,
        ObjKind::Ellipse(x) | ObjKind::Rectangle(x) => x.rect = r,
        ObjKind::Titre(x) => x.rect = r,
        ObjKind::Prof(x) => x.rect = r,
        ObjKind::Cif(x) => {
            x.rect = r;
            x.center = centre;
        }
        ObjKind::Contrainte(x) => {
            x.rect = r;
            x.center = centre;
        }
        ObjKind::Heritage(x) => {
            x.rect = r;
            x.rect2 = r;
        }
        // a free-hand stroke is stretched to the new rectangle
        ObjKind::Crayon(c) => {
            let pts: Vec<Point> = c.points.iter().map(|p| Point { x: c.origin.x + p.x, y: c.origin.y + p.y }).collect();
            let (Some(x0), Some(x1)) = (pts.iter().map(|p| p.x).min(), pts.iter().map(|p| p.x).max()) else { return };
            let (y0, y1) = (pts.iter().map(|p| p.y).min().unwrap(), pts.iter().map(|p| p.y).max().unwrap());
            let scale = |v: i32, a: i32, b: i32, lo: i32, hi: i32| {
                if b == a {
                    lo
                } else {
                    lo + ((v - a) as i64 * (hi - lo) as i64 / (b - a) as i64) as i32
                }
            };
            let moved: Vec<Point> = pts
                .iter()
                .map(|p| Point { x: scale(p.x, x0, x1, r.left, r.right), y: scale(p.y, y0, y1, r.top, r.bottom) })
                .collect();
            c.origin = moved[0];
            c.points = moved.iter().map(|p| Point { x: p.x - moved[0].x, y: p.y - moved[0].y }).collect();
        }
        _ => {}
    }
}

/// Inserts a break point at `at` after segment `seg` of a link: a segment
/// of the same class without owner, anchored at absolute coordinates
/// (`CRattach` mode 0), with its three texts (named `???`, as Looping does).
pub fn add_break(doc: &mut Document, seg: ObjId, at: Point) -> Option<ObjId> {
    let o = doc.object(seg)?;
    let (class, kind, next) = (o.class.clone(), o.base.kind, o.base.links.next);
    if next == 0 {
        return None;
    }
    let mut b = base(kind, 0, DEFAULT_FILL);
    b.f338 = -1;
    let obj = DrawObj {
        class,
        base: b,
        kind: ObjKind::Arc(Arc { anchor: Rattach { mode: 0, offset: at, ..Default::default() }, ..Default::default() }),
    };
    let id = add_with_texts(doc, obj, "???");
    obj_mut(doc, id)?.base.links.prev = seg;
    obj_mut(doc, id)?.base.links.next = next;
    obj_mut(doc, seg)?.base.links.next = id;
    obj_mut(doc, next)?.base.links.prev = id;
    Some(id)
}

/// Removes a break point (a segment without owner) and its texts.
pub fn remove_break(doc: &mut Document, seg: ObjId) {
    let Some(o) = doc.object(seg) else { return };
    if o.base.links.owner != 0 || o.base.links.prev == 0 || o.base.links.next == 0 {
        return;
    }
    let (prev, next, texts) = (o.base.links.prev, o.base.links.next, o.base.links.texts);
    if let Some(p) = obj_mut(doc, prev) {
        p.base.links.next = next;
    }
    if let Some(n) = obj_mut(doc, next) {
        n.base.links.prev = prev;
    }
    doc.objects.retain(|x| x.base.id != seg && !texts.contains(&x.base.id));
}

/// Objects to copy for `ids`: the objects, their texts, and the links whose
/// two ends are copied (all their segments), in document order.
pub fn copy(doc: &Document, ids: &[ObjId]) -> Vec<DrawObj> {
    // a selected link is copied below only when both its ends are: alone,
    // its pasted copy would be a single segment without owner
    let mut keep: Vec<ObjId> =
        ids.iter().copied().filter(|&id| doc.object(id).is_some_and(|o| !o.class.starts_with("CDrawArc"))).collect();
    // a free line goes with all its segments
    let lines: Vec<ObjId> = keep
        .iter()
        .filter(|&&id| is_free_line(doc, id))
        .flat_map(|&id| chain_ids(doc, first_segment(doc, id)))
        .collect();
    keep.extend(lines.into_iter().filter(|id| !ids.contains(id)));
    for o in doc.objects.iter().filter(|o| o.class.starts_with("CDrawArc") && o.base.links.prev == 0) {
        let chain = chain_ids(doc, o.base.id);
        let owners: Vec<ObjId> = chain.iter().filter_map(|&c| doc.object(c)).map(|c| c.base.links.owner).collect();
        let ends = [owners.first().copied().unwrap_or(0), owners.last().copied().unwrap_or(0)];
        if ends.iter().all(|e| *e != 0 && keep.contains(e)) {
            keep.extend(chain);
        }
    }
    let texts: Vec<ObjId> =
        keep.iter().filter_map(|&id| doc.object(id)).flat_map(|o| o.base.links.texts).filter(|&t| t != 0).collect();
    keep.extend(texts);
    doc.objects.iter().filter(|o| keep.contains(&o.base.id)).cloned().collect()
}

/// Pastes copied objects with new ids, moved by (`dx`, `dy`); returns the
/// new ids of the objects that are neither texts nor link segments.
pub fn paste(doc: &mut Document, objs: &[DrawObj], dx: i32, dy: i32) -> Vec<ObjId> {
    let mut next = next_id(doc);
    let map: std::collections::HashMap<ObjId, ObjId> = objs
        .iter()
        .map(|o| {
            let n = next;
            next += 1;
            (o.base.id, n)
        })
        .collect();
    let re = |id: ObjId| if id == 0 { 0 } else { map.get(&id).copied().unwrap_or(0) };
    let mut top = Vec::new();
    for o in objs {
        let mut c = o.clone();
        c.base.id = re(o.base.id);
        let l = &mut c.base.links;
        l.texts = l.texts.map(re);
        l.prev = re(l.prev);
        l.next = re(l.next);
        l.owner = re(l.owner);
        let later_segment = matches!(c.kind, ObjKind::Trait(_)) && c.base.links.prev != 0;
        if !c.class.starts_with("CDrawArc") && c.class != "CDrawTexte" && !later_segment {
            top.push(c.base.id);
        }
        doc.objects.push(c);
    }
    // break points move with the objects
    let bends: Vec<ObjId> = doc
        .objects
        .iter()
        .filter(|o| map.values().any(|&v| v == o.base.id) && o.class.starts_with("CDrawArc"))
        .map(|o| o.base.id)
        .collect();
    for id in top.iter().chain(&bends) {
        translate(doc, *id, dx, dy);
    }
    top
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Order {
    /// « Premier plan »: drawn last.
    Front,
    /// « Arrière-plan »: drawn first.
    Back,
    Forward,
    Backward,
}

/// Changes the drawing order (the order of the document's object list,
/// `0x14008f220`); an object moves with its texts.
pub fn reorder(doc: &mut Document, ids: &[ObjId], order: Order) {
    // groups: an object followed by its texts
    let mut groups: Vec<Vec<DrawObj>> = Vec::new();
    for o in std::mem::take(&mut doc.objects) {
        let owned =
            o.class == "CDrawTexte" && groups.last().is_some_and(|g| g[0].base.links.texts.contains(&o.base.id));
        if owned {
            groups.last_mut().unwrap().push(o);
        } else {
            groups.push(vec![o]);
        }
    }
    let moving = |g: &Vec<DrawObj>| ids.contains(&g[0].base.id);
    match order {
        Order::Front | Order::Back => {
            let (sel, rest): (Vec<_>, Vec<_>) = groups.into_iter().partition(|g| moving(g));
            groups = if order == Order::Front { [rest, sel].concat() } else { [sel, rest].concat() };
        }
        Order::Forward => {
            for i in (0..groups.len().saturating_sub(1)).rev() {
                if moving(&groups[i]) && !moving(&groups[i + 1]) {
                    groups.swap(i, i + 1);
                }
            }
        }
        Order::Backward => {
            for i in 1..groups.len() {
                if moving(&groups[i]) && !moving(&groups[i - 1]) {
                    groups.swap(i, i - 1);
                }
            }
        }
    }
    doc.objects = groups.into_iter().flatten().collect();
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Alignment {
    Left,
    Centre,
    Right,
    Top,
    Middle,
    Bottom,
}

/// Aligns objects (`0x14008ece0`): on the leftmost left edge, the rightmost
/// right edge… and for the centres on the centre of the first object.
pub fn align(doc: &mut Document, ids: &[ObjId], how: Alignment) {
    let rects: Vec<(ObjId, Rect)> = ids.iter().filter_map(|&id| object_rect(doc, id).map(|r| (id, r))).collect();
    let Some(&(_, first)) = rects.first() else { return };
    let target = match how {
        Alignment::Left => rects.iter().map(|(_, r)| r.left).min().unwrap(),
        Alignment::Right => rects.iter().map(|(_, r)| r.right).max().unwrap(),
        Alignment::Top => rects.iter().map(|(_, r)| r.top).min().unwrap(),
        Alignment::Bottom => rects.iter().map(|(_, r)| r.bottom).max().unwrap(),
        Alignment::Centre => (first.left + first.right) / 2,
        Alignment::Middle => (first.top + first.bottom) / 2,
    };
    for (id, r) in rects {
        let (dx, dy) = match how {
            Alignment::Left => (target - r.left, 0),
            Alignment::Right => (target - r.right, 0),
            Alignment::Centre => (target - (r.left + r.right) / 2, 0),
            Alignment::Top => (0, target - r.top),
            Alignment::Bottom => (0, target - r.bottom),
            Alignment::Middle => (0, target - (r.top + r.bottom) / 2),
        };
        translate(doc, id, dx, dy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Mcd, Mld};

    #[test]
    fn build_model_save_and_reload() {
        let mut doc = Document::default();
        let race = add_entity(&mut doc, "Race", Point { x: 400, y: 100 });
        let chien = add_entity(&mut doc, "Chien", Point { x: 0, y: 100 });
        let asso = add_association(&mut doc, "Appartenir", Point { x: 250, y: 150 });
        add_link(&mut doc, chien, asso, 1);
        add_link(&mut doc, race, asso, 2);
        let bytes = doc.save().unwrap();
        let back = Document::load(&bytes).unwrap();
        let mcd = Mcd::from_document(&back);
        assert_eq!(mcd.entities.len(), 2);
        assert_eq!(mcd.associations[0].legs.len(), 2);
        let mld = Mld::from_mcd(&mcd);
        let chien_t = mld.tables.iter().find(|t| t.name == "Chien").unwrap();
        assert_eq!(chien_t.foreign_keys[0].table, "Race");
        assert!(chien_t.columns.iter().any(|c| c.name == "Id_Race" && c.not_null));
        assert_eq!(mld.tables.len(), 2);
    }

    #[test]
    fn delete_removes_links_and_texts() {
        let mut doc = Document::default();
        let a = add_entity(&mut doc, "A", Point { x: 0, y: 0 });
        let b = add_entity(&mut doc, "B", Point { x: 300, y: 0 });
        let r = add_association(&mut doc, "R", Point { x: 150, y: 50 });
        add_link(&mut doc, a, r, 2);
        add_link(&mut doc, b, r, 2);
        delete(&mut doc, a);
        assert!(doc.object(a).is_none());
        let mcd = Mcd::from_document(&doc);
        assert_eq!(mcd.associations[0].legs.len(), 1);
        // no dangling texts
        for o in &doc.objects {
            if o.class == "CDrawTexte" {
                assert!(doc.object(o.base.links.owner).is_some());
            }
        }
    }

    fn model() -> (Document, ObjId, ObjId, ObjId, ObjId) {
        let mut doc = Document::default();
        let a = add_entity(&mut doc, "A", Point { x: 0, y: 0 });
        let b = add_entity(&mut doc, "B", Point { x: 400, y: 0 });
        let r = add_association(&mut doc, "R", Point { x: 250, y: 200 });
        let l = add_link(&mut doc, a, r, 2);
        add_link(&mut doc, b, r, 2);
        (doc, a, b, r, l)
    }

    #[test]
    fn copy_paste_duplicates_objects_and_links() {
        let (mut doc, a, b, r, _) = model();
        let clip = copy(&doc, &[a, b, r]);
        let new = paste(&mut doc, &clip, 30, 30);
        assert_eq!(new.len(), 3);
        let back = Document::load(&doc.save().unwrap()).unwrap();
        let mcd = Mcd::from_document(&back);
        assert_eq!(mcd.entities.len(), 4);
        assert_eq!(mcd.associations.len(), 2);
        assert!(mcd.associations.iter().all(|x| x.legs.len() == 2));
        assert_eq!(Mld::from_mcd(&mcd).tables.len(), 6);
        // a link to an object left out is not copied
        let clip = copy(&doc, &[a, r]);
        assert_eq!(clip.iter().filter(|o| o.class == "CDrawArc").count(), 2);
    }

    #[test]
    fn break_points() {
        let (mut doc, a, _, _, l) = model();
        let bend = add_break(&mut doc, l, Point { x: 100, y: 150 }).unwrap();
        assert_eq!(chain_ids(&doc, l).len(), 3);
        assert_eq!(first_segment(&doc, bend), l);
        let back = Document::load(&doc.save().unwrap()).unwrap();
        assert_eq!(Mcd::from_document(&back).associations[0].legs.len(), 2);
        remove_break(&mut doc, bend);
        assert_eq!(chain_ids(&doc, l).len(), 2);
        // a segment with an owner is not a break point
        remove_break(&mut doc, l);
        assert!(doc.object(l).is_some());
        let _ = a;
    }

    #[test]
    fn order_align_resize() {
        let (mut doc, a, b, r, _) = model();
        reorder(&mut doc, &[a], Order::Front);
        let tops: Vec<ObjId> = doc.objects.iter().filter(|o| o.class != "CDrawTexte").map(|o| o.base.id).collect();
        assert_eq!(*tops.last().unwrap(), a);
        // texts stay after their owner
        let pos = doc.objects.iter().position(|o| o.base.id == a).unwrap();
        assert_eq!(doc.objects[pos + 1].base.links.owner, a);
        reorder(&mut doc, &[a], Order::Back);
        assert_eq!(doc.objects[0].base.id, a);
        align(&mut doc, &[r, a, b], Alignment::Top);
        let tops: Vec<i32> = [a, b, r].iter().map(|&id| object_rect(&doc, id).unwrap().top).collect();
        assert!(tops.iter().all(|&t| t == tops[0]));
        resize(&mut doc, r, Rect { left: 0, top: 0, right: 200, bottom: 80 });
        match &doc.object(r).unwrap().kind {
            ObjKind::Relation(x) => assert_eq!((x.center, x.f398, x.f3a0), (Point { x: 100, y: 40 }, 100, 40)),
            _ => unreachable!(),
        }
    }

    /// An entity linked to its inheritance as supertype and as subtype: the
    /// MLD text used to recurse forever (stack overflow in the application).
    #[test]
    fn inheritance_cycle_does_not_overflow() {
        let mut doc = Document::default();
        let a = add_entity(&mut doc, "A", Point { x: 0, y: 0 });
        let b = add_entity(&mut doc, "B", Point { x: 300, y: 0 });
        let h = add_heritage(&mut doc, Point { x: 150, y: 200 }, 0);
        add_heritage_link(&mut doc, a, h, true);
        add_heritage_link(&mut doc, b, h, false);
        add_heritage_link(&mut doc, a, h, false);
        let mld = Mld::from_mcd(&Mcd::from_document(&doc));
        let o = crate::sql::SqlOptions::from_settings(&doc.settings);
        assert!(mld.to_text(&o).contains("A = ("));
        let _ = crate::sql::generate_with(&mld, &o);
    }

    #[test]
    fn illustrations_save_reload_and_remember_style() {
        let mut doc = Document::default();
        let r = Rect { left: 0, top: 0, right: 100, bottom: 50 };
        let rect = add_rectangle(&mut doc, r);
        let ell = add_ellipse(&mut doc, r);
        let txt = add_texte(&mut doc, r, &["a", "b"]);
        let img = add_image(&mut doc, r, true);
        let cr = add_crayon(&mut doc, &[Point { x: 5, y: 5 }, Point { x: 25, y: 15 }]);
        let link = add_illustration_link(&mut doc, rect, txt);
        let back = Document::load(&doc.save().unwrap()).unwrap();
        assert_eq!(back.objects, doc.objects);
        assert_eq!(back.object(link).unwrap().class, "CDrawArcObjetLibre");
        assert!(matches!(&back.object(img).unwrap().kind, ObjKind::Image(i) if i.f3c0 == 1));
        assert!(matches!(&back.object(cr).unwrap().kind, ObjKind::Crayon(c) if c.points[1] == Point { x: 20, y: 10 }));
        assert_eq!(texte_mut(&mut doc, txt, 0).unwrap().lines, ["a", "b"]);
        assert!(doc.object(ell).is_some());
        // the dialog's « OK » keeps the look for the next rectangle
        if let Some(ObjKind::Rectangle(x)) = obj_mut(&mut doc, rect).map(|o| &mut o.kind) {
            x.f3b0 = 40;
            x.f3a0 = 3;
        }
        remember_style(&mut doc, rect);
        let next = add_rectangle(&mut doc, r);
        assert!(matches!(&doc.object(next).unwrap().kind, ObjKind::Rectangle(x) if x.f3b0 == 40 && x.f3a0 == 3));
    }

    #[test]
    fn free_line_moves_copies_and_deletes_as_a_whole() {
        let mut doc = Document::default();
        let pts = [Point { x: 0, y: 0 }, Point { x: 50, y: 0 }, Point { x: 50, y: 50 }];
        let first = add_free_line(&mut doc, &pts, true);
        let anchors = |doc: &Document, first: ObjId| -> Vec<Point> {
            chain_ids(doc, first)
                .iter()
                .filter_map(|&id| match &doc.object(id)?.kind {
                    ObjKind::Trait(t) => Some(t.anchor.offset),
                    _ => None,
                })
                .collect()
        };
        translate(&mut doc, first, 10, 0);
        assert_eq!(anchors(&doc, first)[2], Point { x: 60, y: 50 });
        let clip = copy(&doc, &[first]);
        let pasted = paste(&mut doc, &clip, 0, 100);
        assert_eq!(pasted.len(), 1);
        assert_eq!(
            anchors(&doc, pasted[0]),
            vec![Point { x: 10, y: 100 }, Point { x: 60, y: 100 }, Point { x: 60, y: 150 }]
        );
        let n = doc.objects.len();
        delete(&mut doc, pasted[0]);
        assert_eq!(doc.objects.len(), n - 3 * 4);
    }

    #[test]
    fn crayon_translates_and_resizes() {
        let mut doc = Document::default();
        let c = add_crayon(&mut doc, &[Point { x: 0, y: 0 }, Point { x: 10, y: 20 }]);
        translate(&mut doc, c, 5, 5);
        resize(&mut doc, c, Rect { left: 5, top: 5, right: 25, bottom: 45 });
        let ObjKind::Crayon(x) = &doc.object(c).unwrap().kind else { panic!() };
        assert_eq!(x.origin, Point { x: 5, y: 5 });
        assert_eq!(x.points[1], Point { x: 20, y: 40 });
    }

    #[test]
    fn magnetism_rounds_to_the_nearest_step() {
        let mut doc = Document::default();
        let e = add_entity(&mut doc, "E", Point { x: 13, y: 26 });
        doc.f340 = 1;
        let step = grid_step(&doc).unwrap();
        assert_eq!(step, 5);
        snap(&mut doc, e, step);
        let ObjKind::Entite(x) = &doc.object(e).unwrap().kind else { panic!() };
        assert_eq!((x.rect.left, x.rect.top, x.rect.right, x.rect.bottom), (15, 25, 175, 115));
        doc.f33c = 0;
        assert_eq!(grid_step(&doc), None);
    }

    #[test]
    fn views_filter_the_display() {
        let mut doc = Document::default();
        let a = add_entity(&mut doc, "A", Point { x: 0, y: 0 });
        let b = add_entity(&mut doc, "B", Point { x: 300, y: 0 });
        let bit = 1 << add_view(&mut doc, "V").unwrap();
        set_views(&mut doc, a, bit, true);
        let d = crate::render::Display { views: bit, ..crate::render::Display::of(&doc) };
        assert!(d.shows(&doc, doc.object(a).unwrap()));
        assert!(!d.shows(&doc, doc.object(b).unwrap()));
        let all = crate::render::Display::of(&doc);
        assert!(all.shows(&doc, doc.object(b).unwrap()));
        remove_view(&mut doc, 1);
        assert!(doc.views.is_empty());
        assert_eq!(doc.object(a).unwrap().base.f338 as u32 & bit, 0);
    }
}
