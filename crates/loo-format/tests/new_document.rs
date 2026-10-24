//! A new document is the one Looping 4.1 creates (« Nouveau », properties
//! not taken from another model), saved under Wine by `tools/wine/newdoc.py`.

#[test]
fn new_document_is_loopings() {
    let looping = include_bytes!("data/new_document.loo");
    assert_eq!(loo_format::Document::default().save().unwrap(), looping.to_vec());
}
