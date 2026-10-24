//! MCD -> MLD (relational schema), following the output of Looping 4.1
//! (checked against `samples/ref/`, see `tests/reference.rs`):
//!
//! * every non-fictive entity becomes a table, identifiers first; an entity
//!   without identifier gets a `***CLE_ABSENTE***` placeholder key;
//! * an association with a `x,1` leg has no table and drops its attributes:
//!   the keys of the other legs migrate into that entity (the `1,1` leg,
//!   else the last `x,1` one), after its attributes and in reverse
//!   association order. NOT NULL when `1,1`, optional `*` when `0,1`,
//!   UNIQUE for a binary 1-1; two `1,1` sides each take the other's key
//!   (hence circular references). A relative identification "(R)" puts the
//!   key at the head of the child's key. A parent without key migrates
//!   nothing but still orders the tables;
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
            comment: comment.to_vec(),
        }
    }

    pub fn primary_key(&self) -> Vec<&Column> {
        self.columns.iter().filter(|c| c.primary).collect()
    }

    fn unique_name(&self, base: &str) -> String {
        if base == MISSING_KEY || !self.columns.iter().any(|c| c.name == base) {
            return base.to_string();
        }
        (1..).map(|i| format!("{base}{RANK_SEP}{i}")).find(|n| !self.columns.iter().any(|c| &c.name == n)).unwrap()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Mld {
    pub tables: Vec<Table>,
    pub warnings: Vec<String>,
    /// Lines Looping puts first in the textual MLD, `Name = (message );`
    /// (CIF and constraint diagnostics); they do not prevent the script.
    pub notes: Vec<(String, String)>,
    /// SQL of rules, inheritances and constraints, in output order.
    pub scripts: Vec<SqlScript>,
    /// "Compléments script LDD", see [`Mcd::complements`].
    pub complements: [String; 4],
    /// Names of the inter-association constraints, in document order.
    pub constraints: Vec<String>,
    /// "Nom clé étrangère" mask used while building.
    #[serde(skip)]
    pub(crate) fk_mask: String,
}

const CIF_ASSOCIATION: &str = "Une CIF doit être rattachée à une association reliant plus de 2 entités, \
et toutes les cardinalités impliquées doivent être multiples";
const CIF_TARGET: &str = "Une CIF nécessite une entité cible";
const CONSTRAINT_ASSOCIATIONS: &str = "Une contrainte inter-associations concerne au moins deux associations";
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
    for a in e.attributes.iter().filter(|a| a.identifier) {
        t.columns.push(column(a, true));
    }
    for a in e.attributes.iter().filter(|a| !a.identifier) {
        t.columns.push(column(a, false));
        if a.unique {
            t.unique.push(vec![a.name.clone()]);
        }
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
    // the target entities' foreign keys, by referenced table name
    let targets: Vec<&str> = a
        .legs
        .iter()
        .enumerate()
        .filter(|(_, l)| cif.targets.contains(&l.entity))
        .filter_map(|(i, _)| t.foreign_keys.get(i).map(|fk| fk.table.as_str()))
        .collect();
    let moved: Vec<String> = t
        .foreign_keys
        .iter()
        .filter(|fk| targets.contains(&fk.table.as_str()))
        .flat_map(|fk| fk.columns.clone())
        .collect();
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
        if c.name == MISSING_KEY {
            return c.name.clone();
        }
        if self.rename && !self.role.is_empty() {
            return self.role.to_string();
        }
        let base = if c.foreign {
            c.name.clone()
        } else {
            let mask = if self.mask.trim_matches(' ').is_empty() { "[FK]" } else { self.mask };
            mask.replace("[Classe]", &parent.name).replace("[FK]", &c.name)
        };
        if self.suffix && !self.role.is_empty() {
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
            referenced: pk.iter().map(|c| c.name.clone()).collect(),
            actions,
        });
    }
    names
}

impl Mld {
    pub fn from_mcd(mcd: &Mcd) -> Mld {
        let mut mld = Mld { fk_mask: mcd.fk_mask.clone(), ..Mld::default() };
        let subtypes: Vec<ObjId> = mcd.inheritances.iter().flat_map(|h| h.subtypes.clone()).collect();
        for e in mcd.entities.iter().filter(|e| !e.fictive) {
            if !e.attributes.iter().any(|a| a.identifier) && !subtypes.contains(&e.id) {
                mld.warnings.push(format!("L'entité {} n'a pas d'identifiant", shown(&e.name)));
            }
            mld.tables.push(entity_table(e));
        }
        let entity_tables = mld.tables.len();
        let mut external: Vec<ObjId> = mcd.entities.iter().filter(|e| e.external).map(|e| e.id).collect();
        // supertypes before their subtypes
        let mut pending: Vec<&Inheritance> = mcd.inheritances.iter().collect();
        while !pending.is_empty() {
            let ready = pending
                .iter()
                .position(|h| !pending.iter().any(|g| h.supertype.is_some_and(|s| g.subtypes.contains(&s))))
                .unwrap_or(0);
            let h = pending.remove(ready);
            if h.migrate_all {
                external.extend(h.supertype);
            }
            mld.add_inheritance(h);
        }
        // Relative identifications first, in document order but parents
        // before children so that weak entities have their full key before
        // other tables reference them; then reverse document order, which
        // gives Looping's order of migrated keys.
        let (mut relative, others): (Vec<&Association>, Vec<&Association>) =
            mcd.associations.iter().rev().partition(|a| a.legs.iter().any(|l| l.cardinality.relative));
        relative.reverse();
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
        order.extend(others);
        let mut assoc_tables = Vec::new();
        for a in order {
            if let Some(mut t) = mld.add_association(a) {
                for cif in mcd.cifs.iter().filter(|c| c.associations == [a.id]) {
                    apply_cif(&mut t, a, cif);
                }
                assoc_tables.push(t);
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
                mld.notes.push((cif.name.clone(), n.to_string()));
            }
        }
        for c in &mcd.constraints {
            let mut msgs = Vec::new();
            if c.associations < 2 {
                msgs.push(CONSTRAINT_ASSOCIATIONS);
            }
            if c.entities == 0 {
                msgs.push(CONSTRAINT_PIVOT);
            }
            if !msgs.is_empty() {
                mld.notes.push((c.name.clone(), msgs.join("  / ")));
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
                let ready =
                    t.foreign_keys.iter().all(|fk| fk.table == t.name || !pending.iter().any(|p| p.name == fk.table));
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

    fn table_index(&self, source: ObjId) -> Option<usize> {
        self.tables.iter().position(|t| t.source == source)
    }

    /// Inheritance: a subtype without identifier takes the supertype's key
    /// as its key (at the head, as a foreign key); one with its own key gets
    /// the supertype's key as a NOT NULL, UNIQUE foreign key. With
    /// "migration de toutes les rubriques", each subtype instead starts with
    /// all the supertype's columns and the supertype has no table.
    fn add_inheritance(&mut self, h: &Inheritance) {
        let Some(si) = h.supertype.and_then(|s| self.table_index(s)) else {
            return;
        };
        let parent = self.tables[si].clone();
        for &sub in &h.subtypes {
            let Some(ti) = self.table_index(sub) else {
                continue;
            };
            let t = &mut self.tables[ti];
            let own_key = t.columns.iter().any(|c| c.primary && c.name != MISSING_KEY);
            // a supertype without identifier gives no key: the subtype keeps
            // its own missing key besides the migrated one (checked under Wine)
            let parent_key = parent.columns.iter().any(|c| c.primary && c.name != MISSING_KEY);
            if h.migrate_all || (!own_key && parent_key) {
                t.columns.retain(|c| c.name != MISSING_KEY);
            }
            if h.migrate_all {
                let mut cols = parent.columns.clone();
                cols.append(&mut t.columns);
                t.columns = cols;
                let mut unique = parent.unique.clone();
                unique.append(&mut t.unique);
                t.unique = unique;
            } else if own_key {
                let cols = migrate(t, &parent, true, false, FkNaming::plain(&self.fk_mask), Actions::default());
                t.unique.push(cols);
            } else {
                migrate(t, &parent, true, true, FkNaming::plain(&self.fk_mask), Actions::default());
            }
        }
    }

    /// Migrates keys for `a`; returns its own table when it needs one.
    fn add_association(&mut self, a: &Association) -> Option<Table> {
        if a.legs.len() < 2 {
            self.warnings.push(format!("L'association {} doit posséder au moins 2 pattes", shown(&a.name)));
            return None;
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
        let children: Vec<usize> = if a.legs.len() == 2 && one_legs.len() == 2 && strict(0) && strict(1) {
            vec![0, 1]
        } else {
            vec![*one_legs
                .iter()
                .max_by_key(|&&i| (a.legs[i].cardinality.relative, !a.legs[i].cardinality.min_zero))
                .unwrap()]
        };
        let mask = self.fk_mask.clone();
        for &child_leg in &children {
            let cl = &a.legs[child_leg];
            let ci = self.table_index(cl.entity)?;
            for (pi_leg, pl) in a.legs.iter().enumerate() {
                if pi_leg == child_leg {
                    continue;
                }
                let parent = self.tables[self.table_index(pl.entity)?].clone();
                let child = &mut self.tables[ci];
                if parent.columns.iter().any(|c| c.name == MISSING_KEY) {
                    // Looping migrates nothing into an entity from a parent
                    // without key, but the dependency still orders the tables
                    child.foreign_keys.push(ForeignKey {
                        columns: Vec::new(),
                        table: parent.name.clone(),
                        referenced: Vec::new(),
                        actions: pl.actions.clone(),
                    });
                    continue;
                }
                let cols = migrate(
                    child,
                    &parent,
                    !cl.cardinality.min_zero,
                    cl.cardinality.relative,
                    FkNaming::of_leg(&mask, pl),
                    pl.actions.clone(),
                );
                if a.legs.len() == 2 && one_legs.len() == 2 && !cl.cardinality.relative {
                    child.unique.push(cols);
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
        if let Some(ci) = self.table_index(cl.entity) {
            migrate(&mut t, &self.tables[ci], true, true, FkNaming::of_leg(&self.fk_mask, cl), cl.actions.clone());
        }
        for at in &a.attributes {
            let name = t.unique_name(&at.name);
            t.columns.push(Column { name, ..column(at, false) });
        }
        if let Some(pi) = self.table_index(pl.entity) {
            let cols =
                migrate(&mut t, &self.tables[pi], true, false, FkNaming::of_leg(&self.fk_mask, pl), pl.actions.clone());
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
        let mut legs: Vec<&Leg> = a.legs.iter().collect();
        legs.sort_by_key(|l| l.place);
        for l in legs {
            let Some(pi) = self.table_index(l.entity) else {
                continue;
            };
            migrate(&mut t, &self.tables[pi], true, true, FkNaming::of_leg(&self.fk_mask, l), l.actions.clone());
        }
        for at in &a.attributes {
            let name = t.unique_name(&at.name);
            t.columns.push(Column { name, ..column(at, false) });
        }
        t
    }

    /// False when Looping refuses to generate SQL (missing keys, cycles).
    pub fn is_valid(&self) -> bool {
        self.tables.iter().all(|t| !t.circular && !t.columns.iter().any(|c| c.name == MISSING_KEY))
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
        for (name, note) in &self.notes {
            s.push_str(&format!("{name} = ({note} );\n"));
        }
        for t in &self.tables {
            let cols: Vec<(&Column, &str)> = t.columns.iter().map(|c| (c, c.name.as_str())).collect();
            let cols = self.text_columns(t, &cols, o, o.mld_types, &[]);
            s.push_str(&format!("{} = ({});", o.name(&t.name), cols.join(", ")));
            if t.circular {
                s.push_str(" Attention ! Références circulaires");
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
        // tables being expanded: a key referencing one of them (an entity
        // that is its own supertype…) is not expanded again
        expanding: &[&str],
    ) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = expanding.to_vec();
        stack.push(&t.name);
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
            let parent = fk.and_then(|fk| self.tables.iter().find(|p| p.name == fk.table));
            let referenced_fk = |fk: &ForeignKey, parent: &Table| {
                fk.referenced.iter().any(|r| parent.columns.iter().any(|p| &p.name == r && p.foreign))
            };
            match (fk, parent) {
                (Some(fk), Some(parent))
                    if (fk.columns.len() > 1 || referenced_fk(fk, parent))
                        && !stack.contains(&parent.name.as_str()) =>
                {
                    // the key is written once, at its first column
                    if owner[..i].contains(&owner[i]) {
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
