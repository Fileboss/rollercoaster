//! MCD -> MLD (relational schema), following the output of Looping 4.1
//! (checked against `samples/ref/`, see `tests/reference.rs`):
//!
//! * every non-fictive entity becomes a table, identifiers first; an entity
//!   without identifier gets a `***CLE_ABSENTE***` placeholder key;
//! * an association with a `x,1` leg has no table and drops its attributes:
//!   the keys of the other legs migrate into that entity (the `1,1` leg,
//!   else the last `x,1` one), after its attributes and in the creation
//!   order of the links (see [`Mld::from_mcd`]). NOT NULL when `1,1`,
//!   optional `*` when `0,1`, UNIQUE for a binary 1-1; two `1,1` sides each
//!   take the other's key (hence circular references). A relative
//!   identification "(R)" puts the key at the head of the child's key. The
//!   missing key of a parent without identifier only stays where the table
//!   has another foreign key (see [`drop_lone_missing_keys`]);
//! * any other association becomes a table keyed by the migrated keys
//!   (in leg order), followed by its attributes; so does a binary one with a
//!   `0,1` side when "table de correspondance" is forced (`+0x3ac`), keyed
//!   by that side's key, then attributes, then the other key;
//! * an entity of an external model has no table, but its key migrates;
//! * a CIF on an n-ary association takes its target entities' keys out of
//!   the association table's key (after the attributes, NOT NULL);
//! * inheritance (see [`Mld::add_inheritance`]): subtypes take the
//!   supertype's key, or all its columns when "migration de toutes les
//!   rubriques" removes the supertype table;
//! * a migrated column is named by the "Nom clé étrangère" mask (`[FK]`,
//!   `[Classe]_[FK]`...) unless the parent itself migrated it, then takes
//!   the leg's role as suffix (`Id_Personne_connu`) or, with "Renommer clé
//!   étrangère", the role as name; a rank when it duplicates another
//!   column (`Id_student_1`). The textual MLD groups a composite or chained
//!   key under these names (`#(#Hotel_Num_hotel, Chambre_Num_chambre)`);
//! * an association table orders its keys by "place dans la clé primaire"
//!   (stable); relative identifications keep document order;
//! * entity tables are ordered by successive passes, each emitting the
//!   tables whose referenced tables are already emitted; tables left over
//!   reference each other ("Références circulaires"). Association tables
//!   follow, in document order.

use serde::Serialize;

use crate::mcd::{Actions, Association, Attribute, Cif, Entity, Inheritance, Leg, Max, Mcd, ObjId, SqlScript};
use crate::types::DataType;

/// Placeholder key of an entity without identifier.
pub const MISSING_KEY: &str = "***CLE_ABSENTE***";

/// Placeholder key of an entity without identifier that is relatively
/// identified (0x1400cc4e5, r006): its key holds the parent's, not its own.
pub const INCOMPLETE_KEY: &str = "***CLE_INCOMPLETE***";

/// A placeholder key column, see [`MISSING_KEY`] and [`INCOMPLETE_KEY`].
pub fn is_placeholder(name: &str) -> bool {
    name == MISSING_KEY || name == INCOMPLETE_KEY
}

/// Separates a column name from the rank that makes it unique in its table
/// (`Id_student` + `1`): Looping appends `_1` after the naming convention
/// (`idStudent_1` in Camel case), see [`crate::sql::SqlOptions::name`].
pub const RANK_SEP: char = '\u{1f}';

/// Marks a logical name (« Nom logique » of the entity, association or
/// attribute dialogs: `CDrawTexte+0x5f8`, `CUnParametre+0x18`): Looping
/// emits it as typed, without naming convention nor accent removal, in the
/// table, its migrated keys and its constraint names (checked under Wine,
/// case c44).
pub const LOGICAL: char = '\u{1e}';

/// `logical` when set, else the conceptual name.
pub fn logical_or(conceptual: &str, logical: &str) -> String {
    if logical.is_empty() {
        conceptual.to_string()
    } else {
        format!("{LOGICAL}{logical}")
    }
}

/// A name as shown in messages (without the markers).
pub fn shown(name: &str) -> String {
    name.chars().filter(|&c| c != LOGICAL).collect()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Column {
    pub name: String,
    pub data_type: DataType,
    pub primary: bool,
    pub foreign: bool,
    /// NOT NULL outside the primary key (which is never marked).
    pub not_null: bool,
    /// The attribute's own UNIQUE flag (not the UNIQUE of a 1-1 key).
    pub unique: bool,
    pub comment: String,
    /// SQL appended after the type and NOT NULL.
    pub complement: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ForeignKey {
    pub columns: Vec<String>,
    pub table: String,
    /// Source of the referenced table: two entities may share a name (r024).
    pub parent: ObjId,
    /// Role of the leg, when used in the column names: also in the
    /// constraint name (cases c94, c95).
    pub role: String,
    pub referenced: Vec<String>,
    /// From the leg of the referenced entity.
    pub actions: Actions,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Table {
    pub name: String,
    /// Entity or association the table comes from.
    pub source: ObjId,
    pub columns: Vec<Column>,
    pub foreign_keys: Vec<ForeignKey>,
    pub unique: Vec<Vec<String>>,
    /// Part of a cycle of foreign keys between entity tables.
    pub circular: bool,
    /// Association with fewer than 2 legs: only its attributes, flagged in
    /// the textual MLD, and no script.
    pub too_few_legs: bool,
    /// UNIQUE attributes, added to `unique` after the keys of 1-1
    /// associations and inheritances (r002, r040).
    #[serde(skip)]
    pub(crate) attribute_unique: Vec<Vec<String>>,
    /// Comment lines of the entity or association.
    pub comment: Vec<String>,
}

impl Table {
    fn new(name: &str, source: ObjId, comment: &[String]) -> Table {
        Table {
            name: name.to_string(),
            source,
            columns: Vec::new(),
            foreign_keys: Vec::new(),
            unique: Vec::new(),
            circular: false,
            too_few_legs: false,
            attribute_unique: Vec::new(),
            comment: comment.to_vec(),
        }
    }

    pub fn primary_key(&self) -> Vec<&Column> {
        self.columns.iter().filter(|c| c.primary).collect()
    }

    fn unique_name(&self, base: &str) -> String {
        if is_placeholder(base) || !self.columns.iter().any(|c| c.name == base) {
            return base.to_string();
        }
        (1..).map(|i| format!("{base}{RANK_SEP}{i}")).find(|n| !self.columns.iter().any(|c| &c.name == n)).unwrap()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Mld {
    pub tables: Vec<Table>,
    pub warnings: Vec<String>,
    /// Lines Looping puts first in the textual MLD, `Name = (message );`:
    /// constraint, then CIF, then inheritance diagnostics.
    pub notes: Vec<Note>,
    /// SQL of rules, inheritances and constraints, in output order.
    pub scripts: Vec<SqlScript>,
    /// "Compléments script LDD", see [`Mcd::complements`].
    pub complements: [String; 4],
    /// Names of the inter-association constraints, in document order.
    pub constraints: Vec<String>,
    /// "Nom clé étrangère" mask used while building.
    #[serde(skip)]
    pub(crate) fk_mask: String,
    /// Tables of the previous pass, whose keys the migrations read, see
    /// [`Mld::from_mcd`].
    #[serde(skip)]
    pub(crate) previous: Vec<Table>,
}

/// A diagnostic line of the textual MLD: `Name = (message , message );`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Note {
    pub name: String,
    pub messages: Vec<String>,
    /// Between messages: `" / "` for constraints, `", "` for inheritances.
    pub separator: &'static str,
    /// Inheritance diagnostics prevent the script; CIF and constraint ones
    /// do not.
    pub blocking: bool,
}

const HERITAGE_SUPERTYPE: &str = "Il faut au moins un surtype";
const HERITAGE_SUBTYPE: &str = "Il faut au moins un sous-type";
const TOO_FEW_LEGS: &str = " Attention ! Une association doit posséder au moins 2 pattes";
const CIF_ASSOCIATION: &str = "Une CIF doit être rattachée à une association reliant plus de 2 entités, \
et toutes les cardinalités impliquées doivent être multiples";
const CIF_TARGET: &str = "Une CIF nécessite une entité cible";
const CONSTRAINT_ASSOCIATIONS: &str = "Une contrainte inter-associations concerne au moins deux associations";
const INCLUSION_TARGET: &str = "Une contrainte d'inclusion nécessite une association cible";
const INCLUSION_EMITTER: &str = "Une contrainte d'inclusion nécessite au moins une association émettrice";
const CONSTRAINT_PIVOT: &str = "Une contrainte inter-associations nécessite au moins une entité pivot";
const CIF_NO_EMITTER: &str =
    "Aucun émetteur n\u{2019}étant indiqué, toutes les entités non ciblées sont considérées comme des émetteurs";

fn column(a: &Attribute, primary: bool) -> Column {
    Column {
        name: a.name.clone(),
        data_type: a.data_type.clone(),
        primary,
        foreign: false,
        not_null: a.not_null && !primary,
        unique: a.unique && !primary,
        comment: a.comment.clone(),
        complement: a.complement.clone(),
    }
}

fn entity_table(e: &Entity) -> Table {
    let mut t = Table::new(&e.name, e.id, &e.comment);
    if !e.attributes.iter().any(|a| a.identifier) {
        t.columns.push(Column {
            name: MISSING_KEY.into(),
            data_type: DataType::Free(String::new()),
            primary: true,
            foreign: false,
            not_null: false,
            unique: false,
            comment: String::new(),
            complement: String::new(),
        });
    }
    // homonymous attributes are ranked like migrated keys (`Nom`, `Nom_1`)
    for a in e.attributes.iter().filter(|a| a.identifier) {
        let name = t.unique_name(&a.name);
        t.columns.push(Column { name, ..column(a, true) });
    }
    for a in e.attributes.iter().filter(|a| !a.identifier) {
        let name = t.unique_name(&a.name);
        if a.unique {
            t.attribute_unique.push(vec![name.clone()]);
        }
        t.columns.push(Column { name, ..column(a, false) });
    }
    t
}

/// A CIF on an n-ary association: the keys of its target entities leave the
/// primary key and go after the attributes, NOT NULL.
fn apply_cif(t: &mut Table, a: &Association, cif: &Cif) {
    let n_ary = a.legs.len() > 2 && a.legs.iter().all(|l| l.cardinality.max == Max::Many);
    if !n_ary || cif.targets.is_empty() {
        return;
    }
    // the target entities' foreign keys, by referenced table
    let targets: Vec<ObjId> = a
        .legs
        .iter()
        .enumerate()
        .filter(|(_, l)| cif.targets.contains(&l.entity))
        .filter_map(|(i, _)| t.foreign_keys.get(i).map(|fk| fk.parent))
        .collect();
    let moved: Vec<String> =
        t.foreign_keys.iter().filter(|fk| targets.contains(&fk.parent)).flat_map(|fk| fk.columns.clone()).collect();
    let (mut out, keep): (Vec<Column>, Vec<Column>) = t.columns.drain(..).partition(|c| moved.contains(&c.name));
    for c in &mut out {
        c.primary = false;
        c.not_null = true;
    }
    t.columns = keep;
    t.columns.append(&mut out);
    // foreign keys follow the column order
    let columns = &t.columns;
    t.foreign_keys.sort_by_key(|fk| fk.columns.first().and_then(|c| columns.iter().position(|x| &x.name == c)));
}

/// How the columns of a migrated key are named.
#[derive(Clone, Copy)]
struct FkNaming<'a> {
    /// "Nom clé étrangère": `[FK]` = referenced column, `[Classe]` =
    /// referenced entity. Only for the parent's own columns: a column the
    /// parent itself migrated keeps its name.
    mask: &'a str,
    role: &'a str,
    /// Append the role (`Id_Personne_connu`).
    suffix: bool,
    /// The role replaces the name (rename wins over suffix).
    rename: bool,
}

impl<'a> FkNaming<'a> {
    fn plain(mask: &'a str) -> Self {
        FkNaming { mask, role: "", suffix: false, rename: false }
    }

    fn of_leg(mask: &'a str, l: &'a Leg) -> Self {
        FkNaming { mask, role: &l.role, suffix: l.role_suffix, rename: l.role_rename }
    }

    fn name(&self, parent: &Table, c: &Column) -> String {
        if is_placeholder(&c.name) {
            return c.name.clone();
        }
        // the role replaces the name of the parent's own column; one the
        // parent itself migrated takes it as suffix (r137, r150)
        if self.rename && !self.role.is_empty() && !c.foreign {
            return self.role.to_string();
        }
        // a migrated column passes on its name without its rank in the
        // parent (r066, r117), the child ranks it again
        let base = if c.foreign {
            c.name.rsplit_once(RANK_SEP).map_or(c.name.as_str(), |(b, _)| b).to_string()
        } else {
            let mask = if self.mask.trim_matches(' ').is_empty() { "[FK]" } else { self.mask };
            mask.replace("[Classe]", &parent.name).replace("[FK]", &c.name)
        };
        if (self.suffix || self.rename) && !self.role.is_empty() {
            format!("{base}_{}", self.role)
        } else {
            base
        }
    }
}

/// Adds the key of `parent` to `child` as a foreign key; returns the new
/// column names (see [`FkNaming`]), duplicates getting a rank. A relative
/// identification puts the key at the head of the child's key, after the
/// keys already migrated the same way.
fn migrate(
    child: &mut Table,
    parent: &Table,
    not_null: bool,
    into_pk: bool,
    naming: FkNaming,
    actions: Actions,
) -> Vec<String> {
    let pk: Vec<Column> = parent.primary_key().into_iter().cloned().collect();
    let naming_role = if (naming.suffix || naming.rename) && !naming.role.is_empty() {
        naming.role.to_string()
    } else {
        String::new()
    };
    let mut names = Vec::new();
    let mut at =
        if into_pk { child.columns.iter().take_while(|c| c.primary && c.foreign).count() } else { child.columns.len() };
    for c in &pk {
        let name = child.unique_name(&naming.name(parent, c));
        child.columns.insert(
            at,
            Column {
                name: name.clone(),
                data_type: c.data_type.clone(),
                primary: into_pk,
                foreign: true,
                not_null: not_null && !into_pk,
                unique: false,
                comment: String::new(),
                complement: String::new(),
            },
        );
        at += 1;
        names.push(name);
    }
    if !names.is_empty() {
        child.foreign_keys.push(ForeignKey {
            columns: names.clone(),
            table: parent.name.clone(),
            parent: parent.source,
            role: naming_role.clone(),
            referenced: pk.iter().map(|c| c.name.clone()).collect(),
            actions,
        });
    }
    names
}

/// « Migration de toutes les rubriques du surtype vers les sous-types »
/// (`CDrawHeritage+0x418`, applied by 0x1400cf0b0, in document order):
/// unless the supertype has association, constraint or CIF links, the
/// supertype and the inheritance leave the model and each subtype gets a
/// copy of the supertype's own attributes (not those it received itself,
/// case c60): before its own ones, or after them when it has an
/// identifier, the copied identifiers then becoming UNIQUE (case c59). A
/// supertype removed this way no longer counts in other inheritances
/// (cases c62, c69).
fn migrate_all(mcd: &Mcd) -> Mcd {
    let mut out = mcd.clone();
    let mut removed_entities = Vec::new();
    let mut removed_inheritances = Vec::new();
    let mut before: Vec<(ObjId, Vec<Attribute>)> = Vec::new();
    let mut after: Vec<(ObjId, Vec<Attribute>)> = Vec::new();
    for h in mcd.inheritances.iter().filter(|h| h.migrate_all) {
        let Some(&sup) = h.supertypes.first() else {
            continue;
        };
        let linked = mcd.associations.iter().any(|a| a.legs.iter().any(|l| l.entity == sup))
            || mcd.constraints.iter().any(|c| c.pivots.contains(&sup))
            || mcd.cifs.iter().any(|c| c.targets.contains(&sup) || c.emitters.contains(&sup));
        let Some(parent) = mcd.entity(sup).filter(|_| !linked) else {
            continue;
        };
        removed_entities.push(sup);
        removed_inheritances.push(h.id);
        for &sub in &h.subtypes {
            let keyed = mcd.entity(sub).is_some_and(|e| e.attributes.iter().any(|a| a.identifier));
            let (list, copies) = if keyed {
                let copies = parent.attributes.iter().map(|a| Attribute {
                    identifier: false,
                    unique: a.unique || a.identifier,
                    ..a.clone()
                });
                (&mut after, copies.collect())
            } else {
                (&mut before, parent.attributes.clone())
            };
            match list.iter_mut().find(|(id, _)| *id == sub) {
                Some((_, l)) => l.extend(copies),
                None => list.push((sub, copies)),
            }
        }
    }
    for e in &mut out.entities {
        let head = before.iter().find(|(id, _)| *id == e.id).map(|(_, l)| l.clone()).unwrap_or_default();
        let tail = after.iter().find(|(id, _)| *id == e.id).map(|(_, l)| l.clone()).unwrap_or_default();
        e.attributes = head.into_iter().chain(std::mem::take(&mut e.attributes)).chain(tail).collect();
    }
    out.entities.retain(|e| !removed_entities.contains(&e.id));
    out.inheritances.retain(|h| !removed_inheritances.contains(&h.id));
    for h in &mut out.inheritances {
        h.supertypes.retain(|e| !removed_entities.contains(e));
        h.subtypes.retain(|e| !removed_entities.contains(e));
    }
    out
}

/// The missing key of a parent without identifier migrates like any key,
/// but outside the primary key Looping only keeps it in a table that has
/// another foreign key outside its primary key (cases c78 to c93; a
/// relative identification keeps it, c81). The emptied foreign key still
/// orders the tables.
fn drop_lone_missing_keys(t: &mut Table) {
    let lone = |c: &Column| c.foreign && !c.primary && is_placeholder(&c.name);
    if t.columns.iter().any(|c| c.foreign && !c.primary && !is_placeholder(&c.name)) || !t.columns.iter().any(lone) {
        return;
    }
    let pk: Vec<String> = t.primary_key().iter().map(|c| c.name.clone()).collect();
    t.columns.retain(|c| !lone(c));
    for fk in &mut t.foreign_keys {
        if fk.columns.iter().all(|c| is_placeholder(c)) && !fk.columns.iter().any(|c| pk.contains(c)) {
            fk.columns.clear();
            fk.referenced.clear();
        }
    }
    t.unique.retain(|u| !u.iter().all(|c| is_placeholder(c)));
}

impl Mld {
    /// Looping migrates the keys in the order of the links, a migrated key
    /// standing for the parent's whole key, whatever comes later (r117,
    /// r150). Emulated by passes: the first one takes the relative
    /// identifications first, so that keys are complete; each next one
    /// follows the order of the links and reads the parents' keys from the
    /// previous one, until nothing changes.
    pub fn from_mcd(mcd: &Mcd) -> Mld {
        let migrated = migrate_all(mcd);
        let first = Mld::build(&migrated, Vec::new());
        let mut mld = first.clone();
        let width = |m: &Mld| m.tables.iter().map(|t| t.columns.len()).sum::<usize>();
        for _ in 0..10 {
            let mut next = Mld::build(&migrated, mld.tables.clone());
            if next.tables == mld.tables {
                next.previous.clear();
                return next;
            }
            // keys that keep growing come from a cycle of relative
            // identifications, which would grow forever
            if width(&next) > 2 * width(&first) {
                break;
            }
            mld = next;
        }
        // no fixed point (a weak entity identified through itself, r057)
        first
    }

    fn build(mcd: &Mcd, previous: Vec<Table>) -> Mld {
        let link_order = !previous.is_empty();
        let mut mld = Mld { fk_mask: mcd.fk_mask.clone(), previous, ..Mld::default() };
        let subtypes: Vec<ObjId> = mcd.inheritances.iter().flat_map(|h| h.subtypes.clone()).collect();
        for e in mcd.entities.iter().filter(|e| !e.fictive) {
            if !e.attributes.iter().any(|a| a.identifier) && !subtypes.contains(&e.id) {
                mld.warnings.push(format!("L'entité {} n'a pas d'identifiant", shown(&e.name)));
            }
            mld.tables.push(entity_table(e));
        }
        let entity_tables = mld.tables.len();
        let external: Vec<ObjId> = mcd.entities.iter().filter(|e| e.external).map(|e| e.id).collect();
        // supertypes before their subtypes
        let mut inheritances = Vec::new();
        // an incomplete one (0x1400d19f0) only gives its diagnostic
        let mut pending: Vec<&Inheritance> =
            mcd.inheritances.iter().filter(|h| !h.supertypes.is_empty() && !h.subtypes.is_empty()).collect();
        while !pending.is_empty() {
            let ready = pending
                .iter()
                .position(|h| !pending.iter().any(|g| h.supertypes.iter().any(|s| g.subtypes.contains(s))))
                .unwrap_or(0);
            let h = pending.remove(ready);
            // after the associations in link order (r117)
            if !link_order {
                mld.add_inheritance(h, mcd, false);
            }
            inheritances.push(h);
        }
        // Relative identifications first, in document order but parents
        // before children so that weak entities have their full key before
        // other tables reference them; then the order of the links, which
        // gives Looping's order of migrated keys (cases c71 to c73).
        let (mut relative, mut others): (Vec<&Association>, Vec<&Association>) =
            mcd.associations.iter().partition(|a| !link_order && a.legs.iter().any(|l| l.cardinality.relative));
        // the link that matters is the child's (the x,1 leg): beeven, c72
        let rank = |a: &Association| {
            a.legs.iter().filter(|l| l.cardinality.max == Max::One).map(|l| l.position).min().unwrap_or(usize::MAX)
        };
        others.sort_by_key(|a| rank(a));
        let mut order = Vec::new();
        while !relative.is_empty() {
            let weak = |a: &Association| -> Vec<ObjId> {
                a.legs.iter().filter(|l| l.cardinality.relative).map(|l| l.entity).collect()
            };
            let ready = relative
                .iter()
                .position(|a| {
                    let parents: Vec<ObjId> =
                        a.legs.iter().filter(|l| !l.cardinality.relative).map(|l| l.entity).collect();
                    !relative.iter().any(|b| b.id != a.id && weak(b).iter().any(|e| parents.contains(e)))
                })
                .unwrap_or(0);
            order.push(relative.remove(ready));
        }
        // a 1,1–1,1 association with a relative identification on one side:
        // that side with the relative ones, the other in link order (r044)
        let split = |a: &Association| -> Option<(usize, usize)> {
            let [l0, l1] = &a.legs[..] else { return None };
            if link_order {
                return None;
            }
            let both = [l0, l1].iter().all(|l| l.cardinality.max == Max::One && !l.cardinality.min_zero);
            match (l0.cardinality.relative, l1.cardinality.relative) {
                (true, false) if both => Some((0, 1)),
                (false, true) if both => Some((1, 0)),
                _ => None,
            }
        };
        let mut steps: Vec<(&Association, Option<usize>)> = order.iter().map(|&a| (a, split(a).map(|s| s.0))).collect();
        let mut later: Vec<(&Association, Option<usize>, usize)> =
            others.into_iter().map(|a| (a, None, rank(a))).collect();
        for &a in &order {
            if let Some((_, other)) = split(a) {
                later.push((a, Some(other), a.legs[other].position));
            }
        }
        later.sort_by_key(|x| x.2);
        steps.extend(later.into_iter().map(|(a, only, _)| (a, only)));
        let mut assoc_tables = Vec::new();
        for (a, only) in steps {
            if let Some(mut t) = mld.add_association(a, only) {
                for cif in mcd.cifs.iter().filter(|c| c.associations == [a.id]) {
                    apply_cif(&mut t, a, cif);
                }
                assoc_tables.push(t);
            }
        }
        // a subtype with its own identifier gets its supertype's key after
        // those of its associations (r008, r030, r035)
        for h in inheritances {
            if link_order {
                mld.add_inheritance(h, mcd, false);
            }
            mld.add_inheritance(h, mcd, true);
        }
        for c in &mcd.constraints {
            // in the order of 0x1400bfe60
            let mut msgs: Vec<String> = Vec::new();
            for &p in &c.pivots {
                let concerned = c
                    .associations
                    .iter()
                    .any(|&(a, _)| mcd.association(a).is_some_and(|a| a.legs.iter().any(|l| l.entity == p)));
                if !concerned {
                    let name = mcd.entity(p).map_or(String::new(), |e| shown(&e.name));
                    msgs.push(format!("L'entité pivot '{name}' ne concerne aucune association"));
                }
            }
            if c.kind == 4 {
                if !c.associations.iter().any(|a| a.1) {
                    msgs.push(INCLUSION_TARGET.into());
                }
                if !c.associations.iter().any(|a| !a.1) {
                    msgs.push(INCLUSION_EMITTER.into());
                }
            }
            if c.associations.len() < 2 {
                msgs.push(CONSTRAINT_ASSOCIATIONS.into());
            }
            if c.pivots.is_empty() {
                msgs.push(CONSTRAINT_PIVOT.into());
            }
            if !msgs.is_empty() {
                mld.notes.push(Note { name: c.name.clone(), messages: msgs, separator: " / ", blocking: false });
            }
        }
        for cif in &mcd.cifs {
            let assoc = match cif.associations[..] {
                [id] => mcd.association(id),
                _ => None,
            };
            let n_ary =
                assoc.is_some_and(|a| a.legs.len() > 2 && a.legs.iter().all(|l| l.cardinality.max == Max::Many));
            let note = if !n_ary {
                Some(CIF_ASSOCIATION)
            } else if cif.targets.is_empty() {
                Some(CIF_TARGET)
            } else if cif.emitters.is_empty() {
                Some(CIF_NO_EMITTER)
            } else {
                None
            };
            if let Some(n) = note {
                mld.notes.push(Note {
                    name: cif.name.clone(),
                    messages: vec![n.to_string()],
                    separator: "",
                    blocking: false,
                });
            }
        }
        for h in &mcd.inheritances {
            let mut msgs = Vec::new();
            if h.supertypes.is_empty() {
                msgs.push(HERITAGE_SUPERTYPE.to_string());
            }
            if h.subtypes.is_empty() {
                msgs.push(HERITAGE_SUBTYPE.to_string());
            }
            if !msgs.is_empty() {
                mld.notes.push(Note { name: h.name.clone(), messages: msgs, separator: ", ", blocking: true });
            }
        }
        // scripts by position, then document order (the sort is stable)
        mld.scripts = mcd.scripts.clone();
        mld.scripts.sort_by_key(|s| s.position);
        mld.complements = mcd.complements.clone();
        mld.constraints = mcd.constraints.iter().map(|c| c.name.clone()).collect();
        assoc_tables.sort_by_key(|t| mcd.associations.iter().position(|a| a.id == t.source));
        mld.order_entity_tables(entity_tables);
        mld.tables.retain(|t| !external.contains(&t.source));
        mld.tables.extend(assoc_tables);
        for t in &mut mld.tables {
            drop_lone_missing_keys(t);
            // constraints in the order of their columns (r117)
            let at = |fk: &ForeignKey| {
                fk.columns.first().and_then(|c| t.columns.iter().position(|x| &x.name == c)).unwrap_or(usize::MAX)
            };
            let mut fks = std::mem::take(&mut t.foreign_keys);
            fks.sort_by_key(|fk| at(fk));
            t.foreign_keys = fks;
        }
        // UNIQUE: keys of 1-1 associations, of inheritances, then
        // attributes (r002, r032, r040)
        for t in &mut mld.tables {
            let attribute = std::mem::take(&mut t.attribute_unique);
            t.unique.extend(attribute);
        }
        mld
    }

    /// Orders the first `n` (entity) tables by successive passes, marking
    /// those left over as circular.
    fn order_entity_tables(&mut self, n: usize) {
        let mut pending: Vec<Table> = self.tables.drain(..n).collect();
        let mut done: Vec<Table> = Vec::new();
        loop {
            let before = done.len();
            let mut i = 0;
            while i < pending.len() {
                let t = &pending[i];
                // a table not pending (done, or an external entity) is ready
                let ready = t
                    .foreign_keys
                    .iter()
                    .all(|fk| fk.parent == t.source || !pending.iter().any(|p| p.source == fk.parent));
                if ready {
                    done.push(pending.remove(i));
                } else {
                    i += 1;
                }
            }
            if done.len() == before {
                break;
            }
        }
        for mut t in pending {
            t.circular = true;
            done.push(t);
        }
        done.append(&mut self.tables);
        self.tables = done;
    }

    /// The table of a parent as its keys stand: from the previous pass when
    /// there is one, see [`Mld::from_mcd`].
    fn parent(&self, source: ObjId) -> Option<Table> {
        self.previous.iter().chain(&self.tables).find(|t| t.source == source).cloned()
    }

    fn table_index(&self, source: ObjId) -> Option<usize> {
        self.tables.iter().position(|t| t.source == source)
    }

    /// Inheritance: a subtype without identifier takes the supertype's key
    /// as its key (at the head, as a foreign key); one with its own key gets
    /// the supertype's key as a NOT NULL, UNIQUE foreign key. With
    /// "migration de toutes les rubriques", each subtype instead starts with
    /// all the supertype's columns and the supertype has no table.
    /// Subtypes take the key of each supertype: into their primary key,
    /// after the keys already migrated, when they have no identifier of
    /// their own (case c58), else as a UNIQUE foreign key.
    fn add_inheritance(&mut self, h: &Inheritance, mcd: &Mcd, keyed_subtypes: bool) {
        for &sup in &h.supertypes {
            let Some(parent) = self.parent(sup) else {
                continue;
            };
            let parent_key = parent.columns.iter().any(|c| c.primary && !is_placeholder(&c.name));
            for &sub in &h.subtypes {
                let own_key = mcd.entity(sub).is_some_and(|e| e.attributes.iter().any(|a| a.identifier));
                if own_key != keyed_subtypes {
                    continue;
                }
                let Some(ti) = self.table_index(sub) else {
                    continue;
                };
                let t = &mut self.tables[ti];
                // a supertype without identifier gives no key: the subtype keeps
                // its own missing key besides the migrated one (checked under Wine)
                if !own_key && parent_key {
                    t.columns.retain(|c| !is_placeholder(&c.name));
                }
                if own_key {
                    let cols = migrate(t, &parent, true, false, FkNaming::plain(&self.fk_mask), Actions::default());
                    t.unique.push(cols);
                } else {
                    migrate(t, &parent, true, true, FkNaming::plain(&self.fk_mask), Actions::default());
                }
            }
        }
    }

    /// Migrates keys for `a`; returns its own table when it needs one.
    /// `only`: migrate into that leg's entity only.
    fn add_association(&mut self, a: &Association, only: Option<usize>) -> Option<Table> {
        if a.legs.len() < 2 {
            self.warnings.push(format!("L'association {} doit posséder au moins 2 pattes", shown(&a.name)));
            // its line keeps its place among the association tables
            let mut t = Table::new(&a.name, a.id, &a.comment);
            for at in &a.attributes {
                let name = t.unique_name(&at.name);
                t.columns.push(Column { name, ..column(at, false) });
            }
            t.too_few_legs = true;
            return Some(t);
        }
        let one_legs: Vec<usize> = (0..a.legs.len()).filter(|&i| a.legs[i].cardinality.max == Max::One).collect();
        if one_legs.is_empty() {
            return Some(self.association_table(a));
        }
        // x,1 association: no table, its attributes are dropped. The child
        // is the x,1 side (1,1 then relative preferred, the last one on a
        // tie); two 1,1 sides each take the other's key.
        let strict = |i: usize| !a.legs[i].cardinality.min_zero && !a.legs[i].cardinality.relative;
        if a.forced_table && a.legs.len() == 2 {
            let child = *one_legs.iter().max_by_key(|&&i| !a.legs[i].cardinality.min_zero).unwrap();
            if a.legs[child].cardinality.min_zero {
                return Some(self.forced_table(a, child, one_legs.len() == 2));
            }
        }
        // with a relative identification on one side, that side first, so
        // that the other gets its whole key (r044)
        let both_11 = |i: usize| !a.legs[i].cardinality.min_zero;
        let children: Vec<usize> = if a.legs.len() == 2 && one_legs.len() == 2 && strict(0) && strict(1) {
            vec![0, 1]
        } else if a.legs.len() == 2 && one_legs.len() == 2 && both_11(0) && both_11(1) {
            if a.legs[1].cardinality.relative && !a.legs[0].cardinality.relative {
                vec![1, 0]
            } else {
                vec![0, 1]
            }
        } else {
            vec![*one_legs
                .iter()
                .max_by_key(|&&i| (a.legs[i].cardinality.relative, !a.legs[i].cardinality.min_zero))
                .unwrap()]
        };
        let mask = self.fk_mask.clone();
        let children: Vec<usize> = children.into_iter().filter(|&c| only.is_none_or(|o| o == c)).collect();
        for &child_leg in &children {
            let cl = &a.legs[child_leg];
            let ci = self.table_index(cl.entity)?;
            for (pi_leg, pl) in a.legs.iter().enumerate() {
                if pi_leg == child_leg {
                    continue;
                }
                let parent = self.parent(pl.entity)?;
                let child = &mut self.tables[ci];
                let cols = migrate(
                    child,
                    &parent,
                    !cl.cardinality.min_zero,
                    cl.cardinality.relative,
                    FkNaming::of_leg(&mask, pl),
                    pl.actions.clone(),
                );
                if a.legs.len() == 2 && one_legs.len() == 2 {
                    child.unique.push(cols);
                }
                // a weak entity without identifier: its key is incomplete
                // (r006), unless the parent's key is enough, through a 1-1
                // association (r096)
                if cl.cardinality.relative {
                    let placeholder = |c: &Column| c.primary && !c.foreign && c.name == MISSING_KEY;
                    if pl.cardinality.max == Max::One {
                        child.columns.retain(|c| !placeholder(c));
                    } else {
                        for c in child.columns.iter_mut().filter(|c| placeholder(c)) {
                            c.name = INCOMPLETE_KEY.into();
                        }
                    }
                }
            }
        }
        None
    }

    /// "Table de correspondance" forced on a binary association whose x,1
    /// side is `0,1`: keyed by that side's key, then the attributes, then
    /// the other key (NOT NULL, UNIQUE for a 0,1-0,1).
    fn forced_table(&self, a: &Association, child_leg: usize, one_one: bool) -> Table {
        let mut t = Table::new(&a.name, a.id, &a.comment);
        let (cl, pl) = (&a.legs[child_leg], &a.legs[1 - child_leg]);
        if let Some(child) = self.parent(cl.entity) {
            migrate(&mut t, &child, true, true, FkNaming::of_leg(&self.fk_mask, cl), cl.actions.clone());
        }
        for at in &a.attributes {
            let name = t.unique_name(&at.name);
            if at.unique {
                t.attribute_unique.push(vec![name.clone()]);
            }
            t.columns.push(Column { name, ..column(at, false) });
        }
        if let Some(parent) = self.parent(pl.entity) {
            let cols = migrate(&mut t, &parent, true, false, FkNaming::of_leg(&self.fk_mask, pl), pl.actions.clone());
            if one_one {
                t.unique.push(cols);
            }
        }
        t
    }

    /// Table of an association without x,1 leg: the migrated keys in leg
    /// order (stable by "place dans la clé primaire"), then its attributes.
    fn association_table(&self, a: &Association) -> Table {
        let mut t = Table::new(&a.name, a.id, &a.comment);
        // the attributes keep their names, a homonymous key gets the rank
        // (r032); the keys go before them all the same
        for at in &a.attributes {
            let name = t.unique_name(&at.name);
            if at.unique {
                t.attribute_unique.push(vec![name.clone()]);
            }
            t.columns.push(Column { name, ..column(at, false) });
        }
        let mut legs: Vec<&Leg> = a.legs.iter().collect();
        legs.sort_by_key(|l| (l.place, l.position));
        for l in legs {
            let Some(parent) = self.parent(l.entity) else {
                continue;
            };
            migrate(&mut t, &parent, true, true, FkNaming::of_leg(&self.fk_mask, l), l.actions.clone());
        }
        t
    }

    /// False when Looping refuses to generate SQL (missing keys, cycles).
    pub fn is_valid(&self) -> bool {
        self.tables.iter().all(|t| !t.circular && !t.too_few_legs && !t.columns.iter().any(|c| is_placeholder(&c.name)))
            && !self.notes.iter().any(|n| n.blocking)
    }

    /// Tables in creation order (the MLD order).
    pub fn creation_order(&self) -> Vec<&Table> {
        self.tables.iter().collect()
    }

    /// Looping's textual MLD: `Table = (key TYPE, col TYPE, #fk, #optional_fk*);`
    /// Types are only given for columns that are not foreign keys; a
    /// composite foreign key, or one referencing a foreign key, is grouped
    /// as the referenced key appears in its own table: `#(#Num_hotel,
    /// Num_chambre)`, `#(#Id_Personne)`. Names follow the naming
    /// convention of `o`.
    pub fn to_text(&self, o: &crate::sql::SqlOptions) -> String {
        let mut s = String::new();
        // each message is followed by the empty type slot of the line
        let slot = if o.mld_types { " " } else { "" };
        for n in &self.notes {
            let messages: Vec<String> = n.messages.iter().map(|m| format!("{m}{slot}")).collect();
            s.push_str(&format!("{} = ({});\n", n.name, messages.join(n.separator)));
        }
        for t in &self.tables {
            let cols: Vec<(&Column, &str)> = t.columns.iter().map(|c| (c, c.name.as_str())).collect();
            let cols = self.text_columns(t, &cols, o, o.mld_types, &[]);
            s.push_str(&format!("{} = ({});", o.name(&t.name), cols.join(", ")));
            if t.circular {
                s.push_str(" Attention ! Références circulaires");
            }
            if t.too_few_legs {
                s.push_str(TOO_FEW_LEGS);
            }
            s.push('\n');
        }
        s
    }

    /// Items of a textual MLD line for `cols` of `t`, each shown under the
    /// given name; `typed` adds the types of plain columns. A composite or
    /// chained foreign key is grouped following the referenced table's
    /// structure, under the names it has in `t`.
    fn text_columns(
        &self,
        t: &Table,
        cols: &[(&Column, &str)],
        o: &crate::sql::SqlOptions,
        typed: bool,
        // tables being expanded, by source (two entities may share a name,
        // r006): a key referencing one of them (an entity that is its own
        // supertype…) is not expanded again
        expanding: &[ObjId],
    ) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = expanding.to_vec();
        stack.push(t.source);
        // foreign key of each column, by position: several columns can share
        // a name (`***CLE_ABSENTE***` of entities without identifier)
        let mut owner: Vec<Option<usize>> = vec![None; cols.len()];
        for (k, fk) in t.foreign_keys.iter().enumerate() {
            for name in &fk.columns {
                if let Some(i) =
                    (0..cols.len()).find(|&i| owner[i].is_none() && cols[i].0.foreign && &cols[i].0.name == name)
                {
                    owner[i] = Some(k);
                }
            }
        }
        for (i, &(c, shown)) in cols.iter().enumerate() {
            let name = o.name(shown);
            if !c.foreign {
                out.push(if typed { format!("{name} {}", c.data_type.sql(o.dbms, false)) } else { name });
                continue;
            }
            let optional = if c.primary || c.not_null { "" } else { "*" };
            let fk = owner[i].map(|k| &t.foreign_keys[k]);
            let parent = fk.and_then(|fk| self.tables.iter().find(|p| p.source == fk.parent));
            let referenced_fk = |fk: &ForeignKey, parent: &Table| {
                fk.referenced.iter().any(|r| parent.columns.iter().any(|p| &p.name == r && p.primary && p.foreign))
            };
            match (fk, parent) {
                (Some(fk), Some(parent)) if fk.columns.len() > 1 || referenced_fk(fk, parent) => {
                    // the key is written once, at its first column
                    if owner[..i].contains(&owner[i]) {
                        continue;
                    }
                    // a key back to a table being expanded (a cycle): its own
                    // columns, without going deeper (r119); a reflexive one
                    // is expanded once (r072)
                    if stack.contains(&parent.source) && stack.len() > 1 {
                        let names: Vec<String> = (0..cols.len())
                            .filter(|&j| owner[j] == owner[i])
                            .enumerate()
                            .map(|(n, j)| {
                                let chained = fk.referenced.get(n).is_some_and(|r| {
                                    parent.columns.iter().any(|p| &p.name == r && p.primary && p.foreign)
                                });
                                format!("{}{}", if chained { "#" } else { "" }, o.name(cols[j].1))
                            })
                            .collect();
                        out.push(format!("#({}){optional}", names.join(", ")));
                        continue;
                    }
                    // the referenced columns, shown as this key's columns
                    // matched by position too (homonymous columns)
                    let mut used: Vec<usize> = Vec::new();
                    let mine: Vec<usize> = (0..cols.len()).filter(|&j| owner[j] == owner[i]).collect();
                    let referenced: Vec<(&Column, &str)> = fk
                        .referenced
                        .iter()
                        .enumerate()
                        .filter_map(|(n, r)| {
                            let k = (0..parent.columns.len())
                                .find(|k| {
                                    !used.contains(k) && parent.columns[*k].primary && &parent.columns[*k].name == r
                                })
                                .or_else(|| {
                                    (0..parent.columns.len())
                                        .find(|k| !used.contains(k) && &parent.columns[*k].name == r)
                                })?;
                            used.push(k);
                            let shown = mine.get(n).map_or(fk.columns[n].as_str(), |&j| cols[j].1);
                            Some((&parent.columns[k], shown))
                        })
                        .collect();
                    let inner = self.text_columns(parent, &referenced, o, false, &stack);
                    out.push(format!("#({}){optional}", inner.join(", ")));
                }
                _ => out.push(format!("#{name}{optional}")),
            }
        }
        out
    }
}
