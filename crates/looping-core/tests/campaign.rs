//! Verification campaign of v0.1.0 (ROADMAP point 10): random sequences of
//! the critical path operations never panic, and after each one every
//! output (MLD, SQL for the 9 DBMS, XML, VBA, scene) is produced and the
//! document survives a save / reopen / save round trip.
//!
//! `CAMPAIGN_SEEDS=<n>` runs more seeds (default 300 with `--release`,
//! 30 in a debug build, about 20 times slower).

mod support;

use loo_format::Document;
use looping_core::{edit, render, sql, Mcd, Mld};
use std::panic::{catch_unwind, AssertUnwindSafe};
use support::random::Gen;

const STEPS: usize = 60;

/// Everything the application derives from a document.
fn check(doc: &Document) {
    let mcd = Mcd::from_document(doc);
    let mld = Mld::from_mcd(&mcd);
    let stored = sql::SqlOptions::from_settings(&doc.settings);
    let _ = mld.to_text(&stored);
    let _ = sql::xml(&mld, &stored, "campagne");
    let _ = sql::vba(&mld, &stored);
    let _ = sql::script_file(&mld, &stored);
    for d in 0..9 {
        let o = sql::SqlOptions { dbms: looping_core::Dbms::from_index(d), ..stored };
        let _ = sql::generate_with(&mld, &o);
    }
    let _ = render::build(doc);
    let mut fitted = doc.clone();
    edit::fit_all(&mut fitted);

    let bytes = doc.save().expect("save");
    let reopened = Document::load(&bytes).expect("reopen");
    assert_eq!(reopened.save().expect("save again"), bytes, "save / reopen / save is not stable");
}

#[test]
fn random_edits_never_panic() {
    let seeds: u64 = std::env::var("CAMPAIGN_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(if cfg!(debug_assertions) { 30 } else { 300 });
    let mut failures = Vec::new();
    for seed in 0..seeds {
        let mut g = Gen::new(seed);
        for step in 0..STEPS {
            let r = catch_unwind(AssertUnwindSafe(|| {
                g.step();
                check(&g.doc);
            }));
            if let Err(e) = r {
                let msg = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                let tail = g.log[g.log.len().saturating_sub(5)..].join("\n    ");
                failures.push(format!("seed {seed} step {step}: {msg}\n    {tail}"));
                break;
            }
        }
    }
    assert!(failures.is_empty(), "{} seeds panicked:\n{}", failures.len(), failures.join("\n"));
}
