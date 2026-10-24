//! Compares our MLD and SQL with the output of the real Looping, read from
//! `samples/ref/` (built by `tools/wine/make-refs.sh`; skipped when absent).
//!
//! `<stem>.sql` / `<stem>.mld.txt` come from `samples/<stem>.loo` (or the
//! synthetic `samples/cases/<stem>.loo`, see `examples/cases.rs`) as stored,
//! `<stem>[.<dbms>][.set-<off>-<val>][.fk-<n>].*` from the same file with
//! the DBMS (`+0x52c`), the settings i32 at `+0x<off>` and/or the foreign
//! key mask [`FK_MASKS`]`[n]` forced.

use loo_format::Document;
use looping_core::options::FK_MASKS;
use looping_core::{sql, Mcd, Mld};
use std::path::PathBuf;

const DBMS: [&str; 9] = ["access", "mysql", "oracle", "postgresql", "sqlserver", "sybase", "db2", "mariadb", "sqlite"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples")
}

struct Case {
    name: String,
    doc: Document,
    /// Looping's output, CRLF turned into LF.
    want: String,
}

/// Every reference `<name><ext>` with its document.
fn cases(ext: &str) -> Vec<Case> {
    let dir = root().join("ref");
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> =
        rd.filter_map(|e| e.ok()?.file_name().into_string().ok()?.strip_suffix(ext).map(str::to_string)).collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            // trailing ".<dbms>" / ".set-<off>-<val>" segments are settings to force
            let mut stem = name.as_str();
            let mut set = Vec::new();
            let mut mask = None;
            while let Some((s, seg)) = stem.rsplit_once('.') {
                if let Some(d) = DBMS.iter().position(|x| *x == seg) {
                    set.push((0x52c, d as i32));
                } else if let Some((off, val)) = seg.strip_prefix("set-").and_then(|o| o.split_once('-')) {
                    set.push((usize::from_str_radix(off, 16).unwrap(), val.parse().unwrap()));
                } else if let Some(n) = seg.strip_prefix("fk-") {
                    mask = Some(FK_MASKS[n.parse::<usize>().unwrap()]);
                } else {
                    break;
                }
                stem = s;
            }
            let path = [root(), root().join("cases")]
                .iter()
                .flat_map(|d| ["loo", "Loo"].map(|e| d.join(format!("{stem}.{e}"))))
                .find(|p| p.exists());
            let mut doc = Document::open(path.unwrap_or_else(|| panic!("no sample for {name}"))).unwrap();
            for (off, val) in set {
                doc.settings.set_i32(off, val);
            }
            if let Some(m) = mask {
                doc.settings.set_fk_mask(m);
            }
            // panes are saved as UTF-8, export files as ISO-8859-1
            let bytes = std::fs::read(dir.join(format!("{name}{ext}"))).unwrap();
            let text = String::from_utf8(bytes.clone()).unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect());
            Case { want: text.replace("\r\n", "\n"), doc, name }
        })
        .collect()
}

/// RichEdit drops the final line break: compare without trailing whitespace.
fn same(a: &str, b: &str) -> bool {
    a.trim_end() == b.trim_end()
}

fn first_difference(want: &str, got: &str) -> String {
    let (w, g): (Vec<_>, Vec<_>) = (want.lines().collect(), got.lines().collect());
    let i = (0..w.len().max(g.len())).find(|&i| w.get(i) != g.get(i)).unwrap_or(0);
    format!("line {}:\n  looping: {:?}\n  ours:    {:?}", i + 1, w.get(i), g.get(i))
}

/// Compares `ours(case)` with each reference `*<ext>`; `exact` also checks
/// trailing whitespace (export files, unlike the RichEdit panes).
/// Known differences, on invalid models (Looping refuses their script):
/// random documents of `examples/random_cases.rs` by name.
const KNOWN: &[(&str, &str)] = &[
    ("r057", "références circulaires : identification relative revenant sur l'entité, rangs"),
    ("r102", "références circulaires : ordre des rangs"),
    ("r106", "deux CIF contradictoires sur une même association"),
    ("r126", "références circulaires : clé du parent lue avant l'héritage"),
    ("r137", "références circulaires : ordre des rangs"),
];

fn check(ext: &str, exact: bool, ours: impl Fn(&Case) -> String) {
    let cases = cases(ext);
    let mut failures = Vec::new();
    for c in &cases {
        let got = ours(c);
        if !(same(&c.want, &got) && (!exact || c.want == got)) {
            if let Some((_, why)) = KNOWN.iter().find(|(name, _)| c.name == *name) {
                eprintln!("{}{ext}: differs (known: {why})", c.name);
                continue;
            }
            failures.push(format!("{}{ext} {}", c.name, first_difference(&c.want, &got)));
        }
    }
    assert!(
        failures.is_empty(),
        "{}/{} {ext} outputs differ from Looping:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

fn mld_of(doc: &Document) -> Mld {
    Mld::from_mcd(&Mcd::from_document(doc))
}

fn options(doc: &Document) -> sql::SqlOptions {
    sql::SqlOptions::from_settings(&doc.settings)
}

#[test]
fn mld_text_matches_looping() {
    check(".mld.txt", false, |c| mld_of(&c.doc).to_text(&options(&c.doc)));
}

#[test]
fn sql_matches_looping() {
    check(".sql", false, |c| sql::generate_with(&mld_of(&c.doc), &options(&c.doc)));
}

/// "MLD textuel au format XML..."; the database is named after the file
/// Looping opened, i.e. the reference name.
#[test]
fn xml_export_matches_looping() {
    check(".xml", true, |c| sql::xml(&mld_of(&c.doc), &options(&c.doc), &c.name).unwrap());
}

/// "Script LDD au format TXT..." with Access: a VBA module.
#[test]
fn vba_export_matches_looping() {
    check(".vba", true, |c| sql::vba(&mld_of(&c.doc), &options(&c.doc)));
}
