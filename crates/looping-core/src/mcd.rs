//! Semantic view of a document: entities, associations and their legs.
//!
//! Looping stores everything as drawing objects. Entities and associations
//! own up to three `CDrawTexte` (slot 0 = name, slot 1 = attributes); a leg
//! between an entity and an association is a chain of `CDrawArc` segments
//! whose first and last `owner` are the two ends, and whose slot-1 text
//! holds the cardinality parameter.

use loo_format::{Document, DrawObj, ObjKind, Parametre, Rect};
use serde::Serialize;

use crate::types::DataType;

pub type ObjId = i32;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Attribute {
    pub name: String,
    /// Part of the identifier (`CUnParametre+0x08`).
    pub identifier: bool,
    pub data_type: DataType,
    /// `CUnParametre+0x8c`, set on attributes declared unique in the samples.
    pub unique: bool,
    /// `CUnParametre+0x88`: NOT NULL (only emitted outside the primary key).
    pub not_null: bool,
    /// `CUnParametre+0xf8`: "Commentaire" of the attribute dialog.
    pub comment: String,
    /// `CUnParametre+0x118`: "Complément", SQL appended to the column.
    pub complement: String,
}

impl Attribute {
    pub fn from_param(p: &Parametre) -> Self {
        Attribute {
            name: crate::mld::logical_or(&p.name, &p.s18),
            identifier: p.f8 != 0,
            data_type: DataType::from_param(p),
            unique: p.f8c != 0,
            not_null: p.f88 != 0,
            comment: p.sf8.clone(),
            complement: p.s118.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entity {
    pub id: ObjId,
    pub name: String,
    pub attributes: Vec<Attribute>,
    /// "Commentaire": lines of the slot-2 text.
    pub comment: Vec<String>,
    /// Fictive entity: generates no table.
    pub fictive: bool,
    /// Entity of an external model: no table, its key still migrates.
    pub external: bool,
    pub rect: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Max {
    One,
    Many,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cardinality {
    pub text: String,
    pub min_zero: bool,
    pub max: Max,
    /// Relative identification "(R)".
    pub relative: bool,
}

impl Cardinality {
    pub fn from_param(p: &Parametre) -> Self {
        let text = p.cardinality_text();
        let (min_zero, max) = match p.cardinality {
            0 => (true, Max::One),
            1 => (false, Max::One),
            3 => (false, Max::Many),
            4 => {
                let max = if p.card_max.trim() == "1" { Max::One } else { Max::Many };
                (p.card_min.trim() == "0", max)
            }
            _ => (true, Max::Many),
        };
        Cardinality { text, min_zero, max, relative: p.f84 != 0 && p.cardinality == 1 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Leg {
    /// First arc segment of the chain.
    pub arc: ObjId,
    pub entity: ObjId,
    pub cardinality: Cardinality,
    pub role: String,
    /// "Suffixer clés étrangères" (role parameter `+0x130`, stored
    /// inverted): the role is appended to the migrated key names.
    pub role_suffix: bool,
    /// "Renommer clé étrangère" (role parameter `+0x134`): the role
    /// replaces the migrated key names.
    pub role_rename: bool,
    /// "Place clé étrangère dans clé primaire associée" (cardinality
    /// parameter `+0x14c`): association tables order their keys by it.
    pub place: i32,
    /// Referential actions of the foreign key that references this leg's
    /// entity, from the first arc segment (`CDrawArc+0x3c8..0x3d8`).
    pub actions: Actions,
}

/// Per-link referential actions, only generated when the document option
/// `+0x888` is set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Actions {
    pub on_delete_cascade: bool,
    pub on_update_cascade: bool,
    /// "Autre": free SQL (`CDrawArc+0x3d8`) appended after a space.
    pub other: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Association {
    pub id: ObjId,
    pub name: String,
    pub attributes: Vec<Attribute>,
    /// "Commentaire": lines of the slot-2 text.
    pub comment: Vec<String>,
    pub legs: Vec<Leg>,
    /// "Générer une table de correspondance" (`CDrawRelation+0x3ac`).
    pub forced_table: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Mcd {
    pub entities: Vec<Entity>,
    pub associations: Vec<Association>,
    pub inheritances: Vec<Inheritance>,
    pub cifs: Vec<Cif>,
    pub constraints: Vec<Constraint>,
    /// "Script SQL associé" of rules, inheritances and constraints, in
    /// document order.
    pub scripts: Vec<SqlScript>,
    /// "Compléments script LDD" (document strings at 0xea0): script start,
    /// script end, table start, table end (`[Nom_Table]` = table name).
    pub complements: [String; 4],
    /// "Nom clé étrangère" mask, see [`loo_format::Settings::fk_mask`].
    pub fk_mask: String,
}

/// SQL attached to a rule, an inheritance or a constraint.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SqlScript {
    pub owner: ObjId,
    /// Lines of the owner's slot-1 text (`CDrawTexte+0x4a0`).
    pub lines: Vec<String>,
    /// Order among scripts (`CDrawTexte+0x4d8`).
    pub position: i32,
    /// Rules only: "Avant la création des tables" (`CDrawRegle+0x3ac` = 0).
    pub before_tables: bool,
    /// Rules with "Ajouter en commentaire" (`CDrawRegle+0x3a8`): the
    /// description lines, put first as `--line`.
    pub comment: Vec<String>,
}

/// Inter-association constraint (`CDrawContrainte`), kept for its diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Constraint {
    pub id: ObjId,
    pub name: String,
    /// Linked associations and entities (`CDrawArcContrainte`).
    pub associations: usize,
    pub entities: usize,
}

/// Functional integrity constraint (`CDrawCif` and its `CDrawArcCif` links).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cif {
    pub id: ObjId,
    pub name: String,
    /// Linked associations (exactly one is expected).
    pub associations: Vec<ObjId>,
    /// Entities whose link is "Lien vers l'entité ciblée" (`CUnParametre+0x114`).
    pub targets: Vec<ObjId>,
    /// The other linked entities.
    pub emitters: Vec<ObjId>,
}

/// Inheritance (`CDrawHeritage` and its `CDrawArcHeritage` links).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Inheritance {
    pub id: ObjId,
    /// Entity whose link has "Surtype de l'héritage" (`CUnParametre+0x114`).
    pub supertype: Option<ObjId>,
    /// The other linked entities, in link order.
    pub subtypes: Vec<ObjId>,
    /// `CUnParametre+0x110` of the symbol: 0 none, 1 exclusion, 2 totality,
    /// 3 partition (no effect on the MLD).
    pub constraint: i32,
    /// "Migration de toutes les rubriques du surtype vers les sous-types"
    /// (`CDrawHeritage+0x418`): no supertype table.
    pub migrate_all: bool,
}

/// Name of an object = first line of its slot-0 text.
/// Name of the table of an entity or association: its logical name
/// (`CDrawTexte+0x5f8` of the name text) when set, see [`crate::mld::LOGICAL`].
fn table_name(doc: &Document, o: &DrawObj) -> String {
    let logical = doc.object(o.base.links.texts[0]).and_then(DrawObj::as_texte).map_or("", |t| t.s5f8.as_str());
    crate::mld::logical_or(&object_name(doc, o), logical)
}

pub fn object_name(doc: &Document, o: &DrawObj) -> String {
    doc.object(o.base.links.texts[0])
        .and_then(DrawObj::as_texte)
        .map(|t| t.first_line().to_string())
        .unwrap_or_default()
}

fn attributes(doc: &Document, o: &DrawObj) -> Vec<Attribute> {
    doc.object(o.base.links.texts[1])
        .and_then(DrawObj::as_texte)
        .map(|t| t.params.iter().filter(|p| p.role == 0).map(Attribute::from_param).collect())
        .unwrap_or_default()
}

fn comment(doc: &Document, o: &DrawObj) -> Vec<String> {
    doc.object(o.base.links.texts[2]).and_then(DrawObj::as_texte).map(|t| t.lines.clone()).unwrap_or_default()
}

/// Follows `next` from the first segment; returns the far owner.
fn chain_end(doc: &Document, first: &DrawObj) -> ObjId {
    let mut cur = first;
    let mut guard = 0;
    while cur.base.links.next != 0 && guard < 10_000 {
        match doc.object(cur.base.links.next) {
            Some(n) => cur = n,
            None => break,
        }
        guard += 1;
    }
    cur.base.links.owner
}

impl Mcd {
    pub fn from_document(doc: &Document) -> Mcd {
        let mut mcd = Mcd::default();
        for o in &doc.objects {
            match (&o.kind, o.class.as_str()) {
                (ObjKind::Entite(e), "CDrawEntite") => mcd.entities.push(Entity {
                    id: o.id(),
                    name: table_name(doc, o),
                    attributes: attributes(doc, o),
                    comment: comment(doc, o),
                    fictive: e.is_fictive(),
                    external: e.is_external(),
                    rect: e.rect,
                }),
                (ObjKind::Relation(r), _) => mcd.associations.push(Association {
                    id: o.id(),
                    name: table_name(doc, o),
                    attributes: attributes(doc, o),
                    comment: comment(doc, o),
                    legs: Vec::new(),
                    forced_table: r.f3ac != 0,
                }),
                _ => {}
            }
        }
        for o in &doc.objects {
            if o.class != "CDrawArc" || o.base.links.prev != 0 {
                continue;
            }
            let a = o.base.links.owner;
            let b = chain_end(doc, o);
            let (ent, asso) = if mcd.entity(a).is_some() && mcd.association(b).is_some() {
                (a, b)
            } else if mcd.entity(b).is_some() && mcd.association(a).is_some() {
                (b, a)
            } else {
                continue;
            };
            let card_param = doc
                .object(o.base.links.texts[1])
                .and_then(DrawObj::as_texte)
                .and_then(|t| t.params.iter().find(|p| p.role == 1));
            let cardinality = card_param.map(Cardinality::from_param).unwrap_or(Cardinality {
                text: "0,n".into(),
                min_zero: true,
                max: Max::Many,
                relative: false,
            });
            let role_param = doc
                .object(o.base.links.texts[1])
                .and_then(DrawObj::as_texte)
                .and_then(|t| t.params.iter().find(|p| p.role == 2));
            let role = role_param.map(|p| p.sf0.clone()).unwrap_or_default();
            let role_suffix = role_param.is_none_or(|p| p.n130 == 0);
            let role_rename = role_param.is_some_and(|p| p.f134 != 0);
            let place = card_param.map_or(1, |p| p.f14c);
            if let Some(asso) = mcd.associations.iter_mut().find(|x| x.id == asso) {
                let actions = match &o.kind {
                    ObjKind::Arc(arc) => Actions {
                        on_delete_cascade: arc.f3c8[0] != 0,
                        on_update_cascade: arc.f3c8[1] != 0,
                        other: (arc.f3c8[2] != 0).then(|| arc.label.clone()),
                    },
                    _ => Actions::default(),
                };
                asso.legs.push(Leg {
                    arc: o.id(),
                    entity: ent,
                    cardinality,
                    role,
                    role_suffix,
                    role_rename,
                    place,
                    actions,
                });
            }
        }
        mcd.read_inheritances(doc);
        mcd.read_cifs(doc);
        mcd.read_scripts(doc);
        mcd.fk_mask = doc.settings.fk_mask();
        mcd
    }

    fn read_scripts(&mut self, doc: &Document) {
        for (i, c) in self.complements.iter_mut().enumerate() {
            // multi-line values are stored with CRLF (Windows edit controls)
            *c = doc.strings_ea0[i].replace("\r\n", "\n");
        }
        for o in &doc.objects {
            let (before_tables, with_comment) = match &o.kind {
                ObjKind::Regle(r) => (r.raw_flag != 0, r.f3a8 != 0),
                ObjKind::Heritage(_) | ObjKind::Contrainte(_) => (false, false),
                _ => continue,
            };
            let Some(t) = doc.object(o.base.links.texts[1]).and_then(DrawObj::as_texte) else {
                continue;
            };
            if matches!(o.kind, ObjKind::Contrainte(_)) {
                self.constraints.push(Constraint {
                    id: o.id(),
                    name: object_name(doc, o),
                    associations: 0,
                    entities: 0,
                });
            }
            let comment = if with_comment { t.params.iter().map(|p| p.name.clone()).collect() } else { Vec::new() };
            if t.lines2.is_empty() && comment.is_empty() {
                continue;
            }
            self.scripts.push(SqlScript {
                owner: o.id(),
                lines: t.lines2.clone(),
                position: t.f4d8,
                before_tables,
                comment,
            });
        }
        for o in &doc.objects {
            if o.class != "CDrawArcContrainte" || o.base.links.prev != 0 {
                continue;
            }
            let ends = [o.base.links.owner, chain_end(doc, o)];
            let is_assoc = ends.iter().any(|&e| self.association(e).is_some());
            let is_entity = ends.iter().any(|&e| self.entity(e).is_some());
            if let Some(c) = self.constraints.iter_mut().find(|c| ends.contains(&c.id)) {
                c.associations += is_assoc as usize;
                c.entities += is_entity as usize;
            }
        }
    }

    fn read_cifs(&mut self, doc: &Document) {
        for o in &doc.objects {
            if matches!(o.kind, ObjKind::Cif(_)) {
                let name = object_name(doc, o);
                self.cifs.push(Cif {
                    id: o.id(),
                    name,
                    associations: Vec::new(),
                    targets: Vec::new(),
                    emitters: Vec::new(),
                });
            }
        }
        for o in &doc.objects {
            if o.class != "CDrawArcCif" || o.base.links.prev != 0 {
                continue;
            }
            let (a, b) = (o.base.links.owner, chain_end(doc, o));
            let (end, sym) = if self.cifs.iter().any(|c| c.id == b) { (a, b) } else { (b, a) };
            let target = doc
                .object(o.base.links.texts[1])
                .and_then(DrawObj::as_texte)
                .and_then(|t| t.params.iter().find(|p| p.role == 4))
                .is_some_and(|p| p.f114 != 0);
            let is_assoc = self.association(end).is_some();
            let is_entity = self.entity(end).is_some();
            if let Some(c) = self.cifs.iter_mut().find(|c| c.id == sym) {
                if is_assoc {
                    c.associations.push(end);
                } else if is_entity && target {
                    c.targets.push(end);
                } else if is_entity {
                    c.emitters.push(end);
                }
            }
        }
    }

    fn read_inheritances(&mut self, doc: &Document) {
        for o in &doc.objects {
            let ObjKind::Heritage(h) = &o.kind else {
                continue;
            };
            let constraint = doc
                .object(o.base.links.texts[1])
                .and_then(DrawObj::as_texte)
                .and_then(|t| t.params.first())
                .map_or(0, |p| p.f110);
            self.inheritances.push(Inheritance {
                id: o.id(),
                supertype: None,
                subtypes: Vec::new(),
                constraint,
                migrate_all: h.f418 != 0,
            });
        }
        for o in &doc.objects {
            if o.class != "CDrawArcHeritage" || o.base.links.prev != 0 {
                continue;
            }
            let (a, b) = (o.base.links.owner, chain_end(doc, o));
            let (ent, sym) = if self.entity(a).is_some() { (a, b) } else { (b, a) };
            let supertype = doc
                .object(o.base.links.texts[1])
                .and_then(DrawObj::as_texte)
                .and_then(|t| t.params.iter().find(|p| p.role == 3))
                .is_some_and(|p| p.f114 != 0);
            if let Some(h) = self.inheritances.iter_mut().find(|h| h.id == sym) {
                if supertype {
                    h.supertype = Some(ent);
                } else {
                    h.subtypes.push(ent);
                }
            }
        }
    }

    pub fn entity(&self, id: ObjId) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == id)
    }

    pub fn association(&self, id: ObjId) -> Option<&Association> {
        self.associations.iter().find(|a| a.id == id)
    }
}
