//! Narrows down a difference with Looping: writes one copy of a document
//! per model object (entity, association, inheritance, CIF, constraint,
//! rule), without that object and its links, to be run through Looping:
//!
//!     cargo run -p looping-core --example shrink -- DOC.loo OUT_DIR
//!     tools/wine/extract.sh OUT_DIR OUT_DIR/*.loo
//!
//! `<stem>.del-<id>.loo` lacks object `<id>`; the copies whose output still
//! shows the difference tell which objects it does not need.

use loo_format::{Document, ObjKind};

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(path), Some(out)) = (args.next(), args.next()) else {
        panic!("usage: shrink DOC.loo OUT_DIR");
    };
    let doc = Document::load(&std::fs::read(&path).unwrap()).unwrap();
    let stem = std::path::Path::new(&path).file_stem().unwrap().to_string_lossy().to_string();
    std::fs::create_dir_all(&out).unwrap();
    let ids: Vec<i32> = doc
        .objects
        .iter()
        .filter(|o| {
            matches!(
                o.kind,
                ObjKind::Entite(_)
                    | ObjKind::Relation(_)
                    | ObjKind::Heritage(_)
                    | ObjKind::Cif(_)
                    | ObjKind::Contrainte(_)
                    | ObjKind::Regle(_)
            )
        })
        .map(|o| o.base.id)
        .collect();
    for id in ids {
        let mut d = doc.clone();
        looping_core::edit::delete(&mut d, id);
        std::fs::write(format!("{out}/{stem}.del-{id}.loo"), d.save().unwrap()).unwrap();
    }
}
