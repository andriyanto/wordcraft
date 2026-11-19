//! Several drawings at once: selecting those inside a box drawn on the page, moving them
//! together, lining them up with each other, spacing them evenly and deleting them. The selected
//! ones are the one the selection holds and those added to it (Shift+click, or the box).

use wordcraft_geom::Rect;

use super::arrange::{MAX_OFFSET, unalign_at};
use super::select::{edit_float, edit_obj, min_size, object_selection, picked_objects};
use super::{CmdError, CmdResult, InlineObject, Pos, Selection, Session, StoryRef, Value, json, p};

/// Most drawings selected by one box.
const MAX_PICKED: usize = 1000;

/// `select.objects` with a box: the body's floating drawings on `page` that lie wholly inside
/// the box become the selection (the first of them, the others added to it). An empty box
/// leaves the selection as it is.
pub(super) fn select_in(s: &mut Session, v: &Value) -> CmdResult {
    let num = |k: &str| p::f32(v, k).filter(|n| n.is_finite()).ok_or_else(|| CmdError::Params(format!("`{k}`: a number")));
    let (x, y, w, h) = (num("x")?, num("y")?, num("width")?, num("height")?);
    let (x0, x1, y0, y1) = (x.min(x + w), x.max(x + w), y.min(y + h), y.max(y + h));
    let page = p::u64(v, "page").map_or(s.page_hint, |n| usize::try_from(n).unwrap_or(usize::MAX));
    let layout = s.layout();
    let pg = layout.pages.get(page).ok_or_else(|| CmdError::Params("no such page".into()))?;
    let mut found: Vec<Pos> = Vec::new();
    for it in &pg.items {
        let wordcraft_layout::Placed::Object { rect, spin, story: StoryRef::Body, path, off, .. } = it else { continue };
        let pos = Pos { story: StoryRef::Body, path: path.clone(), off: *off };
        let floating = s.doc.para_at(&pos).and_then(|q| q.object_at(pos.off)).is_some_and(|o| o.is_drawing() && o.is_floating());
        let b = spin.bounds(*rect);
        if floating && b.x >= x0 && b.right() <= x1 && b.y >= y0 && b.bottom() <= y1 && !found.contains(&pos) && found.len() < MAX_PICKED {
            found.push(pos);
        }
    }
    found.sort();
    if let Some(first) = found.first().cloned() {
        s.sel = Selection { anchor: first.clone(), focus: Pos { off: first.off + wordcraft_doc::para::OBJ.len_utf8(), ..first } };
        s.also_selected = found.iter().skip(1).cloned().collect();
    }
    Ok(json!({"selected": found.iter().map(crate::cmd::pos_json).collect::<Vec<_>>()}))
}

/// The selected floating drawings laid out on the selection's page: where each is, and the page
/// area it covers (its frame's bounds, turned).
fn picked_bounds(s: &mut Session) -> Vec<(Pos, Rect)> {
    let picks = picked_objects(s);
    let layout = s.layout();
    let page = picks.first().and_then(|first| layout.object(first, s.page_hint)).map(|h| h.page);
    picks
        .into_iter()
        .take(MAX_PICKED)
        .filter(|pos| s.doc.para_at(pos).and_then(|q| q.object_at(pos.off)).is_some_and(InlineObject::is_floating))
        .filter_map(|pos| {
            let hit = layout.object(&pos, s.page_hint).filter(|h| Some(h.page) == page)?;
            Some((pos, hit.bounds()))
        })
        .collect()
}

/// Move the floating drawing at `pos` by (`dx`, `dy`) points: an aligned position becomes an
/// offset first. Its anchor stays, so its position in the document does.
fn shift(s: &mut Session, pos: &Pos, dx: f32, dy: f32) -> Result<(), CmdError> {
    if !(dx.is_finite() && dy.is_finite()) || (dx == 0.0 && dy == 0.0) {
        return Ok(());
    }
    unalign_at(s, pos)?;
    edit_float(s, pos, |f| {
        f.x = (f.x + dx).clamp(-MAX_OFFSET, MAX_OFFSET);
        f.y = (f.y + dy).clamp(-MAX_OFFSET, MAX_OFFSET);
    })?;
    Ok(())
}

/// How several selected drawings change together ([`together_frames`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Together {
    /// Each one's width and height times `sx` and `sy`; the point of its own frame at `keep`
    /// (shares of its width and height from its top left, before its turn: (1, 1) is its
    /// bottom right corner) stays where it is on the page. What dragging a handle of one of
    /// them does to all: the handle opposite the dragged one stays put on each.
    Scale { sx: f32, sy: f32, keep: (f32, f32) },
    /// Turned as one block by `by` degrees clockwise about the middle of the box around them
    /// all: each one goes round that middle, and turns as much itself.
    Turn { by: f32 },
    /// Mirrored as one block across the upright (`h`) or level (`v`) line through the middle of
    /// the box around them all: each one goes to the other side, and is mirrored itself.
    Flip { h: bool, v: bool },
}

/// Smallest and largest factor [`Together::Scale`] takes.
const SCALE_RANGE: (f32, f32) = (0.01, 100.0);

/// The middle of the box around the selected floating drawings on their page: the page, and
/// the point. What [`Together::Turn`] and [`Together::Flip`] turn and mirror about.
pub fn together_pivot(s: &Session, layout: &wordcraft_layout::DocLayout) -> Option<(usize, (f32, f32))> {
    let found = picked_frames(s, layout);
    let page = found.first().map(|f| f.1)?;
    let lo = |f: fn(&Rect) -> f32| found.iter().map(|(_, _, r, sp)| f(&sp.bounds(*r))).fold(f32::MAX, f32::min);
    let hi = |f: fn(&Rect) -> f32| found.iter().map(|(_, _, r, sp)| f(&sp.bounds(*r))).fold(f32::MIN, f32::max);
    Some((page, ((lo(|b| b.x) + hi(Rect::right)) / 2.0, (lo(|b| b.y) + hi(Rect::bottom)) / 2.0)))
}

/// The selected floating drawings laid out on the selection's page: where each is in the
/// document, its page, its frame (before its turn) and its turn.
fn picked_frames(s: &Session, layout: &wordcraft_layout::DocLayout) -> Vec<(Pos, usize, Rect, wordcraft_geom::Spin)> {
    let picks = picked_objects(s);
    let page = picks.first().and_then(|first| layout.object(first, s.page_hint)).map(|h| h.page);
    picks
        .into_iter()
        .take(MAX_PICKED)
        .filter(|pos| s.doc.para_at(pos).and_then(|q| q.object_at(pos.off)).is_some_and(InlineObject::is_floating))
        .filter_map(|pos| {
            let hit = layout.object(&pos, s.page_hint).filter(|h| Some(h.page) == page)?;
            Some((pos, hit.page, hit.rect, hit.spin))
        })
        .collect()
}

/// Where each selected floating drawing would be after `how`: its frame on its page (before its
/// turn) and its turn. The document is not touched: what a drag of one of them shows, and what
/// [`apply_together`] then does.
pub fn together_frames(s: &Session, layout: &wordcraft_layout::DocLayout, how: Together) -> Vec<(Pos, Rect, wordcraft_geom::Spin)> {
    use wordcraft_geom::Spin;
    let found = picked_frames(s, layout);
    let Some((_, pivot)) = together_pivot(s, layout) else { return Vec::new() };
    let fin = |v: f32, or: f32| if v.is_finite() { v } else { or };
    found
        .into_iter()
        .map(|(pos, _, r, spin)| {
            let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
            let (frame, turned) = match how {
                Together::Scale { sx, sy, keep } => {
                    let min = s.doc.para_at(&pos).and_then(|q| q.object_at(pos.off)).map_or(1.0, min_size);
                    let k = |v: f32| fin(v, 1.0).clamp(SCALE_RANGE.0, SCALE_RANGE.1);
                    let (w, h) = ((r.w * k(sx)).clamp(min, MAX_OFFSET), (r.h * k(sy)).clamp(min, MAX_OFFSET));
                    let (ax, ay) = (fin(keep.0, 0.0).clamp(0.0, 1.0), fin(keep.1, 0.0).clamp(0.0, 1.0));
                    // The kept point on the page, and where it is from the new frame's middle.
                    let kept = spin.apply(cx, cy, r.x + ax * r.w, r.y + ay * r.h);
                    let off = spin.apply(0.0, 0.0, (ax - 0.5) * w, (ay - 0.5) * h);
                    let (ncx, ncy) = (kept.0 - off.0, kept.1 - off.1);
                    (Rect::new(ncx - w / 2.0, ncy - h / 2.0, w, h), spin)
                }
                Together::Turn { by } => {
                    let by = fin(by, 0.0);
                    let (ncx, ncy) = Spin::new(by, false, false).apply(pivot.0, pivot.1, cx, cy);
                    (Rect::new(ncx - r.w / 2.0, ncy - r.h / 2.0, r.w, r.h), spin.within(Spin::new(by, false, false)))
                }
                Together::Flip { h, v } => {
                    let (ncx, ncy) = Spin::new(0.0, h, v).apply(pivot.0, pivot.1, cx, cy);
                    (Rect::new(ncx - r.w / 2.0, ncy - r.h / 2.0, r.w, r.h), spin.within(Spin::new(0.0, h, v)))
                }
            };
            (pos, frame, turned)
        })
        .collect()
}

/// Every selected floating drawing put where [`together_frames`] says `how` takes it, in one
/// step. They stay selected together.
pub(super) fn apply_together(s: &mut Session, how: Together) -> CmdResult {
    let layout = s.layout();
    let was: std::collections::HashMap<Pos, Rect> = picked_frames(s, &layout).into_iter().map(|(pos, _, r, _)| (pos, r)).collect();
    let targets = together_frames(s, &layout, how);
    if targets.is_empty() {
        return Err(CmdError::Disabled("select floating pictures or shapes on one page first".into()));
    }
    for (pos, to, spin) in &targets {
        let Some(from) = was.get(pos) else { continue };
        // Its top left stays where it is through the new size and turn: only `shift` moves it.
        unalign_at(s, pos)?;
        edit_obj(s, pos, |o| {
            if (to.w, to.h) != (from.w, from.h) {
                o.set_size(to.w, to.h);
            }
            if let Some(f) = o.float_mut() {
                f.set_spin(*spin);
            }
        })?;
        shift(s, pos, to.x - from.x, to.y - from.y)?;
    }
    Ok(json!({"changed": targets.len()}))
}

/// `arrange.scale`: every selected floating drawing resized by the same factors.
pub(super) fn scale(s: &mut Session, v: &Value) -> CmdResult {
    let factor = |k: &str| match p::f32(v, k) {
        None => Ok(1.0),
        Some(f) if f.is_finite() && (SCALE_RANGE.0..=SCALE_RANGE.1).contains(&f) => Ok(f),
        Some(_) => Err(CmdError::Params(format!("`{k}`: a factor from {} to {}", SCALE_RANGE.0, SCALE_RANGE.1))),
    };
    let (sx, sy) = (factor("x")?, factor("y")?);
    let keep = match v.get("keep") {
        None => (0.0, 0.0),
        Some(k) => {
            let share = |i: usize| k.get(i).and_then(Value::as_f64).map(|n| n as f32).filter(|n| n.is_finite() && (0.0..=1.0).contains(n));
            match (share(0), share(1)) {
                (Some(ax), Some(ay)) => (ax, ay),
                _ => return Err(CmdError::Params("`keep`: [x, y], each from 0 (left, top) to 1 (right, bottom)".into())),
            }
        }
    };
    apply_together(s, Together::Scale { sx, sy, keep })
}

/// Whether more than one drawing is selected.
pub(super) fn several(s: &Session) -> bool {
    picked_objects(s).len() > 1
}

/// A change to how each of the selected drawings is placed (its turn, its flips): every one
/// selected, in one step.
pub(super) fn each_float(s: &mut Session, f: impl Fn(&mut wordcraft_doc::para::Float)) -> CmdResult {
    let picks: Vec<Pos> = picked_objects(s).into_iter().take(MAX_PICKED).collect();
    for pos in &picks {
        edit_float(s, pos, &f)?;
    }
    Ok(json!({"changed": picks.len()}))
}

/// `arrange.nudge` with several drawings selected: they all move by (`dx`, `dy`).
pub(super) fn nudge_all(s: &mut Session, dx: f32, dy: f32) -> CmdResult {
    let picks: Vec<Pos> = picked_objects(s)
        .into_iter()
        .filter(|pos| s.doc.para_at(pos).and_then(|q| q.object_at(pos.off)).is_some_and(InlineObject::is_floating))
        .collect();
    for pos in &picks {
        shift(s, pos, dx, dy)?;
    }
    Ok(json!({"moved": picks.len()}))
}

/// `arrange.align` with several drawings selected: they line up with each other, on the side
/// (or through the middle) of the box around them all. The one farthest that way stays.
pub(super) fn align_all(s: &mut Session, how: &str) -> CmdResult {
    align_in(s, how, None)
}

/// `arrange.align` with `to`: the selected floating drawings (one or more) each line up with
/// that side (or the middle) of the page, or of the area inside its margins. They keep their
/// places the other way, and those already there stay.
pub(super) fn align_to(s: &mut Session, how: &str, to: &str) -> CmdResult {
    let sect = crate::cmd::page::sect(s);
    let area = match to {
        "page" => Rect::new(0.0, 0.0, sect.page_w, sect.page_h),
        // Where the text is: inside the margins (and the gutter, on the side the text starts).
        "margin" => Rect::new(sect.page_w - sect.margin_right - sect.text_width(), sect.margin_top, sect.text_width(), sect.text_height()),
        other => return Err(CmdError::Params(format!("`to`: page or margin, not `{}`", other.chars().take(20).collect::<String>()))),
    };
    if !(area.x.is_finite() && area.y.is_finite() && area.w.is_finite() && area.h.is_finite()) {
        return Err(CmdError::Failed("the page has no size to align to".into()));
    }
    align_in(s, how, Some(area))
}

/// The selected floating drawings lined up on a side (or through the middle) of `area`, or,
/// without one, of the box around them all.
fn align_in(s: &mut Session, how: &str, area: Option<Rect>) -> CmdResult {
    let found = picked_bounds(s);
    if area.is_none() && found.len() < 2 {
        return Err(CmdError::Disabled("select two or more floating pictures or shapes on one page to align them with each other".into()));
    }
    if found.is_empty() {
        return Err(CmdError::Disabled("select a floating picture or shape to align it to the page".into()));
    }
    let lo = |f: fn(&Rect) -> f32| found.iter().map(|(_, b)| f(b)).fold(f32::MAX, f32::min);
    let hi = |f: fn(&Rect) -> f32| found.iter().map(|(_, b)| f(b)).fold(f32::MIN, f32::max);
    let (left, right, top, bottom) = match area {
        Some(a) => (a.x, a.right(), a.y, a.bottom()),
        None => (lo(|b| b.x), hi(Rect::right), lo(|b| b.y), hi(Rect::bottom)),
    };
    let moves: Vec<(Pos, f32, f32)> = found
        .iter()
        .map(|(pos, b)| {
            let (dx, dy) = match how {
                "left" => (left - b.x, 0.0),
                "right" => (right - b.right(), 0.0),
                "center" => ((left + right) / 2.0 - (b.x + b.w / 2.0), 0.0),
                "top" => (0.0, top - b.y),
                "bottom" => (0.0, bottom - b.bottom()),
                _ => (0.0, (top + bottom) / 2.0 - (b.y + b.h / 2.0)),
            };
            (pos.clone(), dx, dy)
        })
        .collect();
    for (pos, dx, dy) in &moves {
        shift(s, pos, *dx, *dy)?;
    }
    Ok(json!({"aligned": moves.len()}))
}

/// `arrange.distribute`: the selected drawings (three or more) spaced evenly between the two
/// outermost, which stay: the gaps between one and the next made the same.
pub(super) fn distribute(s: &mut Session, v: &Value) -> CmdResult {
    let across = match p::str(v, "value").unwrap_or("horizontal") {
        "horizontal" => true,
        "vertical" => false,
        other => return Err(CmdError::Params(format!("`value`: horizontal or vertical, not `{}`", other.chars().take(20).collect::<String>()))),
    };
    let mut found = picked_bounds(s);
    if found.len() < 3 {
        return Err(CmdError::Disabled("select three or more floating pictures or shapes on one page to space them evenly".into()));
    }
    found.sort_by(|a, b| start_of(&a.1, across).total_cmp(&start_of(&b.1, across)));
    let first = found.first().map(|(_, b)| start_of(b, across)).unwrap_or(0.0);
    let last = found.iter().map(|(_, b)| start_of(b, across) + size_of(b, across)).fold(f32::MIN, f32::max);
    let taken: f32 = found.iter().map(|(_, b)| size_of(b, across)).sum();
    let gap = (last - first - taken) / (found.len() as f32 - 1.0);
    let mut at = first;
    let mut moves = Vec::with_capacity(found.len());
    for (pos, b) in &found {
        let d = at - start_of(b, across);
        moves.push((pos.clone(), if across { d } else { 0.0 }, if across { 0.0 } else { d }));
        at += size_of(b, across) + gap;
    }
    for (pos, dx, dy) in &moves {
        shift(s, pos, *dx, *dy)?;
    }
    Ok(json!({"distributed": moves.len()}))
}

fn start_of(b: &Rect, across: bool) -> f32 {
    if across { b.x } else { b.y }
}

fn size_of(b: &Rect, across: bool) -> f32 {
    if across { b.w } else { b.h }
}

/// Delete with several drawings selected: all of them go (tracked, when changes are). Returns
/// whether that was the case; with one selected, or none, nothing is done here.
pub(crate) fn delete_picked(s: &mut Session) -> Result<bool, CmdError> {
    if s.also_selected.is_empty() || object_selection(s).is_none() {
        return Ok(false);
    }
    let mut picks = picked_objects(s);
    if picks.len() < 2 {
        return Ok(false);
    }
    s.also_selected.clear();
    // From the last one back: taking one out moves only what comes after it.
    picks.sort();
    for pos in picks.into_iter().rev() {
        if !s.doc.para_at(&pos).and_then(|q| q.object_at(pos.off)).is_some_and(InlineObject::is_drawing) {
            continue;
        }
        s.sel = Selection { anchor: pos.clone(), focus: Pos { off: pos.off + wordcraft_doc::para::OBJ.len_utf8(), ..pos } };
        crate::cmd::delete_selection_effect(s)?;
    }
    Ok(true)
}
