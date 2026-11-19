//! Drawing a shape on the page (Insert › Shapes). Choosing a shape in the gallery picks it up:
//! the next drag on the page draws it from corner to corner, and a click without a drag puts it
//! there at its usual size. A line or connector is drawn from its start to its end, and a
//! connector's ends glue to the shapes they are dropped on. One shape per choice, as in Word;
//! Esc puts the tool down. The shape floats in front of the text where it was drawn
//! (`insert.shape` with page coordinates, one undo step).

use egui::{Pos2, Rect, Stroke, Ui, pos2};
use serde_json::{Value, json};
use wordcraft_layout::DocLayout;

use crate::WordApp;
use crate::theme::Tokens;

/// The shape picked up: `insert.shape`'s `kind` and, for a line or connector, its `arrow`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShapeTool {
    pub kind: String,
    pub arrow: Option<String>,
}

/// The kind of the tool that selects what is inside the box it draws, instead of drawing a shape
/// (Home › Select › Select Objects).
const SELECT: &str = "selectObjects";

impl ShapeTool {
    /// Drawn from one end to the other, not corner to corner.
    fn open(&self) -> bool {
        self.kind == "line" || self.kind.ends_with("Connector")
    }
    /// The box selects the pictures and shapes inside it.
    fn selects(&self) -> bool {
        self.kind == SELECT
    }
}

/// `ui.selectObjects`: pick the selecting box up (`{}`), or put it down (`{"off": true}`). The
/// next drag on the page selects the floating pictures and shapes wholly inside the box drawn
/// (`select.objects`); they then move, align and are deleted together.
pub fn pick_select(app: &mut WordApp, p: &Value) -> Value {
    let on = p.get("off").and_then(Value::as_bool) != Some(true);
    app.canvas.shape_stroke = None;
    app.canvas.shape_tool = on.then(|| ShapeTool { kind: SELECT.into(), arrow: None });
    if on {
        app.canvas.want_focus = true;
        app.status(tl!("Drag a box on the page around the pictures and shapes to select. Esc stops."));
    }
    json!({"tool": on})
}

/// A drag in progress: its page and its start and current end (page points).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ShapeStroke {
    page: usize,
    from: (f32, f32),
    to: (f32, f32),
}

/// A drag shorter than this (points, both ways) is a click.
const CLICK: f32 = 3.0;
/// The size a click gives a shape, and the length it gives a line, points.
const DEFAULT: (f32, f32) = (108.0, 72.0);

/// `ui.drawShape`: pick the shape `kind` up (`{"kind": …, "arrow"?: …}`), or put the tool down
/// (`{"off": true}`, or no kind).
pub fn pick(app: &mut WordApp, p: &Value) -> Value {
    let kind = p.get("kind").and_then(Value::as_str).filter(|_| p.get("off").and_then(Value::as_bool) != Some(true));
    app.canvas.shape_stroke = None;
    app.canvas.shape_tool = kind.map(|k| ShapeTool {
        kind: k.chars().take(40).collect(),
        arrow: p.get("arrow").and_then(Value::as_str).map(|a| a.chars().take(8).collect()),
    });
    if let Some(tool) = &app.canvas.shape_tool {
        app.canvas.want_focus = true;
        app.status(if tool.open() {
            tl!("Drag on the page to draw the line; ends dropped on a shape stick to it. Esc stops.")
        } else {
            tl!("Drag on the page to draw the shape, or click to place it. Esc stops.")
        });
    }
    json!({"tool": app.canvas.shape_tool.as_ref().map(|t| t.kind.clone())})
}

/// Put the tool down (Esc).
pub fn stop(app: &mut WordApp) -> bool {
    app.canvas.shape_stroke = None;
    app.canvas.shape_tool.take().is_some()
}

/// The page point under screen point `p` on page `page`, kept on that page.
fn on_page(rects: &[Rect], layout: &DocLayout, scale: f32, page: usize, p: Pos2) -> Option<(f32, f32)> {
    let r = rects.get(page)?;
    let pg = layout.pages.get(page)?;
    let scale = if pg.w > 0.0 { r.width() / pg.w } else { scale };
    (scale > 0.0).then(|| (((p.x - r.min.x) / scale).clamp(0.0, pg.w), ((p.y - r.min.y) / scale).clamp(0.0, pg.h)))
}

/// What `insert.shape` is asked for a finished drag (or click) of `tool`.
fn params(tool: &ShapeTool, st: &ShapeStroke) -> Value {
    let (dx, dy) = (st.to.0 - st.from.0, st.to.1 - st.from.1);
    let click = dx.abs() < CLICK && dy.abs() < CLICK;
    let mut v = json!({"kind": tool.kind, "page": st.page});
    if let Some(arrow) = &tool.arrow {
        v["arrow"] = json!(arrow);
    }
    if tool.open() {
        // From the press to the release; a click draws the usual length to the right.
        let to = if click { (st.from.0 + DEFAULT.0, st.from.1) } else { st.to };
        v["x"] = json!(st.from.0);
        v["y"] = json!(st.from.1);
        v["x1"] = json!(to.0);
        v["y1"] = json!(to.1);
    } else if click {
        v["x"] = json!(st.from.0);
        v["y"] = json!(st.from.1);
        // A shape that points up or down is placed upright.
        let upright = upright(tool.kind.as_str());
        v["width"] = json!(if upright { DEFAULT.1 } else { DEFAULT.0 });
        v["height"] = json!(if upright { DEFAULT.0 } else { DEFAULT.1 });
    } else {
        v["x"] = json!(st.from.0.min(st.to.0));
        v["y"] = json!(st.from.1.min(st.to.1));
        v["width"] = json!(dx.abs());
        v["height"] = json!(dy.abs());
    }
    v
}

/// Mouse input while a shape is picked up; `false` (nothing done) when none is, so the usual
/// editing runs.
pub fn pointer(app: &mut WordApp, ui: &Ui, resp: &egui::Response, rects: &[Rect], layout: &DocLayout, scale: f32) -> bool {
    let Some(tool) = app.canvas.shape_tool.clone() else { return false };
    if !app.canvas.focused && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        stop(app);
        return true;
    }
    let latest = ui.input(|i| i.pointer.latest_pos());
    if ui.input(|i| i.pointer.primary_pressed()) && resp.contains_pointer() {
        if let Some(p) = latest
            && let Some((page, x, y)) = crate::canvas::page_at(rects, layout, scale, p)
        {
            app.canvas.shape_stroke = Some(ShapeStroke { page, from: (x, y), to: (x, y) });
        }
        return true;
    }
    let Some(mut st) = app.canvas.shape_stroke else { return true };
    if let Some(p) = latest
        && let Some(to) = on_page(rects, layout, scale, st.page, p)
    {
        st.to = to;
        app.canvas.shape_stroke = Some(st);
    }
    if ui.input(|i| i.pointer.primary_down()) {
        return true;
    }
    // Released: the shape is drawn and selected, and the tool is put down.
    stop(app);
    if tool.selects() {
        // What the box holds is selected; an empty box leaves the selection as it was.
        let (x, y) = (st.from.0.min(st.to.0), st.from.1.min(st.to.1));
        let size = ((st.to.0 - st.from.0).abs(), (st.to.1 - st.from.1).abs());
        match app.run("select.objects", json!({"page": st.page, "x": x, "y": y, "width": size.0, "height": size.1})) {
            Ok(r) if r["selected"].as_array().is_some_and(Vec::is_empty) => app.status(tl!("No picture or shape lies wholly inside the box.")),
            Ok(_) => {}
            Err(e) => app.status(e),
        }
        return true;
    }
    if let Err(e) = app.run("insert.shape", params(&tool, &st)) {
        app.status(e);
    }
    true
}

/// The pointer while a shape is picked up: a crosshair.
pub fn cursor(app: &WordApp, ui: &Ui, resp: &egui::Response) {
    if app.canvas.shape_tool.is_some() && (resp.hovered() || app.canvas.shape_stroke.is_some()) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
    }
}

/// The live preview of the drag: the shape's box, or the line.
pub fn paint(app: &WordApp, painter: &egui::Painter, t: &Tokens, layout: &DocLayout, pages: &[Rect], scale: f32) {
    let (Some(tool), Some(st)) = (&app.canvas.shape_tool, app.canvas.shape_stroke) else { return };
    let to_screen = |x: f32, y: f32| -> Option<Pos2> {
        let r = pages.get(st.page)?;
        let sc = layout.pages.get(st.page).filter(|pg| pg.w > 0.0).map_or(scale, |pg| r.width() / pg.w);
        Some(pos2(r.min.x + x * sc, r.min.y + y * sc))
    };
    let (Some(a), Some(b)) = (to_screen(st.from.0, st.from.1), to_screen(st.to.0, st.to.1)) else { return };
    let ink = Stroke::new(1.5, t.accent);
    if tool.selects() {
        let area = Rect::from_two_pos(a, b);
        painter.rect_filled(area, 0.0, t.accent.gamma_multiply(0.12));
        painter.rect_stroke(area, 0.0, Stroke::new(1.0, t.accent), egui::StrokeKind::Middle);
    } else if tool.open() {
        painter.line_segment([a, b], ink);
    } else {
        painter.rect_stroke(Rect::from_two_pos(a, b), 0.0, ink, egui::StrokeKind::Middle);
    }
}

/// Whether the gallery shape `name` stands taller than it is wide: the arrows that point up or
/// down, and a single bracket or brace. A click places it upright, and its tile shows it so.
pub(crate) fn upright(name: &str) -> bool {
    matches!(name, "arrowUp" | "arrowDown" | "upDownArrow" | "leftBracket" | "rightBracket" | "leftBrace" | "rightBrace")
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordcraft_doc::para::{InlineObject, ShapeKind, Wrap};

    fn app() -> WordApp {
        WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), crate::Services::default())
    }

    fn shapes(a: &WordApp) -> Vec<InlineObject> {
        a.session.doc.body.iter().filter_map(|b| b.as_para()).flat_map(|p| p.objects.iter().cloned()).collect()
    }

    /// Choosing a shape picks it up and inserts nothing yet; Esc or `off` puts it down.
    #[test]
    fn choosing_a_shape_picks_it_up_without_inserting_it() {
        let mut a = app();
        assert_eq!(a.run("ui.drawShape", json!({"kind": "diamond"})).unwrap()["tool"], "diamond");
        assert_eq!(a.canvas.shape_tool, Some(ShapeTool { kind: "diamond".into(), arrow: None }));
        assert!(shapes(&a).is_empty());
        assert!(stop(&mut a) && a.canvas.shape_tool.is_none());
        a.run("ui.drawShape", json!({"kind": "elbowConnector", "arrow": "end"})).unwrap();
        assert_eq!(a.run("ui.drawShape", json!({"off": true})).unwrap()["tool"], Value::Null);
        assert!(a.canvas.shape_tool.is_none());
    }

    /// A drag asks for the box it covered, whichever way it went; a click for the usual size
    /// there; a line or connector for its two ends.
    #[test]
    fn a_drag_becomes_the_shapes_box_and_a_lines_two_ends() {
        let rect = ShapeTool { kind: "rectangle".into(), arrow: None };
        let drag = ShapeStroke { page: 2, from: (300.0, 400.0), to: (180.0, 340.0) };
        assert_eq!(params(&rect, &drag), json!({"kind": "rectangle", "page": 2, "x": 180.0, "y": 340.0, "width": 120.0, "height": 60.0}));
        let click = ShapeStroke { page: 0, from: (50.0, 60.0), to: (51.0, 61.0) };
        assert_eq!(params(&rect, &click), json!({"kind": "rectangle", "page": 0, "x": 50.0, "y": 60.0, "width": 108.0, "height": 72.0}));
        let conn = ShapeTool { kind: "elbowConnector".into(), arrow: Some("end".into()) };
        assert_eq!(
            params(&conn, &drag),
            json!({"kind": "elbowConnector", "arrow": "end", "page": 2, "x": 300.0, "y": 400.0, "x1": 180.0, "y1": 340.0})
        );
        assert_eq!(params(&conn, &click)["x1"], json!(158.0));
        // A shape that points up or down is placed upright by a click (lying down, an up-down
        // arrow's heads leave it no shaft); a preset of the gallery goes by its name.
        let up_down = ShapeTool { kind: "upDownArrow".into(), arrow: None };
        assert_eq!(params(&up_down, &click), json!({"kind": "upDownArrow", "page": 0, "x": 50.0, "y": 60.0, "width": 72.0, "height": 108.0}));
        assert_eq!(params(&up_down, &drag)["width"], json!(120.0));
    }

    /// Drawn with the mouse: the shape is where the drag was, floating in front of the text and
    /// selected, the tool is put down, and one undo takes the shape away.
    #[test]
    fn dragging_on_the_page_draws_the_shape_there() {
        use egui::{Event, PointerButton};
        let mut a = app();
        a.run("view.commentsPane", json!({"value": false})).unwrap();
        let before = a.session.doc.clone();
        a.run("ui.drawShape", json!({"kind": "ellipse"})).unwrap();
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1440.0, 1100.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            a,
        );
        h.run_steps(6);
        let from = crate::canvas::page_to_screen(h.state_mut(), 0, 200.0, 300.0).unwrap();
        let to = crate::canvas::page_to_screen(h.state_mut(), 0, 320.0, 380.0).unwrap();
        h.event(Event::PointerMoved(from));
        h.event(Event::PointerButton { pos: from, button: PointerButton::Primary, pressed: true, modifiers: Default::default() });
        h.run_steps(1);
        for k in 1..=4 {
            h.event(Event::PointerMoved(from + (to - from) * (k as f32 / 4.0)));
            h.run_steps(1);
        }
        assert!(h.state().canvas.shape_stroke.is_some(), "the drag is the tool's");
        assert!(shapes(h.state()).is_empty(), "nothing is inserted before the release");
        h.event(Event::PointerButton { pos: to, button: PointerButton::Primary, pressed: false, modifiers: Default::default() });
        h.run_steps(3);
        assert!(h.state().canvas.shape_tool.is_none(), "one shape a choice");
        let drawn = shapes(h.state());
        let [InlineObject::Shape { kind: ShapeKind::Ellipse, w, h: height, float, .. }] = drawn.as_slice() else { panic!("{drawn:?}") };
        assert!((w - 120.0).abs() < 0.5 && (height - 80.0).abs() < 0.5, "{w} × {height}");
        assert_eq!(float.wrap, Wrap::InFrontOfText);
        let (pos, _) = crate::objects::selected(h.state()).expect("the new shape is selected");
        let hit = h.state_mut().session.layout().object(&pos, 0).unwrap();
        assert!((hit.rect.x - 200.0).abs() < 0.5 && (hit.rect.y - 300.0).abs() < 0.5, "{:?}", hit.rect);
        h.state_mut().run("edit.undo", json!({})).unwrap();
        assert_eq!(h.state().session.doc, before);
    }
    /// Select Objects: a box dragged on the page selects the pictures and shapes inside it, and
    /// dragging one of them then moves them all. Each had to be Shift+clicked before, and only
    /// the first one moved.
    #[test]
    fn a_box_selects_the_shapes_inside_it_and_they_move_together() {
        use egui::{Event, PointerButton};
        let mut a = app();
        a.run("view.commentsPane", json!({"value": false})).unwrap();
        for (x, y) in [(100, 100), (250, 140), (100, 420)] {
            a.run("insert.shape", json!({"kind": "rectangle", "x": x, "y": y, "page": 0, "width": 80, "height": 50})).unwrap();
        }
        assert_eq!(a.run("ui.selectObjects", json!({})).unwrap()["tool"], true);
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1440.0, 1100.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            a,
        );
        h.run_steps(6);
        let drag = |h: &mut egui_kittest::Harness<'_, WordApp>, from: (f32, f32), to: (f32, f32)| {
            let a = crate::canvas::page_to_screen(h.state_mut(), 0, from.0, from.1).unwrap();
            let b = crate::canvas::page_to_screen(h.state_mut(), 0, to.0, to.1).unwrap();
            h.event(Event::PointerMoved(a));
            h.step();
            h.event(Event::PointerButton { pos: a, button: PointerButton::Primary, pressed: true, modifiers: Default::default() });
            h.step();
            for i in 1..=4 {
                h.event(Event::PointerMoved(a + (b - a) * (i as f32 / 4.0)));
                h.step();
            }
            h.event(Event::PointerButton { pos: b, button: PointerButton::Primary, pressed: false, modifiers: Default::default() });
            h.run_steps(3);
        };
        // A box around the upper two.
        drag(&mut h, (90.0, 90.0), (350.0, 210.0));
        assert_eq!(wordcraft_engine::cmd::objects::picked_objects(&h.state().session).len(), 2, "the two inside the box");
        assert!(h.state().canvas.shape_tool.is_none(), "the tool is put down");
        let frames = |h: &mut egui_kittest::Harness<'_, WordApp>| {
            let layout = h.state_mut().session.layout();
            let mut out: Vec<(i32, i32)> = layout
                .pages
                .first()
                .map(|pg| {
                    pg.items
                        .iter()
                        .filter_map(|it| match it {
                            wordcraft_layout::Placed::Object { rect, .. } => Some((rect.x.round() as i32, rect.y.round() as i32)),
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default();
            out.sort();
            out.dedup();
            out
        };
        assert_eq!(frames(&mut h), vec![(100, 100), (100, 420), (250, 140)]);
        // One of them dragged 60 pt right and 30 pt down, with the alignment guides held off:
        // both go, and the third stays.
        h.state_mut().ui.alignment_guides = false;
        drag(&mut h, (280.0, 160.0), (340.0, 190.0));
        assert_eq!(frames(&mut h), vec![(100, 420), (160, 130), (310, 170)]);
        assert_eq!(wordcraft_engine::cmd::objects::picked_objects(&h.state().session).len(), 2, "still selected together");
        // One undo puts both back.
        h.state_mut().run("edit.undo", json!({})).unwrap();
        assert_eq!(frames(&mut h), vec![(100, 100), (100, 420), (250, 140)]);
    }
}
