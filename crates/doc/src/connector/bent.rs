//! Bent and curved connectors: the path each preset draws, and the route between two shapes.
//!
//! DrawingML has four bent and four curved connector presets (`bentConnector2`–`5`,
//! `curvedConnector2`–`5`, ECMA-376 Part 1 §20.1.10.56 and its preset shape definitions). All
//! leave their frame's top-left corner along the frame's x axis; the number is how many straight
//! runs the bent one has, and the adjust values (`adj1`–`adj3`, 100000ths of the frame's width or
//! height) say where the runs in between lie. A value may lie outside the frame (below 0 or above
//! 100000): the path then leaves its frame, which is how a connector first goes away from its
//! target. A connector that must leave along the page's y axis is the same preset in a frame
//! turned a quarter.
//!
//! [`route_connector`] picks the preset, the turn and the adjust values so that the connector
//! leaves each shape it is glued to away from the side it is glued to, instead of always going
//! out sideways (which ran a connector from a shape's top or bottom along that side).

use super::{Frame, PresetGeom, route};
use crate::graphic::PathSeg;
use crate::para::ShapeKind;

/// An adjust value of the whole width or height.
const FULL: f32 = 100_000.0;
/// How far (points) a connector goes straight out of a shape before it may turn back.
pub const STUB: f32 = 18.0;
/// Smallest frame side (points) an adjust value outside the frame is measured against: a frame
/// thinner than this is made this thin, since a share of nothing reaches nowhere.
const MIN_SIDE: f32 = 0.01;
/// Largest adjust value written (it must fit a 32-bit integer).
const MAX_ADJUST: f32 = 2.0e9;
/// Farthest (points) a kept run may lie outside the frame.
const MAX_REACH: f32 = 4000.0;

/// The preset family of a connector `kind` and the number of the preset `preset` names in it
/// (2–5): `(curved, n)`. A connector with no preset of its family is number 3. `None` for
/// straight connectors and other shapes.
fn family(kind: ShapeKind, preset: Option<&PresetGeom>) -> Option<(bool, u8)> {
    let (curved, stem) = match kind {
        ShapeKind::ElbowConnector => (false, "bentConnector"),
        ShapeKind::CurvedConnector => (true, "curvedConnector"),
        _ => return None,
    };
    let n = preset.and_then(|p| p.name.strip_prefix(stem)).and_then(|d| d.parse::<u8>().ok()).filter(|n| (2..=5).contains(n)).unwrap_or(3);
    Some((curved, n))
}

/// `preset` when it is a connector preset of `kind`'s family: only those give adjust values.
fn of_family(kind: ShapeKind, preset: Option<&PresetGeom>) -> Option<&PresetGeom> {
    let stem = match kind {
        ShapeKind::ElbowConnector => "bentConnector",
        ShapeKind::CurvedConnector => "curvedConnector",
        _ => return None,
    };
    preset.filter(|p| p.name.strip_prefix(stem).is_some_and(|d| matches!(d, "2" | "3" | "4" | "5")))
}

/// Adjust value `adj{i}` of `preset` as a share (1.0 = the whole side); a half when not given.
fn share(preset: Option<&PresetGeom>, i: u8) -> f32 {
    let name = ["adj1", "adj2", "adj3"].get(usize::from(i.saturating_sub(1))).copied().unwrap_or("adj1");
    let v = preset.and_then(|p| p.adjust.iter().find(|(n, _)| n == name)).map_or(50_000, |(_, v)| *v);
    v as f32 / FULL
}

/// The path of a connector `kind` drawn as `preset` (its kind's own preset when `None` or not a
/// connector preset of that kind) in a `w` × `h` frame at the origin, from the top-left corner to
/// the bottom-right (flips and the turn are the frame's). `None` for other kinds.
pub fn connector_path(kind: ShapeKind, preset: Option<&PresetGeom>, w: f32, h: f32) -> Option<Vec<PathSeg>> {
    use PathSeg::{Cubic, Line, Move};
    let (w, h) = (wordcraft_geom::finite(w), wordcraft_geom::finite(h));
    if kind == ShapeKind::StraightConnector {
        return Some(vec![Move(0.0, 0.0), Line(w, h)]);
    }
    let preset = of_family(kind, preset);
    let (curved, n) = family(kind, preset)?;
    let fin = wordcraft_geom::finite;
    // The runs in between: x of the first upright one, y of the level one after it, x of the
    // second upright one.
    let (x1, y2, x3) = (fin(w * share(preset, 1)), fin(h * share(preset, 2)), fin(w * share(preset, 3)));
    let mid = |a: f32, b: f32| (a + b) / 2.0;
    Some(match (curved, n) {
        (false, n) => {
            corners(n, (x1, y2, x3), w, h).into_iter().enumerate().map(|(i, (x, y))| if i == 0 { Move(x, y) } else { Line(x, y) }).collect()
        }
        (true, 2) => vec![Move(0.0, 0.0), Cubic(w / 2.0, 0.0, w, h / 2.0, w, h)],
        // The curved presets pass through the middle of each run of the bent one, along it.
        (true, 3) => vec![Move(0.0, 0.0), Cubic(x1 / 2.0, 0.0, x1, h / 4.0, x1, h / 2.0), Cubic(x1, h * 0.75, mid(x1, w), h, w, h)],
        (true, 4) => {
            let (ya, xb) = (y2 / 2.0, mid(x1, w));
            vec![
                Move(0.0, 0.0),
                Cubic(x1 / 2.0, 0.0, x1, ya / 2.0, x1, ya),
                Cubic(x1, mid(ya, y2), mid(x1, xb), y2, xb, y2),
                Cubic(mid(xb, w), y2, w, mid(y2, h), w, h),
            ]
        }
        (true, _) => {
            let (ya, xb, yc) = (y2 / 2.0, mid(x1, x3), mid(y2, h));
            vec![
                Move(0.0, 0.0),
                Cubic(x1 / 2.0, 0.0, x1, ya / 2.0, x1, ya),
                Cubic(x1, mid(ya, y2), mid(x1, xb), y2, xb, y2),
                Cubic(mid(xb, x3), y2, x3, mid(y2, yc), x3, yc),
                Cubic(x3, mid(yc, h), mid(x3, w), h, w, h),
            ]
        }
    })
}

/// The direction (unit vector) the path `segs` arrives at its end (`at_end`), or the one pointing
/// back out of its start: where an arrowhead there points. `None` for a path with no length.
pub fn path_direction(segs: &[PathSeg], at_end: bool) -> Option<(f32, f32)> {
    let mut pts: Vec<(f32, f32)> = Vec::with_capacity(segs.len().saturating_mul(3).min(64));
    for s in segs.iter().take(16) {
        match *s {
            PathSeg::Move(x, y) | PathSeg::Line(x, y) => pts.push((x, y)),
            PathSeg::Cubic(a, b, c, d, e, f) => pts.extend([(a, b), (c, d), (e, f)]),
            PathSeg::Close => {}
        }
    }
    if at_end {
        pts.reverse();
    }
    // From the nearest point that isn't the tip itself, to the tip.
    let tip = *pts.first()?;
    pts.iter().skip(1).find_map(|p| {
        let (dx, dy) = (tip.0 - p.0, tip.1 - p.1);
        let len = dx.hypot(dy);
        (len.is_finite() && len > 1e-3).then(|| (dx / len, dy / len))
    })
}

/// One end of a connector to route: where it is on the page, and, for an end glued to a shape,
/// the way out of that shape there (a unit vector along the page's x or y axis).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RouteEnd {
    pub at: (f32, f32),
    pub out: Option<(f32, f32)>,
}

impl RouteEnd {
    /// An end glued to nothing.
    pub fn free(at: (f32, f32)) -> RouteEnd {
        RouteEnd { at, out: None }
    }
}

/// What a connector was before it is routed again: its preset and frame size (so a run the user
/// placed stays where it was put), and whether it left along the page's y axis.
#[derive(Clone, Copy, Debug, Default)]
pub struct Was<'a> {
    pub preset: Option<&'a PresetGeom>,
    pub w: f32,
    pub h: f32,
    pub upright: bool,
}

/// A routed connector: its frame, the frame's turn (degrees: 0, or 90 for one that leaves along
/// the page's y axis) and the preset to draw it as (`None`: its kind's own).
#[derive(Clone, Debug, PartialEq)]
pub struct Routed {
    pub frame: Frame,
    pub rot: f32,
    pub preset: Option<PresetGeom>,
}

/// The axis direction nearest to `v`: a unit vector along x or y. `None` for no direction.
pub fn axis_direction(v: (f32, f32)) -> Option<(f32, f32)> {
    let (x, y) = v;
    if !x.is_finite() || !y.is_finite() || (x.abs() < 1e-3 && y.abs() < 1e-3) {
        return None;
    }
    Some(if x.abs() >= y.abs() { (x.signum(), 0.0) } else { (0.0, y.signum()) })
}

/// Where a run lies along one side of the frame.
#[derive(Clone, Copy, PartialEq)]
enum Zone {
    /// Before the start: the connector first goes away from its end.
    Before,
    /// Between the ends.
    Between,
    /// Past the end: the connector comes back to it.
    Past,
}

/// The adjust value putting a run in `zone` of a side `side` points long. A run the connector
/// already had there (`old` share of a side `old_side` long, when it was the same preset) stays
/// where it was: at the same share between the ends, or as far outside them.
fn adjust(zone: Zone, side: f32, old: Option<(f32, f32)>) -> i32 {
    let side = side.max(MIN_SIDE);
    let reach = |r: f32| if r.is_finite() && r >= 1.0 { r.min(MAX_REACH) } else { STUB };
    let v = match zone {
        Zone::Between => old.map(|(s, _)| s).filter(|s| *s > 0.0 && *s < 1.0).unwrap_or(0.5),
        Zone::Before => -reach(old.filter(|(s, _)| *s < 0.0).map_or(STUB, |(s, l)| -s * l)) / side,
        Zone::Past => 1.0 + reach(old.filter(|(s, _)| *s > 1.0).map_or(STUB, |(s, l)| (s - 1.0) * l)) / side,
    };
    (v * FULL).clamp(-MAX_ADJUST, MAX_ADJUST).round() as i32
}

/// Route a connector `kind` from `start` to `end`.
///
/// A straight connector is the line between them. A bent or curved one leaves a shape it is
/// glued to straight away from it and arrives at the other the same way: it leaves along the
/// start's way out (a free start: along the end's; both free: as `was` did), going around when
/// an end faces away from the other, and its last run is along the end's way out.
pub fn route_connector(kind: ShapeKind, start: RouteEnd, end: RouteEnd, was: Was) -> Routed {
    let fin = wordcraft_geom::finite;
    let (s, e) = ((fin(start.at.0), fin(start.at.1)), (fin(end.at.0), fin(end.at.1)));
    let stem = match kind {
        ShapeKind::ElbowConnector => "bentConnector",
        ShapeKind::CurvedConnector => "curvedConnector",
        _ => return Routed { frame: route(s, e), rot: 0.0, preset: was.preset.cloned() },
    };
    let (out_s, out_e) = (start.out.and_then(axis_direction), end.out.and_then(axis_direction));
    let upright = match (out_s, out_e) {
        (Some(d), _) | (None, Some(d)) => d.0 == 0.0,
        (None, None) => was.upright,
    };
    // Into the frame's own unflipped axes: a quarter-turned frame's x axis is the page's y.
    let local = |(x, y): (f32, f32)| if upright { (y, -x) } else { (x, y) };
    let (mut dx, mut dy) = local((e.0 - s.0, e.1 - s.1));
    let (flip_h, flip_v) = (dx < 0.0, dy < 0.0);
    // A way out in the frame's flipped axes, where the path runs from (0, 0) to (w, h).
    let own = |d: Option<(f32, f32)>| d.map(local).map(|(x, y)| (if flip_h { -x } else { x }, if flip_v { -y } else { y }));
    let (own_s, own_e) = (own(out_s), own(out_e));
    let turns = matches!((own_s, own_e), (Some(_), Some(b)) if b.0 == 0.0);
    // The start faces away from the end; the end faces away from the start.
    let back = own_s.is_some_and(|d| d.0 < 0.0);
    let (n, zones): (u8, [Option<Zone>; 3]) = if turns {
        let over = own_e.is_some_and(|d| d.1 > 0.0);
        if !back && !over {
            (2, [None; 3])
        } else {
            (4, [Some(if back { Zone::Before } else { Zone::Between }), Some(if over { Zone::Past } else { Zone::Between }), None])
        }
    } else {
        match (back, own_e.is_some_and(|d| d.0 > 0.0)) {
            (false, false) => (3, [Some(Zone::Between), None, None]),
            (true, false) => (3, [Some(Zone::Before), None, None]),
            (false, true) => (3, [Some(Zone::Past), None, None]),
            (true, true) => (5, [Some(Zone::Before), Some(Zone::Between), Some(Zone::Past)]),
        }
    };
    // A run outside the frame is a share of the frame's side: give a side of no length one.
    let outside = |z: &Option<Zone>| matches!(z, Some(Zone::Before | Zone::Past));
    let widen = |d: f32, flipped: bool| if d.abs() < MIN_SIDE { if flipped { -MIN_SIDE } else { MIN_SIDE } } else { d };
    if outside(&zones[0]) || outside(&zones[2]) {
        dx = widen(dx, flip_h);
    }
    if outside(&zones[1]) {
        dy = widen(dy, flip_v);
    }
    let (w, h) = (dx.abs(), dy.abs());
    let name = format!("{stem}{n}");
    let same = of_family(kind, was.preset).filter(|p| p.name == name);
    let adjust: Vec<(String, i32)> = zones
        .iter()
        .zip([(1u8, w, was.w), (2, h, was.h), (3, w, was.w)])
        .filter_map(|(z, (i, side, old_side))| {
            let old = same.map(|p| (share(Some(p), i), fin(old_side).max(MIN_SIDE)));
            z.map(|z| (format!("adj{i}"), adjust(z, side, old)))
        })
        .collect();
    let plain = n == 3 && adjust.first().is_none_or(|(_, v)| *v == 50_000);
    let preset = if plain { None } else { PresetGeom::new(&name, adjust) };
    // The frame is centred between its ends, whatever its turn.
    let (ex, ey) = if upright { (-dy, dx) } else { (dx, dy) };
    let (cx, cy) = (s.0 + ex / 2.0, s.1 + ey / 2.0);
    Routed { frame: Frame { x: cx - w / 2.0, y: cy - h / 2.0, w, h, flip_h, flip_v }, rot: if upright { 90.0 } else { 0.0 }, preset }
}

/// What a connector goes around: the bounds (x0, y0, x1, y1, page points) of the shapes its
/// start and end are glued to, and of the other drawings on its page.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Around {
    pub start: Option<[f32; 4]>,
    pub end: Option<[f32; 4]>,
    pub others: Vec<[f32; 4]>,
}

/// Most drawings a route is checked against, and most places tried for each run.
const MAX_AROUND: usize = 16;
const MAX_TRIES: usize = 10;
/// How far (points) inside a drawing's bounds a run may graze it.
const GRAZE: f32 = 1.0;
/// Shortest first and last run (points) of a route found around drawings.
const LEAST_RUN: f32 = 6.0;

/// The corners of a bent connector of `n` runs in a `w` × `h` frame, its runs in between at
/// `x1`, `y2` and `x3`.
fn corners(n: u8, (x1, y2, x3): (f32, f32, f32), w: f32, h: f32) -> Vec<(f32, f32)> {
    match n {
        2 => vec![(0.0, 0.0), (w, 0.0), (w, h)],
        3 => vec![(0.0, 0.0), (x1, 0.0), (x1, h), (w, h)],
        4 => vec![(0.0, 0.0), (x1, 0.0), (x1, y2), (w, y2), (w, h)],
        _ => vec![(0.0, 0.0), (x1, 0.0), (x1, y2), (x3, y2), (x3, h), (w, h)],
    }
}

/// [`route_connector`], going around the drawings of `around` when the route it gives would
/// cross one: the shortest way of two to five runs that leaves and arrives the same way and
/// crosses none. When there is no such way, the route is the one that crosses.
pub fn route_around(kind: ShapeKind, start: RouteEnd, end: RouteEnd, was: Was, around: &Around) -> Routed {
    let base = route_connector(kind, start, end, was);
    let stem = match kind {
        ShapeKind::ElbowConnector => "bentConnector",
        ShapeKind::CurvedConnector => "curvedConnector",
        _ => return base,
    };
    if around.start.is_none() && around.end.is_none() && around.others.is_empty() {
        return base;
    }
    let fin = wordcraft_geom::finite;
    let s = (fin(start.at.0), fin(start.at.1));
    let (f, upright) = (base.frame, base.rot != 0.0);
    let (w, h) = (f.w, f.h);
    // Into the frame's own axes, from its start: where the path runs from (0, 0) to (w, h).
    let own = |(x, y): (f32, f32)| {
        let (x, y) = if upright { (y, -x) } else { (x, y) };
        (if f.flip_h { -x } else { x }, if f.flip_v { -y } else { y })
    };
    let boxed = |b: &[f32; 4]| -> Option<[f32; 4]> {
        let b = b.map(fin);
        let (p, q) = (own((b[0] - s.0, b[1] - s.1)), own((b[2] - s.0, b[3] - s.1)));
        let r = [p.0.min(q.0) + GRAZE, p.1.min(q.1) + GRAZE, p.0.max(q.0) - GRAZE, p.1.max(q.1) - GRAZE];
        (r[2] > r[0] && r[3] > r[1]).then_some(r)
    };
    let (box_s, box_e) = (around.start.as_ref().and_then(boxed), around.end.as_ref().and_then(boxed));
    // The drawings near the way between the ends.
    let reach = 6.0 * STUB;
    let near = |r: &[f32; 4]| r[2] > -reach && r[0] < w + reach && r[3] > -reach && r[1] < h + reach;
    let others: Vec<[f32; 4]> = around.others.iter().filter_map(boxed).filter(near).take(MAX_AROUND).collect();
    let crosses = |pts: &[(f32, f32)]| {
        let last = pts.len().saturating_sub(2);
        pts.windows(2).enumerate().any(|(i, seg)| {
            let (Some(a), Some(b)) = (seg.first(), seg.get(1)) else { return false };
            let hit = |r: &[f32; 4]| a.0.min(b.0) < r[2] && a.0.max(b.0) > r[0] && a.1.min(b.1) < r[3] && a.1.max(b.1) > r[1];
            // The first run leaves the start's shape and the last arrives at the end's: those
            // two touch their own shape and are not held against it.
            others.iter().any(hit) || (i != 0 && box_s.as_ref().is_some_and(hit)) || (i != last && box_e.as_ref().is_some_and(hit))
        })
    };
    let (_, n0) = family(kind, base.preset.as_ref()).unwrap_or((false, 3));
    let p0 = of_family(kind, base.preset.as_ref());
    if !crosses(&corners(n0, (w * share(p0, 1), h * share(p0, 2), w * share(p0, 3)), w, h)) {
        return base;
    }
    let (own_s, own_e) = (start.out.and_then(axis_direction).map(own), end.out.and_then(axis_direction).map(own));
    let turns = matches!((own_s, own_e), (Some(_), Some(b)) if b.0 == 0.0);
    // Places to try for a run: the middle, a stub out of each end, and just clear of each
    // drawing's sides; the nearest to the middle first.
    let tries = |mid: f32, ends: [Option<f32>; 2], sides: &dyn Fn(&[f32; 4]) -> (f32, f32)| {
        let mut at: Vec<f32> = vec![mid];
        at.extend(ends.into_iter().flatten());
        for r in box_s.iter().chain(box_e.iter()).chain(others.iter()) {
            let (lo, hi) = sides(r);
            at.extend([lo - STUB, hi + STUB]);
        }
        at.retain(|v| v.is_finite());
        at.sort_by(|a, b| (a - mid).abs().total_cmp(&(b - mid).abs()));
        at.dedup_by(|a, b| (*a - *b).abs() < 0.5);
        at.truncate(MAX_TRIES);
        at
    };
    let xs =
        tries(w / 2.0, [own_s.filter(|d| d.0 != 0.0).map(|d| d.0 * STUB), own_e.filter(|d| d.0 != 0.0).map(|d| w + d.0 * STUB)], &|r| (r[0], r[2]));
    let ys = tries(h / 2.0, [None, own_e.filter(|d| d.1 != 0.0).map(|d| h + d.1 * STUB)], &|r| (r[1], r[3]));
    // The first run leaves along the start's way out, the last arrives against the end's.
    let leaves = |x1: f32| own_s.is_none_or(|d| if d.0 > 0.0 { x1 >= LEAST_RUN } else { x1 <= -LEAST_RUN });
    let arrives = |last: f32, side: f32, out: f32| if out < 0.0 { last <= side - LEAST_RUN } else { last >= side + LEAST_RUN };
    let mut best: Option<(f32, u8, (f32, f32, f32))> = None;
    let mut offer = |n: u8, at: (f32, f32, f32)| {
        let pts = corners(n, at, w, h);
        if crosses(&pts) {
            return;
        }
        // Its length, and a little for each run: of two ways as long, the one with fewer corners.
        let run = |p: &[(f32, f32)]| match p {
            [a, b] => (b.0 - a.0).abs() + (b.1 - a.1).abs(),
            _ => 0.0,
        };
        let len: f32 = pts.windows(2).map(run).sum::<f32>() + 8.0 * f32::from(n);
        if len.is_finite() && best.is_none_or(|(l, ..)| len < l - 0.01) {
            best = Some((len, n, at));
        }
    };
    if turns {
        let out = own_e.map_or(-1.0, |d| d.1);
        if leaves(w) && arrives(0.0, h, out) {
            offer(2, (0.0, 0.0, 0.0));
        }
        for &x1 in xs.iter().filter(|x| leaves(**x)) {
            for &y2 in ys.iter().filter(|y| arrives(**y, h, out)) {
                offer(4, (x1, y2, 0.0));
            }
        }
    } else {
        let ok_last = |x: f32| own_e.is_none_or(|d| arrives(x, w, d.0));
        for &x1 in xs.iter().filter(|x| leaves(**x)) {
            if ok_last(x1) {
                offer(3, (x1, 0.0, 0.0));
            }
            for &y2 in &ys {
                for &x3 in xs.iter().filter(|x| ok_last(**x)) {
                    offer(5, (x1, y2, x3));
                }
            }
        }
    }
    let Some((_, n, (x1, y2, x3))) = best else { return base };
    // As adjust values: shares of the frame's sides, a side of no length given one.
    let (wide, high) = (n >= 3 && w < MIN_SIDE, n >= 4 && h < MIN_SIDE);
    let (w2, h2) = (if wide { MIN_SIDE } else { w }, if high { MIN_SIDE } else { h });
    let value = |at: f32, side: f32| (at / side * FULL).clamp(-MAX_ADJUST, MAX_ADJUST).round() as i32;
    let all = [("adj1", value(x1, w2)), ("adj2", value(y2, h2)), ("adj3", value(x3, w2))];
    let adjust: Vec<(String, i32)> = all.into_iter().take(usize::from(n.saturating_sub(2))).map(|(name, v)| (name.to_string(), v)).collect();
    let plain = n == 3 && adjust.first().is_none_or(|(_, v)| *v == 50_000);
    let preset = if plain { None } else { PresetGeom::new(&format!("{stem}{n}"), adjust) };
    // The frame keeps its start; a side that was given a length moves the far corner by it.
    let (dx, dy) = (if f.flip_h { -w2 } else { w2 }, if f.flip_v { -h2 } else { h2 });
    let (ex, ey) = if upright { (-dy, dx) } else { (dx, dy) };
    let (cx, cy) = (s.0 + ex / 2.0, s.1 + ey / 2.0);
    Routed { frame: Frame { x: cx - w2 / 2.0, y: cy - h2 / 2.0, w: w2, h: h2, ..f }, rot: base.rot, preset }
}

/// A handle on a bent or curved connector that moves one of its runs: which adjust value
/// (1–3), where it is in the frame (the middle of the run), and whether it moves along the
/// frame's x axis (else y).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BendHandle {
    pub adjust: u8,
    pub at: (f32, f32),
    pub along_x: bool,
}

/// The handles of a connector `kind` drawn as `preset` in a `w` × `h` frame: one for each run
/// between its first and its last. None for straight connectors and two-run ones.
pub fn bend_handles(kind: ShapeKind, preset: Option<&PresetGeom>, w: f32, h: f32) -> Vec<BendHandle> {
    let preset = of_family(kind, preset);
    let Some((_, n)) = family(kind, preset) else { return Vec::new() };
    let (w, h) = (wordcraft_geom::finite(w), wordcraft_geom::finite(h));
    let (x1, y2, x3) = (w * share(preset, 1), h * share(preset, 2), w * share(preset, 3));
    let at = |adjust: u8, x: f32, y: f32, along_x: bool| BendHandle { adjust, at: (wordcraft_geom::finite(x), wordcraft_geom::finite(y)), along_x };
    match n {
        3 => vec![at(1, x1, h / 2.0, true)],
        4 => vec![at(1, x1, y2 / 2.0, true), at(2, (x1 + w) / 2.0, y2, false)],
        5 => vec![at(1, x1, y2 / 2.0, true), at(2, (x1 + x3) / 2.0, y2, false), at(3, x3, (y2 + h) / 2.0, true)],
        _ => Vec::new(),
    }
}

/// The preset of a connector `kind` (now drawn as `preset` in a `w` × `h` frame) with its run
/// `adjust` (1–3) moved to the frame point `to`: `Some(None)` when that is the kind's own preset
/// again, `None` when the connector has no such run.
pub fn bend_to(kind: ShapeKind, preset: Option<&PresetGeom>, w: f32, h: f32, adjust: u8, to: (f32, f32)) -> Option<Option<PresetGeom>> {
    let preset = of_family(kind, preset);
    let handle = bend_handles(kind, preset, w, h).into_iter().find(|b| b.adjust == adjust)?;
    let (curved, n) = family(kind, preset)?;
    let (at, side) = if handle.along_x { (to.0, w) } else { (to.1, h) };
    let side = wordcraft_geom::finite(side).max(MIN_SIDE);
    let value = (wordcraft_geom::finite(at) / side * FULL).clamp(-MAX_ADJUST, MAX_ADJUST).round() as i32;
    let values: Vec<(String, i32)> =
        (1..n.saturating_sub(1)).map(|i| (format!("adj{i}"), if i == adjust { value } else { (share(preset, i) * FULL).round() as i32 })).collect();
    if n == 3 && value == 50_000 {
        return Some(None);
    }
    Some(PresetGeom::new(&format!("{}{n}", if curved { "curvedConnector" } else { "bentConnector" }), values))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connector::ends_spun;
    use wordcraft_geom::Spin;

    const UP: (f32, f32) = (0.0, -1.0);
    const DOWN: (f32, f32) = (0.0, 1.0);
    const LEFT: (f32, f32) = (-1.0, 0.0);
    const RIGHT: (f32, f32) = (1.0, 0.0);

    fn glued(at: (f32, f32), out: (f32, f32)) -> RouteEnd {
        RouteEnd { at, out: Some(out) }
    }

    /// The routed connector's corner points on the page.
    fn on_page(kind: ShapeKind, r: &Routed) -> Vec<(f32, f32)> {
        let f = r.frame;
        let spin = Spin::new(r.rot, f.flip_h, f.flip_v);
        let (cx, cy) = (f.x + f.w / 2.0, f.y + f.h / 2.0);
        let mut pts = Vec::new();
        for s in connector_path(kind, r.preset.as_ref(), f.w, f.h).unwrap() {
            match s {
                PathSeg::Move(x, y) | PathSeg::Line(x, y) => pts.push(spin.apply(cx, cy, f.x + x, f.y + y)),
                PathSeg::Cubic(a, b, c, d, e, g) => {
                    pts.extend([spin.apply(cx, cy, f.x + a, f.y + b), spin.apply(cx, cy, f.x + c, f.y + d), spin.apply(cx, cy, f.x + e, f.y + g)])
                }
                PathSeg::Close => {}
            }
        }
        pts
    }

    fn near(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 0.05 && (a.1 - b.1).abs() < 0.05
    }

    fn unit(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy);
        ((dx / len).round(), (dy / len).round())
    }

    #[test]
    fn every_route_leaves_and_arrives_away_from_the_glued_sides() {
        let spots =
            [(0.0, 0.0), (120.0, 80.0), (-120.0, 80.0), (120.0, -80.0), (-120.0, -80.0), (0.0, 80.0), (120.0, 0.0), (0.0, -80.0), (-120.0, 0.0)];
        for kind in [ShapeKind::ElbowConnector, ShapeKind::CurvedConnector] {
            for out_s in [UP, DOWN, LEFT, RIGHT] {
                for out_e in [UP, DOWN, LEFT, RIGHT] {
                    for to in spots {
                        let (s, e) = ((300.0, 300.0), (300.0 + to.0, 300.0 + to.1));
                        let r = route_connector(kind, glued(s, out_s), glued(e, out_e), Was::default());
                        let pts = on_page(kind, &r);
                        let what = format!("{kind:?} {out_s:?} -> {out_e:?} by {to:?}: {r:?} {pts:?}");
                        assert!(near(pts[0], s), "starts at its start: {what}");
                        assert!(near(*pts.last().unwrap(), e), "ends at its end: {what}");
                        let first = pts.iter().find(|p| !near(**p, pts[0])).copied();
                        let last = pts.iter().rev().find(|p| !near(**p, e)).copied();
                        // An end straight above, below or beside the other can leave a run of
                        // no length: only where the ends are apart both ways is each run there.
                        if let (Some(first), Some(last), true) = (first, last, to.0 != 0.0 && to.1 != 0.0) {
                            assert_eq!(unit(pts[0], first), out_s, "leaves away from its shape: {what}");
                            assert_eq!(unit(e, last), out_e, "arrives from outside its shape: {what}");
                        }
                        let f = r.frame;
                        let (a, b) = ends_spun([f.x, f.y, f.w, f.h], Spin::new(r.rot, f.flip_h, f.flip_v));
                        assert!(near(a, s) && near(b, e), "the frame's corners are its ends: {what}");
                    }
                }
            }
        }
    }

    #[test]
    fn facing_shapes_get_the_plain_elbow_and_stacked_ones_an_upright_one() {
        // Side by side, right side to left side: the kind's own preset, unturned.
        let r = route_connector(ShapeKind::ElbowConnector, glued((100.0, 100.0), RIGHT), glued((200.0, 160.0), LEFT), Was::default());
        assert_eq!((r.rot, r.preset.clone()), (0.0, None));
        assert_eq!(on_page(ShapeKind::ElbowConnector, &r), vec![(100.0, 100.0), (150.0, 100.0), (150.0, 160.0), (200.0, 160.0)]);
        // One above the other, bottom to top: down, across, down. It used to leave sideways.
        let r = route_connector(ShapeKind::ElbowConnector, glued((100.0, 100.0), DOWN), glued((160.0, 200.0), UP), Was::default());
        assert_eq!((r.rot, r.preset.clone()), (90.0, None));
        let pts = on_page(ShapeKind::ElbowConnector, &r);
        for (p, want) in pts.iter().zip([(100.0, 100.0), (100.0, 150.0), (160.0, 150.0), (160.0, 200.0)]) {
            assert!(near(*p, want), "{pts:?}");
        }
        // A side to a top: one corner.
        let r = route_connector(ShapeKind::ElbowConnector, glued((100.0, 100.0), RIGHT), glued((200.0, 160.0), UP), Was::default());
        assert_eq!(r.preset.as_ref().map(|p| p.name.as_str()), Some("bentConnector2"));
        assert_eq!(on_page(ShapeKind::ElbowConnector, &r), vec![(100.0, 100.0), (200.0, 100.0), (200.0, 160.0)]);
    }

    #[test]
    fn a_loop_between_the_same_sides_of_stacked_shapes_goes_out_and_back() {
        // Right side to right side, straight below: out by the stub, down, back in.
        let r = route_connector(ShapeKind::ElbowConnector, glued((200.0, 100.0), RIGHT), glued((200.0, 300.0), RIGHT), Was::default());
        let pts = on_page(ShapeKind::ElbowConnector, &r);
        assert_eq!(pts.len(), 4);
        assert!((pts[1].0 - (200.0 + STUB)).abs() < 0.1 && (pts[2].0 - (200.0 + STUB)).abs() < 0.1, "{pts:?}");
        assert!(near(pts[3], (200.0, 300.0)), "{pts:?}");
        assert!(r.frame.w >= MIN_SIDE && r.frame.w < 0.02, "a frame of no width still carries its adjust value: {:?}", r.frame);
    }

    #[test]
    fn a_run_the_user_placed_stays_when_the_connector_is_routed_again() {
        let kind = ShapeKind::ElbowConnector;
        let moved = PresetGeom::new("bentConnector3", [("adj1".to_string(), 25_000)]).unwrap();
        let was = Was { preset: Some(&moved), w: 100.0, h: 60.0, upright: false };
        let r = route_connector(kind, glued((100.0, 100.0), RIGHT), glued((300.0, 160.0), LEFT), was);
        assert_eq!(r.preset, Some(moved.clone()), "a quarter of the way stays a quarter of the way");
        // A run outside the frame stays as far outside when the frame changes size.
        let out = PresetGeom::new("bentConnector3", [("adj1".to_string(), 140_000)]).unwrap();
        let was = Was { preset: Some(&out), w: 100.0, h: 60.0, upright: false };
        let r = route_connector(kind, glued((100.0, 100.0), RIGHT), glued((300.0, 160.0), RIGHT), was);
        assert_eq!(on_page(kind, &r)[1], (340.0, 100.0), "40 pt past the end, as before");
        // One that no longer fits (the end now faces the start) goes back to the middle.
        let r = route_connector(kind, glued((100.0, 100.0), RIGHT), glued((300.0, 160.0), LEFT), was);
        assert_eq!(r.preset, None);
    }

    /// A route that would cross a drawing goes around it; one that crosses none is left as it is.
    #[test]
    fn a_route_goes_around_what_is_in_its_way() {
        let kind = ShapeKind::ElbowConnector;
        // Two boxes side by side, right side to left side, and a third box between them.
        let (a, b) = ([100.0, 100.0, 200.0, 160.0], [400.0, 100.0, 500.0, 160.0]);
        let (s, e) = (glued((200.0, 130.0), RIGHT), glued((400.0, 130.0), LEFT));
        let free = Around { start: Some(a), end: Some(b), others: vec![[260.0, 300.0, 340.0, 360.0]] };
        assert_eq!(route_around(kind, s, e, Was::default(), &free), route_connector(kind, s, e, Was::default()), "nothing in the way");
        let blocked = Around { start: Some(a), end: Some(b), others: vec![[260.0, 90.0, 340.0, 170.0]] };
        let r = route_around(kind, s, e, Was::default(), &blocked);
        let pts = on_page(kind, &r);
        assert!(near(pts[0], s.at) && near(*pts.last().unwrap(), e.at), "{pts:?}");
        assert_eq!((unit(pts[0], pts[1]), unit(e.at, pts[pts.len() - 2])), (RIGHT, LEFT), "it still leaves and arrives straight: {pts:?}");
        let clear = |p: &[(f32, f32)], r: [f32; 4]| {
            p.windows(2).all(|w| {
                !(w[0].0.min(w[1].0) < r[2] - 1.0
                    && w[0].0.max(w[1].0) > r[0] + 1.0
                    && w[0].1.min(w[1].1) < r[3] - 1.0
                    && w[0].1.max(w[1].1) > r[1] + 1.0)
            })
        };
        assert!(clear(&pts, [260.0, 90.0, 340.0, 170.0]), "around the box in between: {pts:?}");
        assert_eq!(pts.len(), 6, "over or under it: {pts:?}");
        // From a shape's left side to a shape on its right: around its own shape, not through it.
        let (s, e) = (glued((100.0, 130.0), LEFT), glued((400.0, 130.0), LEFT));
        let own = Around { start: Some(a), end: Some(b), others: Vec::new() };
        let through = on_page(kind, &route_connector(kind, s, e, Was::default()));
        assert!(!clear(&through[1..], a), "the plain route runs back through its own shape: {through:?}");
        let pts = on_page(kind, &route_around(kind, s, e, Was::default(), &own));
        assert!(clear(&pts[1..], a) && near(*pts.last().unwrap(), e.at), "{pts:?}");
        assert_eq!((unit(pts[0], pts[1]), unit(e.at, pts[pts.len() - 2])), (LEFT, LEFT), "{pts:?}");
        // Walled in: no way round is found, and the route is the plain one.
        let walled = Around {
            start: Some(a),
            end: Some(b),
            others: vec![[-1000.0, -1000.0, 1000.0, 99.0], [-1000.0, 161.0, 1000.0, 1000.0], [205.0, -1000.0, 210.0, 1000.0]],
        };
        let (s, e) = (glued((200.0, 130.0), RIGHT), glued((400.0, 130.0), LEFT));
        assert_eq!(route_around(kind, s, e, Was::default(), &walled), route_connector(kind, s, e, Was::default()));
        // Hostile bounds and a straight connector are no trouble.
        let wild = Around { start: Some([f32::NAN; 4]), end: Some([f32::INFINITY, 0.0, f32::NEG_INFINITY, 5.0]), others: vec![[f32::MAX; 4]; 100] };
        let r = route_around(kind, s, e, Was::default(), &wild);
        assert!([r.frame.x, r.frame.y, r.frame.w, r.frame.h].into_iter().all(f32::is_finite));
        assert_eq!(route_around(ShapeKind::StraightConnector, s, e, Was::default(), &blocked).rot, 0.0);
    }

    #[test]
    fn free_ends_keep_the_connectors_own_direction() {
        let kind = ShapeKind::ElbowConnector;
        let r = route_connector(kind, RouteEnd::free((10.0, 10.0)), RouteEnd::free((110.0, 60.0)), Was::default());
        assert_eq!((r.rot, r.frame), (0.0, route((10.0, 10.0), (110.0, 60.0))));
        let r = route_connector(kind, RouteEnd::free((10.0, 10.0)), RouteEnd::free((110.0, 60.0)), Was { upright: true, ..Was::default() });
        assert_eq!(r.rot, 90.0);
        // A free start follows the glued end's side.
        let r = route_connector(kind, RouteEnd::free((10.0, 10.0)), glued((110.0, 60.0), UP), Was::default());
        let pts = on_page(kind, &r);
        assert_eq!(unit(*pts.last().unwrap(), pts[pts.len() - 2]), UP, "{pts:?}");
        // A straight connector is the line between its ends, its preset untouched.
        let r = route_connector(ShapeKind::StraightConnector, glued((10.0, 10.0), UP), glued((110.0, 60.0), UP), Was::default());
        assert_eq!((r.rot, r.frame, r.preset), (0.0, route((10.0, 10.0), (110.0, 60.0)), None));
    }

    #[test]
    fn each_preset_draws_its_own_runs() {
        let kind = ShapeKind::ElbowConnector;
        let p = |name: &str, adj: &[i32]| PresetGeom::new(name, adj.iter().enumerate().map(|(i, v)| (format!("adj{}", i + 1), *v))).unwrap();
        use PathSeg::{Line, Move};
        assert_eq!(connector_path(kind, None, 100.0, 40.0).unwrap(), vec![Move(0.0, 0.0), Line(50.0, 0.0), Line(50.0, 40.0), Line(100.0, 40.0)]);
        assert_eq!(
            connector_path(kind, Some(&p("bentConnector2", &[])), 100.0, 40.0).unwrap(),
            vec![Move(0.0, 0.0), Line(100.0, 0.0), Line(100.0, 40.0)]
        );
        assert_eq!(
            connector_path(kind, Some(&p("bentConnector4", &[-20_000, 150_000])), 100.0, 40.0).unwrap(),
            vec![Move(0.0, 0.0), Line(-20.0, 0.0), Line(-20.0, 60.0), Line(100.0, 60.0), Line(100.0, 40.0)]
        );
        assert_eq!(
            connector_path(kind, Some(&p("bentConnector5", &[-10_000, 50_000, 110_000])), 100.0, 40.0).unwrap(),
            vec![Move(0.0, 0.0), Line(-10.0, 0.0), Line(-10.0, 20.0), Line(110.0, 20.0), Line(110.0, 40.0), Line(100.0, 40.0)]
        );
        // A preset of another family, and hostile values, draw the kind's own shape, finitely.
        assert_eq!(connector_path(kind, Some(&p("hexagon", &[1])), 100.0, 40.0), connector_path(kind, None, 100.0, 40.0));
        let wild =
            connector_path(ShapeKind::CurvedConnector, Some(&p("curvedConnector5", &[i32::MAX, i32::MIN, i32::MAX])), f32::MAX, f32::NAN).unwrap();
        for s in wild {
            if let PathSeg::Cubic(a, b, c, d, e, f) = s {
                assert!([a, b, c, d, e, f].into_iter().all(f32::is_finite));
            }
        }
        assert_eq!(connector_path(ShapeKind::Rectangle, None, 1.0, 1.0), None);
        // Every curved preset ends at the far corner and passes the bent one's handles.
        for n in 2..=5 {
            let geom = p(&format!("curvedConnector{n}"), &[30_000, 60_000, 80_000]);
            let segs = connector_path(ShapeKind::CurvedConnector, Some(&geom), 100.0, 40.0).unwrap();
            assert_eq!(segs.len() as i32, n);
            assert!(matches!(segs.last(), Some(PathSeg::Cubic(.., x, y)) if (*x, *y) == (100.0, 40.0)));
            for hnd in bend_handles(ShapeKind::CurvedConnector, Some(&geom), 100.0, 40.0) {
                assert!(segs.iter().any(|s| matches!(s, PathSeg::Cubic(.., x, y) if near((*x, *y), hnd.at))), "{n}: {hnd:?} {segs:?}");
            }
        }
    }

    #[test]
    fn arrowheads_follow_the_last_run() {
        let kind = ShapeKind::ElbowConnector;
        let segs = connector_path(kind, None, 100.0, 40.0).unwrap();
        assert_eq!(path_direction(&segs, true), Some((1.0, 0.0)));
        assert_eq!(path_direction(&segs, false), Some((-1.0, 0.0)));
        let one = PresetGeom::new("bentConnector2", []).unwrap();
        let segs = connector_path(kind, Some(&one), 100.0, 40.0).unwrap();
        assert_eq!(path_direction(&segs, true), Some((0.0, 1.0)), "a one-corner elbow arrives downwards");
        // Coming back to its end from past it, the head points back.
        let past = PresetGeom::new("bentConnector3", [("adj1".to_string(), 120_000)]).unwrap();
        let segs = connector_path(kind, Some(&past), 100.0, 40.0).unwrap();
        assert_eq!(path_direction(&segs, true), Some((-1.0, 0.0)));
        assert_eq!(path_direction(&[PathSeg::Move(1.0, 1.0), PathSeg::Line(1.0, 1.0)], true), None);
    }

    #[test]
    fn a_handle_moves_its_run_and_only_that() {
        let kind = ShapeKind::ElbowConnector;
        assert_eq!(bend_handles(kind, None, 100.0, 40.0), vec![BendHandle { adjust: 1, at: (50.0, 20.0), along_x: true }]);
        assert_eq!(bend_to(kind, None, 100.0, 40.0, 1, (80.0, 5.0)), Some(PresetGeom::new("bentConnector3", [("adj1".to_string(), 80_000)])));
        assert_eq!(bend_to(kind, None, 100.0, 40.0, 1, (50.0, 0.0)), Some(None), "back in the middle: the plain elbow again");
        assert_eq!(bend_to(kind, None, 100.0, 40.0, 2, (50.0, 0.0)), None, "a three-run elbow has one handle");
        let four = PresetGeom::new("bentConnector4", [("adj1".to_string(), -20_000), ("adj2".to_string(), 150_000)]).unwrap();
        let hs = bend_handles(kind, Some(&four), 100.0, 40.0);
        assert_eq!(hs, vec![BendHandle { adjust: 1, at: (-20.0, 30.0), along_x: true }, BendHandle { adjust: 2, at: (40.0, 60.0), along_x: false }]);
        let moved = bend_to(kind, Some(&four), 100.0, 40.0, 2, (0.0, 80.0)).flatten().unwrap();
        assert_eq!(moved.adjust, vec![("adj1".to_string(), -20_000), ("adj2".to_string(), 200_000)]);
        assert!(bend_handles(ShapeKind::StraightConnector, None, 100.0, 40.0).is_empty());
        assert!(bend_handles(kind, Some(&PresetGeom::new("bentConnector2", []).unwrap()), 100.0, 40.0).is_empty());
    }
}
