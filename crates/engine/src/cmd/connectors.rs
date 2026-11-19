//! Connectors: gluing a connector's ends to shapes' connection sites (`shape.connect`), and
//! re-routing glued connectors when the shapes they join move or change size ([`reroute`], run by
//! the session after object edits).
//!
//! Floating connectors in the body are routed on the page (from the layout); connectors inside a
//! group, in the group's own space. A glue to a shape that is gone (or on another page) leaves
//! that end where it is.

use std::collections::HashMap;

use serde_json::{Value, json};
use wordcraft_doc::connector::{
    Around, ConnEnd, Glued, RouteEnd, Routed, Was, ends_spun, leaves_upright, nearest_site, route_around, route_connector, site_point, site_way_out,
};
use wordcraft_doc::para::{Anchor, InlineObject, Wrap};
use wordcraft_doc::{Pos, StoryRef};

use crate::{CmdError, CmdResult, CommandSpec, Session, p};

/// How close (points) a dragged end must come to a connection site to glue to it.
pub const SNAP: f32 = 12.0;
/// Largest offset a routed connector gets, points.
const MAX_OFFSET: f32 = 4000.0;

fn has_connector(s: &Session) -> Option<&'static str> {
    match super::objects::selected(s) {
        Some((_, InlineObject::Shape { kind, .. })) if kind.is_connector() => None,
        _ => Some("select a connector first"),
    }
}

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("shape.connect", "Connect Shapes", "Shape Format › Insert Shapes", connect)
            .params(
                r#"{"start"?: end, "end"?: end}  end: {"path": [..], "off": n, "site"?: n} (glue to that body shape's connection site; nearest one when omitted) | {"x": pt, "y": pt, "page"?: n} (drag the end there: it glues to a shape's site within 12 pt, else stays free there) | null (unglue)"#,
            )
            .when(has_connector),
        CommandSpec::new("shape.bend", "Move Connector Bend", "Shape Format › Insert Shapes", bend)
            .params(
                r#"{"handle"?: 1-3 (which run between the first and the last; 1), "x": pt, "y": pt} (move that run of the selected bent or curved connector to this point of its page) | {"handle"?: n, "value": n} (the run's adjust value: 100000ths of the frame's width or height, outside 0-100000 for a run outside the frame)"#,
            )
            .when(has_connector),
        CommandSpec::new("shape.reroute", "Reroute Connectors", "Shape Format › Insert Shapes", reroute_connectors)
            .params(
                r#"{}  (the selected connectors, and those glued to the selected shapes, are glued to the connection sites that make them shortest: the sides of their shapes that face each other. A connector glued at one end goes to the site nearest its free end. Each is then routed anew, around the drawings in its way)"#,
            )
            .when(has_reroutable),
    ]
}

fn has_reroutable(s: &Session) -> Option<&'static str> {
    (!reroutable(s).is_empty()).then_some(()).map_or(Some("select a glued connector, or a shape connectors are glued to"), |()| None)
}

/// The glued connectors `shape.reroute` applies to: those selected, and those glued to a
/// selected shape.
fn reroutable(s: &Session) -> Vec<Pos> {
    let picked = super::objects::picked_objects(s);
    if picked.is_empty() {
        return Vec::new();
    }
    let picked_ids: Vec<u32> = picked
        .iter()
        .filter_map(|p| match s.doc.para_at(p)?.object_at(p.off)? {
            InlineObject::Shape { kind, extra, .. } if !kind.is_connector() && extra.id != 0 => Some(vec![extra.id]),
            // A group: its members' connectors.
            InlineObject::Group { children, .. } => Some(
                children
                    .iter()
                    .filter_map(|c| match &c.obj {
                        InlineObject::Shape { extra, .. } if extra.id != 0 => Some(extra.id),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .flatten()
        .collect();
    let (_, conns, _) = scan(s);
    conns
        .into_iter()
        .filter(|c| {
            picked.contains(c)
                || matches!(s.doc.para_at(c).and_then(|p| p.object_at(c.off)), Some(InlineObject::Shape { extra, .. })
                    if [extra.start, extra.end].into_iter().flatten().any(|e| picked_ids.contains(&e.id)))
        })
        .collect()
}

/// The length of the path a connector `kind` takes from `start` to `end`: what `shape.reroute`
/// makes least.
fn route_length(kind: wordcraft_doc::para::ShapeKind, start: RouteEnd, end: RouteEnd) -> f32 {
    use wordcraft_doc::graphic::PathSeg;
    let to = route_connector(kind, start, end, Was::default());
    let segs = wordcraft_doc::connector::connector_path(kind, to.preset.as_ref(), to.frame.w, to.frame.h).unwrap_or_default();
    let (mut len, mut at) = (0.0f32, (0.0f32, 0.0f32));
    for seg in segs {
        let next = match seg {
            PathSeg::Move(x, y) => {
                at = (x, y);
                continue;
            }
            PathSeg::Line(x, y) => (x, y),
            // A curve by its control points: longer than the curve, the more it bends.
            PathSeg::Cubic(a, b, c, d, x, y) => {
                len += (a - at.0).hypot(b - at.1) + (c - a).hypot(d - b);
                at = (c, d);
                (x, y)
            }
            PathSeg::Close => continue,
        };
        len += (next.0 - at.0).hypot(next.1 - at.1);
        at = next;
    }
    if len.is_finite() { len } else { f32::MAX }
}

/// `shape.reroute`: glue connectors to the sites of their shapes that make them shortest.
fn reroute_connectors(s: &mut Session, _v: &Value) -> CmdResult {
    let chosen = reroutable(s);
    if chosen.is_empty() {
        return Err(CmdError::Disabled("select a glued connector, or a shape connectors are glued to".into()));
    }
    let (ids, ..) = scan(s);
    let layout = s.layout();
    let objects = layout.object_index();
    let mut plans: Vec<(Pos, Option<u32>, Option<u32>)> = Vec::new();
    for pos in &chosen {
        let Some(hit) = objects.object(pos, s.page_hint) else { continue };
        let Some(InlineObject::Shape { kind, extra, .. }) = s.doc.para_at(pos).and_then(|p| p.object_at(pos.off)) else { continue };
        let r = hit.rect;
        let (s0, e0) = ends_spun([r.x, r.y, r.w, r.h], hit.spin);
        // Each end's choices: every site of the shape it is glued to, or where it is.
        let choices = |end: Option<ConnEnd>, free: (f32, f32)| -> Vec<(Option<u32>, RouteEnd)> {
            let Some((shape, rect, spin)) = end.and_then(|e| target_frame(s, &objects, &ids, (e.id, hit.page))) else {
                return vec![(None, RouteEnd::free(free))];
            };
            let n = wordcraft_doc::connector::shape_sites(shape, rect[2], rect[3]).len().min(wordcraft_doc::connector::MAX_SITE as usize) as u32;
            let sites: Vec<_> = (0..n)
                .filter_map(|i| Some((Some(i), RouteEnd { at: site_point(shape, rect, spin, i)?, out: site_way_out(shape, rect, spin, i) })))
                .collect();
            if sites.is_empty() { vec![(None, RouteEnd::free(free))] } else { sites }
        };
        let (starts, ends) = (choices(extra.start, s0), choices(extra.end, e0));
        let mut best: Option<(f32, Option<u32>, Option<u32>)> = None;
        for (a, from) in &starts {
            for (b, to) in &ends {
                let len = route_length(*kind, *from, *to);
                if best.is_none_or(|(l, ..)| len < l - 0.01) {
                    best = Some((len, *a, *b));
                }
            }
        }
        if let Some((_, a, b)) = best {
            plans.push((pos.clone(), a, b));
        }
    }
    let mut changed = Vec::new();
    for (pos, a, b) in plans {
        let para = s.doc.para_mut(pos.story, &pos.path)?;
        let Some(InlineObject::Shape { extra, .. }) = para.object_at_mut(pos.off) else { continue };
        let before = (extra.start, extra.end);
        if let (Some(e), Some(site)) = (extra.start.as_mut(), a) {
            e.site = site;
        }
        if let (Some(e), Some(site)) = (extra.end.as_mut(), b) {
            e.site = site;
        }
        if before != (extra.start, extra.end) {
            para.touch();
            changed.push(pos);
        }
    }
    if !changed.is_empty() {
        s.touch();
    }
    // Each one routed anew, its sites changed or not: around what has come to stand in its way.
    let moved = reroute_all(s, &chosen);
    if moved {
        s.touch();
    }
    Ok(json!({"rerouted": changed.len(), "moved": moved, "connectors": chosen.len()}))
}

/// Commands after which glued connectors are re-routed: those that move, resize, turn, add or
/// change drawings.
pub fn reroutes_after(id: &str) -> bool {
    id.starts_with("arrange.") || id.starts_with("shape.") || id.starts_with("picture.") || id == "insert.shape" || id.starts_with("draw.")
}

/// Where a shape a connector may be glued to is: in the body itself, or a member of a group there
/// (the group's place and the member's index in it).
#[derive(Clone, Debug, PartialEq)]
struct Target {
    pos: Pos,
    member: Option<usize>,
}

/// The body's floating drawings with an id (by id; a group's members too) and its glued
/// connectors.
fn scan(s: &Session) -> (HashMap<u32, Target>, Vec<Pos>, Vec<Pos>) {
    let (mut ids, mut conns, mut groups) = (HashMap::new(), Vec::new(), Vec::new());
    for path in s.doc.para_paths(StoryRef::Body) {
        let Some(para) = s.doc.para(StoryRef::Body, &path) else { continue };
        for (off, obj) in para.objects_with_offsets() {
            let pos = Pos { story: StoryRef::Body, path: path.clone(), off };
            match obj {
                InlineObject::Shape { kind, extra, float, .. } => {
                    if extra.id != 0 {
                        ids.entry(extra.id).or_insert(Target { pos: pos.clone(), member: None });
                    }
                    if kind.is_connector() && float.wrap != Wrap::Inline && (extra.start.is_some() || extra.end.is_some()) {
                        conns.push(pos);
                    }
                }
                InlineObject::Group { children, .. } => {
                    // Its members can be glued to from outside the group: a connector between
                    // two shapes stays between them when one is grouped with something else.
                    for (i, c) in children.iter().enumerate().take(wordcraft_doc::para::MAX_GROUP_CHILDREN) {
                        if let InlineObject::Shape { kind, extra, .. } = &c.obj
                            && extra.id != 0
                            && !kind.is_connector()
                        {
                            ids.entry(extra.id).or_insert(Target { pos: pos.clone(), member: Some(i) });
                        }
                    }
                    if children.iter().any(|c| matches!(&c.obj, InlineObject::Shape { kind, extra, .. } if kind.is_connector() && (extra.start.is_some() || extra.end.is_some()))) {
                        groups.push(pos)
                    }
                }
                _ => {}
            }
        }
    }
    (ids, conns, groups)
}

/// Re-route every glued connector to the shapes it joins. Returns whether anything moved.
pub fn reroute(s: &mut Session) -> bool {
    reroute_with(s, None)
}

/// Member `i` of the group `group` laid out at `hit`, as connectors see it: the shape, its frame
/// on the page (carried round with the group when the group is turned) and its spin there.
fn member_frame<'a>(
    group: &'a InlineObject,
    hit: &wordcraft_layout::hit::ObjectHit,
    i: usize,
) -> Option<(Glued<'a>, [f32; 4], wordcraft_geom::Spin)> {
    member_frame_at(group, (hit.rect, hit.spin), i)
}

/// [`member_frame`] for a group whose frame is `r`, turned `spin`.
fn member_frame_at<'a>(
    group: &'a InlineObject,
    (r, spin): (wordcraft_geom::Rect, wordcraft_geom::Spin),
    i: usize,
) -> Option<(Glued<'a>, [f32; 4], wordcraft_geom::Spin)> {
    let InlineObject::Group { children, .. } = group else { return None };
    let child = &children.get(i)?.obj;
    let [x, y, w, h] = group.member_rects(r.x, r.y, r.w, r.h).into_iter().find(|(n, _)| *n == i)?.1;
    // The group turns about its own middle: the member's middle goes round with it, and the
    // member's own turn is within the group's.
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let (nx, ny) = spin.apply(r.x + r.w / 2.0, r.y + r.h / 2.0, cx, cy);
    let (_, _, float) = child.frame()?;
    Some((Glued::of(child)?, [x + nx - cx, y + ny - cy, w, h], float.spin().within(spin)))
}

/// The connection sites, on its page, of the drawing at `pos` whose frame there is `frame`: a
/// shape's own, or those of every shape in a group. None for a connector or a picture. Where a
/// connector's end dragged near them can be glued (`shape.connect`).
pub fn sites_on_page(s: &Session, pos: &Pos, frame: (wordcraft_geom::Rect, wordcraft_geom::Spin)) -> Vec<(f32, f32)> {
    let Some(obj) = s.doc.para_at(pos).and_then(|p| p.object_at(pos.off)) else { return Vec::new() };
    let shapes: Vec<(Glued<'_>, [f32; 4], wordcraft_geom::Spin)> = match obj {
        InlineObject::Group { children, .. } => {
            (0..children.len().min(wordcraft_doc::para::MAX_GROUP_CHILDREN)).filter_map(|m| member_frame_at(obj, frame, m)).collect()
        }
        _ => Glued::of(obj).map(|shape| (shape, [frame.0.x, frame.0.y, frame.0.w, frame.0.h], frame.1)).into_iter().collect(),
    };
    let mut out = Vec::new();
    for (shape, rect, spin) in shapes {
        if shape.kind.is_connector() {
            continue;
        }
        let sites = u32::try_from(wordcraft_doc::connector::shape_sites(shape, rect[2], rect[3]).len()).unwrap_or(0);
        out.extend((0..sites).filter_map(|n| site_point(shape, rect, spin, n)));
    }
    out
}

/// How far (points) a glued end may be from its site and still be where it belongs.
const AT_SITE: f32 = 0.05;

/// [`reroute`], routing the connector at `fresh` anew even when its ends are where they belong
/// (its glue was just changed: the way it leaves its shapes may have).
fn reroute_with(s: &mut Session, fresh: Option<&Pos>) -> bool {
    reroute_all(s, fresh.map_or(&[], std::slice::from_ref))
}

/// The shape with drawing id `id` on `page` as connectors see it: the shape, its frame there and
/// its spin. `None` for a shape that is gone or on another page.
fn target_frame<'a>(
    s: &'a Session,
    objects: &wordcraft_layout::hit::ObjectIndex,
    ids: &HashMap<u32, Target>,
    (id, page): (u32, usize),
) -> Option<(Glued<'a>, [f32; 4], wordcraft_geom::Spin)> {
    let target = ids.get(&id)?;
    let hit = objects.object(&target.pos, page)?;
    if hit.page != page {
        return None;
    }
    let obj = s.doc.para_at(&target.pos)?.object_at(target.pos.off)?;
    match target.member {
        None => Some((Glued::of(obj)?, [hit.rect.x, hit.rect.y, hit.rect.w, hit.rect.h], hit.spin)),
        Some(i) => member_frame(obj, hit, i),
    }
}

/// The connectors glued to the drawing at `moved` (a shape, or a group with glued members) as
/// they would run with that drawing's frame at `to` on `page`, turned `spin`: each one's path in
/// page points. What a drag shows before the drawing is dropped; the document is not touched.
pub fn follow_routes(
    s: &Session,
    layout: &wordcraft_layout::DocLayout,
    moved: &Pos,
    to: (usize, wordcraft_geom::Rect, wordcraft_geom::Spin),
) -> Vec<Vec<wordcraft_doc::graphic::PathSeg>> {
    followed(s, layout, moved, to).into_iter().map(|f| f.1).collect()
}

/// The labels of the connectors [`follow_routes`] gives, where each would then be: its frame on
/// the page, about the middle of its connector's new path. Nothing for a document with no
/// connector labels.
pub fn follow_labels(
    s: &Session,
    layout: &wordcraft_layout::DocLayout,
    moved: &Pos,
    to: (usize, wordcraft_geom::Rect, wordcraft_geom::Spin),
) -> Vec<wordcraft_geom::Rect> {
    // Each labelled connector's labels' sizes, by the connector's drawing id.
    let mut labels: HashMap<u32, Vec<(f32, f32)>> = HashMap::new();
    for path in s.doc.para_paths(StoryRef::Body) {
        let Some(para) = s.doc.para(StoryRef::Body, &path) else { continue };
        for (_, obj) in para.objects_with_offsets() {
            if let InlineObject::Shape { extra, w, h, .. } = obj
                && let Some(id) = extra.label_of
                && labels.len() < 256
            {
                labels.entry(id).or_default().push((*w, *h));
            }
        }
    }
    if labels.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (id, path) in followed(s, layout, moved, to) {
        let (Some(sizes), Some((mx, my))) = (labels.get(&id), wordcraft_doc::connector::path_midpoint(&path)) else { continue };
        out.extend(sizes.iter().take(8).map(|(w, h)| wordcraft_geom::Rect::new(mx - w / 2.0, my - h / 2.0, *w, *h)));
    }
    out
}

/// [`follow_routes`], each path with its connector's drawing id (0: none).
fn followed(
    s: &Session,
    layout: &wordcraft_layout::DocLayout,
    moved: &Pos,
    (page, to, spin): (usize, wordcraft_geom::Rect, wordcraft_geom::Spin),
) -> Vec<(u32, Vec<wordcraft_doc::graphic::PathSeg>)> {
    use wordcraft_doc::graphic::PathSeg;
    let (ids, conns, _) = scan(s);
    if conns.is_empty() || !ids.values().any(|t| t.pos == *moved) {
        return Vec::new();
    }
    let objects = layout.object_index();
    // An end's site: on the moved drawing where it would be, else where its shape is.
    let drawings = drawings_on(s, layout, page, Some((moved, to, spin)));
    let site = |e: Option<ConnEnd>| -> Option<(RouteEnd, bool, &Pos, [f32; 4])> {
        let e = e?;
        let target = ids.get(&e.id)?;
        let on_moved = target.pos == *moved;
        let (shape, rect, at) = if on_moved {
            let obj = s.doc.para_at(moved)?.object_at(moved.off)?;
            match target.member {
                None => (Glued::of(obj)?, [to.x, to.y, to.w, to.h], spin),
                Some(i) => member_frame_at(obj, (to, spin), i)?,
            }
        } else {
            target_frame(s, &objects, &ids, (e.id, page))?
        };
        Some((
            RouteEnd { at: site_point(shape, rect, at, e.site)?, out: site_way_out(shape, rect, at, e.site) },
            on_moved,
            &target.pos,
            bounds_of(rect, at),
        ))
    };
    let mut out = Vec::new();
    for pos in conns.iter().take(256) {
        let Some(hit) = objects.object(pos, page).filter(|h| h.page == page) else { continue };
        let Some(InlineObject::Shape { kind, extra, float, w, h, .. }) = s.doc.para_at(pos).and_then(|p| p.object_at(pos.off)) else { continue };
        let (a, b) = (site(extra.start), site(extra.end));
        if !a.is_some_and(|x| x.1) && !b.is_some_and(|x| x.1) {
            continue;
        }
        let r = hit.rect;
        let (s0, e0) = ends_spun([r.x, r.y, r.w, r.h], hit.spin);
        let was = Was { preset: extra.preset.as_deref(), w: *w, h: *h, upright: leaves_upright(float.rot) };
        let round = around(&drawings, pos, [a.map(|x| (x.2, x.3)), b.map(|x| (x.2, x.3))]);
        let routed = route_around(*kind, a.map_or(RouteEnd::free(s0), |x| x.0), b.map_or(RouteEnd::free(e0), |x| x.0), was, &round);
        let f = routed.frame;
        let Some(segs) = wordcraft_doc::connector::connector_path(*kind, routed.preset.as_ref(), f.w, f.h) else { continue };
        let turn = wordcraft_geom::Spin::new(routed.rot, f.flip_h, f.flip_v);
        let on_page = |x: f32, y: f32| turn.apply(f.x + f.w / 2.0, f.y + f.h / 2.0, f.x + x, f.y + y);
        out.push((
            extra.id,
            segs.into_iter()
                .map(|seg| match seg {
                    PathSeg::Move(x, y) => {
                        let p = on_page(x, y);
                        PathSeg::Move(p.0, p.1)
                    }
                    PathSeg::Line(x, y) => {
                        let p = on_page(x, y);
                        PathSeg::Line(p.0, p.1)
                    }
                    PathSeg::Cubic(a, b, c, d, x, y) => {
                        let (p1, p2, p3) = (on_page(a, b), on_page(c, d), on_page(x, y));
                        PathSeg::Cubic(p1.0, p1.1, p2.0, p2.1, p3.0, p3.1)
                    }
                    PathSeg::Close => PathSeg::Close,
                })
                .collect(),
        ));
    }
    out
}

/// The bounds (x0, y0, x1, y1) on its page of a drawing whose frame is `rect`, turned `spin`.
fn bounds_of(rect: [f32; 4], spin: wordcraft_geom::Spin) -> [f32; 4] {
    let b = spin.bounds(wordcraft_geom::Rect::new(rect[0], rect[1], rect[2], rect[3]));
    [b.x, b.y, b.right(), b.bottom()]
}

/// The body's drawings on `page` a connector may have to go around (shapes, pictures, groups;
/// not lines and connectors): where each is, and its bounds. The drawing at `moved` is taken to
/// be at the frame and spin given with it.
fn drawings_on(
    s: &Session,
    layout: &wordcraft_layout::DocLayout,
    page: usize,
    moved: Option<(&Pos, wordcraft_geom::Rect, wordcraft_geom::Spin)>,
) -> Vec<(Pos, [f32; 4])> {
    let Some(pg) = layout.pages.get(page) else { return Vec::new() };
    let mut out = Vec::new();
    for it in pg.items.iter().take(4096) {
        let wordcraft_layout::Placed::Object { rect, spin, story: StoryRef::Body, path, off, .. } = it else { continue };
        let pos = Pos { story: StoryRef::Body, path: path.clone(), off: *off };
        let Some(obj) = s.doc.para_at(&pos).and_then(|p| p.object_at(pos.off)) else { continue };
        // Lines and connectors cross each other freely, and a connector's label sits on it.
        if !obj.is_drawing() || matches!(obj, InlineObject::Shape { kind, extra, .. } if kind.is_open() || extra.label_of.is_some()) {
            continue;
        }
        let (rect, spin) = match moved {
            Some((m, to, turned)) if *m == pos => (to, turned),
            _ => (*rect, *spin),
        };
        out.push((pos, bounds_of([rect.x, rect.y, rect.w, rect.h], spin)));
    }
    out
}

/// What a connector at `conn`, its ends glued to the drawings at `hosts` (bounds given), goes
/// around among `drawings`: everything but itself and what it is glued to (a group it is glued
/// into: only the member it is glued to counts, as its end's shape).
fn around(drawings: &[(Pos, [f32; 4])], conn: &Pos, hosts: [Option<(&Pos, [f32; 4])>; 2]) -> Around {
    let is_host = |p: &Pos| hosts.iter().flatten().any(|(h, _)| *h == p);
    Around {
        start: hosts[0].map(|(_, b)| b),
        end: hosts[1].map(|(_, b)| b),
        others: drawings.iter().filter(|(p, _)| p != conn && !is_host(p)).map(|(_, b)| *b).collect(),
    }
}

/// [`reroute`], routing the connectors at `fresh` anew even when their ends are where they belong.
fn reroute_all(s: &mut Session, fresh: &[Pos]) -> bool {
    let (ids, conns, groups) = scan(s);
    let mut moved = false;
    for pos in groups {
        if let Ok(para) = s.doc.para_mut(pos.story, &pos.path)
            && let Some(group @ InlineObject::Group { .. }) = para.object_at_mut(pos.off)
            && wordcraft_doc::connector::reroute_group(group)
        {
            para.touch();
            moved = true;
        }
    }
    if conns.is_empty() || ids.is_empty() {
        return moved;
    }
    if moved {
        s.touch();
    }
    let layout = s.layout();
    let objects = layout.object_index();
    // A glued end: its site on the page, and the way out of its shape there.
    let site = |e: Option<ConnEnd>, page: usize| -> Option<(RouteEnd, &Pos, [f32; 4])> {
        let e = e?;
        let (shape, rect, spin) = target_frame(s, &objects, &ids, (e.id, page))?;
        let end = RouteEnd { at: site_point(shape, rect, spin, e.site)?, out: site_way_out(shape, rect, spin, e.site) };
        Some((end, &ids.get(&e.id)?.pos, bounds_of(rect, spin)))
    };
    // The drawings of each page a connector is on, to go around: found once a page.
    let mut on_page: HashMap<usize, Vec<(Pos, [f32; 4])>> = HashMap::new();
    let mut plans = Vec::new();
    for pos in conns {
        let Some(hit) = objects.object(&pos, s.page_hint) else { continue };
        let Some(InlineObject::Shape { kind, extra, float, w, h, .. }) = s.doc.para_at(&pos).and_then(|p| p.object_at(pos.off)) else { continue };
        let r = hit.rect;
        let (s0, e0) = ends_spun([r.x, r.y, r.w, r.h], hit.spin);
        let (from, to) = (site(extra.start, hit.page), site(extra.end, hit.page));
        let (start, end) = (from.map_or(RouteEnd::free(s0), |x| x.0), to.map_or(RouteEnd::free(e0), |x| x.0));
        // A connector whose ends are at their sites stays as it is, as a file or the user made
        // it: only one that a moved shape left behind is routed again.
        let near = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).abs() < AT_SITE && (a.1 - b.1).abs() < AT_SITE;
        if !fresh.contains(&pos) && near(start.at, s0) && near(end.at, e0) {
            continue;
        }
        let was = Was { preset: extra.preset.as_deref(), w: *w, h: *h, upright: leaves_upright(float.rot) };
        let drawings = on_page.entry(hit.page).or_insert_with(|| drawings_on(s, &layout, hit.page, None));
        let round = around(drawings, &pos, [from.map(|x| (x.1, x.2)), to.map(|x| (x.1, x.2))]);
        plans.push((pos, route_around(*kind, start, end, was, &round), (r.x, r.y), (hit.origin.x, hit.origin.y)));
    }
    for (pos, to, at, origin) in plans {
        moved |= place(s, &pos, to, at, origin);
    }
    moved
}

/// Put the floating connector at `pos` (now with its frame's top-left at page point `at`, its
/// anchor's origin at `origin`) where `to` routes it. Returns whether it changed.
fn place(s: &mut Session, pos: &Pos, to: Routed, at: (f32, f32), origin: (f32, f32)) -> bool {
    let Ok(para) = s.doc.para_mut(pos.story, &pos.path) else { return false };
    let Some(InlineObject::Shape { kind, w, h, float, extra, .. }) = para.object_at_mut(pos.off) else { return false };
    let near = |a: f32, b: f32| (a - b).abs() < 0.01;
    let Routed { frame: to, rot, preset } = to;
    // Only bent and curved connectors are drawn from their preset: a straight one keeps its own.
    let preset = if *kind == wordcraft_doc::para::ShapeKind::StraightConnector { extra.preset.clone() } else { preset.map(std::sync::Arc::new) };
    if near(to.x, at.0)
        && near(to.y, at.1)
        && near(to.w, *w)
        && near(to.h, *h)
        && to.flip_h == float.flip_h
        && to.flip_v == float.flip_v
        && float.rot == rot
        && extra.preset == preset
    {
        return false;
    }
    let clamp = |v: f32| wordcraft_geom::finite(v).clamp(-MAX_OFFSET, MAX_OFFSET);
    // An aligned position becomes an offset from the column / paragraph (where `origin` is).
    if float.h_align.take().is_some() {
        float.h_rel = Anchor::Column;
        float.x = clamp(to.x - origin.0);
    } else {
        float.x = clamp(float.x + to.x - at.0);
    }
    if float.v_align.take().is_some() {
        float.v_rel = Anchor::Paragraph;
        float.y = clamp(to.y - origin.1);
    } else {
        float.y = clamp(float.y + to.y - at.1);
    }
    *w = to.w.clamp(0.0, MAX_OFFSET);
    *h = to.h.clamp(0.0, MAX_OFFSET);
    float.rot = rot;
    float.flip_h = to.flip_h;
    float.flip_v = to.flip_v;
    extra.preset = preset;
    para.touch();
    true
}

/// A body position from `{"path": [..], "off": n}` (or any form `parse_pos` reads).
fn body_pos(e: &Value) -> Option<Pos> {
    let path: Option<Vec<u32>> =
        e.get("path").and_then(Value::as_array).map(|a| a.iter().take(64).filter_map(Value::as_u64).filter_map(|x| u32::try_from(x).ok()).collect());
    match (path, p::u64(e, "off").and_then(|o| usize::try_from(o).ok())) {
        (Some(path), Some(off)) if !path.is_empty() => Some(Pos { story: StoryRef::Body, path: wordcraft_doc::Path(path), off }),
        _ => super::parse_pos(e).filter(|t| t.story == StoryRef::Body),
    }
}

/// Give the drawing at `pos` (member `member` of the group there, when given) a drawing id if
/// it has none; returns it.
fn ensure_id(s: &mut Session, pos: &Pos, member: Option<usize>) -> Result<u32, CmdError> {
    let next = s.doc.next_shape_id();
    let para = s.doc.para_mut(pos.story, &pos.path)?;
    let target = match (para.object_at_mut(pos.off), member) {
        (Some(InlineObject::Group { children, .. }), Some(i)) => children.get_mut(i).map(|c| &mut c.obj),
        (o, None) => o,
        _ => None,
    };
    let (id, fresh) = match target {
        Some(InlineObject::Shape { extra, kind, .. }) if !kind.is_connector() => {
            let fresh = extra.id == 0;
            if fresh {
                extra.id = next;
            }
            (extra.id, fresh)
        }
        _ => return Err(CmdError::Params("connectors glue to shapes and text boxes".into())),
    };
    if fresh {
        para.touch();
    }
    Ok(id)
}

/// `shape.connect`: glue, move or unglue the selected connector's ends.
pub(crate) fn connect(s: &mut Session, v: &Value) -> CmdResult {
    let (pos, obj) = super::objects::selected(s).ok_or_else(|| CmdError::Disabled("select a connector first".into()))?;
    let InlineObject::Shape { kind, float, .. } = &obj else { return Err(CmdError::Disabled("select a connector first".into())) };
    if !kind.is_connector() {
        return Err(CmdError::Disabled("select a connector first".into()));
    }
    if pos.story != StoryRef::Body || float.wrap == Wrap::Inline {
        return Err(CmdError::Disabled("only floating connectors in the body text can be glued".into()));
    }
    let (kind, upright) = (*kind, leaves_upright(float.rot));
    let layout = s.layout();
    let hit = layout.object(&pos, s.page_hint).ok_or_else(|| CmdError::Failed("the connector isn't laid out".into()))?;
    let r = hit.rect;
    let (mut p0, mut p1) = ends_spun([r.x, r.y, r.w, r.h], hit.spin);
    let mut glue: [Option<Option<ConnEnd>>; 2] = [None, None];
    for (i, key) in ["start", "end"].into_iter().enumerate() {
        let Some(e) = v.get(key) else { continue };
        if e.is_null() {
            glue[i] = Some(None);
            continue;
        }
        let here = if i == 0 { p0 } else { p1 };
        let (target, site) = if e.get("path").is_some() {
            let at = body_pos(e).ok_or_else(|| CmdError::Params(format!("`{key}`: no shape there")))?;
            let th = layout
                .object(&at, hit.page)
                .filter(|t| t.page == hit.page)
                .ok_or_else(|| CmdError::Params(format!("`{key}`: no shape there on this page")))?;
            let tkind = match s.doc.para_at(&at).and_then(|p| p.object_at(at.off)).and_then(Glued::of) {
                Some(shape) if !shape.kind.is_connector() => shape,
                _ => return Err(CmdError::Params(format!("`{key}`: connectors glue to shapes and text boxes"))),
            };
            let rect = [th.rect.x, th.rect.y, th.rect.w, th.rect.h];
            let site = match p::u64(e, "site") {
                Some(n) => u32::try_from(n)
                    .ok()
                    .filter(|n| site_point(tkind, rect, th.spin, *n).is_some())
                    .ok_or_else(|| CmdError::Params(format!("`{key}`: no such connection site")))?,
                None => nearest_site(tkind, rect, th.spin, here).map(|(n, ..)| n).unwrap_or(0),
            };
            (Some((at, None)), site)
        } else {
            let (x, y) = (p::f32(e, "x").ok_or_else(|| CmdError::Params(format!("`{key}`: a shape or a point")))?, p::f32(e, "y").unwrap_or(here.1));
            let page = p::u64(e, "page").map_or(hit.page, |n| usize::try_from(n).unwrap_or(usize::MAX));
            if page != hit.page {
                return Err(CmdError::Params("a connector's ends stay on its page".into()));
            }
            // The nearest site of a shape (not a connector) within reach.
            let mut best: Option<((Pos, Option<usize>), u32, f32)> = None;
            if let Some(pg) = layout.pages.get(page) {
                for it in &pg.items {
                    let wordcraft_layout::Placed::Object { rect, spin, story: StoryRef::Body, path, off, .. } = it else { continue };
                    let at = Pos { story: StoryRef::Body, path: path.clone(), off: *off };
                    if at == pos {
                        continue;
                    }
                    let Some(obj) = s.doc.para_at(&at).and_then(|p| p.object_at(at.off)) else { continue };
                    // The shape itself, or each shape of a group: a connector is glued to the
                    // shape in the group, not to the group.
                    let members: Vec<(Option<usize>, Glued<'_>, [f32; 4], wordcraft_geom::Spin)> = match obj {
                        InlineObject::Group { children, .. } => (0..children.len().min(wordcraft_doc::para::MAX_GROUP_CHILDREN))
                            .filter_map(|m| member_frame_at(obj, (*rect, *spin), m).map(|(shape, r, sp)| (Some(m), shape, r, sp)))
                            .collect(),
                        _ => Glued::of(obj).map(|shape| (None, shape, [rect.x, rect.y, rect.w, rect.h], *spin)).into_iter().collect(),
                    };
                    for (member, shape, frame, turned) in members {
                        if shape.kind.is_connector() {
                            continue;
                        }
                        if let Some((n, _, d)) = nearest_site(shape, frame, turned, (x, y))
                            && d <= SNAP
                            && best.as_ref().is_none_or(|b| d < b.2)
                        {
                            best = Some(((at.clone(), member), n, d));
                        }
                    }
                }
            }
            match best {
                Some((at, n, _)) => (Some(at), n),
                None => {
                    // A free end, moved there.
                    if i == 0 {
                        p0 = (x, y);
                    } else {
                        p1 = (x, y);
                    }
                    glue[i] = Some(None);
                    continue;
                }
            }
        };
        if let Some((at, member)) = target {
            let id = ensure_id(s, &at, member)?;
            glue[i] = Some(Some(ConnEnd { id, site }));
        }
    }
    // Free ends first, then glued ones are routed.
    let para = s.doc.para_mut(pos.story, &pos.path)?;
    if let Some(InlineObject::Shape { extra, .. }) = para.object_at_mut(pos.off) {
        if let Some(g) = glue[0] {
            extra.start = g;
        }
        if let Some(g) = glue[1] {
            extra.end = g;
        }
        para.touch();
    }
    // The ends where they were dragged, in the direction the connector had; then the glued
    // ones go to their sites, and the connector leaves its shapes away from them.
    let was_preset = match &obj {
        InlineObject::Shape { extra, .. } => extra.preset.clone(),
        _ => None,
    };
    let was = Was { preset: was_preset.as_deref(), w: r.w, h: r.h, upright };
    place(s, &pos, route_connector(kind, RouteEnd::free(p0), RouteEnd::free(p1), was), (r.x, r.y), (hit.origin.x, hit.origin.y));
    s.touch();
    reroute_with(s, Some(&pos));
    let o = s.doc.para_at(&pos).and_then(|p| p.object_at(pos.off)).cloned();
    Ok(json!({"object": serde_json::to_value(o).unwrap_or(Value::Null)}))
}

/// `shape.bend`: move one of the selected bent or curved connector's runs.
fn bend(s: &mut Session, v: &Value) -> CmdResult {
    let (pos, obj) = super::objects::selected(s).ok_or_else(|| CmdError::Disabled("select a connector first".into()))?;
    let InlineObject::Shape { kind, extra, w, h, .. } = &obj else { return Err(CmdError::Disabled("select a connector first".into())) };
    let (kind, preset) = (*kind, extra.preset.clone());
    let run = p::u64(v, "handle").map_or(Some(1), |n| u8::try_from(n).ok()).ok_or_else(|| CmdError::Params("`handle`: 1 to 3".into()))?;
    let handle = wordcraft_doc::connector::bend_handles(kind, preset.as_deref(), *w, *h)
        .into_iter()
        .find(|b| b.adjust == run)
        .ok_or_else(|| CmdError::Params("this connector has no such run to move".into()))?;
    // Where the run goes, in the connector's own frame.
    let (size, to) = match (p::f32(v, "value"), p::f32(v, "x"), p::f32(v, "y")) {
        (Some(value), ..) => {
            let share = wordcraft_geom::finite(value) / 100_000.0;
            ((*w, *h), (share * *w, share * *h))
        }
        (None, x, y) if x.is_some() || y.is_some() => {
            let layout = s.layout();
            let hit = layout.object(&pos, s.page_hint).ok_or_else(|| CmdError::Failed("the connector isn't laid out".into()))?;
            let r = hit.rect;
            let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
            // A missing coordinate leaves the run where it is that way.
            let now = hit.spin.apply(cx, cy, r.x + handle.at.0, r.y + handle.at.1);
            let own = hit.spin.unapply(cx, cy, wordcraft_geom::finite(x.unwrap_or(now.0)), wordcraft_geom::finite(y.unwrap_or(now.1)));
            ((r.w, r.h), (own.0 - r.x, own.1 - r.y))
        }
        _ => return Err(CmdError::Params("a point (`x`, `y`) or a `value`".into())),
    };
    let bent = wordcraft_doc::connector::bend_to(kind, preset.as_deref(), size.0, size.1, run, to)
        .ok_or_else(|| CmdError::Params("this connector has no such run to move".into()))?;
    let para = s.doc.para_mut(pos.story, &pos.path)?;
    if let Some(InlineObject::Shape { extra, .. }) = para.object_at_mut(pos.off) {
        extra.preset = bent.map(std::sync::Arc::new);
        para.touch();
    }
    s.touch();
    let o = s.doc.para_at(&pos).and_then(|p| p.object_at(pos.off)).cloned();
    Ok(json!({"object": serde_json::to_value(o).unwrap_or(Value::Null)}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordcraft_doc::connector::{ShapeExtra, ends};
    use wordcraft_doc::para::{Float, GroupChild, ShapeKind};

    fn shape(kind: ShapeKind, extra: ShapeExtra, float: Float) -> InlineObject {
        InlineObject::Shape {
            kind,
            w: 40.0,
            h: 20.0,
            fill: None,
            stroke: None,
            stroke_width: 1.0,
            float,
            story: None,
            freeform: None,
            effects: Default::default(),
            extra,
        }
    }

    /// Drawing commands must not search every page again for each glued connector.
    #[test]
    fn rerouting_many_pages_does_not_rescan_all_objects_per_connector() {
        use wordcraft_doc::{Document, Paragraph, para_block};
        let mut doc = Document::new();
        doc.body = (0..8000u32)
            .map(|i| {
                let mut p = Paragraph::default();
                p.props.page_break_before = Some(i > 0);
                let id = i * 2 + 1;
                for (kind, x, extra) in [
                    (ShapeKind::Rectangle, 100.0, ShapeExtra { id, ..Default::default() }),
                    (ShapeKind::Ellipse, 300.0, ShapeExtra { id: id + 1, ..Default::default() }),
                    (
                        ShapeKind::ElbowConnector,
                        0.0,
                        ShapeExtra { start: Some(ConnEnd { id, site: 3 }), end: Some(ConnEnd { id: id + 1, site: 2 }), ..Default::default() },
                    ),
                ] {
                    let float = Float { wrap: Wrap::InFrontOfText, h_rel: Anchor::Page, v_rel: Anchor::Page, x, y: 200.0, ..Default::default() };
                    p.insert_object(p.len(), shape(kind, extra, float), &Default::default()).unwrap();
                }
                para_block(p)
            })
            .collect();
        let mut s = Session::new(doc);
        s.view.proofing = false;
        s.page_hint = 0;
        assert_eq!(s.layout().pages.len(), 8000);
        let started = std::time::Instant::now();
        assert!(reroute(&mut s));
        let elapsed = started.elapsed();
        eprintln!("8000-page reroute: {elapsed:?}");
        assert!(elapsed < std::time::Duration::from_secs(5), "reroute took {elapsed:?}");
        for path in s.doc.para_paths(StoryRef::Body) {
            let para = s.doc.para(StoryRef::Body, &path).unwrap();
            let InlineObject::Shape { w, h, float, .. } = para.object_at(6).unwrap() else { panic!("connector") };
            assert_eq!((*w, *h, float.x, float.y), (160.0, 0.0, 140.0, 210.0));
        }
        s.touch();
        assert!(!reroute(&mut s), "rerouting must settle after refreshing the layout");
    }

    /// The per-case shape/geometry of one scaled-group reroute case.
    struct ScaledGroupSpec {
        w: f32,
        h: f32,
        kind: ShapeKind,
        rot: f32,
        flip_h: bool,
        flip_v: bool,
    }

    impl ScaledGroupSpec {
        fn new(w: f32, h: f32, kind: ShapeKind, rot: f32, flip_h: bool, flip_v: bool) -> Self {
            Self { w, h, kind, rot, flip_h, flip_v }
        }
    }

    fn visual_ends(group: &InlineObject) -> ((f32, f32), (f32, f32)) {
        let InlineObject::Group { w, h, float, .. } = group else { panic!("group") };
        let rects = group.group_rects(0.0, 0.0, *w, *h);
        let ([x, y, w, h], obj) = rects[1];
        let own = obj.frame().unwrap().2.spin();
        let outer = float.spin();
        let turn = |(x, y)| {
            let (x, y) = own.apply(rects[1].0[0] + w / 2.0, rects[1].0[1] + h / 2.0, x, y);
            let InlineObject::Group { w, h, .. } = group else { panic!("group") };
            outer.apply(*w / 2.0, *h / 2.0, x, y)
        };
        (turn((x, y)), turn((x + w, y + h)))
    }

    /// One scaled-group case: rerouting keeps the visual free end and the resolved sites.
    fn assert_scaled_group_reroute(spec: ScaledGroupSpec, free: Option<ConnEnd>, free_start: bool) {
        let attached = Some(ConnEnd { id: 7, site: 3 });
        let (start, end) = if free_start { (free, attached) } else { (attached, free) };
        let group = InlineObject::Group {
            w: spec.w,
            h: spec.h,
            ch_w: 200.0,
            ch_h: 300.0,
            float: Float { rot: 25.0, flip_h: true, ..Default::default() },
            canvas: None,
            children: vec![
                GroupChild { x: 100.0, y: 200.0, obj: shape(ShapeKind::Rectangle, ShapeExtra { id: 7, ..Default::default() }, Float::default()) },
                GroupChild {
                    x: 10.0,
                    y: 30.0,
                    obj: shape(
                        spec.kind,
                        ShapeExtra { start, end, ..Default::default() },
                        Float { rot: spec.rot, flip_h: spec.flip_h, flip_v: spec.flip_v, ..Default::default() },
                    ),
                },
            ],
        };
        // Rendering scales each child rectangle before applying its own spin.
        let before = visual_ends(&group);
        let expected = if free_start { before.0 } else { before.1 };
        let mut s = Session::new(wordcraft_doc::Document::new());
        let pos = s.doc.start_of(StoryRef::Body);
        s.doc.para_mut(pos.story, &pos.path).unwrap().insert_object(pos.off, group, &Default::default()).unwrap();
        assert!(reroute(&mut s));
        let group = s.doc.para_at(&pos).unwrap().object_at(pos.off).unwrap();
        let after = visual_ends(group);
        let actual = if free_start { after.0 } else { after.1 };
        assert!(
            (actual.0 - expected.0).abs() < 0.001 && (actual.1 - expected.1).abs() < 0.001,
            "{}x{}, {:?}, {}, {}, {}, {}: {:?} != {:?}",
            spec.w,
            spec.h,
            spec.kind,
            spec.rot,
            spec.flip_h,
            spec.flip_v,
            free_start,
            actual,
            expected
        );
        let InlineObject::Group { children, .. } = group else { panic!("group") };
        let c = &children[1];
        let InlineObject::Shape { w, h, float, .. } = &c.obj else { panic!("connector") };
        let routed = ends(c.x, c.y, *w, *h, float.flip_h, float.flip_v);
        assert_eq!(if free_start { routed.1 } else { routed.0 }, (140.0, 210.0), "resolved sites stay in child coordinates");
        assert_eq!(float.rot, 0.0);
        assert!(!reroute(&mut s), "rerouting must settle");
    }
    #[test]
    fn rerouting_scaled_group_preserves_visual_free_and_unresolved_ends() {
        for (w, h) in [(200.0, 300.0), (400.0, 600.0), (400.0, 150.0)] {
            for kind in [ShapeKind::StraightConnector, ShapeKind::ElbowConnector, ShapeKind::CurvedConnector] {
                for rot in [30.0, 90.0] {
                    for (flip_h, flip_v) in [(false, false), (true, false), (false, true), (true, true)] {
                        for free in [None, Some(ConnEnd { id: 404, site: 0 }), Some(ConnEnd { id: 7, site: 99 })] {
                            for free_start in [false, true] {
                                assert_scaled_group_reroute(ScaledGroupSpec::new(w, h, kind, rot, flip_h, flip_v), free, free_start);
                            }
                        }
                    }
                }
            }
        }
    }
}
