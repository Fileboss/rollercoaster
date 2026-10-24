//! Object dialogs, one per kind of object as in Looping (opened by a
//! double-click): they edit copies of the object and of its texts, applied
//! together by « OK » (one undo step) or dropped by « Annuler ».
//!
//! Fields and where Looping keeps them (dialog classes of `Looping.exe`):
//! * Entité / Association (`CEntiteRelationDlg`, 165 / 167): name = slot-0
//!   text, logical name `CDrawTexte+0x5f8` of that text, comment = slot-2
//!   lines, attributes = slot-1 parameters; `CDrawEntite+0x4f0` fictive /
//!   `+0x4f4` external model; `CDrawRelation+0x3ac` forced table.
//! * Rubrique (`CDlgRubrique`, 172): `CUnParametre` name `+0x10`, logical
//!   name `+0x18`, type (`+0x20` category, `+0x24`/`+0x28` text kind and
//!   charset, `+0x2c..+0x38`, lengths `+0x64..+0x78`, collations
//!   `+0x48..+0x58`, `+0x7c`/`+0x80` precision/scale, `+0x60` time zone, free
//!   text `+0x40`), identifier `+0x08`, place in key `+0x14c`, NOT NULL
//!   `+0x88`, UNIQUE `+0x8c`, other index `+0x138`, complement `+0x118`,
//!   comment `+0xf8`.
//! * Lien (`CLienDlg`, 173…196): cardinality parameter (role 1) and role
//!   parameter (role 2) of the first segment, actions `CDrawArc+0x3c8..`.
//! * Héritage, Contrainte, Règle: parameter `+0x110` (kind), script lines
//!   `CDrawTexte+0x4a0` and position `+0x4d8` of the slot-1 text;
//!   `CDrawHeritage+0x418`; `CDrawRegle+0x3a0` (show description), `+0x3a4`
//!   (show script), `+0x3a8` (description as comment), `+0x3ac` (after the
//!   tables), description = slot-1 parameters.
//! * Lien héritage (`CLienHeritageDlg`, `0x1400304b0`): parameter of role 3,
//!   `+0x114` supertype, `+0x120` criterion, the role options of a link.
//! * Liens de CIF / contrainte / règle (`CLienCifDlg` …): one parameter,
//!   role 4 (5 for a rule): `+0x114` target, `+0x120` label, `+0x128`
//!   smoothing.
//!
//! Illustrations (`CDlgTexteForme` and its subclasses; common part loaded
//! by 0x14003ee80 and applied by 0x14003fd00, then each class's own fields
//! by its vtable `+0x318` / `+0x340`):
//! * Rectangle, Ellipse, Image (`CRectangleDlg` 182, `CEllipseDlg` 183,
//!   `CImageDlg` 203 / 204): outline shown `+0x314`, fill shown `+0x310`,
//!   pen and fill colours; rectangle and ellipse: width `+0x3a0`, style
//!   `+0x3a4`, shadow `+0x3a8` in `+0x3ac`, rounded corners `+0x3b0`
//!   (rectangle); image: width `+0x3a8`, style `+0x3ac`, shadow `+0x3b0` in
//!   `+0x3b4`, file `+0x390`, and for a « Vignette » (`+0x3c0`) the model
//!   `+0x3b8`.
//! * Texte (`CTitreDlg`, 171, on a `CDrawTitre`): lines and font of its
//!   slot-0 text, alignments `+0x318` / `+0x390`; each property with its
//!   « Par défaut » flag (document values of the « Textes » section):
//!   outline `+0x30c`/`+0x314`, width `+0x3b8`/`+0x3ac`, pen colour style
//!   `a`, corners `+0x3a4`/`+0x3a8`, shadow `+0x3bc`/`+0x3b0` and its colour
//!   `+0x3c0`/`+0x3b4`, fill `+0x308`/`+0x310`, fill colour style `b`.
//! * Ligne, Flèche (`CTraitDlg` 185, `CFlecheDlg` 184): width `+0x3a8`,
//!   style `+0x3ac`, arrow size `+0x3b0` and shape `+0x3b4`, smoothing
//!   `+0x3b8` (only with break points), applied to every segment.
//! * Crayon (`CCrayonDlg`, 191): width `+0x464`, style `+0x468`.
//! * Lien entre illustrations (`CLienObjetLibreDlg`, 186): parameter of
//!   role 6, label `+0x120`, smoothing `+0x128`.

use eframe::egui;
use loo_format::{Document, DrawObj, ObjKind, Parametre, Texte};
use looping_core::{edit, mcd::ObjId};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Entite,
    Relation,
    Lien,
    Heritage,
    LienHeritage,
    Contrainte,
    LienContrainte,
    Cif,
    LienCif,
    Regle,
    LienRegle,
    Rectangle,
    Ellipse,
    Texte,
    Image,
    Vignette,
    Trait,
    Fleche,
    Crayon,
    LienIllustration,
}

impl Kind {
    /// Illustrations: their « OK » also keeps their look for the next ones.
    pub fn is_illustration(self) -> bool {
        matches!(
            self,
            Kind::Rectangle
                | Kind::Ellipse
                | Kind::Texte
                | Kind::Image
                | Kind::Vignette
                | Kind::Trait
                | Kind::Fleche
                | Kind::Crayon
        )
    }

    fn title(self) -> &'static str {
        match self {
            Kind::Entite => "Entité",
            Kind::Relation => "Association",
            Kind::Lien => "Lien",
            Kind::Heritage => "Héritage",
            Kind::LienHeritage => "Lien héritage",
            Kind::Contrainte => "Contrainte inter-associations",
            Kind::LienContrainte => "Lien contrainte inter-associations",
            Kind::Cif => "Contrainte d'intégrité fonctionnelle (CIF)",
            Kind::LienCif => "Lien contrainte d'intégrité fonctionnelle",
            Kind::Regle => "Règle",
            Kind::LienRegle => "Lien règle",
            Kind::Rectangle => "Rectangle",
            Kind::Ellipse => "Ellipse",
            Kind::Texte => "Texte",
            Kind::Image => "Image",
            Kind::Vignette => "Vignette",
            Kind::Trait => "Ligne",
            Kind::Fleche => "Flèche",
            Kind::Crayon => "Crayon",
            Kind::LienIllustration => "Lien entre illustrations",
        }
    }
}

pub enum Outcome {
    Open,
    Apply(Vec<DrawObj>),
    Close,
    /// « Ouvrir le modèle » of a sub-model: its path as stored.
    OpenModel(String),
}

pub struct Dialog {
    pub kind: Kind,
    /// The object, or the first segment of a link.
    pub target: ObjId,
    objs: Vec<DrawObj>,
    /// Attribute open in the « Rubrique » dialog: index in the slot-1 params
    /// (`None` with a draft = new attribute) and its draft.
    rubrique: Option<(Option<usize>, Parametre)>,
    selected: Option<usize>,
    replace: (String, String),
    /// Names of the two ends (links).
    ends: (String, String),
}

fn first_segment(doc: &Document, id: ObjId) -> ObjId {
    let mut cur = id;
    let mut guard = 0;
    while let Some(p) = doc.object(cur).map(|o| o.base.links.prev).filter(|&p| p != 0) {
        cur = p;
        guard += 1;
        if guard > 1000 {
            break;
        }
    }
    cur
}

fn last_segment(doc: &Document, first: ObjId) -> ObjId {
    let mut cur = first;
    let mut guard = 0;
    while let Some(n) = doc.object(cur).map(|o| o.base.links.next).filter(|&n| n != 0) {
        cur = n;
        guard += 1;
        if guard > 1000 {
            break;
        }
    }
    cur
}

impl Dialog {
    /// Dialog for the object `id` (a link is edited from its first segment).
    pub fn open(doc: &Document, id: ObjId) -> Option<Dialog> {
        let mut obj = doc.object(id)?;
        if obj.class == "CDrawTexte" {
            obj = doc.object(obj.base.links.owner)?;
        }
        let is_link = obj.class.starts_with("CDrawArc");
        // links and free lines are edited from their first segment
        let chained = is_link || matches!(obj.kind, ObjKind::Trait(_));
        let target = if chained { first_segment(doc, obj.id()) } else { obj.id() };
        let obj = doc.object(target)?;
        let kind = match (&obj.kind, obj.class.as_str()) {
            (ObjKind::Entite(_), "CDrawEntite") => Kind::Entite,
            (ObjKind::Relation(_), _) => Kind::Relation,
            (ObjKind::Heritage(_), _) => Kind::Heritage,
            (ObjKind::Contrainte(_), _) => Kind::Contrainte,
            (ObjKind::Cif(_), _) => Kind::Cif,
            (ObjKind::Regle(_), _) => Kind::Regle,
            (_, "CDrawArc") => Kind::Lien,
            (_, "CDrawArcHeritage") => Kind::LienHeritage,
            (_, "CDrawArcContrainte") => Kind::LienContrainte,
            (_, "CDrawArcCif") => Kind::LienCif,
            (_, "CDrawArcRegle") => Kind::LienRegle,
            (ObjKind::Rectangle(_), _) => Kind::Rectangle,
            (ObjKind::Ellipse(_), _) => Kind::Ellipse,
            (ObjKind::Titre(_), _) => Kind::Texte,
            (ObjKind::Image(i), _) if i.f3c0 != 0 => Kind::Vignette,
            (ObjKind::Image(_), _) => Kind::Image,
            (ObjKind::Trait(_), "CDrawFleche") => Kind::Fleche,
            (ObjKind::Trait(_), _) => Kind::Trait,
            (ObjKind::Crayon(_), _) => Kind::Crayon,
            (_, "CDrawArcObjetLibre") => Kind::LienIllustration,
            _ => return None,
        };
        let mut objs = vec![obj.clone()];
        objs.extend(obj.base.links.texts.iter().filter_map(|&t| doc.object(t)).cloned());
        let ends = if is_link {
            let name = |o: ObjId| doc.object(o).map(|o| looping_core::mcd::object_name(doc, o)).unwrap_or_default();
            let last = last_segment(doc, target);
            (name(obj.base.links.owner), name(doc.object(last).map_or(0, |o| o.base.links.owner)))
        } else {
            Default::default()
        };
        Some(Dialog { kind, target, objs, rubrique: None, selected: None, replace: Default::default(), ends })
    }

    pub fn show(&mut self, ctx: &egui::Context, doc: &Document) -> Outcome {
        let mut outcome = Outcome::Open;
        let mut open = true;
        egui::Window::new(self.kind.title())
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(460.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(560.0).show(ui, |ui| match self.kind {
                    Kind::Entite | Kind::Relation => self.entite_relation(ui, doc),
                    Kind::Lien => self.lien(ui),
                    Kind::Heritage => self.heritage(ui),
                    Kind::LienHeritage => self.lien_heritage(ui),
                    Kind::Contrainte => self.contrainte(ui),
                    Kind::Cif => self.cif(ui),
                    Kind::Regle => self.regle(ui),
                    Kind::LienContrainte | Kind::LienCif | Kind::LienRegle | Kind::LienIllustration => {
                        self.lien_simple(ui)
                    }
                    Kind::Rectangle | Kind::Ellipse | Kind::Image | Kind::Vignette => {
                        if let Some(o) = self.figure(ui) {
                            outcome = o;
                        }
                    }
                    Kind::Texte => self.texte(ui, doc),
                    Kind::Trait | Kind::Fleche => self.trait_(ui, doc),
                    Kind::Crayon => self.crayon(ui),
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        outcome = Outcome::Apply(self.objs.clone());
                    }
                    if ui.button("Annuler").clicked() {
                        outcome = Outcome::Close;
                    }
                });
            });
        if !open {
            outcome = Outcome::Close;
        }
        // the « Rubrique » dialog, on top of the entity / association one
        if let Some((index, mut draft)) = self.rubrique.take() {
            let mut keep = true;
            let mut apply = false;
            egui::Window::new("Rubrique").collapsible(false).resizable(true).default_width(440.0).show(ctx, |ui| {
                rubrique(ui, &mut draft, self.kind == Kind::Entite);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        apply = true;
                        keep = false;
                    }
                    if ui.button("Annuler").clicked() {
                        keep = false;
                    }
                });
            });
            if apply {
                if let Some(t) = slot(&mut self.objs, 1) {
                    match index {
                        Some(i) if i < t.params.len() => t.params[i] = draft.clone(),
                        _ => t.params.push(draft.clone()),
                    }
                }
            }
            if keep {
                self.rubrique = Some((index, draft));
            }
        }
        outcome
    }

    // ---- Entité / Association (165 / 167) ----

    fn entite_relation(&mut self, ui: &mut egui::Ui, doc: &Document) {
        let entity = self.kind == Kind::Entite;
        name_fields(ui, slot(&mut self.objs, 0));
        let has_x1_leg = self.has_x1_leg(doc);
        match &mut self.objs[0].kind {
            ObjKind::Entite(e) => {
                let mut external = e.is_external();
                if ui.checkbox(&mut external, "Classe d'entités appartenant à un modèle externe").changed() {
                    e.flags = (e.flags & !2) | ((external as u16) << 1);
                }
            }
            ObjKind::Relation(r) => {
                ui.add_enabled_ui(has_x1_leg, |ui| {
                    let mut forced = r.f3ac != 0;
                    if ui
                        .checkbox(&mut forced, "Générer une table de correspondance dans le MLD")
                        .on_disabled_hover_text("Seulement pour une association avec une patte x,1")
                        .changed()
                    {
                        r.f3ac = forced as i32;
                    }
                });
            }
            _ => {}
        }
        ui.separator();
        ui.strong("Rubriques");
        self.attribute_list(ui, entity, doc);
        ui.separator();
        ui.strong("Commentaire");
        if let Some(t) = slot(&mut self.objs, 2) {
            lines_edit(ui, &mut t.lines, 3);
        }
        if let ObjKind::Entite(e) = &mut self.objs[0].kind {
            ui.separator();
            ui.strong("Génération MLD");
            let mut fictive = e.is_fictive();
            if ui
                .checkbox(
                    &mut fictive,
                    "Classe d'entités fictive (ne pas générer de table pour cette classe dans le MLD)",
                )
                .changed()
            {
                e.flags = (e.flags & !1) | fictive as u16;
            }
        }
    }

    /// An association with a leg of maximum cardinality 1 (the forced table
    /// option is only offered then, `0x1400319f0`).
    fn has_x1_leg(&self, doc: &Document) -> bool {
        looping_core::render::relation_is_functional(doc, &self.objs[0])
            || matches!(&self.objs[0].kind, ObjKind::Relation(r) if r.f3ac != 0)
    }

    fn attribute_list(&mut self, ui: &mut egui::Ui, entity: bool, doc: &Document) {
        let class_name =
            slot(&mut self.objs, 0).map(|t| t.lines.first().cloned().unwrap_or_default()).unwrap_or_default();
        let Some(t) = slot(&mut self.objs, 1) else { return };
        let mut open_index: Option<usize> = None;
        egui::Grid::new("attributes").striped(true).num_columns(6).show(ui, |ui| {
            if entity {
                ui.label("Id");
            }
            for h in ["Nom conceptuel", "Nom logique", "Type", "NOT NULL", "UNIQUE"] {
                ui.label(h);
            }
            ui.end_row();
            for (i, p) in t.params.iter_mut().enumerate().filter(|(_, p)| p.role == 0) {
                if entity {
                    check_i32(ui, &mut p.f8, "");
                }
                let sel = self.selected == Some(i);
                let r = ui.selectable_label(sel, if p.name.is_empty() { "(sans nom)" } else { &p.name });
                if r.clicked() {
                    self.selected = Some(i);
                }
                if r.double_clicked() {
                    open_index = Some(i);
                }
                ui.label(&p.s18);
                ui.label(looping_core::DataType::from_param(p).label());
                check_i32(ui, &mut p.f88, "");
                check_i32(ui, &mut p.f8c, "");
                ui.end_row();
            }
        });
        let n = t.params.len();
        let sel = self.selected.filter(|&i| i < n && t.params[i].role == 0);
        ui.horizontal_wrapped(|ui| {
            if ui.button("Ajouter").clicked() {
                let k = t.params.iter().filter(|p| p.role == 0).count() + 1;
                self.rubrique = Some((None, edit::new_parametre(&format!("rubrique_{k}"))));
            }
            if ui.add_enabled(sel.is_some(), egui::Button::new("Dupliquer")).clicked() {
                if let Some(i) = sel {
                    let mut p = t.params[i].clone();
                    p.f8 = 0;
                    t.params.insert(i + 1, p);
                    self.selected = Some(i + 1);
                }
            }
            if ui.add_enabled(sel.is_some(), egui::Button::new("Modifier")).clicked() {
                open_index = sel;
            }
            if ui.add_enabled(sel.is_some(), egui::Button::new("Supprimer")).clicked() {
                if let Some(i) = sel {
                    t.params.remove(i);
                    self.selected = None;
                }
            }
            if ui.add_enabled(sel.is_some_and(|i| i > 0), egui::Button::new("⏶ Monter")).clicked() {
                if let Some(i) = sel {
                    t.params.swap(i, i - 1);
                    self.selected = Some(i - 1);
                }
            }
            if ui.add_enabled(sel.is_some_and(|i| i + 1 < n), egui::Button::new("⏷ Descendre")).clicked() {
                if let Some(i) = sel {
                    t.params.swap(i, i + 1);
                    self.selected = Some(i + 1);
                }
            }
            if entity
                && ui
                    .button("Clé auto")
                    .on_hover_text("Ajoute un identifiant compteur nommé selon le modèle d'identifiant")
                    .clicked()
            {
                t.params.insert(0, edit::auto_identifier(doc, &class_name));
            }
        });
        // « Renommer » (dialog 208): replace characters in the attribute names
        ui.horizontal(|ui| {
            ui.label("Remplacer");
            ui.add(egui::TextEdit::singleline(&mut self.replace.0).desired_width(60.0));
            ui.label("par");
            ui.add(egui::TextEdit::singleline(&mut self.replace.1).desired_width(60.0));
            if ui.add_enabled(!self.replace.0.is_empty(), egui::Button::new("Renommer les rubriques")).clicked() {
                for p in t.params.iter_mut().filter(|p| p.role == 0) {
                    p.name = p.name.replace(&self.replace.0, &self.replace.1);
                }
            }
        });
        if let Some(i) = open_index {
            if let Some(p) = t.params.get(i) {
                self.rubrique = Some((Some(i), p.clone()));
            }
        }
    }

    // ---- Lien (173…196) ----

    fn lien(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("Entre « {} » et « {} »", self.ends.0, self.ends.1));
        let Some(t) = slot(&mut self.objs, 1) else { return };
        let card = param_with_role(t, 1);
        ui.strong("Cardinalité");
        ui.horizontal(|ui| {
            for (code, text) in [(0, "0,1"), (1, "1,1"), (2, "0,n"), (3, "1,n"), (4, "Autre")] {
                ui.radio_value(&mut card.cardinality, code, text);
            }
        });
        if card.cardinality == 4 {
            ui.horizontal(|ui| {
                ui.label("Min :");
                ui.add(egui::TextEdit::singleline(&mut card.card_min).desired_width(40.0));
                ui.label("Max :");
                ui.add(egui::TextEdit::singleline(&mut card.card_max).desired_width(40.0));
            });
        }
        check_i32(ui, &mut card.f84, "Identifiant relatif");
        check_i32(ui, &mut card.f148, "Récup clé primaire");
        check_i32(ui, &mut card.f140, "Entité ciblée par CIF");
        ui.horizontal(|ui| {
            ui.label("Place clé étrangère dans clé primaire associée :");
            ui.add(egui::DragValue::new(&mut card.f14c).range(1..=99));
        });
        check_i32(ui, &mut card.f144, "Clé étrangère dans index");
        check_i32(ui, &mut card.f128, "Lissage cassure");
        ui.separator();
        ui.strong("Rôle");
        let role = param_with_role(t, 2);
        ui.horizontal(|ui| {
            ui.label("Libellé :");
            ui.text_edit_singleline(&mut role.sf0);
        });
        ui.horizontal(|ui| {
            check_inverted(ui, &mut role.n12c, "Afficher");
            check_inverted(ui, &mut role.n130, "Suffixer clés étrangères");
            check_i32(ui, &mut role.f134, "Renommer clé étrangère");
        });
        // keep an empty role parameter out of the file
        if role.sf0.is_empty() && role.n12c == 0 && role.n130 == 0 && role.f134 == 0 {
            t.params.retain(|p| p.role != 2);
        }
        ui.separator();
        self.actions(ui);
    }

    /// « Complément SQL pour clés étrangères » (`CDrawArc+0x3c8..+0x3d8`).
    fn actions(&mut self, ui: &mut egui::Ui) {
        if let ObjKind::Arc(arc) = &mut self.objs[0].kind {
            ui.strong("Complément SQL pour clés étrangères");
            ui.horizontal(|ui| {
                check_i32(ui, &mut arc.f3c8[0], "ON DELETE CASCADE");
                check_i32(ui, &mut arc.f3c8[1], "ON UPDATE CASCADE");
                check_i32(ui, &mut arc.f3c8[2], "Autre");
            });
            if arc.f3c8[2] != 0 {
                ui.text_edit_singleline(&mut arc.label);
            }
            ui.small("Émis si l'option « CASCADE » du document est activée.");
        }
    }

    // ---- Héritage (174) / Lien héritage (175) ----

    fn heritage(&mut self, ui: &mut egui::Ui) {
        name_line(ui, slot(&mut self.objs, 0));
        if let Some(t) = slot(&mut self.objs, 1) {
            let p = first_param(t);
            ui.strong("Contrainte sur spécialisation / généralisation");
            ui.horizontal(|ui| {
                for (v, label) in [(0, "Pas de contrainte"), (1, "Exclusivité"), (2, "Totalité"), (3, "Partition")] {
                    ui.radio_value(&mut p.f110, v, label);
                }
            });
        }
        if let ObjKind::Heritage(h) = &mut self.objs[0].kind {
            check_i32(ui, &mut h.f418, "Migration de toutes les rubriques du surtype vers les sous-types");
        }
        self.comment_and_script(ui, "Script SQL associé à l'héritage");
    }

    fn lien_heritage(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("Entre « {} » et « {} »", self.ends.0, self.ends.1));
        if let Some(t) = slot(&mut self.objs, 1) {
            let p = param_with_role(t, 3);
            check_i32(ui, &mut p.f114, "Surtype de l'héritage");
            ui.label("Expression du critère de spécialisation / généralisation :");
            ui.text_edit_singleline(&mut p.s120);
            ui.horizontal(|ui| {
                check_inverted(ui, &mut p.n12c, "Afficher");
                check_inverted(ui, &mut p.n130, "Suffixer clés étrangères");
                check_i32(ui, &mut p.f134, "Renommer clé étrangère");
            });
            ui.horizontal(|ui| {
                ui.label("Place clé étrangère dans clé primaire associée :");
                ui.add(egui::DragValue::new(&mut p.f14c).range(1..=99));
            });
            check_i32(ui, &mut p.f144, "Clé étrangère dans index");
            check_i32(ui, &mut p.f128, "Lissage cassure");
        }
        ui.separator();
        self.actions(ui);
    }

    // ---- Contrainte (176), CIF (179), Règle (180) ----

    fn contrainte(&mut self, ui: &mut egui::Ui) {
        name_line(ui, slot(&mut self.objs, 0));
        if let Some(t) = slot(&mut self.objs, 1) {
            let p = first_param(t);
            ui.strong("Type de contrainte");
            ui.horizontal_wrapped(|ui| {
                for (v, label) in
                    [(0, "Exclusivité"), (1, "Totalité"), (2, "Partition"), (3, "Simultanéité"), (4, "Inclusion")]
                {
                    ui.radio_value(&mut p.f110, v, label);
                }
            });
        }
        self.comment_and_script(ui, "Script SQL associé à la contrainte");
    }

    fn cif(&mut self, ui: &mut egui::Ui) {
        name_line(ui, slot(&mut self.objs, 0));
        ui.strong("Commentaire");
        if let Some(t) = slot(&mut self.objs, 2) {
            lines_edit(ui, &mut t.lines, 3);
        }
    }

    fn regle(&mut self, ui: &mut egui::Ui) {
        name_line(ui, slot(&mut self.objs, 0));
        let ObjKind::Regle(r) = &mut self.objs[0].kind else { return };
        let (mut show_desc, mut as_comment, mut show_script) = (r.f3a0, r.f3a8, r.f3a4);
        let mut after = (r.raw_flag == 0) as i32;
        ui.strong("Description");
        if let Some(t) = slot(&mut self.objs, 1) {
            // one parameter per line
            let mut text = t.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join("\n");
            if ui.add(egui::TextEdit::multiline(&mut text).desired_rows(3).desired_width(f32::INFINITY)).changed() {
                let mut params: Vec<Parametre> = Vec::new();
                for (i, line) in text.lines().enumerate() {
                    let mut p = t.params.get(i).cloned().unwrap_or_else(|| edit::new_parametre(""));
                    p.name = line.to_string();
                    params.push(p);
                }
                t.params = params;
            }
        }
        ui.horizontal(|ui| {
            check_i32(ui, &mut show_desc, "Afficher dans le modèle");
            check_i32(ui, &mut as_comment, "Ajouter en commentaire au début du script SQL ci-dessous");
        });
        ui.strong("Script SQL associé à la règle");
        if let Some(t) = slot(&mut self.objs, 1) {
            lines_edit(ui, &mut t.lines2, 5);
            ui.horizontal(|ui| {
                ui.label("Position :");
                ui.add(egui::DragValue::new(&mut t.f4d8));
                ui.radio_value(&mut after, 0, "Avant");
                ui.radio_value(&mut after, 1, "Après");
                ui.label("la création des tables");
            });
        }
        check_i32(ui, &mut show_script, "Afficher dans le modèle");
        if let ObjKind::Regle(r) = &mut self.objs[0].kind {
            r.f3a0 = show_desc;
            r.f3a8 = as_comment;
            r.f3a4 = show_script;
            r.raw_flag = (after == 0) as u16;
        }
    }

    fn comment_and_script(&mut self, ui: &mut egui::Ui, script_title: &str) {
        ui.strong("Commentaire");
        if let Some(t) = slot(&mut self.objs, 2) {
            lines_edit(ui, &mut t.lines, 3);
        }
        ui.strong(script_title);
        if let Some(t) = slot(&mut self.objs, 1) {
            lines_edit(ui, &mut t.lines2, 5);
            ui.horizontal(|ui| {
                ui.label("Position :");
                ui.add(egui::DragValue::new(&mut t.f4d8));
            });
        }
    }

    // ---- Liens de CIF, de contrainte, de règle (177, 178, 181) ----

    fn lien_simple(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("Entre « {} » et « {} »", self.ends.0, self.ends.1));
        let kind = self.kind;
        let Some(t) = slot(&mut self.objs, 1) else { return };
        let role = match kind {
            Kind::LienRegle => 5,
            Kind::LienIllustration => 6,
            _ => 4,
        };
        let p = param_with_role(t, role);
        match kind {
            Kind::LienCif => check_i32(ui, &mut p.f114, "Lien vers l'entité ciblée par la contrainte"),
            Kind::LienContrainte => check_i32(ui, &mut p.f114, "Lien vers l'association ciblée par la contrainte"),
            _ => {}
        }
        ui.horizontal(|ui| {
            ui.label("Libellé :");
            ui.text_edit_singleline(&mut p.s120);
        });
        check_i32(ui, &mut p.f128, "Lissage cassure");
    }
}

impl Dialog {
    // ---- Rectangle, Ellipse, Image, Vignette (182, 183, 203, 204) ----

    fn figure(&mut self, ui: &mut egui::Ui) -> Option<Outcome> {
        let mut outcome = None;
        let o = &mut self.objs[0];
        // these dialogs drop the « Par défaut » colours (0x140036eb0…)
        for st in &mut o.base.styles {
            (st.a, st.b, st.c) = (0, 0, 0);
        }
        let (mut pen, mut fill) = (o.base.styles[0].pen.color, o.base.styles[0].brush.color);
        let [_, _, filled, outlined] = &mut o.base.f308;
        let (mut outlined_v, mut filled_v) = (*outlined, *filled);
        match &mut o.kind {
            ObjKind::Image(i) => {
                if i.f3c0 != 0 {
                    path_row(ui, "Modèle :", &mut i.s3b8, "Modèles Looping", &["loo", "Loo", "LOO"]);
                    path_row(ui, "Vignette :", &mut i.path, "Images", IMAGE_EXTENSIONS);
                    if ui.add_enabled(!i.s3b8.is_empty(), egui::Button::new("Ouvrir le modèle")).clicked() {
                        outcome = Some(Outcome::OpenModel(i.s3b8.clone()));
                    }
                } else {
                    path_row(ui, "Fichier :", &mut i.path, "Images", IMAGE_EXTENSIONS);
                }
                ui.separator();
                let [w, style, shadow, shadow_color] = &mut i.f3a8;
                outline_fields(ui, &mut outlined_v, w, style, None, &mut pen);
                fill_fields(ui, &mut filled_v, &mut fill, shadow, shadow_color);
            }
            ObjKind::Rectangle(x) => {
                outline_fields(ui, &mut outlined_v, &mut x.f3a0, &mut x.f3a4, Some(&mut x.f3b0), &mut pen);
                fill_fields(ui, &mut filled_v, &mut fill, &mut x.f3a8, &mut x.f3ac);
            }
            ObjKind::Ellipse(x) => {
                outline_fields(ui, &mut outlined_v, &mut x.f3a0, &mut x.f3a4, None, &mut pen);
                fill_fields(ui, &mut filled_v, &mut fill, &mut x.f3a8, &mut x.f3ac);
            }
            _ => {}
        }
        let [_, _, filled, outlined] = &mut o.base.f308;
        (*outlined, *filled) = (outlined_v, filled_v);
        set_colors(o, Some(pen), Some(fill));
        outcome
    }

    // ---- Texte (171) ----

    fn texte(&mut self, ui: &mut egui::Ui, doc: &Document) {
        if let Some(t) = slot(&mut self.objs, 0) {
            ui.label("Texte :");
            lines_edit(ui, &mut t.lines, 4);
            font_fields(ui, t, doc);
        }
        let o = &mut self.objs[0];
        let ObjKind::Titre(x) = &mut o.kind else { return };
        ui.separator();
        ui.strong("Alignement");
        ui.horizontal(|ui| {
            ui.label("Horizontal :");
            for (v, label) in [(0, "Gauche"), (6, "Centré"), (2, "Droite")] {
                ui.radio_value(&mut o.base.f318, v, label);
            }
        });
        ui.horizontal(|ui| {
            ui.label("Vertical :");
            for (v, label) in [(0, "Haut"), (6, "Centré"), (8, "Bas")] {
                ui.radio_value(&mut x.f390, v, label);
            }
        });
        let st = o.base.styles[0];
        let (mut pen, mut fill) = (st.pen.color, st.brush.color);
        let (mut pen_default, mut fill_default) = (st.a as i32, st.b as i32);
        let [fill_d, outline_d, filled, outlined] = &mut o.base.f308;
        let [round_d, round] = &mut x.f3a4;
        let [width, shadow, shadow_color, width_d, shadow_d, shadow_color_d] = &mut x.f3ac;
        ui.separator();
        ui.strong("Contour");
        egui::Grid::new("titre_contour").num_columns(3).show(ui, |ui| {
            ui.label("Afficher :");
            check_i32(ui, outline_d, "Par défaut");
            ui.add_enabled_ui(*outline_d == 0, |ui| check_i32(ui, outlined, "Afficher"));
            ui.end_row();
            ui.label("Epaisseur :");
            check_i32(ui, width_d, "Par défaut");
            ui.add_enabled(*width_d == 0, egui::DragValue::new(width).range(1..=5));
            ui.end_row();
            ui.label("Couleur :");
            check_i32(ui, &mut pen_default, "Par défaut");
            ui.add_enabled_ui(pen_default == 0, |ui| color_edit(ui, &mut pen));
            ui.end_row();
            ui.label("Angles arrondis :");
            check_i32(ui, round_d, "Par défaut");
            ui.add_enabled(*round_d == 0, egui::DragValue::new(round).range(0..=100));
            ui.end_row();
        });
        ui.strong("Fond");
        egui::Grid::new("titre_fond").num_columns(3).show(ui, |ui| {
            ui.label("Epaisseur ombre :");
            check_i32(ui, shadow_d, "Par défaut");
            ui.add_enabled(*shadow_d == 0, egui::DragValue::new(shadow).range(0..=20));
            ui.end_row();
            ui.label("Couleur ombre :");
            check_i32(ui, shadow_color_d, "Par défaut");
            ui.add_enabled_ui(*shadow_color_d == 0, |ui| {
                let mut c = *shadow_color as u32;
                color_edit(ui, &mut c);
                *shadow_color = c as i32;
            });
            ui.end_row();
            ui.label("Afficher fond :");
            check_i32(ui, fill_d, "Par défaut");
            ui.add_enabled_ui(*fill_d == 0, |ui| check_i32(ui, filled, "Afficher"));
            ui.end_row();
            ui.label("Couleur fond :");
            check_i32(ui, &mut fill_default, "Par défaut");
            ui.add_enabled_ui(fill_default == 0, |ui| color_edit(ui, &mut fill));
            ui.end_row();
        });
        for st in &mut o.base.styles {
            st.a = pen_default as u16;
            (st.b, st.c) = (fill_default as u16, fill_default as u16);
        }
        set_colors(o, Some(pen), Some(fill));
    }

    // ---- Ligne, Flèche (185, 184) ----

    fn trait_(&mut self, ui: &mut egui::Ui, doc: &Document) {
        // smoothing only applies to a line with break points (0x1400448a0)
        let bent = edit::chain_ids(doc, self.target).len() > 2;
        let arrow = self.kind == Kind::Fleche;
        let o = &mut self.objs[0];
        let mut pen = o.base.styles[0].pen.color;
        let ObjKind::Trait(t) = &mut o.kind else { return };
        egui::Grid::new("trait").num_columns(2).show(ui, |ui| {
            ui.label("Epaisseur du trait :");
            ui.add(egui::DragValue::new(&mut t.f3a8).range(1..=5));
            ui.end_row();
            ui.label("Style du trait :");
            pen_style(ui, "trait_style", &mut t.f3ac);
            ui.end_row();
            if arrow {
                // three owner-drawn sizes in Looping's list
                ui.label("Taille de la flèche :");
                egui::ComboBox::from_id_salt("arrow_size")
                    .selected_text(ARROW_SIZES.get(t.f3b0 as usize).copied().unwrap_or("?"))
                    .show_ui(ui, |ui| {
                        for (i, label) in ARROW_SIZES.iter().enumerate() {
                            ui.selectable_value(&mut t.f3b0, i as i32, *label);
                        }
                    });
                ui.end_row();
                ui.label("Forme de la flèche :");
                ui.add(egui::DragValue::new(&mut t.f3b4).range(0..=50));
                ui.end_row();
            }
            ui.label("Couleur :");
            color_edit(ui, &mut pen);
            ui.end_row();
        });
        ui.add_enabled_ui(bent, |ui| check_i32(ui, &mut t.f3b8, "Lissage cassure"));
        for st in &mut o.base.styles {
            (st.a, st.b, st.c) = (0, 0, 0);
        }
        set_colors(o, Some(pen), None);
    }

    // ---- Crayon (191) ----

    fn crayon(&mut self, ui: &mut egui::Ui) {
        let o = &mut self.objs[0];
        let mut pen = o.base.styles[0].pen.color;
        let ObjKind::Crayon(c) = &mut o.kind else { return };
        egui::Grid::new("crayon").num_columns(2).show(ui, |ui| {
            ui.label("Epaisseur du trait :");
            ui.add(egui::DragValue::new(&mut c.f464).range(1..=5));
            ui.end_row();
            ui.label("Style du trait :");
            pen_style(ui, "crayon_style", &mut c.f468);
            ui.end_row();
            ui.label("Couleur :");
            color_edit(ui, &mut pen);
            ui.end_row();
        });
        for st in &mut o.base.styles {
            (st.a, st.b, st.c) = (0, 0, 0);
        }
        set_colors(o, Some(pen), None);
    }
}

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "bmp", "png", "gif"];

/// Looping's line style list (`CComboBoxStyleTrait`): the index is the
/// `PS_*` pen style.
const PEN_STYLES: [&str; 5] = ["Continu", "Tirets", "Pointillés", "Tiret-point", "Tiret-point-point"];

/// Looping draws these three sizes instead of naming them.
const ARROW_SIZES: [&str; 3] = ["Petite", "Moyenne", "Grande"];

fn pen_style(ui: &mut egui::Ui, id: &str, v: &mut i32) {
    egui::ComboBox::from_id_salt(id).selected_text(PEN_STYLES.get(*v as usize).copied().unwrap_or("?")).show_ui(
        ui,
        |ui| {
            for (i, label) in PEN_STYLES.iter().enumerate() {
                ui.selectable_value(v, i as i32, *label);
            }
        },
    );
}

/// Colour button on a `COLORREF` (0x00BBGGRR).
fn color_edit(ui: &mut egui::Ui, c: &mut u32) {
    let mut rgb = [(*c & 0xff) as u8, (*c >> 8 & 0xff) as u8, (*c >> 16 & 0xff) as u8];
    if egui::color_picker::color_edit_button_srgb(ui, &mut rgb).changed() {
        *c = rgb[0] as u32 | (rgb[1] as u32) << 8 | (rgb[2] as u32) << 16;
    }
}

/// Pen colour and fill colours (both brushes) of the three views.
fn set_colors(o: &mut DrawObj, pen: Option<u32>, fill: Option<u32>) {
    for st in &mut o.base.styles {
        if let Some(p) = pen {
            st.pen.color = p;
        }
        if let Some(f) = fill {
            st.brush.color = f;
            st.brush2.color = f;
        }
    }
}

/// « Contour » group: shown, width (1–5), style, rounded corners, colour.
fn outline_fields(
    ui: &mut egui::Ui,
    shown: &mut i32,
    width: &mut i32,
    style: &mut i32,
    round: Option<&mut i32>,
    pen: &mut u32,
) {
    ui.strong("Contour");
    check_i32(ui, shown, "Afficher le contour");
    ui.add_enabled_ui(*shown != 0, |ui| {
        egui::Grid::new("contour").num_columns(2).show(ui, |ui| {
            ui.label("Epaisseur :");
            ui.add(egui::DragValue::new(width).range(1..=5));
            ui.end_row();
            ui.label("Style :");
            pen_style(ui, "contour_style", style);
            ui.end_row();
            if let Some(r) = round {
                ui.label("Angles arrondis :");
                ui.add(egui::DragValue::new(r).range(0..=100));
                ui.end_row();
            }
            ui.label("Couleur du contour :");
            color_edit(ui, pen);
            ui.end_row();
        });
    });
}

/// « Fond » group: shown, colour, shadow depth and colour.
fn fill_fields(ui: &mut egui::Ui, shown: &mut i32, fill: &mut u32, shadow: &mut i32, shadow_color: &mut i32) {
    ui.strong("Fond");
    check_i32(ui, shown, "Afficher le fond");
    ui.add_enabled_ui(*shown != 0, |ui| {
        egui::Grid::new("fond").num_columns(2).show(ui, |ui| {
            ui.label("Couleur du fond :");
            color_edit(ui, fill);
            ui.end_row();
            ui.label("Epaisseur de l'ombre :");
            ui.add(egui::DragValue::new(shadow).range(0..=20));
            ui.end_row();
            ui.label("Couleur de l'ombre :");
            let mut c = *shadow_color as u32;
            color_edit(ui, &mut c);
            *shadow_color = c as i32;
            ui.end_row();
        });
    });
}

/// A file path with « Parcourir… ».
fn path_row(ui: &mut egui::Ui, label: &str, path: &mut String, filter: &str, extensions: &[&str]) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.text_edit_singleline(path);
        if ui.button("Parcourir…").clicked() {
            if let Some(p) = rfd::FileDialog::new().add_filter(filter, extensions).pick_file() {
                *path = p.to_string_lossy().into_owned();
            }
        }
    });
}

/// Font of a text (`CDrawTexte+0x390`): the document's (« Police par
/// défaut ») or its own `LOGFONTW` (`+0x394`), first taken from the
/// document font « Textes » (`+0x3b0`).
fn font_fields(ui: &mut egui::Ui, t: &mut Texte, doc: &Document) {
    let b = &mut t.font_blocks[0];
    if b.len() < 100 {
        b.resize(100, 0);
    }
    let mut default = i32::from_le_bytes([b[0], b[1], b[2], b[3]]) != 0;
    if ui.checkbox(&mut default, "Police par défaut").changed() {
        b[0..4].copy_from_slice(&(default as i32).to_le_bytes());
        if !default && b[4..8] == [0; 4] {
            let off = 0x3b0 - loo_format::Settings::BASE;
            b[4..96].copy_from_slice(&doc.settings.raw()[off..off + 92]);
        }
    }
    if default {
        return;
    }
    let lf = &mut b[4..96];
    let mut face = loo_format::LogFont::parse(lf).face;
    let height = i32::from_le_bytes([lf[0], lf[1], lf[2], lf[3]]);
    // lfHeight < 0: character height in pixels at 96 dpi
    let mut points = ((-height) as f32 * 72.0 / 96.0).round() as i32;
    let mut bold = i32::from_le_bytes([lf[16], lf[17], lf[18], lf[19]]) >= 700;
    let (mut italic, mut underline) = (lf[20] != 0, lf[21] != 0);
    ui.horizontal(|ui| {
        ui.label("Police :");
        ui.add(egui::TextEdit::singleline(&mut face).desired_width(120.0));
        ui.add(egui::DragValue::new(&mut points).range(4..=96).suffix(" pt"));
        ui.checkbox(&mut bold, "Gras");
        ui.checkbox(&mut italic, "Italique");
        ui.checkbox(&mut underline, "Souligné");
    });
    let height = -((points as f32 * 96.0 / 72.0).round() as i32);
    lf[0..4].copy_from_slice(&height.to_le_bytes());
    lf[16..20].copy_from_slice(&(if bold { 700i32 } else { 400 }).to_le_bytes());
    lf[20] = italic as u8;
    lf[21] = underline as u8;
    let units: Vec<u16> = face.encode_utf16().take(31).collect();
    lf[28..92].fill(0);
    for (i, u) in units.iter().enumerate() {
        lf[28 + 2 * i..30 + 2 * i].copy_from_slice(&u.to_le_bytes());
    }
}

/// The « Rubrique » dialog (172) on a parameter.
fn rubrique(ui: &mut egui::Ui, p: &mut Parametre, entity: bool) {
    egui::Grid::new("rubrique_names").num_columns(2).show(ui, |ui| {
        ui.label("Nom :");
        ui.text_edit_singleline(&mut p.name);
        ui.end_row();
        ui.label("Nom logique :");
        ui.text_edit_singleline(&mut p.s18);
        ui.end_row();
    });
    ui.separator();
    ui.strong("Type");
    ui.horizontal_wrapped(|ui| {
        for (v, label) in [
            (0, "Texte"),
            (1, "Numérique"),
            (2, "Date / Heure"),
            (3, "Booléen"),
            (5, "Xml"),
            (7, "Géométrique"),
            (6, "Géographique"),
            (4, "Libre"),
        ] {
            ui.radio_value(&mut p.f20, v, label);
        }
    });
    match p.f20 {
        0 => {
            ui.horizontal(|ui| {
                for (v, label) in [(0, "Caractères ASCII"), (1, "Caractères Unicode"), (2, "Binaire")] {
                    ui.radio_value(&mut p.f28, v, label);
                }
            });
            ui.horizontal(|ui| {
                for (v, label) in [(1, "Variable"), (0, "Fixe"), (2, "Volumineux")] {
                    ui.radio_value(&mut p.f24, v, label);
                }
            });
            // one length per (charset, kind) pair, as read by `DataType::from_param`
            let len = match (p.f28, p.f24) {
                (1, 0) => &mut p.f68[0],
                (2, 0) => &mut p.f68[1],
                (_, 0) => &mut p.f64,
                (1, _) => &mut p.f68[3],
                (2, _) => &mut p.f68[4],
                _ => &mut p.f68[2],
            };
            ui.horizontal(|ui| {
                if p.f24 != 2 {
                    ui.label("Longueur :");
                    ui.add(egui::DragValue::new(len).range(0..=1_000_000));
                }
                ui.label("Collation :");
                let coll = match p.f24 {
                    0 => &mut p.s50,
                    2 => &mut p.s58,
                    _ => &mut p.s48,
                };
                ui.add(egui::TextEdit::singleline(coll).desired_width(120.0));
            });
        }
        1 => {
            ui.horizontal_wrapped(|ui| {
                for (v, label) in [(0, "Entier"), (1, "Décimal"), (2, "Réel"), (3, "Monétaire"), (4, "Compteur")] {
                    ui.radio_value(&mut p.f2c[0], v, label);
                }
            });
            match p.f2c[0] {
                0 => {
                    ui.horizontal(|ui| {
                        ui.label("Taille :");
                        for (v, label) in [(0, "8 bits"), (1, "16 bits"), (2, "32 bits"), (3, "64 bits")] {
                            ui.radio_value(&mut p.f2c[1], v, label);
                        }
                    });
                }
                1 => {
                    ui.horizontal(|ui| {
                        ui.label("Nb chiffres :");
                        ui.add(egui::DragValue::new(&mut p.f7c).range(1..=65));
                        ui.label("dont :");
                        ui.add(egui::DragValue::new(&mut p.f80).range(0..=30));
                        ui.label("après la virgule");
                    });
                }
                2 => {
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut p.f2c[2], 0, "Simple (32 bits)");
                        ui.radio_value(&mut p.f2c[2], 1, "Double (64 bits)");
                    });
                }
                _ => {}
            }
        }
        2 => {
            ui.horizontal(|ui| {
                for (v, label) in [(0, "Date"), (1, "Heure"), (2, "Date-Heure")] {
                    ui.radio_value(&mut p.f2c[3], v, label);
                }
                if p.f2c[3] == 2 {
                    check_i32(ui, &mut p.f60, "Avec fuseau horaire");
                }
            });
        }
        4 => {
            ui.horizontal(|ui| {
                ui.label("Type :");
                ui.text_edit_singleline(&mut p.s40);
            });
            ui.small("Texte après « # » : description, non reprise dans le script.");
        }
        _ => {}
    }
    ui.separator();
    ui.strong("Propriétés");
    ui.horizontal_wrapped(|ui| {
        if entity {
            check_i32(ui, &mut p.f8, "Identifiant");
            if p.f8 != 0 {
                ui.label("Place dans clé :");
                ui.add(egui::DragValue::new(&mut p.f14c).range(1..=99));
            }
        }
        check_i32(ui, &mut p.f88, "NOT NULL");
        check_i32(ui, &mut p.f8c, "UNIQUE");
    });
    egui::Grid::new("rubrique_props").num_columns(2).show(ui, |ui| {
        ui.label("Autre index :");
        ui.text_edit_singleline(&mut p.s138);
        ui.end_row();
        ui.label("Complément :");
        ui.text_edit_singleline(&mut p.s118);
        ui.end_row();
    });
    ui.strong("Commentaire");
    ui.add(egui::TextEdit::multiline(&mut p.sf8).desired_rows(2).desired_width(f32::INFINITY));
}

/// Name (first line of the slot-0 text) and logical name (`+0x5f8`).
fn name_fields(ui: &mut egui::Ui, t: Option<&mut Texte>) {
    let Some(t) = t else { return };
    egui::Grid::new("names").num_columns(2).show(ui, |ui| {
        ui.label("Nom :");
        name_edit(ui, t);
        ui.end_row();
        ui.label("Nom logique :");
        ui.text_edit_singleline(&mut t.s5f8);
        ui.end_row();
    });
}

fn name_line(ui: &mut egui::Ui, t: Option<&mut Texte>) {
    let Some(t) = t else { return };
    ui.horizontal(|ui| {
        ui.label("Nom :");
        name_edit(ui, t);
    });
}

fn name_edit(ui: &mut egui::Ui, t: &mut Texte) {
    if t.lines.is_empty() {
        t.lines.push(String::new());
    }
    ui.text_edit_singleline(&mut t.lines[0]);
}

/// Multi-line text kept as a list of lines.
fn lines_edit(ui: &mut egui::Ui, lines: &mut Vec<String>, rows: usize) {
    let mut text = lines.join("\n");
    if ui.add(egui::TextEdit::multiline(&mut text).desired_rows(rows).desired_width(f32::INFINITY)).changed() {
        *lines = text.split('\n').map(str::to_string).collect();
    }
}

fn check_i32(ui: &mut egui::Ui, v: &mut i32, label: &str) {
    let mut on = *v != 0;
    if ui.checkbox(&mut on, label).changed() {
        *v = on as i32;
    }
}

/// Flags Looping stores inverted (`CUnParametre+0x12c` / `+0x130`).
fn check_inverted(ui: &mut egui::Ui, v: &mut i32, label: &str) {
    let mut on = *v == 0;
    if ui.checkbox(&mut on, label).changed() {
        *v = (!on) as i32;
    }
}

/// Text of slot `n` of the edited object (the first of `objs`).
fn slot(objs: &mut [DrawObj], n: usize) -> Option<&mut Texte> {
    let id = objs[0].base.links.texts[n];
    objs.iter_mut().find(|o| o.base.id == id).and_then(|o| match &mut o.kind {
        ObjKind::Texte(t) => Some(t),
        _ => None,
    })
}

fn param_with_role(t: &mut Texte, role: i32) -> &mut Parametre {
    match t.params.iter().position(|p| p.role == role) {
        Some(i) => &mut t.params[i],
        None => {
            let mut p = edit::new_parametre("");
            p.role = role;
            t.params.push(p);
            t.params.last_mut().unwrap()
        }
    }
}

fn first_param(t: &mut Texte) -> &mut Parametre {
    if t.params.is_empty() {
        t.params.push(edit::new_parametre(""));
    }
    &mut t.params[0]
}
