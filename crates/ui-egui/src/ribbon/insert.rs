use super::common::{mi, mi_icon, stack};
use super::language::mi_check;
use super::{Ui, WordApp, big, group, json, menu_button, small};

/// Shapes per row of the shape gallery.
const SHAPE_TILES_PER_ROW: usize = 8;
/// Room (points) left above and below the gallery in its window: it scrolls when taller.
const SHAPE_GALLERY_MARGIN: f32 = 140.0;

/// What a click on a tile of the shape gallery does.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum GalleryUse {
    /// Insert › Shapes: picks the shape up, to be drawn on the page (`ui.drawShape`).
    Draw,
    /// Shape Format › Change Shape: makes the selected shape that one (`shape.change`).
    Change,
}

/// The shape named `name` (a kind's or a preset's, as `insert.shape` takes) drawn as an icon in
/// `rect`: its own outline, from the geometry it is drawn with on the page.
pub(crate) fn paint_shape(painter: &egui::Painter, rect: egui::Rect, name: &str, stroke: egui::Stroke) {
    use egui::pos2;
    use wordcraft_doc::graphic::PathSeg;
    use wordcraft_doc::para::ShapeKind;
    let Ok((kind, preset)) = wordcraft_engine::cmd::objects::named_shape(name) else { return };
    // A frame of a shape's usual proportions, in the middle of the tile: upright for the
    // shapes that point up or down.
    let upright = matches!(name, "arrowUp" | "arrowDown" | "upDownArrow");
    let (w, h) = if upright { (rect.height() * 0.7, rect.height()) } else { (rect.width(), rect.width() * 0.7) };
    let (x0, y0) = (rect.center().x - w / 2.0, rect.center().y - h / 2.0);
    let at = |x: f32, y: f32| pos2(x0 + x, y0 + y);
    if kind == ShapeKind::Line {
        painter.line_segment([at(0.0, h), at(w, 0.0)], stroke);
        return;
    }
    let drawn = preset.as_deref().and_then(|p| wordcraft_doc::preset::outline(p, w, h));
    let Some(segs) = drawn else {
        // Its kind's outline; a rectangle's is its frame.
        let unit = wordcraft_doc::wrap::shape_outline(kind, w, h, None).unwrap_or_else(|| vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
        painter.add(egui::Shape::closed_line(unit.into_iter().map(|(x, y)| at(x * w, y * h)).collect(), stroke));
        if kind == ShapeKind::FlowchartOr {
            painter.line_segment([at(w / 2.0, 0.0), at(w / 2.0, h)], stroke);
            painter.line_segment([at(0.0, h / 2.0), at(w, h / 2.0)], stroke);
        }
        return;
    };
    // Each subpath as a line of its own, curves flattened.
    let mut pts: Vec<egui::Pos2> = Vec::new();
    let flush = |pts: &mut Vec<egui::Pos2>, closed: bool| {
        // No point twice in a row: a line of no length has no direction to draw it along.
        pts.dedup_by(|a, b| a.distance(*b) < 0.01);
        if pts.len() >= 2 {
            let line = std::mem::take(pts);
            painter.add(if closed { egui::Shape::closed_line(line, stroke) } else { egui::Shape::line(line, stroke) });
        }
        pts.clear();
    };
    for seg in segs {
        match seg {
            PathSeg::Move(x, y) => {
                flush(&mut pts, false);
                pts.push(at(x, y));
            }
            PathSeg::Line(x, y) => pts.push(at(x, y)),
            PathSeg::Cubic(a, b, c, d, e, f) => {
                let Some(&p0) = pts.last() else { continue };
                let (p1, p2, p3) = (at(a, b), at(c, d), at(e, f));
                for k in 1..=6 {
                    let t = k as f32 / 6.0;
                    let u = 1.0 - t;
                    let mix = |a: f32, b: f32, c: f32, d: f32| u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d;
                    pts.push(pos2(mix(p0.x, p1.x, p2.x, p3.x), mix(p0.y, p1.y, p2.y, p3.y)));
                }
            }
            PathSeg::Close => flush(&mut pts, true),
        }
    }
    flush(&mut pts, false);
}

/// Insert › Shapes: the shape gallery, a click picking the shape up to be drawn on the page.
pub(super) fn shape_gallery(ui: &mut Ui, app: &mut WordApp) {
    shape_gallery_for(ui, app, GalleryUse::Draw);
}

/// The shapes of [`wordcraft_doc::preset::LIBRARY`] as a grid of drawn tiles under their groups'
/// headings (#4349), each named by its tooltip.
pub(super) fn shape_gallery_for(ui: &mut Ui, app: &mut WordApp, usage: GalleryUse) {
    use crate::theme::Tokens;
    use egui::{Rect, Sense, vec2};
    use wordcraft_doc::preset::{LIBRARY, ShapeGroup};
    let t = Tokens::get(ui.ctx());
    let command = if usage == GalleryUse::Draw { "insert.shape" } else { "shape.change" };
    let enabled = crate::widgets::enabled(app, command);
    ui.add_enabled_ui(enabled, |ui| {
        let room = (ui.ctx().content_rect().height() - SHAPE_GALLERY_MARGIN).max(200.0);
        // As tall as its shapes need, up to the room there is: no shorter with shapes scrolled away.
        egui::ScrollArea::vertical().max_height(room).min_scrolled_height(room).show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
            for group in ShapeGroup::ALL {
                // A shape isn't made a line: lines have ends, not a frame.
                if usage == GalleryUse::Change && group == ShapeGroup::Lines {
                    continue;
                }
                ui.label(egui::RichText::new(crate::tl!(group.label())).small().color(t.text_dim));
                let shapes: Vec<_> = LIBRARY.iter().filter(|s| s.group == group).collect();
                for row in shapes.chunks(SHAPE_TILES_PER_ROW) {
                    ui.horizontal(|ui| {
                        for shape in row {
                            let (r, resp) = ui.allocate_exact_size(vec2(34.0, 34.0), Sense::click());
                            let name = crate::tl!(shape.label);
                            resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), name));
                            crate::widgets::bg(ui, r, &resp, false, &t);
                            if ui.is_rect_visible(r) {
                                paint_shape(
                                    ui.painter(),
                                    Rect::from_center_size(r.center(), vec2(22.0, 22.0)),
                                    shape.name,
                                    egui::Stroke::new(1.2, t.icon),
                                );
                            }
                            if resp.on_hover_text(name).clicked() {
                                let id = if usage == GalleryUse::Draw { "ui.drawShape" } else { "shape.change" };
                                crate::widgets::run_from_ribbon(app, id, json!({"kind": shape.name}));
                                ui.close();
                            }
                        }
                    });
                }
            }
        });
    });
}

pub(super) fn insert(app: &mut WordApp, ui: &mut Ui) {
    group(ui, "Pages", None, app, |ui, app| {
        stack(ui, |ui| {
            menu_button(
                ui,
                app,
                crate::widgets::MenuButtonOptions::new("coverPage", Some("Cover Page"), "Cover Page", false, "insert.coverPage"),
                |ui, app| {
                    mi(ui, app, "Studio cover", "insert.coverPage", json!({}));
                },
            );
            small(ui, app, "blankPage", Some("Blank Page"), "Blank Page", crate::widgets::ButtonAction::new("insert.blankPage", json!({}), false));
            small(ui, app, "pageBreak", Some("Page Break"), "Page Break", crate::widgets::ButtonAction::new("insert.pageBreak", json!({}), false));
        });
    });
    group(ui, "Tables", None, app, |ui, app| {
        menu_button(ui, app, crate::widgets::MenuButtonOptions::new("table", Some("Table"), "Add a Table", true, "insert.table"), |ui, app| {
            crate::dialogs::table_grid_picker(ui, app);
            ui.separator();
            mi(ui, app, "Insert Table…", "ui.dialog", json!({"name": "insertTable"}));
            let drawing = app.canvas.table_tool == Some(crate::table_pen::TableTool::Draw);
            mi_check(ui, app, "Draw Table", drawing, "table.draw", json!({}));
            mi(ui, app, "Convert Text to Table…", "table.fromText", json!({}));
            ui.menu_button(tl!("Quick Tables"), |ui| {
                crate::rtl::root(ui, |ui| {
                    mi(ui, app, "Tabular List", "table.quick", json!({"kind": "tabular"}));
                    mi(ui, app, "Matrix", "table.quick", json!({"kind": "matrix"}));
                    mi(ui, app, "Calendar", "table.quick", json!({"kind": "calendar"}));
                })
            });
        });
    });
    group(ui, "Illustrations", None, app, |ui, app| {
        big(ui, app, "picture", "Pictures", "insert.picture", json!({}), false);
        menu_button(ui, app, crate::widgets::MenuButtonOptions::new("shapes", Some("Shapes"), "Shapes", true, "insert.shape"), |ui, app| {
            shape_gallery(ui, app);
            ui.separator();
            mi_icon(ui, app, "canvas", "New Drawing Canvas", "insert.canvas", json!({}));
            // Connectors: glue their ends to shapes by dragging them (shape.connect).
            for (l, k, arrow) in [
                ("Straight Connector", "straightConnector", "none"),
                ("Straight Arrow Connector", "straightConnector", "end"),
                ("Elbow Connector", "elbowConnector", "none"),
                ("Elbow Arrow Connector", "elbowConnector", "end"),
                ("Curved Connector", "curvedConnector", "none"),
                ("Curved Arrow Connector", "curvedConnector", "end"),
            ] {
                let icon = if arrow == "end" { k.replace("Connector", "ArrowConnector") } else { k.to_string() };
                mi_icon(ui, app, &format!("shape.{icon}"), l, "ui.drawShape", json!({"kind": k, "arrow": arrow}));
            }
        });
        stack(ui, |ui| {
            small(ui, app, "icons", Some("Icons"), "Icons", crate::widgets::ButtonAction::new("insert.icon", json!({}), false));
            small(ui, app, "smartArt", Some("SmartArt"), "SmartArt", crate::widgets::ButtonAction::new("insert.smartArt", json!({}), false));
            small(ui, app, "chart", Some("Chart"), "Chart", crate::widgets::ButtonAction::new("insert.chart", json!({}), false));
        });
    });
    group(ui, "Links", None, app, |ui, app| {
        stack(ui, |ui| {
            small(ui, app, "link", Some("Link"), "Link", crate::widgets::ButtonAction::new("ui.dialog", json!({"name": "link"}), false));
            small(
                ui,
                app,
                "bookmark",
                Some("Bookmark"),
                "Bookmark",
                crate::widgets::ButtonAction::new("ui.dialog", json!({"name": "bookmark"}), false),
            );
            small(
                ui,
                app,
                "crossRef",
                Some("Cross-reference"),
                "Cross-reference",
                crate::widgets::ButtonAction::new("insert.crossReference", json!({}), false),
            );
        });
    });
    group(ui, "Comments", None, app, |ui, app| {
        big(ui, app, "newComment", "Comment", "review.newComment", json!({}), false);
    });
    group(ui, "Header & Footer", None, app, |ui, app| {
        menu_button(ui, app, crate::widgets::MenuButtonOptions::new("header", Some("Header"), "Header", true, "insert.header"), |ui, app| {
            mi(ui, app, "Blank", "insert.header", json!({"preset": "blank"}));
            mi(ui, app, "Blank (Three Columns)", "insert.header", json!({"preset": "blankThree"}));
            mi(ui, app, "Document Title", "insert.header", json!({"preset": "title"}));
            ui.separator();
            mi(ui, app, "Edit Header", "insert.editHeader", json!({}));
            mi(ui, app, "Remove Header", "insert.removeHeader", json!({}));
        });
        menu_button(ui, app, crate::widgets::MenuButtonOptions::new("footer", Some("Footer"), "Footer", true, "insert.footer"), |ui, app| {
            mi(ui, app, "Blank", "insert.footer", json!({"preset": "blank"}));
            mi(ui, app, "Blank (Three Columns)", "insert.footer", json!({"preset": "blankThree"}));
            mi(ui, app, "Page Number", "insert.footer", json!({"preset": "pageNumber"}));
            ui.separator();
            mi(ui, app, "Edit Footer", "insert.editFooter", json!({}));
            mi(ui, app, "Remove Footer", "insert.removeFooter", json!({}));
        });
        menu_button(
            ui,
            app,
            crate::widgets::MenuButtonOptions::new("pageNumber", Some("Page\nNumber"), "Page Number", true, "insert.pageNumber"),
            |ui, app| {
                mi(ui, app, "Top of Page", "insert.pageNumber", json!({"position": "top", "align": "right"}));
                mi(ui, app, "Bottom of Page", "insert.pageNumber", json!({"position": "bottom", "align": "center"}));
                mi(ui, app, "Page X of Y", "insert.pageNumber", json!({"position": "bottom", "format": "x of y"}));
                mi(ui, app, "Current Position", "insert.pageNumber", json!({"position": "current"}));
                mi(ui, app, "Format Page Numbers…", "ui.dialog", json!({"name": "pageNumbers"}));
            },
        );
    });
    group(ui, "Text", None, app, |ui, app| {
        big(ui, app, "textBox", "Text\nBox", "insert.textBox", json!({}), false);
        stack(ui, |ui| {
            menu_button(
                ui,
                app,
                crate::widgets::MenuButtonOptions::new("quickParts", Some("Quick Parts"), "Quick Parts", false, "insert.quickParts"),
                |ui, app| {
                    // This session's building blocks, then the document's own Quick Parts.
                    let mut parts: Vec<String> = app.session.building_blocks.keys().cloned().collect();
                    let mut seen: std::collections::HashSet<String> = parts.iter().cloned().collect();
                    let glossary = app.session.doc.glossary.iter().filter(|e| e.gallery == wordcraft_doc::QUICK_PARTS_GALLERY);
                    parts.extend(glossary.filter(|e| seen.insert(e.name.clone())).map(|e| e.name.clone()));
                    for name in parts {
                        if ui.button(&name).clicked() {
                            let _ = app.run("insert.quickParts", json!({"insert": name}));
                            ui.close();
                        }
                    }
                    mi(ui, app, "Field…", "ui.dialog", json!({"name": "field"}));
                },
            );
            menu_button(ui, app, crate::widgets::MenuButtonOptions::new("wordArt", Some("WordArt"), "WordArt", false, ""), |ui, app| {
                crate::wordart_menus::style_gallery(ui, app, "insert.wordArt")
            });
            menu_button(
                ui,
                app,
                crate::widgets::MenuButtonOptions::new("dropCap", Some("Drop Cap"), "Drop Cap", false, "insert.dropCap"),
                drop_cap_items,
            );
        });
        stack(ui, |ui| {
            small(ui, app, "signature", None, "Signature Line", crate::widgets::ButtonAction::new("insert.signatureLine", json!({}), false));
            small(ui, app, "dateTime", None, "Date & Time", crate::widgets::ButtonAction::new("insert.dateTime", json!({}), false));
            small(ui, app, "object", None, "Object", crate::widgets::ButtonAction::new("insert.object", json!({}), false));
        });
    });
    group(ui, "Symbols", None, app, |ui, app| {
        crate::equation_tab::insert_button(ui, app);
        menu_button(ui, app, crate::widgets::MenuButtonOptions::new("symbol", Some("Symbol"), "Symbol", true, "insert.symbol"), |ui, app| {
            // Recently used symbols first (their font on hover), then common ones.
            let mut items: Vec<(String, String)> = app.ui.recent_symbols.iter().map(|r| (r.ch.clone(), r.font.clone())).collect();
            for c in [
                "©", "®", "™", "§", "¶", "€", "£", "¥", "°", "±", "≠", "≤", "≥", "÷", "×", "∞", "µ", "α", "β", "π", "Ω", "∑", "√", "→", "←", "✓",
                "★", "♥", "—", "…",
            ] {
                if !items.iter().any(|(ch, f)| ch == c && f.is_empty()) {
                    items.push((c.to_string(), String::new()));
                }
            }
            items.truncate(30);
            crate::rtl::grid(ui, egui::Grid::new("syms"), |ui| {
                for (i, (c, font)) in items.iter().enumerate() {
                    let b = ui.button(egui::RichText::new(c).size(16.0));
                    let b = if font.is_empty() { b } else { b.on_hover_text(font) };
                    if b.clicked() {
                        let params = if font.is_empty() { json!({"char": c}) } else { json!({"char": c, "font": font}) };
                        if app.run("insert.symbol", params).is_ok() {
                            crate::dialogs_insert::remember_symbol(app, c, font);
                        }
                        ui.close();
                    }
                    if i % 6 == 5 {
                        ui.end_row();
                    }
                }
            });
            ui.separator();
            mi(ui, app, "More Symbols…", "ui.dialog", json!({"name": "symbol"}));
        });
    });
}

/// What a Drop Cap menu choice sends: the position, and `keep` so each selected paragraph that has
/// a drop cap keeps its own lines, distance and font (None takes it all away).
pub(super) fn drop_cap_payload(position: &str) -> serde_json::Value {
    if position == "none" { json!({"position": position}) } else { json!({"position": position, "keep": true}) }
}

/// Insert › Drop Cap's menu (#2601): None, Dropped and In margin (the current one checked),
/// then the Drop Cap dialog.
fn drop_cap_items(ui: &mut Ui, app: &mut WordApp) {
    let state = wordcraft_engine::cmd::insert::drop_cap_state(&app.session);
    for (label, v) in [("None", "none"), ("Dropped", "dropped"), ("In margin", "margin")] {
        mi_check(ui, app, label, state.position == v, "insert.dropCap", drop_cap_payload(v));
    }
    ui.separator();
    mi(ui, app, "Drop Cap Options…", "ui.dialog", json!({"name": "dropCap"}));
}
