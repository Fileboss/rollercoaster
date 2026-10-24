//! Document options of the settings block (`CLoopingDoc` offsets), with
//! the titles and value labels of Looping's property grid (builder near
//! 0x1400f7c00, value getters 0x140078640..0x14007d8e0).

/// An option stored as an i32 whose values are indices into `values`.
pub struct DocOption {
    pub offset: usize,
    pub title: &'static str,
    pub values: &'static [&'static str],
    /// Changes the generated MLD or SQL.
    pub affects_scripts: bool,
}

const fn opt(offset: usize, title: &'static str, values: &'static [&'static str], affects_scripts: bool) -> DocOption {
    DocOption { offset, title, values, affects_scripts }
}

const NO_YES: &[&str] = &["Non", "Oui"];

pub const OPTIONS: &[DocOption] = &[
    opt(
        0x52c,
        "SGBD cible",
        &["Access", "MySQL", "Oracle", "PostgreSQL", "SQL Server", "Sybase", "DB2", "MariaDB", "SQLite"],
        true,
    ),
    opt(
        0xa78,
        "Typologie nom logique",
        &["Standard", "Snake Case", "Screaming Snake Case", "Camel Case", "Pascal Case", "Idem nom conceptuel"],
        true,
    ),
    opt(0xab8, "Suppression accents", NO_YES, true),
    opt(0x7f0, "Types dans MLD textuel", NO_YES, true),
    opt(0x7f4, "Nom contrainte clé", &["Non défini", "Préfixé (PK_ / FK_ / AK_)", "Suffixé (_PK / _FK / _AK)"], true),
    opt(0xb40, "Position contrainte clé", &["Dans CREATE TABLE", "Dans ALTER TABLE", "FK dans ALTER TABLE"], true),
    opt(0x7f8, "Commentaire dans script", &["Non", "Oui avec /* */", "Oui avec --", "Oui avec COMMENT ON"], true),
    // any non-zero value enables the per-link actions; the value is the
    // default proposed for new links
    opt(
        0x888,
        "Proposer cascade",
        &["Non", "Oui - non validé", "Delete validé", "Update validé", "Delete/Update validés"],
        true,
    ),
    opt(0x7fc, "Nom de l'identifiant créé", &["Id_[Classe]", "[Classe]_Id", "Id[Classe]", "[Classe]Id"], false),
    opt(0xa00, "Format Identifiant relatif", &["1,1(R)", "(1,1)", "1,1 souligné"], false),
    opt(0x7c0, "Affichage DF dans MCD", &["Non", "Flèche", "Asso simplifiée"], false),
    opt(0xa20, "Format lien MLD", &["Flèche DF", "Crow's foot"], false),
    opt(0x53c, "Côté texte", &["Entité", "Association"], false),
    opt(0x7ec, "Usage du gras", &["UNIQUE", "NOT NULL"], false),
    opt(0xa18, "Rattachement", &["Automatique", "Auto sauf pour liens", "Clic droit"], false),
    opt(0xac8, "Affichage rubriques", &["Type données", "Noms index"], false),
];

/// Values proposed for "Nom clé étrangère" (any text is accepted): `[FK]`
/// is the referenced column, `[Classe]` the referenced entity.
pub const FK_MASKS: [&str; 3] = ["[FK]", "[Classe]_[FK]", "[FK]_[Classe]"];

/// How a [`Field`] of the settings block is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Int,
    Bool,
    /// COLORREF, -1 = "Par défaut".
    Color,
    /// `LOGFONTW` at the field offset, with its colour at `color`.
    Font {
        color: usize,
    },
    /// NUL-terminated UTF-16 of `len` units.
    Text {
        len: usize,
    },
}

/// A property of Looping's "Propriétés" grid and where the document stores
/// it. Read from the grid builder (0x1400f2000..0x1400f9000: titles) and
/// its refresh code (0x1400f9940..0x1400fb400: `CLoopingDoc` offsets).
#[derive(Debug, Clone, Copy)]
pub struct Field {
    pub offset: usize,
    pub section: &'static str,
    pub title: &'static str,
    pub kind: FieldKind,
}

const fn field(section: &'static str, title: &'static str, offset: usize, kind: FieldKind) -> Field {
    Field { offset, section, title, kind }
}

use FieldKind::{Bool, Color, Font, Int, Text};

/// The settings not covered by [`OPTIONS`] and [`FK_MASKS`].
pub const FIELDS: &[Field] = &[
    field("Textes", "Police", 0x3b0, Font { color: 0x3a0 }),
    field("Textes", "Epaisseur contour", 0x370, Int),
    field("Textes", "Epaisseur ombre", 0x548, Int),
    field("Textes", "Couleur contour", 0x380, Color),
    field("Textes", "Couleur ombre", 0x54c, Color),
    field("Textes", "Couleur fond", 0x390, Color),
    field("Textes", "Affichage fond", 0x740, Bool),
    field("Textes", "Affichage contour", 0x744, Bool),
    field("Textes", "Angles arrondis", 0x7c8, Int),
    field("Entités", "Largeur minimale", 0x520, Int),
    field("Entités", "Hauteur minimale", 0x524, Int),
    field("Entités", "Hauteur toutes rubriques", 0xa84, Bool),
    field("Entités", "Hauteur auto maximale", 0xa88, Int),
    field("Entités", "Hauteur titre", 0x530, Int),
    field("Entités", "Police titre", 0x40c, Font { color: 0x3a4 }),
    field("Entités", "Police rubrique", 0x5d0, Font { color: 0x560 }),
    field("Entités", "Police type", 0x6e4, Font { color: 0x56c }),
    field("Entités", "Epaisseur contour", 0x374, Int),
    field("Entités", "Epaisseur ombre", 0x550, Int),
    field("Entités", "Couleur contour", 0x384, Color),
    field("Entités", "Couleur ombre", 0x554, Color),
    field("Entités", "Couleur fond titre", 0xa04, Color),
    field("Entités", "Couleur fond corps", 0x394, Color),
    field("Entités", "Couleur ascenseur", 0xa8c, Color),
    field("Associations", "Largeur angle", 0x534, Int),
    field("Associations", "Hauteur angle", 0x538, Int),
    field("Associations", "Hauteur titre", 0xa14, Int),
    field("Associations", "Police titre", 0x468, Font { color: 0x3a8 }),
    field("Associations", "Police rubrique", 0x574, Font { color: 0x564 }),
    field("Associations", "Police type", 0x688, Font { color: 0x570 }),
    field("Associations", "Epaisseur contour", 0x378, Int),
    field("Associations", "Epaisseur ombre", 0x558, Int),
    field("Associations", "Couleur contour", 0x388, Color),
    field("Associations", "Couleur ombre", 0x55c, Color),
    field("Associations", "Couleur fond titre 1", 0xa08, Color),
    field("Associations", "Couleur fond corps 1", 0x398, Color),
    field("Associations", "Couleur fond titre n", 0xa0c, Color),
    field("Associations", "Couleur fond corps n", 0x800, Color),
    field("Liens", "Epaisseur trait", 0x37c, Int),
    field("Liens", "Couleur trait", 0x38c, Color),
    field("Liens", "Cassure arrondie", 0xa24, Int),
    field("Liens", "Taille flèche", 0x9f8, Int),
    field("Liens", "Forme flèche", 0x9fc, Int),
    field("Liens", "Police cardinalités", 0x62c, Font { color: 0x568 }),
    field("Liens", "Police autres textes", 0x994, Font { color: 0x9f4 }),
    field("Liens", "Lien entre illustrations", 0x7c4, Bool),
    field("MLD - LDD", "Police script LDD", 0x74c, Font { color: 0x7a8 }),
    field("MLD - LDD", "Police rétroconception", 0xae0, Font { color: 0xb3c }),
    field("MLD - LDD", "Caractères de protection", 0xa6c, Text { len: 6 }),
    field("MLD - LDD", "Caractères de masquage", 0xabc, Text { len: 6 }),
    field("MLD - LDD", "Couleur clé étrangère", 0xa68, Color),
    field("MLD - LDD", "Couleur table MLD", 0x884, Color),
    field("MLD - LDD", "Couleur classe UML", 0x7ac, Color),
    field("MLD - LDD", "Couleur modèle externe", 0x804, Color),
    field("MLD - LDD", "Couleur héritage", 0x7b0, Color),
    field("MLD - LDD", "Couleur contrainte", 0x7b4, Color),
    field("MLD - LDD", "Couleur CIF", 0x7b8, Color),
    field("MLD - LDD", "Couleur règle", 0x7bc, Color),
    field("Options générales", "Résolution export (dpi)", 0x7e4, Int),
    field("Options générales", "Nb copies sauvegarde", 0x7e8, Int),
    field("Options générales", "Proposer index composé", 0xa1c, Bool),
    field("Options générales", "Affichage UML simplifié", 0xa80, Bool),
    field("Options générales", "Epaississement sélection", 0xa7c, Bool),
    field("Options générales", "CIF multicible", 0xa10, Bool),
];

#[cfg(test)]
mod tests {
    use super::*;
    use loo_format::Settings;

    /// The default block (from a Looping 4.1 document) has a face name in
    /// every font field and no field overlapping another.
    #[test]
    fn fields_match_default_block() {
        let s = Settings::new_default();
        let mut spans: Vec<(usize, usize, &str)> = Vec::new();
        for f in FIELDS {
            let len = match f.kind {
                FieldKind::Font { color } => {
                    assert!(!s.logfont(f.offset).face.is_empty(), "{}", f.title);
                    spans.push((color, 4, f.title));
                    0x5c
                }
                FieldKind::Text { len } => 2 * len,
                _ => 4,
            };
            spans.push((f.offset, len, f.title));
        }
        spans.extend(OPTIONS.iter().map(|o| (o.offset, 4, o.title)));
        spans.sort();
        for w in spans.windows(2) {
            assert!(w[0].0 + w[0].1 <= w[1].0, "{} overlaps {}", w[0].2, w[1].2);
        }
    }
}
