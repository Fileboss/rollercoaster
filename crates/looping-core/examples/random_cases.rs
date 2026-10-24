//! Writes random documents built by the critical path operations
//! (`tests/support/random.rs`) for the real Looping, which must reopen them
//! with the same MLD and SQL as ours (ROADMAP point 10):
//!
//!     cargo run -p looping-core --example random_cases -- samples/cases [COUNT] [STEPS] [--chaos]
//!     tools/wine/extract.sh samples/ref samples/cases/r*.loo
//!     cargo test -p looping-core --test reference
//!
//! Each `r<seed>.loo` is followed by the operations that built it in
//! `r<seed>.log`. The models are realistic ones (see `Gen::realistic`);
//! `--chaos` writes `x<seed>.loo` with anything the editor allows, whose
//! differences with Looping on absurd models are known and accepted
//! (docs/ROADMAP.md).

#[path = "../tests/support/random.rs"]
mod random;

fn main() {
    let chaos = std::env::args().any(|a| a == "--chaos");
    let mut args = std::env::args().skip(1).filter(|a| a != "--chaos");
    let out = args.next().expect("usage: random_cases OUT_DIR [COUNT] [STEPS]");
    let count: u64 = args.next().map_or(50, |s| s.parse().unwrap());
    let steps: usize = args.next().map_or(25, |s| s.parse().unwrap());
    std::fs::create_dir_all(&out).unwrap();
    for seed in 1..=count {
        let g = random::Gen::document(seed, steps, !chaos);
        let stem = format!("{out}/{}{seed:03}", if chaos { 'x' } else { 'r' });
        std::fs::write(format!("{stem}.loo"), g.doc.save().unwrap()).unwrap();
        std::fs::write(format!("{stem}.log"), g.log.join("\n") + "\n").unwrap();
    }
}
