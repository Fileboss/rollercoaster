//! Compares our diagram images with Looping's image export, read from
//! `samples/ref/<stem>.png` (built by `tools/wine/make-image-refs.sh`;
//! skipped when absent). Glyph rasterisation and anti-aliasing differ, so
//! the test counts pixels with nothing of their colour around them in the
//! other image, both ways: the geometry (boxes, bands, shadows, links, text
//! positions) has to match.

use resvg::tiny_skia::Pixmap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Largest share of pixels allowed to differ (see [`difference`]); the
/// synthetic cases are small and crossed by legs drawn over the boxes, so
/// line rasterisation weighs more there.
const MAX_SHARE: f64 = 0.005;
const MAX_SHARE_CASES: f64 = 0.012;

/// Documents using what the renderer does not reproduce yet (reported, not
/// checked): prefix of the stem and reason.
const KNOWN: &[(&str, &str)] = &[
    ("TP 1", "rectangles et textes des règles approchés"),
    ("c03_", "rôles : position approchée (1,3 %)"),
    ("c22_", "entité de modèle externe : contour pointillé approché"),
    ("c23_", "cardinalités superposées placées autrement"),
    ("c28_", "CIF : pointillés et libellés approchés (1,3 %)"),
    ("c41_", "traits et flèches libres : étendue"),
    ("c48_", "vignette : icône du sous-modèle ; texte d'une image sans fichier coupé au cadre ; étendue des traits"),
];
/// Per-channel difference under which two pixels are taken as equal.
const TOLERANCE: u8 = 96;

fn samples() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples")
}

fn document(stem: &str) -> Option<PathBuf> {
    let s = samples();
    [s.join(format!("{stem}.loo")), s.join(format!("{stem}.Loo")), s.join("cases").join(format!("{stem}.loo"))]
        .into_iter()
        .find(|p| p.exists())
}

fn render(doc: &Path, out: &Path) -> Pixmap {
    let st = Command::new(env!("CARGO_BIN_EXE_looping-cli")).arg("png").arg(doc).arg(out).status().unwrap();
    assert!(st.success(), "{}", doc.display());
    Pixmap::load_png(out).unwrap()
}

fn rgb(p: &Pixmap, x: u32, y: u32) -> [u8; 3] {
    let c = p.pixel(x, y).unwrap().demultiply();
    [c.red(), c.green(), c.blue()]
}

fn close(a: [u8; 3], b: [u8; 3]) -> bool {
    a.iter().zip(b).all(|(p, q)| p.abs_diff(q) <= TOLERANCE)
}

/// Share of pixels of `a` with no pixel of `b` close in colour in their
/// 3 × 3 neighbourhood (sizes must be equal): tolerates anti-aliasing and
/// glyph shapes, not misplaced lines, boxes or texts.
fn difference(a: &Pixmap, b: &Pixmap) -> f64 {
    let (w, h) = (a.width(), a.height());
    let mut far = 0usize;
    for y in 0..h {
        for x in 0..w {
            let p = rgb(a, x, y);
            let near = (y.saturating_sub(1)..=(y + 1).min(h - 1))
                .any(|v| (x.saturating_sub(1)..=(x + 1).min(w - 1)).any(|u| close(p, rgb(b, u, v))));
            far += usize::from(!near);
        }
    }
    far as f64 / (w * h) as f64
}

#[test]
fn images_match_looping() {
    let Ok(rd) = std::fs::read_dir(samples().join("ref")) else { return };
    let mut stems: Vec<String> =
        rd.filter_map(|e| e.ok()?.file_name().into_string().ok()?.strip_suffix(".png").map(str::to_string)).collect();
    stems.sort();
    let tmp = std::env::temp_dir().join(format!("looping-images-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let mut failures = Vec::new();
    for stem in &stems {
        let Some(doc) = document(stem) else { continue };
        let known = KNOWN.iter().find(|(prefix, _)| stem.starts_with(prefix)).map(|(_, why)| why);
        let want = Pixmap::load_png(samples().join("ref").join(format!("{stem}.png"))).unwrap();
        let got = render(&doc, &tmp.join(format!("{stem}.png")));
        if (got.width(), got.height()) != (want.width(), want.height()) {
            if let Some(why) = known {
                eprintln!("{stem}: size differs (known: {why})");
                continue;
            }
            failures.push(format!(
                "{stem}: size {}x{} instead of {}x{}",
                got.width(),
                got.height(),
                want.width(),
                want.height()
            ));
            continue;
        }
        let d = difference(&got, &want).max(difference(&want, &got));
        eprintln!(
            "{stem}: {:.2} % of the pixels differ{}",
            d * 100.0,
            known.map_or(String::new(), |w| format!(" (known: {w})"))
        );
        let max = if doc.parent().is_some_and(|p| p.ends_with("cases")) { MAX_SHARE_CASES } else { MAX_SHARE };
        if d > max && known.is_none() {
            failures.push(format!("{stem}: {:.2} % of the pixels differ", d * 100.0));
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(failures.is_empty(), "{} image(s) differ:\n{}", failures.len(), failures.join("\n"));
}
