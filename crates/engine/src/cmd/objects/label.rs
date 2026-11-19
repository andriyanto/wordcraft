//! Text on a connector. DrawingML has no such thing, so a label is a text box of its own that
//! names its connector ([`wordcraft_doc::connector::ShapeExtra::label_of`]) and is kept on the
//! middle of it: when the connector is routed again, its label goes with it. The box has no fill
//! and no outline; its text has a white background, so the connector's line does not run
//! through the letters.

use wordcraft_doc::para::ShapeKind;

use super::arrange::{MAX_OFFSET, insert_floating, unalign_at};
use super::select::{edit_float, edit_obj, object_selection, picked_objects};
use super::{CmdError, Float, InlineObject, Pos, Rgb, Session, StoryRef, Wrap};

/// A new label's width (its text wraps past it) and its height before it has text, points.
const LABEL_SIZE: (f32, f32) = (96.0, 18.0);

/// The floating connector in the body that is selected by itself, when there is one: typing with
/// it selected, and Add Text, write its label.
pub(super) fn selected_connector(s: &Session) -> Option<Pos> {
    let (pos, obj) = object_selection(s)?;
    let InlineObject::Shape { kind, float, .. } = obj else { return None };
    (kind.is_connector() && float.wrap != Wrap::Inline && pos.story == StoryRef::Body && s.also_selected.is_empty()).then_some(pos)
}

/// The body's drawings: where each is, and the drawing.
fn body_shapes(s: &Session) -> Vec<(Pos, &InlineObject)> {
    let mut out = Vec::new();
    for path in s.doc.para_paths(StoryRef::Body) {
        let Some(para) = s.doc.para(StoryRef::Body, &path) else { continue };
        for (off, obj) in para.objects_with_offsets() {
            out.push((Pos { story: StoryRef::Body, path: path.clone(), off }, obj));
        }
    }
    out
}

/// The labels of the connector with drawing id `id`: where each is, and its text's story.
fn labels_of(s: &Session, id: u32) -> Vec<(Pos, u32)> {
    if id == 0 {
        return Vec::new();
    }
    body_shapes(s)
        .into_iter()
        .filter_map(|(pos, obj)| match obj {
            InlineObject::Shape { story: Some(story), extra, .. } if extra.label_of == Some(id) => Some((pos, *story)),
            _ => None,
        })
        .collect()
}

/// The middle of the connector at `pos` on its page: the page, and the point.
fn middle(s: &Session, layout: &wordcraft_layout::DocLayout, pos: &Pos) -> Option<(usize, (f32, f32))> {
    let InlineObject::Shape { kind, extra, .. } = s.doc.para_at(pos)?.object_at(pos.off)? else { return None };
    let hit = layout.object(pos, s.page_hint)?;
    let r = hit.rect;
    let segs = wordcraft_doc::connector::connector_path(*kind, extra.preset.as_deref(), r.w, r.h)?;
    let (x, y) = wordcraft_doc::connector::path_midpoint(&segs)?;
    Some((hit.page, hit.spin.apply(r.x + r.w / 2.0, r.y + r.h / 2.0, r.x + x, r.y + y)))
}

/// Put the caret in the selected connector's label, making the label first when it has none: a
/// text box on the middle of the connector. The label's story, or `None` when no connector is
/// selected.
pub(super) fn enter(s: &mut Session) -> Result<Option<u32>, CmdError> {
    let Some(pos) = selected_connector(s) else { return Ok(None) };
    // The connector's drawing id is what its label names.
    let next = s.doc.next_shape_id();
    let id = match edit_obj(s, &pos, |o| {
        if let InlineObject::Shape { extra, .. } = o
            && extra.id == 0
        {
            extra.id = next;
        }
    })? {
        InlineObject::Shape { extra, .. } => extra.id,
        _ => return Ok(None),
    };
    let story = match labels_of(s, id).first() {
        Some((_, story)) => *story,
        None => {
            s.touch();
            let layout = s.layout();
            let (page, (mx, my)) = middle(s, &layout, &pos).ok_or_else(|| CmdError::Failed("the connector isn't laid out".into()))?;
            // Its text: centred, on white, so the connector's line doesn't show through it.
            let mut para = wordcraft_doc::Paragraph::new();
            para.props.align = Some(wordcraft_doc::props::Align::Center);
            para.mark.shading = Some(Rgb::WHITE);
            let story = s.doc.try_add_part(wordcraft_doc::PartKind::TextBox, vec![wordcraft_doc::para_block(para)])?;
            if let Some(part) = s.doc.parts.get_mut(&story) {
                part.body.anchor = wordcraft_doc::props::VAlign::Center;
                part.body.auto_fit = true;
                part.body.inset = Some([0; 4]);
            }
            let label = InlineObject::Shape {
                kind: ShapeKind::TextBox,
                w: LABEL_SIZE.0,
                h: LABEL_SIZE.1,
                fill: None,
                stroke: None,
                stroke_width: 0.0,
                float: Float { wrap: Wrap::InFrontOfText, ..Default::default() },
                story: Some(story),
                freeform: None,
                effects: Default::default(),
                extra: wordcraft_doc::connector::ShapeExtra { label_of: Some(id), ..Default::default() },
            };
            insert_floating(s, label, page, mx - LABEL_SIZE.0 / 2.0, my - LABEL_SIZE.1 / 2.0)?;
            story
        }
    };
    s.sel = crate::Selection::caret(s.doc.end_of(StoryRef::Part(story)));
    Ok(Some(story))
}

/// Whether the caret is in a connector label's text: its box may have grown, and is put back on
/// the middle of its connector.
pub(crate) fn in_label(s: &Session) -> bool {
    let StoryRef::Part(story) = s.sel.focus.story else { return false };
    s.doc.parts.get(&story).is_some_and(|p| p.kind == wordcraft_doc::PartKind::TextBox)
        && body_shapes(s)
            .iter()
            .any(|(_, o)| matches!(o, InlineObject::Shape { story: Some(id), extra, .. } if *id == story && extra.label_of.is_some()))
}

/// Put every connector label on the middle of its connector. Returns whether any moved. A label
/// whose connector is gone, or on another page, stays where it is.
pub(crate) fn place_labels(s: &mut Session) -> bool {
    let labels: Vec<(Pos, u32)> = body_shapes(s)
        .into_iter()
        .filter_map(|(pos, obj)| match obj {
            InlineObject::Shape { extra, float, .. } if float.wrap != Wrap::Inline => extra.label_of.map(|id| (pos, id)),
            _ => None,
        })
        .take(1000)
        .collect();
    if labels.is_empty() {
        return false;
    }
    let connectors: Vec<(u32, Pos)> = body_shapes(s)
        .into_iter()
        .filter_map(|(pos, obj)| match obj {
            InlineObject::Shape { kind, extra, .. } if kind.is_connector() && extra.id != 0 => Some((extra.id, pos)),
            _ => None,
        })
        .collect();
    let layout = s.layout();
    let mut moves = Vec::new();
    for (label, id) in labels {
        let Some((_, conn)) = connectors.iter().find(|(c, _)| *c == id) else { continue };
        let (Some((page, (mx, my))), Some(hit)) = (middle(s, &layout, conn), layout.object(&label, s.page_hint)) else { continue };
        if hit.page != page {
            continue;
        }
        let (dx, dy) = (mx - (hit.rect.x + hit.rect.w / 2.0), my - (hit.rect.y + hit.rect.h / 2.0));
        if dx.is_finite() && dy.is_finite() && (dx.abs() > 0.05 || dy.abs() > 0.05) {
            moves.push((label, dx, dy));
        }
    }
    let mut moved = false;
    for (label, dx, dy) in moves {
        if unalign_at(s, &label).is_err() {
            continue;
        }
        moved |= edit_float(s, &label, |f| {
            f.x = (f.x + dx).clamp(-MAX_OFFSET, MAX_OFFSET);
            f.y = (f.y + dy).clamp(-MAX_OFFSET, MAX_OFFSET);
        })
        .is_ok();
    }
    moved
}

/// Before the selected drawings are deleted: the labels of the connectors among them are
/// selected with them, so a connector's label goes with it.
pub(crate) fn pick_labels(s: &mut Session) {
    let ids: Vec<u32> = picked_objects(s)
        .iter()
        .filter_map(|p| match s.doc.para_at(p)?.object_at(p.off)? {
            InlineObject::Shape { kind, extra, .. } if kind.is_connector() && extra.id != 0 => Some(extra.id),
            _ => None,
        })
        .collect();
    for id in ids {
        for (label, _) in labels_of(s, id) {
            if !s.also_selected.contains(&label) && object_selection(s).is_none_or(|(p, _)| p != label) {
                s.also_selected.push(label);
            }
        }
    }
}
