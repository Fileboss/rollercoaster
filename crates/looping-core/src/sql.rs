//! SQL DDL in the layout Looping produces (0x140066620 / 0x14010bb90):
//!
//! ```sql
//! CREATE TABLE Chien(
//!    Id_Chien COUNTER,
//!    Id_Race INT NOT NULL,
//!    PRIMARY KEY(Id_Chien),
//!    FOREIGN KEY(Id_Race) REFERENCES Race(Id_Race)
//! );
//! ```

use loo_format::Settings;
use serde::Serialize;

use crate::mld::{is_placeholder, ForeignKey, Mld, Table, LOGICAL, RANK_SEP};
use crate::naming::{avoid_reserved, convert, Convention};
use crate::types::Dbms;

/// Constraint naming (`+0x7f4`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ConstraintNames {
    None,
    /// `PK_` / `FK_` / `AK_`
    Prefix,
    /// `_PK` / `_FK` / `_AK`
    Suffix,
}

/// Where constraints go (`+0xb40`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Placement {
    CreateTable,
    AlterTable,
    ForeignKeysInAlter,
}

/// Referential actions (`+0x888`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Cascade {
    None,
    NotValidated,
    Delete,
    Update,
    DeleteUpdate,
}

/// Comment generation (`+0x7f8`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Comments {
    None,
    Block,
    Line,
    CommentOn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SqlOptions {
    pub dbms: Dbms,
    pub convention: Convention,
    /// `+0xab8`
    pub remove_accents: bool,
    pub constraint_names: ConstraintNames,
    pub placement: Placement,
    pub cascade: Cascade,
    pub comments: Comments,
    /// "Types dans MLD textuel" (`+0x7f0`).
    pub mld_types: bool,
}

impl Default for SqlOptions {
    fn default() -> Self {
        SqlOptions {
            dbms: Dbms::MySql,
            convention: Convention::Standard,
            remove_accents: false,
            constraint_names: ConstraintNames::None,
            placement: Placement::CreateTable,
            cascade: Cascade::None,
            comments: Comments::None,
            mld_types: true,
        }
    }
}

impl SqlOptions {
    /// Reads the options stored in the document settings block.
    pub fn from_settings(s: &Settings) -> Self {
        SqlOptions {
            dbms: Dbms::from_index(s.i32(0x52c)),
            convention: Convention::from_index(s.i32(0xa78)),
            remove_accents: s.i32(0xab8) != 0,
            constraint_names: match s.i32(0x7f4) {
                1 => ConstraintNames::Prefix,
                2 => ConstraintNames::Suffix,
                _ => ConstraintNames::None,
            },
            placement: match s.i32(0xb40) {
                1 => Placement::AlterTable,
                2 => Placement::ForeignKeysInAlter,
                _ => Placement::CreateTable,
            },
            cascade: match s.i32(0x888) {
                1 => Cascade::NotValidated,
                2 => Cascade::Delete,
                3 => Cascade::Update,
                4 => Cascade::DeleteUpdate,
                _ => Cascade::None,
            },
            comments: match s.i32(0x7f8) {
                1 => Comments::Block,
                2 => Comments::Line,
                3 => Comments::CommentOn,
                _ => Comments::None,
            },
            mld_types: s.i32(0x7f0) != 0,
        }
    }

    pub fn name(&self, conceptual: &str) -> String {
        physical(conceptual, self.convention, self.remove_accents)
    }

    /// `CONSTRAINT FK_table_referenced ` (or `table_referenced_FK`), made
    /// unique over the script with `_1`, `_2`… like column names.
    fn constraint(&self, kind: &str, parts: &[&str], rank: Option<String>) -> String {
        self.constraint_name(kind, parts, rank).map(|n| format!("CONSTRAINT {n} ")).unwrap_or_default()
    }

    /// `PK_table` / `AK_table[_rank]` / `FK_table_parent[_rank]` (or with
    /// the kind as suffix); the rank goes on the base: FK_t_r_1 / t_r_1_FK.
    /// Looping never ranks PK names, even of homonymous tables (case c76).
    fn constraint_name(&self, kind: &str, parts: &[&str], rank: Option<String>) -> Option<String> {
        if self.constraint_names == ConstraintNames::None {
            return None;
        }
        let mut base = parts.iter().map(|p| self.name(p)).collect::<Vec<_>>().join("_");
        if let Some(r) = rank {
            base = format!("{base}_{r}");
        }
        Some(match self.constraint_names {
            ConstraintNames::Suffix => format!("{base}_{kind}"),
            _ => format!("{kind}_{base}"),
        })
    }

    /// Referential actions of `fk`, generated when `+0x888` is set
    /// (whatever its value, as far as `samples/ref/c17_*` shows).
    fn actions(&self, fk: &ForeignKey) -> String {
        if self.cascade == Cascade::None {
            return String::new();
        }
        let a = &fk.actions;
        let mut s = String::new();
        if a.on_delete_cascade {
            s.push_str(" ON DELETE CASCADE");
        }
        if a.on_update_cascade {
            s.push_str(" ON UPDATE CASCADE");
        }
        if let Some(other) = &a.other {
            s.push(' ');
            s.push_str(other);
        }
        s
    }
}

/// Identifier as emitted with the default (Standard) convention.
pub fn ident(name: &str) -> String {
    physical(name, Convention::Standard, false)
}

fn physical(name: &str, convention: Convention, remove_accents: bool) -> String {
    if is_placeholder(name) {
        return name.to_string();
    }
    // the last rank: a column of a migrated key may already have one
    match name.rsplit_once(RANK_SEP) {
        Some((base, rank)) => format!("{}_{rank}", physical(base, convention, remove_accents)),
        None if name.contains(LOGICAL) => crate::mld::shown(name),
        None => avoid_reserved(convert(name, convention, remove_accents)),
    }
}

/// What Looping shows instead of the script when the MLD has errors.
pub const INVALID_MLD: &str = "Impossible de générer le script SQL : \nvoir les erreurs dans le MLD textuel";

fn quote(s: &str) -> String {
    s.replace('\'', "''")
}

struct Clauses {
    pk: Option<String>,
    unique: Vec<String>,
    fks: Vec<String>,
}

/// The n-th UNIQUE of a table is ranked n (case c77).
fn unique_rank(n: usize) -> Option<String> {
    (n > 0).then(|| n.to_string())
}

/// A foreign key takes the rank of its columns: `Id_Client_1` gives
/// `FK_Commande_Client_1`, a reflexive `Id_Employe_1` `FK_Employe_Employe_1`
/// (cases c74, c75).
fn foreign_key_rank(fk: &ForeignKey) -> Option<String> {
    // the first ranked column: (Prix6, Âge_1) gives _1 (r095)
    fk.columns.iter().find_map(|c| c.rsplit_once(RANK_SEP)).map(|(_, r)| r.to_string())
}

/// The end of a foreign key's constraint name: the role as typed, when the
/// key's names use it (c94, c95, r138), then the rank.
fn fk_suffix(fk: &ForeignKey) -> Option<String> {
    let parts: Vec<String> =
        [Some(fk.role.clone()).filter(|r| !r.is_empty()), foreign_key_rank(fk)].into_iter().flatten().collect();
    (!parts.is_empty()).then(|| parts.join("_"))
}

fn clauses(t: &Table, o: &SqlOptions) -> Clauses {
    let pk: Vec<String> = t.primary_key().iter().map(|c| o.name(&c.name)).collect();
    let pk =
        (!pk.is_empty()).then(|| format!("{}PRIMARY KEY({})", o.constraint("PK", &[&t.name], None), pk.join(", ")));
    let unique = t
        .unique
        .iter()
        .enumerate()
        .map(|(i, u)| {
            format!(
                "{}UNIQUE({})",
                o.constraint("AK", &[&t.name], unique_rank(i)),
                u.iter().map(|c| o.name(c)).collect::<Vec<_>>().join(", ")
            )
        })
        .collect();
    let fks = t
        .foreign_keys
        .iter()
        .map(|fk| {
            format!(
                "{}FOREIGN KEY({}) REFERENCES {}({}){}",
                o.constraint("FK", &[&t.name, &fk.table], fk_suffix(fk)),
                fk.columns.iter().map(|c| o.name(c)).collect::<Vec<_>>().join(", "),
                o.name(&fk.table),
                fk.referenced.iter().map(|c| o.name(c)).collect::<Vec<_>>().join(", "),
                o.actions(fk)
            )
        })
        .collect();
    Clauses { pk, unique, fks }
}

pub fn generate(mld: &Mld, dbms: Dbms) -> String {
    generate_with(mld, &SqlOptions { dbms, ..Default::default() })
}

pub fn generate_with(mld: &Mld, o: &SqlOptions) -> String {
    if !mld.is_valid() {
        return INVALID_MLD.to_string();
    }
    script(mld, o, false)
}

/// The script, or with `vba` the body of the Access VBA module: each
/// `CREATE TABLE` / `ALTER TABLE` becomes a `DoCmd.RunSQL` of quoted lines,
/// everything else (comments, complements, scripts) stays as is.
fn script(mld: &Mld, o: &SqlOptions, vba: bool) -> String {
    let mut out = String::new();
    let [begin, end, table_begin, table_end] = &mld.complements;
    // before the tables: script start, then the rules placed before them
    let before = script_blocks(mld, o, true);
    if !begin.is_empty() {
        out.push_str(&format!("{begin}\n"));
    }
    for b in &before {
        out.push_str(&format!("\n{b}\n"));
    }
    if !before.is_empty() {
        out.push('\n');
    }
    if vba && !table_end.is_empty() {
        // the VBA generator also closes each inter-association constraint
        // (a stray "table end" line named after it)
        for c in &mld.constraints {
            out.push_str(&format!("{}\n", table_end.replace("[Nom_Table]", c)));
        }
    }
    // one ALTER TABLE per table, its clauses separated by commas
    let mut alters: Vec<(String, Vec<String>)> = Vec::new();
    let mut tables = Vec::new();
    for t in mld.creation_order() {
        let mut block = String::new();
        let tname = o.name(&t.name);
        block.push_str(&table_comment(&t.comment, o.comments));
        if !table_begin.is_empty() {
            block.push_str(&format!("{}\n", table_begin.replace("[Nom_Table]", &tname)));
        }
        // (column, comment) pairs; the comment goes after the comma
        let mut lines: Vec<(String, String)> = Vec::new();
        for c in &t.columns {
            let mut l = o.name(&c.name);
            let ty = c.data_type.sql(o.dbms, c.foreign);
            if !ty.is_empty() {
                l.push(' ');
                l.push_str(&ty);
            }
            if c.not_null && !c.primary {
                l.push_str(" NOT NULL");
            }
            if !c.complement.is_empty() {
                l.push(' ');
                l.push_str(&c.complement);
            }
            let comment = match o.comments {
                _ if c.comment.is_empty() => String::new(),
                Comments::Block => format!(" /* {}*/", c.comment),
                Comments::Line => format!(" --{}", c.comment),
                _ => String::new(),
            };
            lines.push((l, comment));
        }
        let cl = clauses(t, o);
        let (inline, deferred): (Vec<String>, Vec<String>) = match o.placement {
            Placement::CreateTable => (cl.pk.into_iter().chain(cl.unique).chain(cl.fks).collect(), Vec::new()),
            Placement::ForeignKeysInAlter => (cl.pk.into_iter().chain(cl.unique).collect(), cl.fks),
            Placement::AlterTable => (Vec::new(), cl.pk.into_iter().chain(cl.unique).chain(cl.fks).collect()),
        };
        lines.extend(inline.into_iter().map(|c| (c, String::new())));
        let n = lines.len();
        let comma = |i: usize| if i + 1 < n { "," } else { "" };
        if vba {
            block.push_str(&format!("DoCmd.RunSQL \"CREATE TABLE {tname}(\" & _\n"));
            for (i, (l, comment)) in lines.iter().enumerate() {
                block.push_str(&format!("   \"{l}{}\" & _{comment}\n", comma(i)));
            }
            block.push_str("\");\"   \n");
            if !table_end.is_empty() {
                block.push_str(&format!("{}\n", table_end.replace("[Nom_Table]", &tname)));
            }
        } else {
            let body: Vec<String> =
                lines.iter().enumerate().map(|(i, (l, comment))| format!("   {l}{}{comment}", comma(i))).collect();
            let close = if table_end.is_empty() {
                String::new()
            } else {
                format!("\n{}\n", table_end.replace("[Nom_Table]", &tname))
            };
            block.push_str(&format!("CREATE TABLE {tname}(\n{}\n){close};\n", body.join("\n")));
        }
        if o.comments == Comments::CommentOn {
            let text: Vec<&str> = t.comment.iter().filter(|l| !l.trim().is_empty()).map(String::as_str).collect();
            if !text.is_empty() {
                block.push_str(&format!("COMMENT ON TABLE {tname} IS '{}';\n", quote(&text.join(" "))));
            }
            for c in t.columns.iter().filter(|c| !c.comment.is_empty()) {
                block.push_str(&format!("COMMENT ON COLUMN {tname}.{} IS '{}';\n", o.name(&c.name), quote(&c.comment)));
            }
        }
        tables.push(block);
        if !deferred.is_empty() {
            alters.push((tname.clone(), deferred));
        }
    }
    let alters: Vec<String> = alters
        .into_iter()
        .map(|(t, clauses)| {
            let n = clauses.len();
            let comma = |i: usize| if i + 1 < n { "," } else { "" };
            if vba {
                let adds: Vec<String> =
                    clauses.iter().enumerate().map(|(i, c)| format!("   \"ADD {c}{}\" & _", comma(i))).collect();
                // the statement end follows the last continuation directly
                format!("DoCmd.RunSQL \"ALTER TABLE {t}\" & _\n{}\";\"   \n", adds.join("\n"))
            } else {
                let adds: Vec<String> =
                    clauses.iter().enumerate().map(|(i, c)| format!("   ADD {c}{}", comma(i))).collect();
                format!("ALTER TABLE {t}\n{};\n", adds.join("\n"))
            }
        })
        .collect();
    // tables separated by a blank line; the ALTER TABLEs come last
    out.push_str(&tables.join("\n"));
    // after the tables: other scripts, then script end
    let after = script_blocks(mld, o, false);
    if !after.is_empty() {
        out.push_str(&format!("\n{}\n", after.join("\n\n")));
    }
    if !end.is_empty() {
        out.push_str(&format!("{end}\n"));
    }
    for a in &alters {
        out.push_str(&format!("\n{a}"));
    }
    out
}

/// A rule, inheritance or constraint script: its comment (rule
/// description) as `--` lines, then the SQL lines. When comments are
/// generated (`+0x7f8`) the comment is never omitted: `/*  */` or `--`.
fn script_block(s: &crate::mcd::SqlScript, mode: Comments) -> String {
    let comment: Vec<String> = match mode {
        Comments::Block => vec![format!("/* {} */", s.comment.join("\n"))],
        // "--" comments: a bare "--" when there is none
        Comments::Line if s.comment.is_empty() => vec!["--".to_string()],
        _ => s.comment.iter().map(|l| format!("--{l}")).collect(),
    };
    comment.into_iter().chain(s.lines.iter().cloned()).collect::<Vec<_>>().join("\n")
}

/// Scripts with SQL text, before or after the tables.
fn script_blocks(mld: &Mld, o: &SqlOptions, before_tables: bool) -> Vec<String> {
    mld.scripts
        .iter()
        .filter(|s| s.before_tables == before_tables && s.lines.iter().any(|l| !l.is_empty()))
        .map(|s| script_block(s, o.comments))
        .collect()
}

/// Comment put before `CREATE TABLE` (0x1400c7d40): the owner's comment
/// lines, blank ones skipped, as `/* a\n     b */` or `--a\n--b`.
fn table_comment(lines: &[String], mode: Comments) -> String {
    let lines: Vec<&String> = lines.iter().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return String::new();
    }
    match mode {
        Comments::Block => {
            let body: Vec<String> = lines
                .iter()
                .enumerate()
                .map(|(i, l)| if i == 0 { format!("/* {l}") } else { format!("     {l}") })
                .collect();
            format!("{} */\n", body.join("\n"))
        }
        Comments::Line => lines.iter().map(|l| format!("--{l}\n")).collect(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mld::Column;
    use crate::types::DataType;

    fn col(name: &str, primary: bool, foreign: bool) -> Column {
        Column {
            name: name.into(),
            data_type: DataType::Integer { bits: 32 },
            primary,
            foreign,
            not_null: foreign,
            unique: false,
            comment: String::new(),
            complement: String::new(),
        }
    }

    fn mld() -> Mld {
        Mld {
            tables: vec![
                Table {
                    name: "Race".into(),
                    source: 1,
                    columns: vec![col("Id Race", true, false)],
                    foreign_keys: vec![],
                    unique: vec![],
                    circular: false,
                    too_few_legs: false,
                    attribute_unique: vec![],
                    comment: vec![],
                },
                Table {
                    name: "Chien".into(),
                    source: 2,
                    columns: vec![col("Id Chien", true, false), col("Id Race", false, true)],
                    foreign_keys: vec![ForeignKey {
                        columns: vec!["Id Race".into()],
                        table: "Race".into(),
                        parent: 1,
                        role: String::new(),
                        referenced: vec!["Id Race".into()],
                        actions: crate::mcd::Actions { on_delete_cascade: true, ..Default::default() },
                    }],
                    unique: vec![],
                    circular: false,
                    too_few_legs: false,
                    attribute_unique: vec![],
                    comment: vec![],
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn default_layout() {
        let s = generate(&mld(), Dbms::MySql);
        assert_eq!(
            s,
            "CREATE TABLE Race(\n   Id_Race INT,\n   PRIMARY KEY(Id_Race)\n);\n\n\
             CREATE TABLE Chien(\n   Id_Chien INT,\n   Id_Race INT NOT NULL,\n   PRIMARY KEY(Id_Chien),\n   \
             FOREIGN KEY(Id_Race) REFERENCES Race(Id_Race)\n);\n"
        );
    }

    #[test]
    fn options() {
        let o = SqlOptions {
            convention: Convention::Snake,
            constraint_names: ConstraintNames::Prefix,
            placement: Placement::ForeignKeysInAlter,
            cascade: Cascade::Delete,
            ..Default::default()
        };
        let s = generate_with(&mld(), &o);
        assert!(s.contains("CONSTRAINT PK_chien PRIMARY KEY(id_chien)"), "{s}");
        assert!(s.contains("ALTER TABLE chien\n   ADD CONSTRAINT FK_chien_race FOREIGN KEY(id_race) REFERENCES race(id_race) ON DELETE CASCADE;"), "{s}");
    }
}

/// Looping's message when the MLD has errors: no export file.
pub const EXPORT_REFUSED: &str = "Erreur dans le MLD : impossible d'exporter le MLD textuel";

/// Non-ASCII characters as numeric entities (`&#233;`), as Looping does.
fn xml_text(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii() { c.to_string() } else { format!("&#{};", c as u32) }).collect()
}

/// "MLD textuel au format XML..." (written with CRLF line ends, see
/// [`to_latin1_crlf`]); `database` is the document title. Refused like
/// Looping when the MLD has errors.
pub fn xml(mld: &Mld, o: &SqlOptions, database: &str) -> Result<String, &'static str> {
    if !mld.is_valid() {
        return Err(EXPORT_REFUSED);
    }
    let mut out =
        format!("<?xml version=\"1.0\" encoding=\"iso-8859-1\"?>\n\n<Database>\n   <Name>{database}</Name>\n");
    let names = |cols: &[String]| cols.iter().map(|c| o.name(c)).collect::<Vec<_>>().join(", ");
    for t in mld.creation_order() {
        out.push_str(&format!("   <Table>\n      <Name>{}</Name>\n", o.name(&t.name)));
        for line in t.comment.iter().filter(|l| !l.trim().is_empty()) {
            out.push_str(&format!("      <Comment>{line}</Comment>\n"));
        }
        for c in &t.columns {
            out.push_str(&format!("      <Column>\n         <Name>{}</Name>\n", o.name(&c.name)));
            let ty = c.data_type.sql(o.dbms, c.foreign);
            if !ty.is_empty() {
                out.push_str(&format!("         <Type>{ty}</Type>\n"));
            }
            // NOT NULL, UNIQUE and the SQL complement in one property
            let mut property = Vec::new();
            if c.not_null && !c.primary {
                property.push("NOT NULL");
            }
            if c.unique {
                property.push("UNIQUE");
            }
            if !c.complement.is_empty() {
                property.push(&c.complement);
            }
            if !property.is_empty() {
                out.push_str(&format!("         <Property>{}</Property>\n", property.join(" ")));
            }
            if !c.comment.is_empty() {
                out.push_str(&format!("         <Comment>{}</Comment>\n", c.comment));
            }
            out.push_str("      </Column>\n");
        }
        let mut block = |tag: &str, name: Option<String>, body: String| {
            out.push_str(&format!("      <{tag}>\n"));
            if let Some(n) = name {
                out.push_str(&format!("         <Name>{n}</Name>\n"));
            }
            out.push_str(&body);
            out.push_str(&format!("      </{tag}>\n"));
        };
        let pk: Vec<String> = t.primary_key().iter().map(|c| c.name.clone()).collect();
        if !pk.is_empty() {
            let n = o.constraint_name("PK", &[&t.name], None);
            block("PK", n, format!("         <Key>{}</Key>\n", names(&pk)));
        }
        for (i, u) in t.unique.iter().enumerate() {
            let n = o.constraint_name("AK", &[&t.name], unique_rank(i));
            block("AK", n, format!("         <Key>{}</Key>\n", names(u)));
        }
        for fk in &t.foreign_keys {
            let n = o.constraint_name("FK", &[&t.name, &fk.table], fk_suffix(fk));
            let mut body = format!(
                "         <Key>{}</Key>\n         <Reference>{}({})</Reference>\n",
                names(&fk.columns),
                o.name(&fk.table),
                names(&fk.referenced)
            );
            // one clause per referential action, as in the script
            if o.cascade != Cascade::None {
                let a = &fk.actions;
                let clauses = [
                    a.on_delete_cascade.then(|| "ON DELETE CASCADE".to_string()),
                    a.on_update_cascade.then(|| "ON UPDATE CASCADE".to_string()),
                    a.other.clone(),
                ];
                for c in clauses.into_iter().flatten() {
                    body.push_str(&format!("         <Clause>{c}</Clause>\n"));
                }
            }
            block("FK", n, body);
        }
        out.push_str("   </Table>\n");
    }
    out.push_str("</Database>\n");
    Ok(xml_text(&out))
}

/// Text as Looping writes its export files: CRLF line ends, ISO-8859-1
/// (characters outside it become `?`).
pub fn to_latin1_crlf(s: &str) -> Vec<u8> {
    s.replace('\n', "\r\n").chars().map(|c| if (c as u32) < 0x100 { c as u8 } else { b'?' }).collect()
}

/// "Script LDD au format TXT..." for Access: the tables created by a VBA
/// module, one `DoCmd.RunSQL` per statement (0x140066620, 0x14010b800).
pub fn vba(mld: &Mld, o: &SqlOptions) -> String {
    format!("Sub Create_Tables()\n\n{}\nEnd Sub", script(mld, o, true))
}

/// Content of "Script LDD au format TXT...": the VBA module for Access, the
/// script otherwise; refused like the XML export when the MLD has errors.
pub fn script_file(mld: &Mld, o: &SqlOptions) -> Result<String, &'static str> {
    if !mld.is_valid() {
        Err(EXPORT_REFUSED)
    } else if o.dbms == Dbms::Access {
        Ok(vba(mld, o))
    } else {
        Ok(script(mld, o, false))
    }
}
