//! Round-trip tests over the `.loo` files in `samples/` (skipped when absent).

use loo_format::{Document, CURRENT_VERSION};
use std::path::PathBuf;

fn samples() -> Vec<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples");
    let Ok(rd) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut v: Vec<_> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("loo")))
        .collect();
    v.sort();
    v
}

#[test]
fn load_all_samples() {
    for p in samples() {
        let doc = Document::open(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        assert!(!doc.objects.is_empty(), "{}", p.display());
    }
}

#[test]
fn current_version_roundtrip_is_byte_identical() {
    for p in samples() {
        let bytes = std::fs::read(&p).unwrap();
        let doc = Document::load(&bytes).unwrap();
        if doc.loaded_version != CURRENT_VERSION {
            continue;
        }
        let out = doc.save().unwrap();
        if out != bytes {
            let i = out.iter().zip(&bytes).position(|(a, b)| a != b).unwrap_or(out.len().min(bytes.len()));
            panic!("{}: first difference at {i:#x} (len {} vs {})", p.display(), out.len(), bytes.len());
        }
    }
}

#[test]
fn old_versions_upgrade_and_reload() {
    for p in samples() {
        let doc = Document::open(&p).unwrap();
        let out = doc.save().unwrap();
        let again = Document::load(&out).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
        assert_eq!(again.loaded_version, CURRENT_VERSION);
        assert_eq!(again.objects.len(), doc.objects.len());
        for (a, b) in again.objects.iter().zip(&doc.objects) {
            assert_eq!(a, b, "{}", p.display());
        }
        assert_eq!(again.save().unwrap(), out, "{}: save is not stable", p.display());
    }
}
