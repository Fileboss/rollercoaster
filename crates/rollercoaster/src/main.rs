//! Rollercoaster, an MCD editor for Looping documents — native app for
//! macOS ARM and Linux x64 (eframe/egui).

mod canvas;
mod dialogs;

use std::path::PathBuf;

use eframe::egui;
use loo_format::{Document, ObjKind, Point};
use looping_core::{edit, mcd::ObjId, render, sql, Dbms, Mcd, Mld};

/// A deferred change of the document (applied as one undo step).
type DocChange = Box<dyn FnOnce(&mut Document)>;

pub fn trace() -> bool {
    std::env::var_os("ROLLERCOASTER_TRACE").is_some()
}

fn main() -> eframe::Result {
    if trace() {
        // panics and their backtrace on stderr, even from other threads
        std::panic::set_hook(Box::new(|info| {
            eprintln!("[trace] PANIC: {info}\n{}", std::backtrace::Backtrace::force_capture());
        }));
    }
    let path = std::env::args().nth(1).map(PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 900.0]).with_title("Rollercoaster"),
        ..Default::default()
    };
    eframe::run_native(
        "Rollercoaster",
        options,
        Box::new(move |cc| {
            canvas::install_fonts(&cc.egui_ctx);
            Ok(Box::new(App::new(path)))
        }),
    )
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Mld,
    Sql,
}

#[derive(PartialEq, Clone, Copy)]
pub enum Tool {
    Select,
    Entity,
    Association,
    Heritage,
    Contrainte,
    Cif,
    Regle,
    Link,
    /// « Cassure »: adds a break point to a link, or removes one.
    Cassure,
    // illustrations
    Texte,
    Ligne,
    Fleche,
    Rectangle,
    Ellipse,
    Crayon,
    Image,
    /// « Sous-modèle »: an image showing another model (« Vignette »).
    SousModele,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::Select => "Sélection",
            Tool::Entity => "Entité",
            Tool::Association => "Association",
            Tool::Heritage => "Héritage",
            Tool::Contrainte => "Contrainte",
            Tool::Cif => "CIF",
            Tool::Regle => "Règle",
            Tool::Link => "Lien",
            Tool::Cassure => "Cassure",
            Tool::Texte => "Texte",
            Tool::Ligne => "Ligne",
            Tool::Fleche => "Flèche",
            Tool::Rectangle => "Rectangle",
            Tool::Ellipse => "Ellipse",
            Tool::Crayon => "Crayon",
            Tool::Image => "Image",
            Tool::SousModele => "Sous-modèle",
        }
    }

    /// Tools that draw a frame (drag, or click for a default size).
    pub fn draws_frame(self) -> bool {
        matches!(self, Tool::Texte | Tool::Rectangle | Tool::Ellipse | Tool::Image | Tool::SousModele)
    }
}

pub struct App {
    doc: Document,
    path: Option<PathBuf>,
    dirty: bool,
    scene: render::Scene,
    mcd: Mcd,
    mld: Mld,
    undo: Vec<Document>,
    redo: Vec<Document>,
    pub view: canvas::View,
    /// Selected objects; the last one is shown in the side panel.
    pub selection: Vec<ObjId>,
    clipboard: Vec<loo_format::DrawObj>,
    pastes: i32,
    pub tool: Tool,
    pub link_from: Option<ObjId>,
    /// Points of the line or free-hand stroke being drawn.
    pub draft: Vec<Point>,
    /// Active views (`CLoopingView+0x130`): bit 0 = every object, else the
    /// bits of the views shown (window state, not saved).
    views_mask: u32,
    /// The window managing the views is open.
    views_window: bool,
    /// « Compléments script LDD » being edited (dialog 190).
    complements: Option<[String; 4]>,
    dbms: Dbms,
    tab: Tab,
    status: String,
    frames: u32,
    dialog: Option<dialogs::Dialog>,
    /// Last state written by the trace (ROLLERCOASTER_TRACE).
    traced: String,
}

impl App {
    /// Document title as Looping shows it: the file name without extension.
    fn title(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map_or_else(|| "modele".to_string(), |s| s.to_string_lossy().into_owned())
    }

    fn new(path: Option<PathBuf>) -> Self {
        let mut app = App {
            doc: Document::default(),
            path: None,
            dirty: false,
            scene: Default::default(),
            mcd: Default::default(),
            mld: Default::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            view: canvas::View::default(),
            selection: Vec::new(),
            clipboard: Vec::new(),
            pastes: 0,
            tool: Tool::Select,
            link_from: None,
            draft: Vec::new(),
            views_mask: 1,
            views_window: false,
            complements: None,
            dbms: Dbms::MySql,
            tab: Tab::Sql,
            status: String::new(),
            frames: 0,
            dialog: None,
            traced: String::new(),
        };
        if let Some(p) = path {
            app.open(p);
        }
        app.refresh();
        // debugging aid, with ROLLERCOASTER_SCREENSHOT: the dialog of an object
        if let Some(id) = std::env::var("ROLLERCOASTER_DIALOG").ok().and_then(|v| v.parse().ok()) {
            app.open_dialog(id);
        }
        // and a selection (ids separated by commas)
        if let Ok(v) = std::env::var("ROLLERCOASTER_SELECT") {
            app.selection = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        }
        app
    }

    /// What the window shows: the document's « Affichage » options and
    /// the active views.
    pub fn display(&self) -> render::Display {
        render::Display { views: self.views_mask, ..render::Display::of(&self.doc) }
    }

    fn refresh(&mut self) {
        // boxes always hold their name and attributes, as in Looping
        edit::fit_all(&mut self.doc);
        self.scene = render::build_with(&self.doc, &self.display());
        self.mcd = Mcd::from_document(&self.doc);
        self.mld = Mld::from_mcd(&self.mcd);
    }

    /// Records the current state for undo before a mutation.
    pub fn checkpoint(&mut self) {
        self.undo.push(self.doc.clone());
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.dirty = true;
    }

    pub fn mutate(&mut self, f: impl FnOnce(&mut Document)) {
        self.checkpoint();
        f(&mut self.doc);
        self.refresh();
    }

    fn open(&mut self, p: PathBuf) {
        match Document::open(&p) {
            Ok(doc) => {
                self.dbms = Dbms::from_index(doc.settings.i32(0x52c));
                self.status = format!("{} — format {}", p.display(), doc.loaded_version);
                self.doc = doc;
                self.path = Some(p);
                self.dirty = false;
                self.undo.clear();
                self.redo.clear();
                self.select(None);
                self.views_mask = 1;
                self.refresh();
                // the zoom saved with the document (view +0x2f0), else fit
                let z = self.doc.frame_state[0][0];
                if (10..=800).contains(&z) {
                    self.view.restore_zoom = Some(z as f32 / 100.0);
                } else {
                    self.view.fit_next = true;
                }
            }
            Err(e) => self.status = format!("Erreur : {e}"),
        }
    }

    /// Enregistrer: false when it failed or was cancelled.
    fn save(&mut self) -> bool {
        match self.path.clone() {
            Some(p) => self.save_to(p),
            None => self.save_as(),
        }
    }

    /// Asks to save a modified document before it is closed or replaced
    /// (« Enregistrer les modifications ? »); false when the user cancels
    /// or the saving fails.
    fn confirm_discard(&mut self) -> bool {
        if !self.dirty {
            return true;
        }
        let answer = rfd::MessageDialog::new()
            .set_title("Rollercoaster")
            .set_description(format!("Enregistrer les modifications de « {} » ?", self.title()))
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show();
        match answer {
            rfd::MessageDialogResult::Yes => self.save(),
            rfd::MessageDialogResult::No => true,
            _ => false,
        }
    }

    /// « Nouveau » (57600): Looping asks whether to take the properties of
    /// an existing model (its settings, options, script complements… but
    /// none of its objects nor views), else uses its defaults.
    fn new_document(&mut self) {
        if !self.confirm_discard() {
            return;
        }
        let answer = rfd::MessageDialog::new()
            .set_title("Rollercoaster")
            .set_description(
                "Création d'un nouveau modèle.\nVoulez-vous récupérer les propriétés d'un modèle existant ?",
            )
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show();
        let doc = match answer {
            rfd::MessageDialogResult::Yes => {
                let Some(p) =
                    rfd::FileDialog::new().add_filter("Documents Looping", &["loo", "Loo", "LOO"]).pick_file()
                else {
                    return;
                };
                match Document::open(&p) {
                    Ok(mut d) => {
                        d.objects.clear();
                        d.views.clear();
                        d.loaded_version = loo_format::CURRENT_VERSION;
                        d
                    }
                    Err(e) => {
                        self.status = format!("Erreur : {e}");
                        return;
                    }
                }
            }
            rfd::MessageDialogResult::No => Document::default(),
            _ => return,
        };
        let mut app = App::new(None);
        app.dbms = Dbms::from_index(doc.settings.i32(0x52c));
        app.doc = doc;
        app.refresh();
        *self = app;
    }

    /// « Modèle graphique au format image » (33136): the diagram as shown
    /// (display options, active views), in PNG or SVG after the extension.
    fn export_image(&mut self) {
        let Some(p) = rfd::FileDialog::new()
            .add_filter("Image PNG", &["png"])
            .add_filter("Image SVG", &["svg"])
            .set_file_name(format!("{}.png", self.title()))
            .save_file()
        else {
            return;
        };
        let svg = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg"));
        let bytes = if svg { Ok(self.scene.to_svg().into_bytes()) } else { self.scene.to_png() };
        self.status = match bytes.map(|b| std::fs::write(&p, b)) {
            Ok(Ok(())) => format!("Image exportée : {}", p.display()),
            Ok(Err(e)) => format!("Erreur : {e}"),
            Err(e) => format!("Erreur : {e}"),
        };
    }

    /// « Ouvrir… », after the question about unsaved changes.
    fn open_dialog_file(&mut self) {
        if !self.confirm_discard() {
            return;
        }
        if let Some(p) = rfd::FileDialog::new().add_filter("Documents Looping", &["loo", "Loo", "LOO"]).pick_file() {
            self.open(p);
        }
    }

    fn save_to(&mut self, p: PathBuf) -> bool {
        // Looping saves the window's zoom (view +0x2f0)
        let z = (self.view.zoom * 100.0).round().clamp(10.0, 800.0) as i32;
        self.doc.frame_state[0] = [z, z];
        match self.doc.save().map(|b| std::fs::write(&p, b)) {
            Ok(Ok(())) => {
                self.status = format!("Enregistré : {}", p.display());
                self.path = Some(p);
                self.dirty = false;
                true
            }
            Ok(Err(e)) => {
                self.status = format!("Erreur : {e}");
                false
            }
            Err(e) => {
                self.status = format!("Erreur : {e}");
                false
            }
        }
    }

    fn save_as(&mut self) -> bool {
        match rfd::FileDialog::new()
            .add_filter("Looping", &["loo"])
            .set_file_name(format!("{}.loo", self.title()))
            .save_file()
        {
            Some(p) => self.save_to(p),
            None => false,
        }
    }

    fn menu(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("Fichier", |ui| {
                if ui.add(egui::Button::new("Nouveau").shortcut_text("Ctrl+N")).clicked() {
                    ui.close();
                    self.new_document();
                }
                if ui.add(egui::Button::new("Ouvrir…").shortcut_text("Ctrl+O")).clicked() {
                    ui.close();
                    self.open_dialog_file();
                }
                if ui.add(egui::Button::new("Enregistrer").shortcut_text("Ctrl+S")).clicked() {
                    self.save();
                    ui.close();
                }
                if ui.button("Enregistrer sous…").clicked() {
                    self.save_as();
                    ui.close();
                }
                ui.separator();
                if ui.button("Exporter l'image…").clicked() {
                    ui.close();
                    self.export_image();
                }
                // as Looping's "Script LDD au format TXT..." (a VBA module for Access)
                if ui.button("Exporter le script LDD…").clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("Script", &["txt", "sql", "bas"])
                        .set_file_name(format!("{}.txt", self.title()))
                        .save_file()
                    {
                        match sql::script_file(&self.mld, &self.sql_options()) {
                            Ok(text) => {
                                let _ = std::fs::write(p, sql::to_latin1_crlf(&text));
                            }
                            Err(e) => self.status = e.to_string(),
                        }
                    }
                    ui.close();
                }
                if ui.button("Exporter le MLD au format XML…").clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("XML", &["xml"])
                        .set_file_name(format!("{}.xml", self.title()))
                        .save_file()
                    {
                        match sql::xml(&self.mld, &self.sql_options(), &self.title()) {
                            Ok(xml) => {
                                let _ = std::fs::write(p, sql::to_latin1_crlf(&xml));
                            }
                            Err(e) => self.status = e.to_string(),
                        }
                    }
                    ui.close();
                }
                ui.separator();
                // closes the window, which asks about unsaved changes
                if ui.button("Quitter").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    ui.close();
                }
            });
            ui.menu_button("Édition", |ui| {
                if ui.add_enabled(!self.undo.is_empty(), egui::Button::new("Annuler")).clicked() {
                    self.undo_step();
                    ui.close();
                }
                if ui.add_enabled(!self.redo.is_empty(), egui::Button::new("Rétablir")).clicked() {
                    self.redo_step();
                    ui.close();
                }
                ui.separator();
                let any = !self.selection.is_empty();
                if ui.add_enabled(any, egui::Button::new("Couper").shortcut_text("Ctrl+X")).clicked() {
                    self.cut();
                    ui.close();
                }
                if ui.add_enabled(any, egui::Button::new("Copier").shortcut_text("Ctrl+C")).clicked() {
                    self.copy();
                    ui.close();
                }
                if ui
                    .add_enabled(!self.clipboard.is_empty(), egui::Button::new("Coller").shortcut_text("Ctrl+V"))
                    .clicked()
                {
                    self.paste();
                    ui.close();
                }
                if ui.add_enabled(any, egui::Button::new("Supprimer").shortcut_text("Suppr")).clicked() {
                    self.delete_selected();
                    ui.close();
                }
                ui.separator();
                if ui.add(egui::Button::new("Tout sélectionner").shortcut_text("Ctrl+A")).clicked() {
                    self.select_all();
                    ui.close();
                }
                if ui.button("Inverser la sélection").clicked() {
                    let all = self.selectable();
                    self.selection = all.into_iter().filter(|id| !self.selection.contains(id)).collect();
                    ui.close();
                }
                ui.separator();
                ui.add_enabled_ui(any, |ui| {
                    ui.menu_button("Ordre", |ui| {
                        for (label, order) in [
                            ("Premier plan", edit::Order::Front),
                            ("Arrière-plan", edit::Order::Back),
                            ("Avancer", edit::Order::Forward),
                            ("Reculer", edit::Order::Backward),
                        ] {
                            if ui.button(label).clicked() {
                                let ids = self.selection.clone();
                                self.mutate(|d| edit::reorder(d, &ids, order));
                                ui.close();
                            }
                        }
                    });
                });
                ui.add_enabled_ui(self.selection.len() > 1, |ui| {
                    ui.menu_button("Aligner", |ui| {
                        for (label, how) in [
                            ("À gauche", edit::Alignment::Left),
                            ("Au centre", edit::Alignment::Centre),
                            ("À droite", edit::Alignment::Right),
                            ("En haut", edit::Alignment::Top),
                            ("Au milieu", edit::Alignment::Middle),
                            ("En bas", edit::Alignment::Bottom),
                        ] {
                            if ui.button(label).clicked() {
                                let ids = self.selection.clone();
                                self.mutate(|d| edit::align(d, &ids, how));
                                ui.close();
                            }
                        }
                    });
                });
            });
            ui.menu_button("Paramètres", |ui| {
                if ui.button("Compléments script LDD…").clicked() {
                    // stored with CRLF line ends, edited with LF
                    self.complements = Some(std::array::from_fn(|i| self.doc.strings_ea0[i].replace("\r\n", "\n")));
                    ui.close();
                }
                ui.separator();
                // decoded options of the settings block, as in Looping's property grid
                for scripts in [true, false] {
                    ui.label(if scripts { "MLD et script SQL" } else { "Affichage et édition" });
                    for o in looping_core::options::OPTIONS.iter().filter(|o| o.affects_scripts == scripts) {
                        let current = self.doc.settings.i32(o.offset);
                        ui.menu_button(o.title, |ui| {
                            for (v, label) in o.values.iter().enumerate() {
                                if ui.radio(current == v as i32, *label).clicked() {
                                    let off = o.offset;
                                    self.mutate(|d| d.settings.set_i32(off, v as i32));
                                    if off == 0x52c {
                                        self.dbms = looping_core::Dbms::from_index(v as i32);
                                    }
                                    ui.close();
                                }
                            }
                        });
                    }
                    if scripts {
                        // "Nom clé étrangère" (+0x7cc / +0xa52), with Looping's proposals
                        let current = self.doc.settings.fk_mask();
                        ui.menu_button("Nom clé étrangère", |ui| {
                            for mask in looping_core::options::FK_MASKS {
                                if ui.radio(current == mask, mask).clicked() {
                                    self.mutate(|d| d.settings.set_fk_mask(mask));
                                    ui.close();
                                }
                            }
                        });
                    }
                    ui.separator();
                }
                // on/off properties of the grid ("Propriétés" sections)
                ui.menu_button("Autres propriétés", |ui| {
                    let fields = looping_core::options::FIELDS
                        .iter()
                        .filter(|f| f.kind == looping_core::options::FieldKind::Bool);
                    for f in fields {
                        let mut on = self.doc.settings.i32(f.offset) != 0;
                        if ui.checkbox(&mut on, format!("{} — {}", f.section, f.title)).changed() {
                            let off = f.offset;
                            self.mutate(|d| d.settings.set_i32(off, on as i32));
                        }
                    }
                });
            });
            ui.menu_button("Affichage", |ui| self.display_menu(ui));
            ui.menu_button("Vues", |ui| self.views_menu(ui));
        });
    }

    /// Ribbon groups « Formalisme » (MCD notation) and « Affichage »,
    /// and the magnetism of « Édition ».
    fn display_menu(&mut self, ui: &mut egui::Ui) {
        // kept in the document as Looping does at saving (`+0x540`,
        // `+0x544`, `+0x748`), without an undo step
        for (off, label) in [(0x540, "Rubriques"), (0x544, "Types de données"), (0x748, "Illustrations")] {
            let mut on = self.doc.settings.i32(off) != 0;
            if ui.checkbox(&mut on, label).changed() {
                self.doc.settings.set_i32(off, on as i32);
                self.dirty = true;
                self.refresh();
            }
        }
        ui.separator();
        ui.label("Notation MCD");
        let notation = self.doc.settings.i32(0x7c0);
        for (v, label) in [(0, "Standard"), (1, "Flèche pour DF"), (2, "Association simplifiée")] {
            if ui.radio(notation == v, label).clicked() {
                self.mutate(|d| d.settings.set_i32(0x7c0, v));
                ui.close();
            }
        }
        ui.separator();
        if ui.button("Ajuster à la fenêtre").clicked() {
            self.view.fit_next = true;
            ui.close();
        }
        for pct in [50, 100, 150] {
            if ui.button(format!("Zoom {pct} %")).clicked() {
                self.view.zoom_request = Some(pct as f32 / 100.0);
                ui.close();
            }
        }
        ui.separator();
        ui.label("Magnétisme");
        let current = (self.doc.f33c != 0, self.doc.f340 != 0);
        for ((on, half), label) in [((true, false), "Grille"), ((true, true), "Demi-grille"), ((false, false), "Aucun")]
        {
            if ui.radio(current == (on, half), label).clicked() {
                self.doc.f33c = on as u16;
                self.doc.f340 = half as u16;
                self.dirty = true;
                ui.close();
            }
        }
    }

    /// « Vue » (33222: every object) and the document's views (33223…):
    /// several can be active together.
    fn views_menu(&mut self, ui: &mut egui::Ui) {
        if ui.radio(self.views_mask & 1 != 0, "Tous les objets").clicked() {
            self.views_mask = 1;
            self.refresh();
        }
        let views = self.doc.views.clone();
        for v in &views {
            let bit = edit::view_bit(v);
            let mut on = self.views_mask & 1 == 0 && self.views_mask & bit != 0;
            if ui.checkbox(&mut on, &v.name).changed() {
                let mask = if on { (self.views_mask & !1) | bit } else { self.views_mask & !bit };
                self.views_mask = if mask == 0 { 1 } else { mask };
                self.refresh();
            }
        }
        ui.separator();
        if ui.button("Gérer les vues…").clicked() {
            self.views_window = true;
            ui.close();
        }
    }

    /// Names of the views: add, rename, remove (Looping does it in
    /// « Urbanisation », dialogs 201 and 202).
    fn views_window(&mut self, ctx: &egui::Context) {
        if !self.views_window {
            return;
        }
        let mut open = true;
        let mut change: Option<DocChange> = None;
        egui::Window::new("Vues").open(&mut open).collapsible(false).show(ctx, |ui| {
            let views = self.doc.views.clone();
            egui::Grid::new("views").num_columns(2).show(ui, |ui| {
                for v in views {
                    let mut name = v.name.clone();
                    if ui.text_edit_singleline(&mut name).changed() {
                        let bit = v.f8;
                        change = Some(Box::new(move |d| {
                            if let Some(x) = d.views.iter_mut().find(|x| x.f8 == bit) {
                                x.name = name;
                            }
                        }));
                    }
                    if ui.button("Supprimer").clicked() {
                        let bit = v.f8;
                        change = Some(Box::new(move |d| edit::remove_view(d, bit)));
                    }
                    ui.end_row();
                }
            });
            if ui.add_enabled(self.doc.views.len() < 20, egui::Button::new("Nouvelle vue")).clicked() {
                let n = self.doc.views.len() + 1;
                change = Some(Box::new(move |d| {
                    edit::add_view(d, &format!("Vue {n}"));
                }));
            }
            ui.weak("Les objets créés quand des vues sont actives leur appartiennent ; le panneau Propriétés règle l'appartenance d'un objet.");
        });
        if let Some(f) = change {
            self.mutate(f);
            // a removed view is no longer active
            let bits: u32 = self.doc.views.iter().map(edit::view_bit).fold(1, |a, b| a | b);
            self.views_mask &= bits;
            if self.views_mask == 0 {
                self.views_mask = 1;
            }
            self.refresh();
        }
        self.views_window = open;
    }

    /// « Compléments script LDD » (dialog 190): SQL put at the start and end
    /// of the script and of each table (`[Nom_Table]`: the table's name).
    fn complements_window(&mut self, ctx: &egui::Context) {
        let Some(draft) = self.complements.as_mut() else { return };
        let mut open = true;
        let (mut ok, mut cancel) = (false, false);
        egui::Window::new("Compléments script LDD").open(&mut open).collapsible(false).default_width(520.0).show(
            ctx,
            |ui| {
                for (i, title) in [
                    "Code SQL de début de script",
                    "Code SQL de fin de script",
                    "Code SQL de début de création de table, mot clé accepté : [Nom_Table]",
                    "Code SQL de fin de création de table, mot clé accepté : [Nom_Table]",
                ]
                .iter()
                .enumerate()
                {
                    ui.label(*title);
                    ui.add(
                        egui::TextEdit::multiline(&mut draft[i])
                            .code_editor()
                            .desired_rows(3)
                            .desired_width(f32::INFINITY),
                    );
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ok = ui.button("OK").clicked();
                    cancel = ui.button("Annuler").clicked();
                });
            },
        );
        if ok {
            let values = draft.clone();
            self.complements = None;
            self.mutate(|d| {
                for (i, v) in values.iter().enumerate() {
                    d.strings_ea0[i] = v.replace("\r\n", "\n").replace('\n', "\r\n");
                }
            });
        } else if cancel || !open {
            self.complements = None;
        }
    }

    /// Magnetism after a move: the moved objects on the grid.
    pub fn snap_selection(&mut self) {
        let Some(step) = edit::grid_step(&self.doc) else { return };
        for &id in &self.selection {
            edit::snap(&mut self.doc, id, step);
        }
        self.refresh();
    }

    fn undo_step(&mut self) {
        if let Some(prev) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.doc, prev));
            self.dirty = true;
            self.refresh();
        }
    }

    fn redo_step(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.doc, next));
            self.dirty = true;
            self.refresh();
        }
    }

    fn delete_selected(&mut self) {
        let ids = std::mem::take(&mut self.selection);
        if !ids.is_empty() {
            self.mutate(|d| {
                for id in ids {
                    edit::delete(d, id);
                }
            });
        }
    }

    pub fn select(&mut self, id: Option<ObjId>) {
        self.selection = id.into_iter().collect();
    }

    /// The object shown in the side panel.
    pub fn primary(&self) -> Option<ObjId> {
        self.selection.last().copied()
    }

    /// Objects that can be selected: all but texts and link segments after
    /// the first.
    pub fn selectable(&self) -> Vec<ObjId> {
        let display = self.display();
        self.doc
            .objects
            .iter()
            .filter(|o| display.shows(&self.doc, o))
            .filter(|o| o.class != "CDrawTexte" && o.class != "CDrawEntiteUML")
            .filter(|o| !o.class.starts_with("CDrawArc") || o.base.links.prev == 0)
            .filter(|o| !(o.class == "CDrawTrait" || o.class == "CDrawFleche") || o.base.links.prev == 0)
            .map(|o| o.base.id)
            .collect()
    }

    fn select_all(&mut self) {
        self.selection = self.selectable();
    }

    /// Internal clipboard (Looping's « Looping Copier Coller » format is a
    /// Windows clipboard format, not reproduced).
    fn copy(&mut self) {
        self.clipboard = edit::copy(&self.doc, &self.selection);
        self.pastes = 0;
    }

    fn cut(&mut self) {
        self.copy();
        self.delete_selected();
    }

    fn paste(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }
        self.pastes += 1;
        let (clip, shift) = (self.clipboard.clone(), 20 * self.pastes);
        let mut ids = Vec::new();
        self.mutate(|d| ids = edit::paste(d, &clip, shift, shift));
        self.selection = ids;
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            for t in [
                Tool::Select,
                Tool::Entity,
                Tool::Association,
                Tool::Heritage,
                Tool::Contrainte,
                Tool::Cif,
                Tool::Regle,
                Tool::Link,
                Tool::Cassure,
            ] {
                self.tool_button(ui, t);
            }
            ui.separator();
            // « Illustration » group of Looping's ribbon
            ui.menu_button(
                if matches!(
                    self.tool,
                    Tool::Texte
                        | Tool::Ligne
                        | Tool::Fleche
                        | Tool::Rectangle
                        | Tool::Ellipse
                        | Tool::Crayon
                        | Tool::Image
                        | Tool::SousModele
                ) {
                    format!("Illustration : {}", self.tool.label())
                } else {
                    "Illustration".to_string()
                },
                |ui| {
                    for t in [
                        Tool::Texte,
                        Tool::Ligne,
                        Tool::Fleche,
                        Tool::Rectangle,
                        Tool::Ellipse,
                        Tool::Crayon,
                        Tool::Image,
                        Tool::SousModele,
                    ] {
                        if self.tool_button(ui, t) {
                            ui.close();
                        }
                    }
                },
            );
            ui.separator();
            ui.label("SGBD :");
            egui::ComboBox::from_id_salt("dbms").selected_text(self.dbms.name()).show_ui(ui, |ui| {
                for d in Dbms::ALL {
                    ui.selectable_value(&mut self.dbms, d, d.name());
                }
            });
            ui.separator();
            // « Zoom » slider of the ribbon (0–200 %, at least 10 %)
            let mut pct = self.view.zoom * 100.0;
            if ui.add(egui::Slider::new(&mut pct, 10.0..=200.0).suffix(" %").integer()).changed() {
                self.view.zoom_request = Some(pct / 100.0);
            }
        });
    }

    fn tool_button(&mut self, ui: &mut egui::Ui, t: Tool) -> bool {
        let clicked = ui.selectable_label(self.tool == t, t.label()).clicked();
        if clicked {
            self.tool = t;
            self.link_from = None;
            self.draft.clear();
        }
        clicked
    }

    fn properties(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.primary() else {
            ui.weak("Aucune sélection");
            ui.add_space(8.0);
            ui.label(format!("{} entités, {} associations", self.mcd.entities.len(), self.mcd.associations.len()));
            return;
        };
        let Some(obj) = self.doc.object(id).cloned() else {
            self.select(None);
            return;
        };
        let kind = match (&obj.kind, obj.class.as_str()) {
            (ObjKind::Entite(_), _) => "Entité",
            (ObjKind::Relation(_), _) => "Association",
            (ObjKind::Arc(_), "CDrawArcObjetLibre") => "Lien entre illustrations",
            (ObjKind::Arc(_), _) => "Lien",
            (ObjKind::Titre(_), _) => "Texte",
            (ObjKind::Rectangle(_), _) => "Rectangle",
            (ObjKind::Ellipse(_), _) => "Ellipse",
            (ObjKind::Image(i), _) if i.f3c0 != 0 => "Sous-modèle",
            (ObjKind::Image(_), _) => "Image",
            (ObjKind::Trait(_), "CDrawFleche") => "Flèche",
            (ObjKind::Trait(_), _) => "Ligne",
            (ObjKind::Crayon(_), _) => "Crayon",
            _ => obj.class.as_str(),
        };
        ui.heading(kind);
        if ui.button("Propriétés…").on_hover_text("Dialogue de l'objet (double-clic)").clicked() {
            self.open_dialog(id);
        }
        let named = matches!(obj.kind, ObjKind::Entite(_) | ObjKind::Relation(_));
        if named {
            let mut name = looping_core::mcd::object_name(&self.doc, &obj);
            ui.horizontal(|ui| {
                ui.label("Nom :");
                if ui.text_edit_singleline(&mut name).changed() {
                    self.mutate(|d| edit::rename(d, id, &name));
                }
            });
            ui.separator();
            ui.label("Rubriques :");
            let params: Vec<_> = self
                .doc
                .object(obj.base.links.texts[1])
                .and_then(|o| o.as_texte())
                .map(|t| t.params.clone())
                .unwrap_or_default();
            egui::Grid::new("attrs").striped(true).show(ui, |ui| {
                for p in params.iter().filter(|p| p.role == 0) {
                    let name = egui::RichText::new(&p.name);
                    ui.label(if p.f8 != 0 { name.underline().strong() } else { name });
                    ui.weak(looping_core::DataType::from_param(p).label());
                    ui.end_row();
                }
            });
        }
        if let ObjKind::Arc(_) = obj.kind {
            let mut first = obj.clone();
            while let Some(p) = self.doc.object(first.base.links.prev) {
                first = p.clone();
            }
            let current = self
                .doc
                .object(first.base.links.texts[1])
                .and_then(|o| o.as_texte())
                .and_then(|t| t.params.iter().find(|p| p.role == 1))
                .map(|p| p.cardinality)
                .unwrap_or(2);
            ui.horizontal(|ui| {
                ui.label("Cardinalité :");
                for (code, text) in [(0, "0,1"), (1, "1,1"), (2, "0,n"), (3, "1,n")] {
                    if ui.selectable_label(current == code, text).clicked() {
                        let fid = first.base.id;
                        self.mutate(|d| edit::set_cardinality(d, fid, code));
                    }
                }
            });
        }
        // views the object belongs to (`CDrawObj+0x338`; Looping's object
        // dialogs have a box per view)
        if !self.doc.views.is_empty() {
            ui.separator();
            ui.label("Vues :");
            let member = obj.base.f338 as u32;
            for v in self.doc.views.clone() {
                let bit = edit::view_bit(&v);
                let mut on = member & bit != 0;
                if ui.checkbox(&mut on, &v.name).changed() {
                    self.mutate(|d| edit::set_views(d, id, bit, on));
                }
            }
        }
        ui.add_space(12.0);
        if ui.button("Supprimer").clicked() {
            self.delete_selected();
        }
    }

    fn outputs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, Tab::Mld, "MLD textuel");
            ui.selectable_value(&mut self.tab, Tab::Sql, format!("Script SQL ({})", self.dbms.name()));
        });
        let mut text = match self.tab {
            Tab::Mld => self.mld.to_text(&self.sql_options()),
            Tab::Sql => sql::generate_with(&self.mld, &self.sql_options()),
        };
        if ui.button("Copier").clicked() {
            ui.ctx().copy_text(text.clone());
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(egui::TextEdit::multiline(&mut text).code_editor().desired_width(f32::INFINITY));
        });
    }

    /// Canvas click with a creation tool.
    pub fn create_at(&mut self, p: Point, hit: Option<ObjId>) {
        match self.tool {
            Tool::Entity => {
                let n = self.mcd.entities.len() + 1;
                self.create(|d| edit::add_entity(d, &format!("Entité_{n}"), p));
            }
            Tool::Association => {
                let n = self.mcd.associations.len() + 1;
                self.create(|d| edit::add_association(d, &format!("Association_{n}"), p));
            }
            Tool::Heritage => self.create(|d| edit::add_heritage(d, p, 0)),
            Tool::Contrainte => {
                let n = self.mcd.constraints.len() + 1;
                self.create(|d| edit::add_contrainte(d, &format!("Contrainte_{n}"), p, 0))
            }
            Tool::Cif => self.create(|d| edit::add_cif(d, p)),
            Tool::Regle => {
                let n = self.doc.objects.iter().filter(|o| matches!(o.kind, ObjKind::Regle(_))).count() + 1;
                self.create(|d| edit::add_regle(d, &format!("Règle_{n}"), p, &[], &[], false, [1, 0, 0]))
            }
            Tool::Link => {
                let Some(h) = hit else { return };
                match self.link_from {
                    None => self.link_from = Some(h),
                    Some(a) => {
                        self.link_from = None;
                        match self.link_kind(a, h) {
                            Ok(make) => {
                                let mut id = 0;
                                self.mutate(|d| id = make(d));
                                self.select(Some(id));
                                self.status = "Lien créé".into();
                            }
                            Err(msg) => self.status = msg.into(),
                        }
                    }
                }
            }
            Tool::Cassure => {
                // on a break point: remove it; on a link: add one there
                let Some(h) = hit else { return };
                let Some(o) = self.doc.object(h) else { return };
                if !o.class.starts_with("CDrawArc") {
                    return;
                }
                let first = edit::first_segment(&self.doc, h);
                let chain = edit::chain_ids(&self.doc, first);
                let near = |id: ObjId| match self.doc.object(id).map(|o| &o.kind) {
                    Some(ObjKind::Arc(a)) if a.anchor.mode == 0 => {
                        (a.anchor.offset.x - p.x).abs() <= 6 && (a.anchor.offset.y - p.y).abs() <= 6
                    }
                    _ => false,
                };
                if let Some(&bend) =
                    chain.iter().find(|&&id| near(id) && self.doc.object(id).is_some_and(|o| o.base.links.owner == 0))
                {
                    self.mutate(|d| edit::remove_break(d, bend));
                    return;
                }
                // segment of the chain nearest to the click
                let seg = canvas::nearest_segment(self, &chain, p);
                self.mutate(|d| {
                    edit::add_break(d, seg, p);
                });
            }
            // a click without drag: a frame of a default size (120 × 60)
            t if t.draws_frame() => {
                let r = loo_format::Rect { left: p.x, top: p.y, right: p.x + 120, bottom: p.y + 60 };
                self.create_frame(r);
            }
            // each click adds a point; double-click or Entrée ends the line
            Tool::Ligne | Tool::Fleche if self.draft.last() != Some(&p) => self.draft.push(p),
            _ => {}
        }
    }

    /// Frame tools: the illustration in `r`, drawn with the mouse.
    pub fn create_frame(&mut self, r: loo_format::Rect) {
        match self.tool {
            Tool::Rectangle => self.create(|d| edit::add_rectangle(d, r)),
            Tool::Ellipse => self.create(|d| edit::add_ellipse(d, r)),
            // Looping asks for the text first (dialog 131)
            Tool::Texte => {
                self.create(|d| edit::add_texte(d, r, &["Texte"]));
                if let Some(id) = self.primary() {
                    self.open_dialog(id);
                }
            }
            Tool::Image => {
                self.create(|d| edit::add_image(d, r, false));
                if let Some(id) = self.primary() {
                    self.choose_image(id);
                }
            }
            Tool::SousModele => {
                self.create(|d| edit::add_image(d, r, true));
                if let Some(id) = self.primary() {
                    self.open_dialog(id);
                }
            }
            _ => {}
        }
    }

    /// Ends the line or arrow being drawn (at least two points).
    pub fn finish_line(&mut self) {
        let points = std::mem::take(&mut self.draft);
        if points.len() >= 2 {
            let arrow = self.tool == Tool::Fleche;
            self.create(|d| edit::add_free_line(d, &points, arrow));
        }
    }

    /// Ends the free-hand stroke being drawn.
    pub fn finish_crayon(&mut self) {
        let points = std::mem::take(&mut self.draft);
        if points.len() >= 2 {
            self.create(|d| edit::add_crayon(d, &points));
        }
    }

    /// Image file of `id`, chosen as Looping does on a double-click
    /// (« *.jpg; *.jpeg; *.bmp; *.png; *.gif »).
    pub fn choose_image(&mut self, id: ObjId) {
        let Some(p) = rfd::FileDialog::new().add_filter("Images", &["jpg", "jpeg", "bmp", "png", "gif"]).pick_file()
        else {
            return;
        };
        let path = p.to_string_lossy().into_owned();
        self.mutate(|d| {
            if let Some(ObjKind::Image(i)) = d.objects.iter_mut().find(|o| o.base.id == id).map(|o| &mut o.kind) {
                i.path = path;
            }
        });
    }

    /// Folder of the document, against which image and model paths that
    /// are not absolute are resolved.
    pub fn doc_dir(&self) -> Option<PathBuf> {
        self.path.as_ref().and_then(|p| p.parent()).map(PathBuf::from)
    }

    /// Creates an object with `make`, selects it and goes back to selection.
    /// It belongs to the active views (constructors: main frame `+0x130`
    /// without bit 0, in `CDrawObj+0x338`).
    fn create(&mut self, make: impl FnOnce(&mut Document) -> ObjId) {
        let mut id = 0;
        let views = self.views_mask & !1;
        self.mutate(|d| {
            id = make(d);
            if views != 0 {
                edit::set_views(d, id, views, true);
            }
        });
        self.select(Some(id));
        self.tool = Tool::Select;
    }

    /// Link between `a` and `b`, its kind following the two ends as in
    /// Looping: entity–association, entity–inheritance, entity or
    /// association–CIF, association or entity–constraint, anything–rule.
    #[allow(clippy::type_complexity)]
    fn link_kind(&self, a: ObjId, b: ObjId) -> Result<Box<dyn FnOnce(&mut Document) -> ObjId>, &'static str> {
        #[derive(PartialEq, Clone, Copy)]
        enum K {
            Ent,
            Asso,
            Her,
            Cont,
            Cif,
            Regle,
            /// rectangle, ellipse, image or text frame
            Illus,
            Other,
        }
        let k = |id: ObjId| match self.doc.object(id).map(|o| &o.kind) {
            Some(ObjKind::Entite(_)) => K::Ent,
            Some(ObjKind::Relation(_)) => K::Asso,
            Some(ObjKind::Heritage(_)) => K::Her,
            Some(ObjKind::Contrainte(_)) => K::Cont,
            Some(ObjKind::Cif(_)) => K::Cif,
            Some(ObjKind::Regle(_)) => K::Regle,
            Some(ObjKind::Rectangle(_) | ObjKind::Ellipse(_) | ObjKind::Image(_) | ObjKind::Titre(_)) => K::Illus,
            _ => K::Other,
        };
        let (ka, kb) = (k(a), k(b));
        // the same two objects linked twice: only legs of a (reflexive)
        // association make sense
        if !matches!((ka, kb), (K::Ent, K::Asso) | (K::Asso, K::Ent)) && self.linked(a, b) {
            return Err("Ces deux objets sont déjà reliés");
        }
        // an illustration links to an entity, an association, a rule or
        // another illustration (masks 0xc408d / 0xc4999 / 0xc489c of 0x1401187c0)
        let illus_end = |x: K| matches!(x, K::Illus | K::Ent | K::Asso | K::Regle);
        if (ka == K::Illus || kb == K::Illus) && illus_end(ka) && illus_end(kb) {
            return Ok(Box::new(move |d| edit::add_illustration_link(d, a, b)));
        }
        let pair = |x: K, y: K| (ka == x && kb == y).then_some((a, b)).or((ka == y && kb == x).then_some((b, a)));
        if let Some((e, r)) = pair(K::Ent, K::Asso) {
            return Ok(Box::new(move |d| edit::add_link(d, e, r, 2)));
        }
        if let Some((e, h)) = pair(K::Ent, K::Her) {
            // the first entity linked to the symbol is the supertype
            let first = !self.doc.objects.iter().any(|o| o.class == "CDrawArcHeritage" && o.base.links.owner == h);
            let first = first && !self.mcd.inheritances.iter().any(|i| i.id == h && i.supertype.is_some());
            return Ok(Box::new(move |d| edit::add_heritage_link(d, e, h, first)));
        }
        for x in [K::Ent, K::Asso] {
            if let Some((o, c)) = pair(x, K::Cif) {
                return Ok(Box::new(move |d| edit::add_cif_link(d, o, c, false)));
            }
            if let Some((o, c)) = pair(x, K::Cont) {
                return Ok(Box::new(move |d| edit::add_contrainte_link(d, o, c)));
            }
        }
        if kb == K::Regle && ka != K::Regle && ka != K::Other {
            return Ok(Box::new(move |d| edit::add_regle_link(d, a, b)));
        }
        if ka == K::Regle && kb != K::Regle && kb != K::Other {
            return Ok(Box::new(move |d| edit::add_regle_link(d, b, a)));
        }
        Err("Liens possibles : entité–association, entité–héritage, entité ou association–CIF ou contrainte, objet–règle, illustration–objet")
    }

    /// True when a link (any class) already joins `a` and `b`.
    fn linked(&self, a: ObjId, b: ObjId) -> bool {
        self.doc.objects.iter().filter(|o| o.class.starts_with("CDrawArc") && o.base.links.prev == 0).any(|first| {
            let chain = edit::chain_ids(&self.doc, first.base.id);
            let end = chain.last().and_then(|&l| self.doc.object(l)).map_or(0, |l| l.base.links.owner);
            let ends = [first.base.links.owner, end];
            ends.contains(&a) && ends.contains(&b)
        })
    }

    /// Opens the dialog of `id` (Looping's double-click).
    pub fn open_dialog(&mut self, id: ObjId) {
        self.select(Some(id));
        self.dialog = dialogs::Dialog::open(&self.doc, id);
    }

    fn dialog_window(&mut self, ctx: &egui::Context) {
        let Some(dlg) = self.dialog.as_mut() else { return };
        match dlg.show(ctx, &self.doc) {
            dialogs::Outcome::Open => {}
            dialogs::Outcome::Close => self.dialog = None,
            dialogs::Outcome::Apply(objs) => {
                let kind = dlg.kind;
                let target = dlg.target;
                self.dialog = None;
                self.mutate(|d| {
                    for o in objs {
                        if let Some(slot) = d.objects.iter_mut().find(|x| x.base.id == o.base.id) {
                            *slot = o;
                        }
                    }
                    if kind == dialogs::Kind::Entite {
                        edit::fit_box(d, target);
                    }
                    if matches!(kind, dialogs::Kind::Trait | dialogs::Kind::Fleche) {
                        edit::sync_free_line(d, target);
                    }
                    if kind.is_illustration() {
                        edit::remember_style(d, target);
                    }
                });
            }
            dialogs::Outcome::OpenModel(path) => {
                // one window per document: the model opens in a new one
                let path = match self.doc_dir() {
                    Some(dir) if !std::path::Path::new(&path).is_absolute() => dir.join(&path),
                    _ => PathBuf::from(&path),
                };
                match std::env::current_exe().and_then(|exe| std::process::Command::new(exe).arg(&path).spawn()) {
                    Ok(_) => self.status = format!("Ouverture du sous-modèle {}", path.display()),
                    Err(e) => self.status = format!("Erreur : {e}"),
                }
            }
        }
    }

    /// Moves the selection by (`dx`, `dy`); `first_step` starts an undo step.
    /// Writes what changed since the last frame (ROLLERCOASTER_TRACE).
    fn trace_state(&mut self) {
        let names = |ids: &[ObjId]| -> Vec<String> {
            ids.iter()
                .filter_map(|&id| {
                    self.doc.object(id).map(|o| format!("{id}:{}", looping_core::mcd::object_name(&self.doc, o)))
                })
                .collect()
        };
        let tool = self.tool.label();
        let state = format!(
            "outil={tool} sélection={:?} objets={} annulations={} dialogue={:?} statut={:?}",
            names(&self.selection),
            self.doc.objects.len(),
            self.undo.len(),
            self.dialog.as_ref().map(|d| d.kind),
            self.status,
        );
        if state != self.traced {
            eprintln!("[trace] frame {} {state}", self.frames);
            if self.doc.objects.len() != self.traced_objects() {
                eprintln!("[trace]   MLD: {}", self.mld.to_text(&self.sql_options()).replace('\n', " | "));
            }
            self.traced = state;
        }
    }

    fn traced_objects(&self) -> usize {
        self.traced.split("objets=").nth(1).and_then(|r| r.split(' ').next()).and_then(|n| n.parse().ok()).unwrap_or(0)
    }

    pub fn move_selected(&mut self, dx: i32, dy: i32, first_step: bool) {
        if self.selection.is_empty() {
            return;
        }
        if first_step {
            self.checkpoint();
        }
        for &id in &self.selection {
            edit::translate(&mut self.doc, id, dx, dy);
        }
        self.refresh();
    }

    /// Applies `f` to the document within the current undo step (drags).
    pub fn apply_drag(&mut self, first_step: bool, f: impl FnOnce(&mut Document)) {
        if first_step {
            self.checkpoint();
        }
        f(&mut self.doc);
        self.refresh();
    }

    fn sql_options(&self) -> sql::SqlOptions {
        sql::SqlOptions { dbms: self.dbms, ..sql::SqlOptions::from_settings(&self.doc.settings) }
    }

    pub fn scene(&self) -> &render::Scene {
        &self.scene
    }

    pub fn doc(&self) -> &Document {
        &self.doc
    }
}

impl eframe::App for App {
    fn on_exit(&mut self) {
        if trace() {
            eprintln!("[trace] on_exit after {} frames", self.frames);
        }
    }

    /// `ROLLERCOASTER_KEYS=ctrl+a,ctrl+c`: keys pressed one per frame from
    /// frame 10 (debugging aid, with ROLLERCOASTER_SCREENSHOT).
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        if trace() {
            for e in &raw.events {
                if matches!(
                    e,
                    egui::Event::Key { .. }
                        | egui::Event::Copy
                        | egui::Event::Cut
                        | egui::Event::Paste(_)
                        | egui::Event::Text(_)
                        | egui::Event::ModifiersChanged(_)
                        | egui::Event::WindowFocused(_)
                ) {
                    eprintln!("[trace] frame {} event {e:?}", self.frames);
                }
            }
            if raw.viewport().close_requested() {
                eprintln!("[trace] frame {} close requested by the window system", self.frames);
            }
        }
        let Ok(keys) = std::env::var("ROLLERCOASTER_KEYS") else { return };
        let Some(k) = keys.split(',').nth((self.frames as usize).wrapping_sub(10)) else { return };
        let ctrl = k.starts_with("ctrl+");
        let name = k.trim_start_matches("ctrl+");
        let modifiers = egui::Modifiers { ctrl, command: ctrl, ..Default::default() };
        raw.events.push(egui::Event::ModifiersChanged(modifiers));
        if let Some(key) = egui::Key::from_name(&name.to_uppercase()) {
            raw.events.push(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers });
        }
        match (ctrl, name) {
            (true, "c") => raw.events.push(egui::Event::Copy),
            (true, "x") => raw.events.push(egui::Event::Cut),
            (true, "v") => raw.events.push(egui::Event::Paste(String::new())),
            _ => {}
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.frames += 1;
        debug_screenshot(&ctx, self.frames);
        // dropped files
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect()
        });
        if let Some(p) = dropped.into_iter().next() {
            if self.confirm_discard() {
                self.open(p);
            }
        }
        // closing the window: « Enregistrer les modifications ? »
        if ctx.input(|i| i.viewport().close_requested()) && !self.confirm_discard() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        // shortcuts
        let typing = ctx.egui_wants_keyboard_input();
        let (undo, redo, del, save, new, open) = ctx.input(|i| {
            (
                i.modifiers.command && !i.modifiers.shift && i.key_pressed(egui::Key::Z),
                i.modifiers.command && i.modifiers.shift && i.key_pressed(egui::Key::Z),
                !typing && (i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)),
                i.modifiers.command && i.key_pressed(egui::Key::S),
                i.modifiers.command && i.key_pressed(egui::Key::N),
                i.modifiers.command && i.key_pressed(egui::Key::O),
            )
        });
        if new {
            self.new_document();
        }
        if open {
            self.open_dialog_file();
        }
        // clipboard and selection shortcuts, when no text field has the focus
        // (egui turns Ctrl+C / X / V into events)
        if !typing && self.dialog.is_none() {
            let (copy, cut, paste, all) = ctx.input(|i| {
                let ev = |want: &egui::Event| i.events.iter().any(|e| e == want);
                (
                    ev(&egui::Event::Copy),
                    ev(&egui::Event::Cut),
                    i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))),
                    i.modifiers.command && i.key_pressed(egui::Key::A),
                )
            });
            if copy {
                self.copy();
            }
            if cut {
                self.cut();
            }
            if paste {
                self.paste();
            }
            if all {
                self.select_all();
            }
        }
        if undo {
            self.undo_step();
        }
        if redo {
            self.redo_step();
        }
        if del {
            self.delete_selected();
        }
        if save {
            self.save();
        }
        let title = format!(
            "Rollercoaster — {}{}",
            self.path
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or("Sans titre".into()),
            if self.dirty { " •" } else { "" }
        );
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        egui::Panel::top("menu").show(ui, |ui| {
            self.menu(ui);
            self.toolbar(ui);
        });
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.label(&self.status);
        });
        egui::Panel::right("props").resizable(true).default_size(320.0).show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| self.properties(ui));
        });
        egui::Panel::bottom("outputs").resizable(true).default_size(240.0).show(ui, |ui| self.outputs(ui));
        egui::CentralPanel::default().show(ui, |ui| canvas::show(self, ui));
        self.dialog_window(&ctx);
        self.views_window(&ctx);
        self.complements_window(&ctx);
        if trace() {
            self.trace_state();
        }
    }
}

/// `ROLLERCOASTER_SCREENSHOT=out.png` saves the window after a few frames and quits
/// (used to check rendering without screen-recording permissions).
fn debug_screenshot(ctx: &egui::Context, frames: u32) {
    let Ok(out) = std::env::var("ROLLERCOASTER_SCREENSHOT") else {
        return;
    };
    ctx.request_repaint();
    if frames == 20 {
        ctx.request_screenshot(move |img| {
            let file = std::fs::File::create(&out).expect("screenshot file");
            let mut enc = png::Encoder::new(std::io::BufWriter::new(file), img.size[0] as u32, img.size[1] as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let data: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
            enc.write_header().and_then(|mut w| w.write_image_data(&data)).expect("png");
            std::process::exit(0);
        });
    }
}
