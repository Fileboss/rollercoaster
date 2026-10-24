//! Writes small models covering the MLD rules the samples lack, to be run
//! through the real Looping by `tools/wine/make-refs.sh`:
//!
//!     cargo run -p looping-core --example cases -- OUT_DIR

use loo_format::{Document, ObjKind, Point, Rect};
use looping_core::edit::{
    add_association, add_cif, add_cif_link, add_contrainte, add_contrainte_link, add_crayon, add_ellipse, add_entity,
    add_free_line, add_heritage, add_heritage_link, add_illustration_link, add_image, add_link, add_rectangle,
    add_regle, add_regle_link, add_texte, add_view, new_parametre, set_script, set_views, texte_mut,
};
use looping_core::mcd::ObjId;

// stored cardinality codes
const C01: i32 = 0;
const C11: i32 = 1;
const C0N: i32 = 2;
const C1N: i32 = 3;

struct Model {
    doc: Document,
    n: i32,
}

impl Model {
    fn new() -> Model {
        Model { doc: Document::default(), n: 0 }
    }

    fn at(&mut self) -> Point {
        self.n += 1;
        Point { x: 250 * (self.n % 4), y: 200 * (self.n / 4) }
    }

    fn entity(&mut self, name: &str) -> ObjId {
        let at = self.at();
        add_entity(&mut self.doc, name, at)
    }

    /// Entity whose identifier is `ids` (plain integers) instead of the default counter.
    fn entity_with_key(&mut self, name: &str, ids: &[&str]) -> ObjId {
        let e = self.entity(name);
        let t = texte_mut(&mut self.doc, e, 1).unwrap();
        t.params.clear();
        for id in ids {
            let mut p = new_parametre(id);
            p.f8 = 1;
            p.f20 = 1;
            p.f2c[0] = 0;
            t.params.push(p);
        }
        e
    }

    /// Adds a VARCHAR(50) attribute; `not_null` / `unique` set `+0x88` / `+0x8c`.
    fn attr(&mut self, owner: ObjId, name: &str, not_null: bool, unique: bool, comment: &str) {
        let mut p = new_parametre(name);
        p.f88 = not_null as i32;
        p.f8c = unique as i32;
        p.s98 = comment.to_string();
        texte_mut(&mut self.doc, owner, 1).unwrap().params.push(p);
    }

    fn assoc(&mut self, name: &str, legs: &[(ObjId, i32)]) -> ObjId {
        let at = self.at();
        let a = add_association(&mut self.doc, name, at);
        for &(e, c) in legs {
            add_link(&mut self.doc, e, a, c);
        }
        a
    }

    /// Like `assoc`, returning the first arc of each leg.
    fn assoc_arcs(&mut self, name: &str, legs: &[(ObjId, i32)]) -> (ObjId, Vec<ObjId>) {
        let at = self.at();
        let a = add_association(&mut self.doc, name, at);
        let arcs = legs.iter().map(|&(e, c)| add_link(&mut self.doc, e, a, c)).collect();
        (a, arcs)
    }

    /// Marks a `1,1` leg as relative identification "(R)".
    fn relative(&mut self, arc: ObjId) {
        let t = texte_mut(&mut self.doc, arc, 1).unwrap();
        t.params.iter_mut().find(|p| p.role == 1).unwrap().f84 = 1;
    }

    /// Sets `CDrawArc+0x3c8..0x3d0` on the first segment of a leg, or on
    /// the second (association side) one.
    fn arc_flags(&mut self, arc: ObjId, second: bool, flags: [i32; 3]) {
        let id = if second { self.doc.object(arc).unwrap().base.links.next } else { arc };
        let o = self.doc.objects.iter_mut().find(|o| o.base.id == id).unwrap();
        if let ObjKind::Arc(a) = &mut o.kind {
            a.f3c8 = flags;
        }
    }

    /// Entity without identifier, with plain VARCHAR attributes.
    fn subtype(&mut self, name: &str, attrs: &[&str]) -> ObjId {
        let e = self.entity(name);
        texte_mut(&mut self.doc, e, 1).unwrap().params.clear();
        for a in attrs {
            self.attr(e, a, false, false, "");
        }
        e
    }

    /// Supertype `sup` specialised into `subs` through one inheritance symbol.
    fn heritage(&mut self, sup: ObjId, subs: &[ObjId], constraint: i32, migrate_all: bool) -> ObjId {
        let at = self.at();
        let h = add_heritage(&mut self.doc, at, constraint);
        if let Some(ObjKind::Heritage(x)) = self.doc.objects.iter_mut().find(|o| o.base.id == h).map(|o| &mut o.kind) {
            x.f418 = migrate_all as i32;
        }
        add_heritage_link(&mut self.doc, sup, h, true);
        for &e in subs {
            add_heritage_link(&mut self.doc, e, h, false);
        }
        h
    }

    /// Sets `CDrawRelation+0x3ac`.
    fn relation_flag(&mut self, assoc: ObjId, value: i32) {
        let o = self.doc.objects.iter_mut().find(|o| o.base.id == assoc).unwrap();
        if let ObjKind::Relation(r) = &mut o.kind {
            r.f3ac = value;
        }
    }

    fn role(&mut self, arc: ObjId, role: &str) {
        self.role_with(arc, role, true, false);
    }

    /// Role with its "Suffixer clés étrangères" / "Renommer clé étrangère"
    /// options (`+0x130`, stored inverted, / `+0x134`).
    fn role_with(&mut self, arc: ObjId, role: &str, suffix: bool, rename: bool) {
        let mut p = new_parametre("");
        p.role = 2;
        p.sf0 = role.to_string();
        p.n130 = !suffix as i32;
        p.f134 = rename as i32;
        texte_mut(&mut self.doc, arc, 1).unwrap().params.push(p);
    }

    /// Sets a field of the cardinality parameter (role 1) of a leg.
    fn leg_param(&mut self, arc: ObjId, f: impl FnOnce(&mut loo_format::objects::Parametre)) {
        let t = texte_mut(&mut self.doc, arc, 1).unwrap();
        f(t.params.iter_mut().find(|p| p.role == 1).unwrap());
    }
}

fn cases() -> Vec<(&'static str, Model)> {
    let mut v = Vec::new();

    let mut m = Model::new();
    let e = m.entity("Employe");
    m.attr(e, "Nom", true, false, "");
    m.assoc("Diriger", &[(e, C01), (e, C0N)]);
    v.push(("c01_reflexive_1n", m));

    let mut m = Model::new();
    let e = m.entity("Employe");
    let (_, arcs) = m.assoc_arcs("Diriger", &[(e, C01), (e, C0N)]);
    m.role(arcs[0], "subordonne");
    m.role(arcs[1], "chef");
    v.push(("c02_reflexive_1n_roles", m));

    let mut m = Model::new();
    let e = m.entity("Personne");
    let (_, arcs) = m.assoc_arcs("Connaitre", &[(e, C0N), (e, C0N)]);
    m.role(arcs[0], "connaisseur");
    m.role(arcs[1], "connu");
    m.assoc("Admirer", &[(e, C0N), (e, C0N)]);
    v.push(("c03_reflexive_nn", m));

    let mut m = Model::new();
    let h = m.entity_with_key("Hotel", &["Num_hotel"]);
    let c = m.entity_with_key("Chambre", &["Num_chambre"]);
    let l = m.entity_with_key("Lit", &["Num_lit"]);
    let (_, arcs) = m.assoc_arcs("Comporter", &[(h, C1N), (c, C11)]);
    m.relative(arcs[1]);
    let (_, arcs) = m.assoc_arcs("Equiper", &[(c, C1N), (l, C11)]);
    m.relative(arcs[1]);
    let cl = m.entity("Client");
    m.assoc("Reserver", &[(cl, C0N), (c, C0N)]);
    v.push(("c04_relative", m));

    let mut m = Model::new();
    let p = m.entity("Pays");
    let c = m.entity("Capitale");
    m.assoc("Avoir", &[(p, C01), (c, C11)]);
    v.push(("c05_one_one_01_11", m));

    let mut m = Model::new();
    let p = m.entity("Pays");
    let c = m.entity("Capitale");
    m.assoc("Avoir", &[(p, C11), (c, C11)]);
    v.push(("c06_one_one_11_11", m));

    let mut m = Model::new();
    let p = m.entity("Pays");
    let c = m.entity("Capitale");
    m.assoc("Avoir", &[(p, C01), (c, C01)]);
    v.push(("c07_one_one_01_01", m));

    let mut m = Model::new();
    let a = m.entity("Professeur");
    let b = m.entity("Classe");
    let c = m.entity("Matiere");
    let t = m.assoc("Enseigner", &[(a, C0N), (b, C1N), (c, C0N)]);
    m.attr(t, "Heures", true, false, "");
    v.push(("c08_ternary", m));

    let mut m = Model::new();
    let a = m.entity("Professeur");
    let b = m.entity("Classe");
    let c = m.entity("Matiere");
    m.assoc("Enseigner", &[(a, C0N), (b, C11), (c, C0N)]);
    v.push(("c09_ternary_with_11", m));

    let mut m = Model::new();
    let cmd = m.entity("Commande");
    let cli = m.entity("Client");
    let a = m.assoc("Passer", &[(cmd, C11), (cli, C0N)]);
    m.attr(a, "Date_commande", true, false, "");
    m.attr(a, "Canal", false, false, "");
    v.push(("c10_attributes_on_11", m));

    let mut m = Model::new();
    let v1 = m.entity_with_key("Vol", &["Num_vol", "Date_vol"]);
    let p = m.entity("Passager");
    m.assoc("Embarquer", &[(p, C0N), (v1, C0N)]);
    let s = m.entity("Siege");
    m.assoc("Affecter", &[(s, C11), (v1, C0N)]);
    v.push(("c11_composite_key", m));

    let mut m = Model::new();
    let vol = m.entity("Vol");
    let aero = m.entity("Aeroport");
    m.assoc("Partir", &[(vol, C11), (aero, C0N)]);
    m.assoc("Arriver", &[(vol, C11), (aero, C0N)]);
    v.push(("c12_two_links_same_entity", m));

    let mut m = Model::new();
    let c = m.entity("C");
    let b = m.entity("B");
    let a = m.entity("A");
    m.assoc("C_B", &[(c, C11), (b, C0N)]);
    m.assoc("B_A", &[(b, C11), (a, C0N)]);
    v.push(("c13_order_chain", m));

    let mut m = Model::new();
    let e = m.entity("Article");
    m.attr(e, "Reference", true, true, "code fournisseur");
    m.attr(e, "Libelle", true, false, "");
    m.attr(e, "Prix", false, false, "prix HT");
    let f = m.entity("Famille");
    m.attr(f, "Libelle", false, true, "");
    m.assoc("Appartenir", &[(e, C11), (f, C0N)]);
    v.push(("c14_column_flags", m));

    // CASCADE flags (CDrawArc+0x3c8 delete, +0x3cc update, +0x3d0 ?) on
    // one leg (child = 1,1 side) and one segment
    for (name, leg, second) in [
        ("c15_arc_flags_child_first", 0, false),
        ("c16_arc_flags_child_second", 0, true),
        ("c17_arc_flags_parent_first", 1, false),
        ("c18_arc_flags_parent_second", 1, true),
    ] {
        let mut m = Model::new();
        let parent = m.entity("Parent");
        for (child, flags) in [("Del", [1, 0, 0]), ("Upd", [0, 1, 0]), ("Both", [1, 1, 0]), ("Third", [0, 0, 1])] {
            let c = m.entity(child);
            let (_, arcs) = m.assoc_arcs(&format!("R_{child}"), &[(c, C11), (parent, C0N)]);
            m.arc_flags(arcs[leg], second, flags);
        }
        v.push((name, m));
    }

    // Which CUnParametre string holds the comment: a distinct marker in each
    let mut m = Model::new();
    let e = m.entity("Produit");
    let mut p = new_parametre("Prix");
    p.s98 = "MK_s98".into();
    p.s118 = "MK_s118".into();
    p.s138 = "MK_s138".into();
    p.sf0 = "MK_sf0".into();
    p.sf8 = "MK_sf8".into();
    p.s100 = "MK_s100".into();
    p.s120 = "MK_s120".into();
    p.s40 = "MK_s40".into();
    texte_mut(&mut m.doc, e, 1).unwrap().params.push(p);
    v.push(("c19_comment_fields", m));

    // Comments (CUnParametre+0xf8, owner text slot 2 lines), complement
    // (+0x118) and the "Autre" referential action with its text (CDrawArc+0x3d8)
    let mut m = Model::new();
    let e = m.entity("Article");
    let mut p = new_parametre("Libelle");
    p.f88 = 1;
    p.f8c = 1;
    p.s118 = "DEFAULT 'x'".into();
    p.sf8 = "Nom d'usage".into();
    texte_mut(&mut m.doc, e, 1).unwrap().params.push(p);
    let mut p = new_parametre("Prix");
    p.sf8 = "prix HT".into();
    texte_mut(&mut m.doc, e, 1).unwrap().params.push(p);
    texte_mut(&mut m.doc, e, 2).unwrap().lines = vec!["Ligne 1".into(), "  ".into(), " L'autre ligne ".into()];
    let f = m.entity("Famille");
    let (a, arcs) = m.assoc_arcs("Appartenir", &[(e, C11), (f, C0N)]);
    m.arc_flags(arcs[1], false, [0, 0, 1]);
    if let Some(ObjKind::Arc(arc)) = m.doc.objects.iter_mut().find(|o| o.base.id == arcs[1]).map(|o| &mut o.kind) {
        arc.label = "ON DELETE SET NULL".into();
    }
    let _ = a;
    let c = m.entity("Client");
    let t = m.assoc("Acheter", &[(c, C0N), (e, C0N)]);
    texte_mut(&mut m.doc, t, 2).unwrap().lines = vec!["Achats".into()];
    v.push(("c20_comments", m));

    // "Générer une table de correspondance" (CDrawRelation+0x3ac?) on x,1 associations
    let mut m = Model::new();
    let cmd = m.entity("Commande");
    let cli = m.entity("Client");
    let a = m.assoc("Passer", &[(cmd, C11), (cli, C0N)]);
    m.attr(a, "Date_commande", false, false, "");
    m.relation_flag(a, 1);
    let p = m.entity("Pays");
    let c = m.entity("Capitale");
    let a = m.assoc("Avoir", &[(p, C11), (c, C11)]);
    m.relation_flag(a, 1);
    let e = m.entity("Employe");
    let a = m.assoc("Diriger", &[(e, C01), (e, C0N)]);
    m.relation_flag(a, 1);
    v.push(("c21_forced_table", m));

    // Forced table, without cycles: which x,1 legs it applies to, its key
    let mut m = Model::new();
    let a1 = m.entity("A");
    let b1 = m.entity("B");
    for (name, ca, cb, flag) in [
        ("R01_0n", C01, C0N, 1),
        ("R01_1n", C01, C1N, 1),
        ("R11_0n", C11, C0N, 1),
        ("R01_01", C01, C01, 1),
        ("R01_0n_2", C01, C0N, 2),
    ] {
        let r = m.assoc(name, &[(a1, ca), (b1, cb)]);
        m.attr(r, &format!("x_{name}"), false, false, "");
        m.relation_flag(r, flag);
    }
    v.push(("c23_forced_table_legs", m));

    // Inheritance: constraint 0..3, "migration de toutes les rubriques" (+0x418)
    for (name, constraint, migrate_all) in
        [("c24_heritage", 0, false), ("c25_heritage_partition", 3, false), ("c26_heritage_migration", 0, true)]
    {
        let mut m = Model::new();
        let p = m.entity("Personne");
        m.attr(p, "Nom", true, false, "");
        let e = m.subtype("Etudiant", &["Ecole"]);
        let n = m.subtype("Enseignant", &["Grade"]);
        m.heritage(p, &[e, n], constraint, migrate_all);
        v.push((name, m));
    }

    // Subtypes in associations, a subtype with its own identifier, two levels
    let mut m = Model::new();
    let p = m.entity("Personne");
    m.attr(p, "Nom", false, false, "");
    let e = m.subtype("Etudiant", &["Ecole"]);
    let n = m.entity("Enseignant");
    m.attr(n, "Grade", false, false, "");
    m.heritage(p, &[e, n], 0, false);
    let d = m.subtype("Doctorant", &["These"]);
    m.heritage(e, &[d], 0, false);
    let c = m.entity("Cours");
    m.assoc("Suivre", &[(e, C0N), (c, C0N)]);
    m.assoc("Assurer", &[(c, C11), (n, C0N)]);
    v.push(("c27_heritage_links", m));

    // CIF on a ternary association: linked to the association and to the
    // target (Professeur), with or without explicit emitters
    for (name, emitters) in [("c28_cif_ternary", true), ("c29_cif_no_emitter", false)] {
        let mut m = Model::new();
        let p = m.entity("Professeur");
        let c = m.entity("Classe");
        let t = m.entity("Matiere");
        let a = m.assoc("Enseigner", &[(p, C0N), (c, C1N), (t, C0N)]);
        m.attr(a, "Heures", false, false, "");
        let at = m.at();
        let cif = add_cif(&mut m.doc, at);
        add_cif_link(&mut m.doc, a, cif, false);
        add_cif_link(&mut m.doc, p, cif, true);
        if emitters {
            add_cif_link(&mut m.doc, c, cif, false);
            add_cif_link(&mut m.doc, t, cif, false);
        }
        v.push((name, m));
    }

    // CIF on a 4-ary association with two emitters out of three
    let mut m = Model::new();
    let es = ["Vol", "Pilote", "Avion", "Jour"].map(|n| m.entity(n));
    let a = m.assoc("Affecter", &[(es[0], C0N), (es[1], C0N), (es[2], C0N), (es[3], C0N)]);
    let at = m.at();
    let cif = add_cif(&mut m.doc, at);
    add_cif_link(&mut m.doc, a, cif, false);
    add_cif_link(&mut m.doc, es[2], cif, true);
    add_cif_link(&mut m.doc, es[0], cif, false);
    add_cif_link(&mut m.doc, es[3], cif, false);
    v.push(("c30_cif_4ary", m));

    // SQL put around the tables: document complements (0xea0 strings), rules
    // before / after, inheritance and constraint scripts
    let mut m = Model::new();
    m.doc.strings_ea0 =
        ["-- DEBUT".into(), "-- FIN".into(), "-- debut [Nom_Table]".into(), "-- fin [Nom_Table]".into(), String::new()];
    let p = m.entity("Personne");
    let e = m.subtype("Etudiant", &["Ecole"]);
    let h = m.heritage(p, &[e], 0, false);
    set_script(&mut m.doc, h, &["-- script heritage"], 0);
    let at = m.at();
    add_regle(
        &mut m.doc,
        "R1",
        at,
        &["Regle avant", "ligne 2"],
        &["INSERT INTO Personne VALUES (1);"],
        false,
        [1, 1, 1],
    );
    let at = m.at();
    add_regle(&mut m.doc, "R2", at, &["Regle apres"], &["-- apres"], true, [1, 0, 1]);
    let at = m.at();
    let k = add_contrainte(&mut m.doc, "X", at, 0);
    set_script(&mut m.doc, k, &["-- script contrainte"], 0);
    v.push(("c31_scripts", m));

    // Rule flags one by one; script order by position (+0x4d8)
    let mut m = Model::new();
    m.entity("T");
    for (i, flags) in [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]].into_iter().enumerate() {
        let at = m.at();
        let (d, sc) = (format!("D{i}"), format!("-- S{i}"));
        add_regle(&mut m.doc, &format!("R{i}"), at, &[&d], &[&sc], false, flags);
    }
    for (name, pos) in [("P2", 2), ("P1", 1)] {
        let at = m.at();
        let sc = format!("-- {name}");
        let r = add_regle(&mut m.doc, name, at, &[], &[&sc], true, [0, 0, 0]);
        set_script(&mut m.doc, r, &[&sc], pos);
    }
    v.push(("c32_regle_flags", m));

    // link options of the role parameter
    let mut m = Model::new();
    let e = m.entity("Employe");
    let (_, arcs) = m.assoc_arcs("Diriger", &[(e, C01), (e, C0N)]);
    m.role_with(arcs[1], "chef", false, false);
    let p = m.entity("Projet");
    let (_, arcs) = m.assoc_arcs("Piloter", &[(p, C11), (e, C0N)]);
    m.role_with(arcs[1], "pilote", false, false);
    v.push(("c33_role_no_suffix", m));

    let mut m = Model::new();
    let e = m.entity("Personne");
    let (_, arcs) = m.assoc_arcs("Diriger", &[(e, C01), (e, C0N)]);
    m.role_with(arcs[1], "chef", false, true);
    let v1 = m.entity_with_key("Vol", &["Num_vol", "Date_vol"]);
    let (_, arcs) = m.assoc_arcs("Reserver", &[(e, C0N), (v1, C0N)]);
    m.role_with(arcs[1], "vol_reserve", false, true);
    m.role_with(arcs[0], "voyageur", true, true);
    v.push(("c34_role_rename", m));

    // "Place clé étrangère dans clé primaire associée" (+0x14c)
    let mut m = Model::new();
    let c = m.entity_with_key("Commande", &["Num_commande"]);
    let pr = m.entity_with_key("Produit", &["Ref_produit"]);
    let l = m.entity_with_key("Ligne", &["Num_ligne"]);
    let (_, a1) = m.assoc_arcs("Contenir", &[(c, C1N), (l, C11)]);
    let (_, a2) = m.assoc_arcs("Concerner", &[(pr, C0N), (l, C11)]);
    m.relative(a1[1]);
    m.relative(a2[1]);
    m.leg_param(a1[1], |p| p.f14c = 2);
    m.leg_param(a2[1], |p| p.f14c = 1);
    v.push(("c35_relative_place", m));

    let mut m = Model::new();
    let c = m.entity_with_key("Commande", &["Num_commande"]);
    let pr = m.entity_with_key("Produit", &["Ref_produit"]);
    let (_, a1) = m.assoc_arcs("Inclure", &[(c, C0N), (pr, C0N)]);
    m.leg_param(a1[0], |p| p.f14c = 2);
    m.leg_param(a1[1], |p| p.f14c = 1);
    let cl = m.entity("Client");
    let (_, a2) = m.assoc_arcs("Passer", &[(c, C11), (cl, C0N)]);
    m.leg_param(a2[0], |p| p.f148 = 1);
    let f = m.entity("Facture");
    let (_, a3) = m.assoc_arcs("Regler", &[(f, C11), (c, C0N)]);
    m.leg_param(a3[1], |p| p.f148 = 1);
    v.push(("c36_place_and_recup", m));

    // complements in the VBA module: which name ends up in the stray
    // "table end" line before the tables
    let complements =
        || ["-- D".into(), "-- F".into(), "-- debut [Nom_Table]".into(), "-- fin [Nom_Table]".into(), String::new()];
    let mut m = Model::new();
    m.doc.strings_ea0 = complements();
    m.entity("Alpha");
    m.entity("Beta");
    v.push(("c42_complements_only", m));

    let mut m = Model::new();
    m.doc.strings_ea0 = complements();
    m.entity("Alpha");
    let at = m.at();
    add_contrainte(&mut m.doc, "K1", at, 0);
    let at = m.at();
    add_regle(&mut m.doc, "R9", at, &[], &["-- r9"], true, [0, 0, 0]);
    m.entity("Beta");
    v.push(("c43_complements_objects", m));

    // free objects: they must not disturb the MLD, and Looping must open them
    let mut m = Model::new();
    let a = m.entity("Article");
    let f = m.entity("Famille");
    m.assoc("Classer", &[(a, C11), (f, C0N)]);
    add_free_line(&mut m.doc, &[Point { x: 40, y: 400 }, Point { x: 200, y: 460 }, Point { x: 320, y: 400 }], true);
    add_free_line(&mut m.doc, &[Point { x: 40, y: 520 }, Point { x: 320, y: 520 }], false);
    v.push(("c41_free_lines", m));

    // illustrations (milestone 5, step 3): each kind with the document's
    // last-used values, some properties changed as their dialogs do, a
    // labelled link between an illustration and an entity
    let mut m = Model::new();
    let a = m.entity("Article");
    let f = m.entity("Famille");
    m.assoc("Classer", &[(a, C11), (f, C0N)]);
    let rect = |l, t, r, b| Rect { left: l, top: t, right: r, bottom: b };
    let r1 = add_rectangle(&mut m.doc, rect(20, 300, 220, 400));
    let r2 = add_rectangle(&mut m.doc, rect(260, 300, 460, 400));
    let e1 = add_ellipse(&mut m.doc, rect(500, 300, 700, 400));
    let t1 = add_texte(&mut m.doc, rect(20, 440, 220, 520), &["Un texte", "sur deux lignes"]);
    let t2 = add_texte(&mut m.doc, rect(260, 440, 460, 520), &["Aligné à gauche"]);
    add_image(&mut m.doc, rect(500, 440, 700, 540), false);
    add_image(&mut m.doc, rect(740, 440, 900, 540), true);
    add_crayon(
        &mut m.doc,
        &[Point { x: 740, y: 300 }, Point { x: 770, y: 340 }, Point { x: 800, y: 310 }, Point { x: 860, y: 380 }],
    );
    add_free_line(&mut m.doc, &[Point { x: 20, y: 580 }, Point { x: 300, y: 600 }], true);
    let link = add_illustration_link(&mut m.doc, r1, a);
    let mut p = new_parametre("");
    p.role = 6;
    p.s120 = "illustre".into();
    texte_mut(&mut m.doc, link, 1).unwrap().params.push(p);
    for o in m.doc.objects.iter_mut() {
        let id = o.base.id;
        match &mut o.kind {
            // rounded, thick, dashed, orange outline, grey fill with a shadow
            ObjKind::Rectangle(x) if id == r2 => {
                (x.f3a0, x.f3a4, x.f3b0, x.f3a8) = (3, 1, 40, 4);
                o.base.f308[2] = 1;
                for st in &mut o.base.styles {
                    st.pen.color = 0x0080ff;
                    st.brush.color = 0xe9e9e9;
                    st.brush2.color = 0xe9e9e9;
                }
            }
            ObjKind::Ellipse(x) if id == e1 => {
                x.f3a0 = 2;
                o.base.f308[2] = 1;
            }
            // own values instead of the document's: left aligned, no frame
            ObjKind::Titre(x) if id == t2 => {
                o.base.f318 = 0;
                x.f390 = 0;
                o.base.f308[1] = 0;
                o.base.f308[3] = 0;
            }
            _ => {}
        }
    }
    let _ = t1;
    v.push(("c48_illustrations", m));

    // display state (milestone 5, step 4): two views sharing the entities,
    // half-grid magnetism, data types shown, « Flèche pour DF », 80 %
    let mut m = Model::new();
    let a = m.entity("Article");
    let f = m.entity("Famille");
    let s = m.entity("Stock");
    m.assoc("Classer", &[(a, C11), (f, C0N)]);
    m.assoc("Stocker", &[(a, C0N), (s, C0N)]);
    let catalogue = 1 << add_view(&mut m.doc, "Catalogue").unwrap();
    let logistique = 1 << add_view(&mut m.doc, "Logistique").unwrap();
    for (id, mask) in [(a, catalogue | logistique), (f, catalogue), (s, logistique)] {
        set_views(&mut m.doc, id, mask, true);
    }
    m.doc.f33c = 1;
    m.doc.f340 = 1;
    m.doc.settings.set_i32(0x544, 1);
    m.doc.settings.set_i32(0x7c0, 1);
    m.doc.frame_state[0] = [80, 80];
    v.push(("c49_views", m));

    // « Compléments script LDD » on several lines, as the dialog's multi-line
    // edit controls store them (CRLF)
    let mut m = Model::new();
    m.doc.strings_ea0 = [
        "-- debut 1\r\n-- debut 2".into(),
        "-- fin 1\r\n-- fin 2".into(),
        "-- table [Nom_Table]\r\n-- suite".into(),
        "-- fin table [Nom_Table]\r\n-- fin suite".into(),
        String::new(),
    ];
    m.entity("Alpha");
    v.push(("c50_complements_multiline", m));

    for (name, places) in [
        ("c37_place_11", [1, 1, 1]),
        ("c38_place_12", [1, 2, 1]),
        ("c39_place_312", [3, 1, 2]),
        ("c40_place_213", [2, 1, 3]),
    ] {
        let mut m = Model::new();
        let c = m.entity_with_key("Commande", &["Num_commande"]);
        let pr = m.entity_with_key("Produit", &["Ref_produit"]);
        let d = m.entity_with_key("Depot", &["Code_depot"]);
        let l = m.entity_with_key("Ligne", &["Num_ligne"]);
        let (_, a1) = m.assoc_arcs("Contenir", &[(c, C1N), (l, C11)]);
        let (_, a2) = m.assoc_arcs("Concerner", &[(pr, C0N), (l, C11)]);
        m.relative(a1[1]);
        m.relative(a2[1]);
        m.leg_param(a1[1], |p| p.f14c = places[0]);
        m.leg_param(a2[1], |p| p.f14c = places[1]);
        let (_, a3) = m.assoc_arcs("Stocker", &[(c, C0N), (pr, C0N), (d, C0N)]);
        for (arc, place) in a3.iter().zip(places) {
            m.leg_param(*arc, |p| p.f14c = place);
        }
        v.push((name, m));
    }

    // logical names (dialogs « Entité » / « Rubrique »): `CDrawTexte+0x5f8`
    // of the name text and `CUnParametre+0x18`; free type `+0x40` with a
    // description after « # »
    let mut m = Model::new();
    let c = m.entity("Client");
    m.attr(c, "Nom client", true, false, "");
    m.attr(c, "Code postal", false, true, "");
    let cmd = m.entity("Commande");
    m.assoc("Passer", &[(c, C0N), (cmd, C11)]);
    texte_mut(&mut m.doc, c, 0).unwrap().s5f8 = "T_CLIENT".into();
    {
        let t = texte_mut(&mut m.doc, c, 1).unwrap();
        t.params[0].s18 = "Num Cli".into();
        t.params[1].s18 = "LIB_NOM".into();
        t.params[2].f20 = 4;
        t.params[2].s40 = "CHAR(5) # code INSEE".into();
    }
    v.push(("c44_logical_names", m));

    // links of rules, constraints and CIFs with their dialog fields
    // (parameter of role 4 / 5: +0x114 target, +0x120 label, +0x128 smoothing)
    let mut m = Model::new();
    let e1 = m.entity("Etudiant");
    let e2 = m.entity("Cours");
    let a1 = m.assoc("Suivre", &[(e1, C0N), (e2, C0N)]);
    let a2 = m.assoc("Enseigner", &[(e1, C0N), (e2, C0N)]);
    let at = m.at();
    let k = add_contrainte(&mut m.doc, "X1", at, 0);
    for a in [a1, a2] {
        let l = add_contrainte_link(&mut m.doc, a, k);
        let mut p = new_parametre("");
        p.role = 4;
        p.s120 = "lien K".into();
        texte_mut(&mut m.doc, l, 1).unwrap().params.push(p);
    }
    let at = m.at();
    let r = add_regle(&mut m.doc, "R1", at, &["Un étudiant suit au plus 5 cours"], &["-- r1"], true, [1, 1, 1]);
    let l = add_regle_link(&mut m.doc, e1, r);
    let mut p = new_parametre("");
    p.role = 5;
    p.s120 = "règle".into();
    texte_mut(&mut m.doc, l, 1).unwrap().params.push(p);
    v.push(("c45_rule_constraint_links", m));

    // editing operations of the application: Looping must open the result
    // (copy/paste with new ids, break point, order, alignment, resize)
    let mut m = Model::new();
    let a = m.entity("Auteur");
    let b = m.entity("Livre");
    let (r, arcs) = m.assoc_arcs("Ecrire", &[(a, C1N), (b, C1N)]);
    looping_core::edit::add_break(&mut m.doc, arcs[0], Point { x: 120, y: 260 });
    let clip = looping_core::edit::copy(&m.doc, &[a, b, r]);
    let new = looping_core::edit::paste(&mut m.doc, &clip, 0, 300);
    looping_core::edit::rename(&mut m.doc, new[0], "Auteur2");
    looping_core::edit::rename(&mut m.doc, new[1], "Livre2");
    looping_core::edit::rename(&mut m.doc, new[2], "Ecrire2");
    looping_core::edit::reorder(&mut m.doc, &[r], looping_core::edit::Order::Front);
    looping_core::edit::align(&mut m.doc, &[a, b], looping_core::edit::Alignment::Top);
    looping_core::edit::resize(&mut m.doc, b, loo_format::Rect { left: 500, top: 0, right: 700, bottom: 120 });
    v.push(("c46_edit_ops", m));

    // inheritance between entities without identifier (found by hand
    // testing): the subtype keeps its own missing key besides the migrated
    // one, and the MLD text groups homonymous key columns by position
    let mut m = Model::new();
    let sup = m.entity("Personne");
    let sub = m.entity("Client");
    let other = m.entity("Produit");
    for e in [sup, sub, other] {
        texte_mut(&mut m.doc, e, 1).unwrap().params.clear();
        m.attr(e, "nom", false, false, "");
    }
    let at = m.at();
    let h = add_heritage(&mut m.doc, at, 0);
    add_heritage_link(&mut m.doc, sup, h, true);
    add_heritage_link(&mut m.doc, sub, h, false);
    m.assoc("Acheter", &[(sub, C0N), (other, C0N)]);
    v.push(("c47_heritage_no_key", m));

    // Entite flags bit 1 (0x4f4): "appartenant à un modèle externe"?
    let mut m = Model::new();
    let ext = m.entity("Externe");
    let loc = m.entity("Local");
    m.assoc("Utiliser", &[(loc, C11), (ext, C0N)]);
    let o = m.doc.objects.iter_mut().find(|o| o.base.id == ext).unwrap();
    if let ObjKind::Entite(en) = &mut o.kind {
        en.flags |= 2;
    }
    v.push(("c22_external_entity", m));

    v
}

fn main() {
    let out = std::env::args().nth(1).expect("usage: cases OUT_DIR");
    std::fs::create_dir_all(&out).unwrap();
    for (name, m) in cases() {
        std::fs::write(format!("{out}/{name}.loo"), m.doc.save().unwrap()).unwrap();
    }
}
