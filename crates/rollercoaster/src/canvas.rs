//! Diagram canvas: paints the core display list, handles pan/zoom,
//! selection, dragging and creation tools.

use eframe::egui::{self, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Vec2};
use loo_format::Point;
use looping_core::render::{self, Align, Color, P};

use crate::{App, Tool};
use looping_core::{edit, mcd::ObjId};

/// egui font families of the embedded Carlito faces (see `install_fonts`).
pub const CARLITO: &str = "carlito";
pub const CARLITO_BOLD: &str = "carlito-bold";

/// Registers Carlito (metric clone of Calibri, the documents' font) for the
/// diagram texts.
pub fn install_fonts(ctx: &egui::Context) {
    let mut defs = egui::FontDefinitions::default();
    for (name, data) in
        [(CARLITO, looping_core::text::CARLITO_REGULAR), (CARLITO_BOLD, looping_core::text::CARLITO_BOLD)]
    {
        defs.font_data.insert(name.into(), std::sync::Arc::new(egui::FontData::from_static(data)));
        defs.families.insert(egui::FontFamily::Name(name.into()), vec![name.into()]);
    }
    ctx.set_fonts(defs);
}

pub struct View {
    /// Screen offset of document origin, relative to the canvas.
    pub offset: Vec2,
    pub zoom: f32,
    pub fit_next: bool,
    /// Zoom asked for (menu, slider), applied around the window's centre.
    pub zoom_request: Option<f32>,
    /// Zoom saved with the document, applied with the diagram at the top left.
    pub restore_zoom: Option<f32>,
    drag: Drag,
    drag_rest: Vec2,
    moved: bool,
    /// Image files shown in the diagram, by resolved path (`None`: unreadable).
    images: std::collections::HashMap<std::path::PathBuf, Option<egui::TextureHandle>>,
}

/// What the current drag does.
#[derive(Clone, Copy, PartialEq)]
enum Drag {
    None,
    Pan,
    /// Moves the selection.
    Move,
    /// Selection rectangle from this document point.
    Band(P),
    /// Resize handle `handle` (0 top-left, clockwise) of `id`, from `rect`.
    Resize {
        id: ObjId,
        handle: usize,
        rect: loo_format::Rect,
    },
    /// Break point of a link.
    Bend(ObjId),
    /// A text moved from its place (`+0x458/+0x45c`).
    Text(ObjId),
    /// Frame of a new illustration, from this document point.
    Frame(P),
    /// Free-hand stroke being drawn (points in `App::draft`).
    Stroke,
}

impl Default for View {
    fn default() -> Self {
        View {
            offset: Vec2::new(40.0, 40.0),
            zoom: 1.0,
            fit_next: true,
            zoom_request: None,
            restore_zoom: None,
            drag: Drag::None,
            drag_rest: Vec2::ZERO,
            moved: false,
            images: Default::default(),
        }
    }
}

impl View {
    fn to_screen(&self, origin: Pos2, p: P) -> Pos2 {
        origin + self.offset + Vec2::new(p.x, p.y) * self.zoom
    }
    fn to_doc(&self, origin: Pos2, s: Pos2) -> P {
        let v = (s - origin - self.offset) / self.zoom;
        P { x: v.x, y: v.y }
    }
}

fn c32(c: Color) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let area = resp.rect;
    let origin = area.min;
    painter.rect_filled(area, 0.0, Color32::WHITE);

    if app.view.fit_next {
        app.view.fit_next = false;
        if let Some(b) = app.scene().bounds() {
            let (w, h) = (b.max.x - b.min.x + 80.0, b.max.y - b.min.y + 80.0);
            let z = (area.width() / w).min(area.height() / h).clamp(0.1, 2.0);
            app.view.zoom = z;
            app.view.offset = Vec2::new(40.0 - b.min.x * z, 40.0 - b.min.y * z);
        }
    }

    if let Some(z) = app.view.restore_zoom.take() {
        app.view.zoom = z;
        if let Some(b) = app.scene().bounds() {
            app.view.offset = Vec2::new(40.0 - b.min.x * z, 40.0 - b.min.y * z);
        }
    }
    if let Some(z) = app.view.zoom_request.take() {
        let centre = area.center();
        let before = app.view.to_doc(origin, centre);
        app.view.zoom = z.clamp(0.1, 8.0);
        let after = app.view.to_screen(origin, before);
        app.view.offset += centre - after;
    }
    // zoom (pinch or cmd+scroll) around the pointer, pan with scroll
    if resp.hovered() {
        let (zoom, scroll, pointer) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta(), i.pointer.hover_pos()));
        if zoom != 1.0 {
            if let Some(pp) = pointer {
                let before = app.view.to_doc(origin, pp);
                app.view.zoom = (app.view.zoom * zoom).clamp(0.05, 8.0);
                let after = app.view.to_screen(origin, before);
                app.view.offset += pp - after;
            }
        }
        app.view.offset += scroll;
    }

    let z = app.view.zoom;
    let doc_pt = |view: &View, s: Pos2| {
        let d = view.to_doc(origin, s);
        Point { x: d.x.round() as i32, y: d.y.round() as i32 }
    };

    // hit testing on objects with a body (topmost first)
    let hit = |app: &App, s: Pos2| -> Option<i32> {
        let p = app.view.to_doc(origin, s);
        let doc = app.doc();
        let display = app.display();
        doc.objects
            .iter()
            .rev()
            .filter(|o| !o.class.starts_with("CDrawArc") && o.class != "CDrawTexte" && o.class != "CDrawEntiteUML")
            .filter(|o| display.shows(doc, o))
            .find(|o| render::bounds(doc, o).is_some_and(|b| b.contains(p)))
            .map(|o| o.base.id)
            .or_else(|| hit_line(app, p))
    };

    let pointer = ui.input(|i| i.pointer.hover_pos());
    let (shift, command) = ui.input(|i| (i.modifiers.shift, i.modifiers.command));
    if resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            let h = hit(app, pos);
            match app.tool {
                // Maj / Ctrl: adds to or removes from the selection
                Tool::Select if shift || command => {
                    if let Some(h) = h {
                        match app.selection.iter().position(|&x| x == h) {
                            Some(i) => {
                                app.selection.remove(i);
                            }
                            None => app.selection.push(h),
                        }
                    }
                }
                Tool::Select => app.select(h),
                _ => {
                    let p = doc_pt(&app.view, pos);
                    app.create_at(p, h);
                }
            }
        }
    }
    // double-click or Entrée ends a line, Échap drops it
    let (enter, escape) = ui.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
    if matches!(app.tool, Tool::Ligne | Tool::Fleche) && (resp.double_clicked() || enter) {
        app.finish_line();
    }
    if escape {
        app.draft.clear();
        app.link_from = None;
    }
    // double-click: the object's dialog, as in Looping
    if resp.double_clicked() && app.tool == Tool::Select {
        if let Some(h) = resp.interact_pointer_pos().and_then(|pos| hit(app, pos)) {
            app.open_dialog(h);
        }
    }
    if resp.drag_started() {
        app.view.drag = Drag::Pan;
        app.view.drag_rest = Vec2::ZERO;
        app.view.moved = false;
        let primary = ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        if let (Some(pos), true) = (resp.interact_pointer_pos(), primary) {
            if app.tool.draws_frame() {
                app.view.drag = Drag::Frame(app.view.to_doc(origin, pos));
            } else if app.tool == Tool::Crayon {
                app.view.drag = Drag::Stroke;
                app.draft = vec![doc_pt(&app.view, pos)];
            }
        }
        if let (Some(pos), true, true) = (resp.interact_pointer_pos(), primary, app.tool == Tool::Select) {
            let p = app.view.to_doc(origin, pos);
            app.view.drag = if let Some((id, handle, rect)) = handle_at(app, origin, pos) {
                Drag::Resize { id, handle, rect }
            } else if let Some(b) = bend_at(app, origin, pos) {
                Drag::Bend(b)
            } else if let Some(t) = text_at(app, p) {
                Drag::Text(t)
            } else {
                match hit(app, pos) {
                    Some(h) if !is_arc(app, Some(h)) => {
                        if !app.selection.contains(&h) {
                            if shift || command {
                                app.selection.push(h);
                            } else {
                                app.select(Some(h));
                            }
                        }
                        Drag::Move
                    }
                    _ => Drag::Band(p),
                }
            };
        }
    }
    if resp.drag_started() && crate::trace() {
        let what = match app.view.drag {
            Drag::None => "rien".to_string(),
            Drag::Pan => "défilement".into(),
            Drag::Move => "déplacement de la sélection".into(),
            Drag::Band(_) => "rectangle de sélection".into(),
            Drag::Resize { id, handle, .. } => format!("poignée {handle} de {id}"),
            Drag::Bend(id) => format!("point de cassure {id}"),
            Drag::Text(id) => format!("texte {id}"),
            Drag::Frame(_) => "cadre d'une illustration".into(),
            Drag::Stroke => "trait de crayon".into(),
        };
        eprintln!("[trace] glisser : {what}");
    }
    if resp.dragged() {
        let d = resp.drag_delta();
        let drag = app.view.drag;
        if drag == Drag::Pan {
            app.view.offset += d;
        } else if drag == Drag::Stroke {
            if let Some(pos) = pointer {
                let p = doc_pt(&app.view, pos);
                if app.draft.last() != Some(&p) {
                    app.draft.push(p);
                }
            }
        } else if !matches!(drag, Drag::Band(_) | Drag::None | Drag::Frame(_)) {
            let total = app.view.drag_rest + d / z;
            let (dx, dy) = (total.x.trunc(), total.y.trunc());
            app.view.drag_rest = total - Vec2::new(dx, dy);
            if dx != 0.0 || dy != 0.0 {
                let first = !app.view.moved;
                app.view.moved = true;
                let (dx, dy) = (dx as i32, dy as i32);
                match drag {
                    Drag::Move => app.move_selected(dx, dy, first),
                    Drag::Bend(id) => app.apply_drag(first, |d| edit::translate(d, id, dx, dy)),
                    Drag::Text(id) => {
                        let (dx, dy) = keep_text_inside(app, id, dx, dy);
                        if dx != 0 || dy != 0 {
                            app.apply_drag(first, |d| edit::move_text(d, id, dx, dy));
                        }
                    }
                    Drag::Resize { id, handle, mut rect } => {
                        // edges moved by each handle, clockwise from the top-left corner
                        let (l, t, r, b) = [
                            (1, 1, 0, 0),
                            (0, 1, 0, 0),
                            (0, 1, 1, 0),
                            (0, 0, 1, 0),
                            (0, 0, 1, 1),
                            (0, 0, 0, 1),
                            (1, 0, 0, 1),
                            (1, 0, 0, 0),
                        ][handle];
                        rect.left += l * dx;
                        rect.top += t * dy;
                        rect.right += r * dx;
                        rect.bottom += b * dy;
                        app.view.drag = Drag::Resize { id, handle, rect };
                        app.apply_drag(first, |d| edit::resize(d, id, rect));
                    }
                    _ => {}
                }
            }
        }
    }
    if resp.drag_stopped() {
        if let (Drag::Frame(start), Some(pos)) = (app.view.drag, pointer) {
            let end = app.view.to_doc(origin, pos);
            let r = loo_format::Rect {
                left: start.x.min(end.x).round() as i32,
                top: start.y.min(end.y).round() as i32,
                right: start.x.max(end.x).round() as i32,
                bottom: start.y.max(end.y).round() as i32,
            };
            // a frame too small to see is given the default size
            let r = if r.width() < 5 && r.height() < 5 {
                loo_format::Rect { right: r.left + 120, bottom: r.top + 60, ..r }
            } else {
                r
            };
            app.view.drag = Drag::None;
            app.create_frame(r);
        }
        if app.view.drag == Drag::Stroke {
            app.view.drag = Drag::None;
            app.finish_crayon();
        }
        // magnetism once the objects are dropped
        if app.view.drag == Drag::Move && app.view.moved {
            app.snap_selection();
        }
        if let (Drag::Band(start), Some(pos)) = (app.view.drag, pointer) {
            let end = app.view.to_doc(origin, pos);
            let band = render::R {
                min: P { x: start.x.min(end.x), y: start.y.min(end.y) },
                max: P { x: start.x.max(end.x), y: start.y.max(end.y) },
            };
            let inside: Vec<ObjId> = app
                .selectable()
                .into_iter()
                .filter(|&id| {
                    app.doc()
                        .object(id)
                        .and_then(|o| render::bounds(app.doc(), o))
                        .is_some_and(|b| band.contains(b.min) && band.contains(b.max))
                })
                .collect();
            if !(shift || command) {
                app.selection.clear();
            }
            for id in inside {
                if !app.selection.contains(&id) {
                    app.selection.push(id);
                }
            }
        }
        app.view.drag = Drag::None;
        app.view.moved = false;
    }

    // image files of the scene, loaded once
    let paths: Vec<String> = app
        .scene()
        .items
        .iter()
        .filter_map(|it| match &it.shape {
            render::Shape::Image { path, .. } => Some(path.clone()),
            _ => None,
        })
        .collect();
    let textures: std::collections::HashMap<String, egui::TextureHandle> =
        paths.into_iter().filter_map(|p| image_texture(app, ui.ctx(), &p).map(|t| (p, t))).collect();
    // paint
    let link_from = app.link_from;
    for it in &app.scene().items {
        let highlight = app.selection.contains(&it.obj) || Some(it.obj) == link_from;
        let stroke_w = if highlight { 2.5 } else { 1.0 };
        match &it.shape {
            render::Shape::Rect { r, fill, stroke, radius } => {
                let rect = Rect::from_min_max(app.view.to_screen(origin, r.min), app.view.to_screen(origin, r.max));
                let cr = CornerRadius::same((radius * z).min(255.0) as u8);
                if let Some(f) = fill {
                    painter.rect_filled(rect, cr, c32(*f));
                }
                let col = if highlight { Some(Color32::from_rgb(0, 120, 215)) } else { stroke.map(c32) };
                if let Some(col) = col {
                    painter.rect_stroke(rect, cr, Stroke::new(stroke_w, col), StrokeKind::Inside);
                }
            }
            render::Shape::Box { r, radius, header, header_fill, body_fill, stroke, .. } => {
                let rect = Rect::from_min_max(app.view.to_screen(origin, r.min), app.view.to_screen(origin, r.max));
                let rad = (radius * z).min(rect.width() / 2.0).min(rect.height() / 2.0).min(255.0);
                let hb = (header * z).min(rect.height());
                painter.rect_filled(rect, CornerRadius::same(rad as u8), c32(*body_fill));
                let band: Vec<Pos2> = render::rounded_band(r, *radius, *header)
                    .into_iter()
                    .map(|p| app.view.to_screen(origin, p))
                    .collect();
                let col = if highlight { Color32::from_rgb(0, 120, 215) } else { c32(*stroke) };
                if let (Some(&bl), Some(&br)) = (band.first(), band.last()) {
                    painter.add(Shape::convex_polygon(band, c32(*header_fill), Stroke::NONE));
                    painter.line_segment([bl, br], Stroke::new(1.0, col));
                }
                let _ = hb;
                painter.rect_stroke(
                    rect,
                    CornerRadius::same(rad as u8),
                    Stroke::new(stroke_w, col),
                    StrokeKind::Inside,
                );
            }
            render::Shape::Ellipse { c, rx, ry, fill, stroke } => {
                let center = app.view.to_screen(origin, *c);
                let radius = Vec2::new(rx * z, ry * z);
                if let Some(f) = fill {
                    painter.add(Shape::ellipse_filled(center, radius, c32(*f)));
                }
                let col = if highlight { Color32::from_rgb(0, 120, 215) } else { c32(*stroke) };
                painter.add(Shape::ellipse_stroke(center, radius, Stroke::new(stroke_w, col)));
            }
            render::Shape::Line { points, stroke, dashed, width } => {
                let pts: Vec<Pos2> = points.iter().map(|p| app.view.to_screen(origin, *p)).collect();
                let col = if highlight { Color32::from_rgb(0, 120, 215) } else { c32(*stroke) };
                let st = Stroke::new(stroke_w.max(width * z), col);
                if *dashed {
                    painter.extend(Shape::dashed_line(&pts, st, 5.0 * z, 3.0 * z));
                } else {
                    painter.add(Shape::line(pts, st));
                }
            }
            render::Shape::Text { pos, text, font, color, align, background } => {
                let size = font.size as f32 * z;
                if size < 3.0 {
                    continue;
                }
                let family = egui::FontFamily::Name(if font.bold { CARLITO_BOLD } else { CARLITO }.into());
                let fid = FontId::new(size, family);
                // GDI puts the baseline `tmAscent` below the top of the cell,
                // egui at the font's hhea ascent (0.75 em for Carlito)
                let top = app.view.to_screen(origin, *pos);
                let baseline = top.y + font.ascent() as f32 * z;
                let p = Pos2::new(top.x, baseline - 0.75 * size);
                let anchor = match align {
                    Align::Center => egui::Align2::CENTER_TOP,
                    Align::Left => egui::Align2::LEFT_TOP,
                    Align::Right => egui::Align2::RIGHT_TOP,
                };
                if let Some(bg) = background {
                    let w = font.width(text) as f32 * z;
                    let x = align.left(top.x, w);
                    let cell = Rect::from_min_size(Pos2::new(x, top.y), Vec2::new(w, font.height() as f32 * z));
                    painter.rect_filled(cell, 0.0, c32(*bg));
                }
                let r = painter.text(p, anchor, text, fid, c32(*color));
                if font.underline {
                    let y = baseline + 2.0 * z;
                    painter.line_segment(
                        [Pos2::new(r.min.x, y), Pos2::new(r.max.x, y)],
                        Stroke::new(z.max(1.0), c32(*color)),
                    );
                }
            }
            render::Shape::Figure { r, ellipse, radius, fill, stroke, width, dashed } => {
                let rect = Rect::from_min_max(app.view.to_screen(origin, r.min), app.view.to_screen(origin, r.max));
                let col = if highlight { Some(Color32::from_rgb(0, 120, 215)) } else { stroke.map(c32) };
                // a hidden outline still shows the selection
                let st =
                    col.map(|c| Stroke::new(if stroke.is_some() { (width * z).max(stroke_w) } else { stroke_w }, c));
                if *ellipse {
                    let (c, rad) = (rect.center(), rect.size() / 2.0);
                    if let Some(f) = fill {
                        painter.add(Shape::ellipse_filled(c, rad, c32(*f)));
                    }
                    if let Some(st) = st {
                        if *dashed {
                            let n = 64;
                            let pts: Vec<Pos2> = (0..=n)
                                .map(|i| {
                                    let a = i as f32 / n as f32 * std::f32::consts::TAU;
                                    c + Vec2::new(rad.x * a.cos(), rad.y * a.sin())
                                })
                                .collect();
                            painter.extend(Shape::dashed_line(&pts, st, 5.0 * z, 3.0 * z));
                        } else {
                            painter.add(Shape::ellipse_stroke(c, rad, st));
                        }
                    }
                } else {
                    let cr = CornerRadius::same(
                        (radius * z).min(rect.width() / 2.0).min(rect.height() / 2.0).min(255.0) as u8,
                    );
                    if let Some(f) = fill {
                        painter.rect_filled(rect, cr, c32(*f));
                    }
                    if let Some(st) = st {
                        if *dashed && *radius == 0.0 {
                            let pts = [
                                rect.left_top(),
                                rect.right_top(),
                                rect.right_bottom(),
                                rect.left_bottom(),
                                rect.left_top(),
                            ];
                            painter.extend(Shape::dashed_line(&pts, st, 5.0 * z, 3.0 * z));
                        } else {
                            painter.rect_stroke(rect, cr, st, StrokeKind::Inside);
                        }
                    }
                }
            }
            render::Shape::Polygon { points, fill } => {
                let pts: Vec<Pos2> = points.iter().map(|p| app.view.to_screen(origin, *p)).collect();
                let col = if highlight { Color32::from_rgb(0, 120, 215) } else { c32(*fill) };
                painter.add(Shape::convex_polygon(pts, col, Stroke::NONE));
            }
            render::Shape::Shadow { r, radius, color } => {
                let rect = Rect::from_min_max(app.view.to_screen(origin, r.min), app.view.to_screen(origin, r.max));
                let rad = (radius * z).min(rect.width() / 2.0).min(rect.height() / 2.0).min(255.0);
                painter.rect_filled(rect, CornerRadius::same(rad as u8), c32(*color));
            }
            // the file, when it can be read, covers the core's placeholder
            render::Shape::Image { r, path } => {
                if let Some(tex) = textures.get(path) {
                    let rect = Rect::from_min_max(app.view.to_screen(origin, r.min), app.view.to_screen(origin, r.max));
                    painter.image(tex.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
                }
            }
        }
    }
    // resize cursor over a handle
    let over_handle = match app.view.drag {
        Drag::Resize { handle, .. } => Some(handle),
        _ => pointer.and_then(|pos| handle_at(app, origin, pos)).map(|(_, h, _)| h),
    };
    if let Some(h) = over_handle {
        let icon = match h {
            0 | 4 => egui::CursorIcon::ResizeNwSe,
            2 | 6 => egui::CursorIcon::ResizeNeSw,
            1 | 5 => egui::CursorIcon::ResizeVertical,
            _ => egui::CursorIcon::ResizeHorizontal,
        };
        ui.ctx().set_cursor_icon(icon);
    }
    // illustration being drawn: frame, stroke, or line up to the pointer
    let preview = Stroke::new(1.0, Color32::from_rgb(0, 120, 215));
    if let (Drag::Frame(start), Some(pos)) = (app.view.drag, pointer) {
        let r = Rect::from_two_pos(app.view.to_screen(origin, start), pos);
        if app.tool == Tool::Ellipse {
            painter.add(Shape::ellipse_stroke(r.center(), r.size() / 2.0, preview));
        } else {
            painter.rect_stroke(r, 0.0, preview, StrokeKind::Inside);
        }
    }
    if !app.draft.is_empty() {
        let mut pts: Vec<Pos2> =
            app.draft.iter().map(|p| app.view.to_screen(origin, P { x: p.x as f32, y: p.y as f32 })).collect();
        if matches!(app.tool, Tool::Ligne | Tool::Fleche) {
            pts.extend(pointer);
        }
        painter.add(Shape::line(pts, preview));
    }
    // selection rectangle
    if let (Drag::Band(start), Some(pos)) = (app.view.drag, pointer) {
        let r = Rect::from_two_pos(app.view.to_screen(origin, start), pos);
        painter.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(0, 120, 215, 30));
        painter.rect_stroke(r, 0.0, Stroke::new(1.0, Color32::from_rgb(0, 120, 215)), StrokeKind::Inside);
    }
    // resize handles of a single selected object
    for h in handles(app, origin) {
        painter.rect_filled(Rect::from_center_size(h, Vec2::splat(9.0)), 0.0, Color32::WHITE);
        painter.rect_stroke(
            Rect::from_center_size(h, Vec2::splat(9.0)),
            0.0,
            Stroke::new(1.0, Color32::from_rgb(0, 120, 215)),
            StrokeKind::Inside,
        );
    }
    // break points of the selected links
    for b in bends(app) {
        let c = app.view.to_screen(origin, b.1);
        painter.circle(c, 3.5, Color32::WHITE, Stroke::new(1.0, Color32::from_rgb(0, 120, 215)));
    }
    if app.tool != Tool::Select {
        let hint = match app.tool {
            Tool::Entity => "Cliquez pour placer une entité",
            Tool::Association => "Cliquez pour placer une association",
            Tool::Heritage => "Cliquez pour placer un héritage",
            Tool::Contrainte => "Cliquez pour placer une contrainte inter-associations",
            Tool::Cif => "Cliquez pour placer une CIF",
            Tool::Regle => "Cliquez pour placer une règle",
            Tool::Link if app.link_from.is_none() => "Cliquez sur le premier objet du lien",
            Tool::Link => "Cliquez sur l'autre extrémité du lien",
            Tool::Cassure => "Cliquez sur un lien pour ajouter un point de cassure, sur un point pour le retirer",
            Tool::Texte => "Tracez le cadre du texte",
            Tool::Rectangle => "Tracez le rectangle",
            Tool::Ellipse => "Tracez l'ellipse",
            Tool::Image => "Tracez le cadre de l'image",
            Tool::SousModele => "Tracez le cadre de la vignette du sous-modèle",
            Tool::Crayon => "Dessinez à main levée",
            Tool::Ligne | Tool::Fleche if app.draft.is_empty() => "Cliquez sur le premier point",
            Tool::Ligne | Tool::Fleche => {
                "Cliquez les points suivants ; double-clic ou Entrée pour terminer, Échap pour annuler"
            }
            Tool::Select => "",
        };
        painter.text(
            area.left_top() + Vec2::new(8.0, 8.0),
            egui::Align2::LEFT_TOP,
            hint,
            FontId::proportional(13.0),
            Color32::DARK_GRAY,
        );
    }
}

/// Grab distance of a resize handle, in screen pixels.
const HANDLE_GRAB: f32 = 11.0;

/// Screen positions of the resize handles of the selected object (one
/// object with a rectangle, not a link), clockwise from the top-left.
fn handles(app: &App, origin: Pos2) -> Vec<Pos2> {
    let [id] = app.selection[..] else { return Vec::new() };
    if is_arc(app, Some(id)) || app.tool != Tool::Select {
        return Vec::new();
    }
    let Some(r) = edit::object_rect(app.doc(), id) else { return Vec::new() };
    let (l, t, rr, b) = (r.left as f32, r.top as f32, r.right as f32, r.bottom as f32);
    let (cx, cy) = ((l + rr) / 2.0, (t + b) / 2.0);
    [(l, t), (cx, t), (rr, t), (rr, cy), (rr, b), (cx, b), (l, b), (l, cy)]
        .iter()
        .map(|&(x, y)| app.view.to_screen(origin, P { x, y }))
        .collect()
}

fn handle_at(app: &App, origin: Pos2, pos: Pos2) -> Option<(ObjId, usize, loo_format::Rect)> {
    let i = handles(app, origin).iter().position(|h| (*h - pos).length() <= HANDLE_GRAB)?;
    let id = app.selection[0];
    Some((id, i, edit::object_rect(app.doc(), id)?))
}

/// Break points (segments without owner) of the selected links.
fn bends(app: &App) -> Vec<(ObjId, P)> {
    let doc = app.doc();
    let mut out = Vec::new();
    for &id in &app.selection {
        if !is_arc(app, Some(id)) {
            continue;
        }
        for seg in edit::chain_ids(doc, edit::first_segment(doc, id)) {
            if let Some(o) = doc.object(seg).filter(|o| o.base.links.owner == 0) {
                if let loo_format::ObjKind::Arc(a) = &o.kind {
                    out.push((seg, P { x: a.anchor.offset.x as f32, y: a.anchor.offset.y as f32 }));
                }
            }
        }
    }
    out
}

fn bend_at(app: &App, origin: Pos2, pos: Pos2) -> Option<ObjId> {
    bends(app).into_iter().find(|(_, p)| (app.view.to_screen(origin, *p) - pos).length() <= 6.0).map(|(id, _)| id)
}

/// Text of a selected object under `p` (names, attribute rows, cardinalities).
fn text_at(app: &App, p: P) -> Option<ObjId> {
    app.scene().items.iter().rev().find_map(|it| match &it.shape {
        render::Shape::Text { pos, text, font, align, .. } if it.text != 0 && app.selection.contains(&it.obj) => {
            let w = font.width(text) as f32;
            let x = align.left(pos.x, w);
            let r = render::R { min: P { x, y: pos.y }, max: P { x: x + w, y: pos.y + font.height() as f32 } };
            r.contains(p).then_some(it.text)
        }
        _ => None,
    })
}

/// Limits a text move so that a name or the attribute rows stay in the box
/// of their entity or association (cardinalities move freely).
fn keep_text_inside(app: &App, text: ObjId, dx: i32, dy: i32) -> (i32, i32) {
    let doc = app.doc();
    let Some(owner) = doc.object(text).and_then(|t| doc.object(t.base.links.owner)) else { return (dx, dy) };
    if !matches!(owner.kind, loo_format::ObjKind::Entite(_) | loo_format::ObjKind::Relation(_)) {
        return (dx, dy);
    }
    let Some(b) = render::bounds(doc, owner) else { return (dx, dy) };
    // extent of the text as drawn
    let mut ext: Option<render::R> = None;
    for it in app.scene().items.iter().filter(|it| it.text == text) {
        if let render::Shape::Text { pos, text, font, align, .. } = &it.shape {
            let w = font.width(text) as f32;
            let x = align.left(pos.x, w);
            let r = render::R { min: P { x, y: pos.y }, max: P { x: x + w, y: pos.y + font.height() as f32 } };
            ext = Some(ext.map_or(r, |e| e.union(&r)));
        }
    }
    let Some(e) = ext else { return (dx, dy) };
    let clamp = |d: i32, lo: f32, hi: f32| if lo > hi { 0 } else { (d as f32).clamp(lo, hi) as i32 };
    (clamp(dx, b.min.x - e.min.x, b.max.x - e.max.x), clamp(dy, b.min.y - e.min.y, b.max.y - e.max.y))
}

/// Segment of `chain` (a link and its break points) nearest to `p`.
pub fn nearest_segment(app: &App, chain: &[ObjId], p: Point) -> ObjId {
    let doc = app.doc();
    let at = |id: ObjId| -> Option<P> {
        let o = doc.object(id)?;
        match (&o.kind, o.base.links.owner) {
            (loo_format::ObjKind::Arc(a), 0) => Some(P { x: a.anchor.offset.x as f32, y: a.anchor.offset.y as f32 }),
            (_, owner) => doc.object(owner).and_then(|w| render::bounds(doc, w)).map(|b| b.center()),
        }
    };
    let q = P { x: p.x as f32, y: p.y as f32 };
    chain
        .windows(2)
        .filter_map(|w| Some((w[0], seg_dist(q, at(w[0])?, at(w[1])?))))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(chain[0], |(id, _)| id)
}

fn is_arc(app: &App, id: Option<i32>) -> bool {
    id.and_then(|i| app.doc().object(i)).is_some_and(|o| o.class.starts_with("CDrawArc"))
}

/// Texture of an image file (`CDrawImage+0x390`): the path as stored, else
/// relative to the document's folder, else its file name in that folder
/// (paths written on Windows).
fn image_texture(app: &mut App, ctx: &egui::Context, stored: &str) -> Option<egui::TextureHandle> {
    if stored.is_empty() {
        return None;
    }
    let dir = app.doc_dir();
    let name = stored.rsplit(['\\', '/']).next().unwrap_or(stored);
    let candidates = [
        Some(std::path::PathBuf::from(stored)),
        dir.as_ref().map(|d| d.join(stored)),
        dir.as_ref().map(|d| d.join(name)),
    ];
    let path = candidates.into_iter().flatten().find(|p| p.is_file())?;
    app.view
        .images
        .entry(path.clone())
        .or_insert_with(|| {
            let img = image::open(&path).ok()?.to_rgba8();
            let size = [img.width() as usize, img.height() as usize];
            let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
            Some(ctx.load_texture(path.to_string_lossy(), color, Default::default()))
        })
        .clone()
}

/// Nearest link or free line (within 5 units) to `p`.
fn hit_line(app: &App, p: P) -> Option<i32> {
    let tol = 5.0 / app.view.zoom.max(0.1);
    for it in app.scene().items.iter().rev() {
        if let render::Shape::Line { points, .. } = &it.shape {
            if !is_arc(app, Some(it.obj)) && !edit::is_free_line(app.doc(), it.obj) {
                continue;
            }
            for w in points.windows(2) {
                if seg_dist(p, w[0], w[1]) < tol {
                    return Some(it.obj);
                }
            }
        }
    }
    None
}

fn seg_dist(p: P, a: P, b: P) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 { 0.0 } else { (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0) };
    let (cx, cy) = (a.x + t * dx, a.y + t * dy);
    ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt()
}
