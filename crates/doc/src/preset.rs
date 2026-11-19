//! The outlines of DrawingML preset shapes beyond the few a [`ShapeKind`] stands for.
//!
//! A shape keeps the preset geometry a file or the shape gallery gave it
//! ([`crate::connector::ShapeExtra::preset`]: the preset's name and its adjust values). For the
//! presets in [`LIBRARY`], and some more that only files bring, [`outline`] gives the path the
//! preset draws in a frame, worked out from the preset shape definitions of ECMA-376 Part 1
//! (§20.1.9.18 `prstGeom`, §20.1.10.56 `ST_ShapeType` and the standard's preset shape
//! definitions): the adjust values are 100000ths, mostly of the frame's shorter side. A preset
//! with no outline here is drawn as its shape's kind, as before.
//!
//! The same outline gives the room for the shape's text ([`text_insets`]), the polygon text wraps
//! around ([`unit_outline`]) and the places connectors glue to ([`side_sites`]).

use crate::connector::PresetGeom;
use crate::graphic::PathSeg;
use crate::para::ShapeKind;

/// A group of the shape gallery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeGroup {
    Lines,
    Basic,
    Rectangles,
    Flowchart,
    Arrows,
    Stars,
    Callouts,
}

impl ShapeGroup {
    pub const ALL: [ShapeGroup; 7] = [
        ShapeGroup::Lines,
        ShapeGroup::Rectangles,
        ShapeGroup::Basic,
        ShapeGroup::Arrows,
        ShapeGroup::Flowchart,
        ShapeGroup::Stars,
        ShapeGroup::Callouts,
    ];

    /// The group's heading in the gallery.
    pub fn label(self) -> &'static str {
        match self {
            ShapeGroup::Lines => "Lines",
            ShapeGroup::Basic => "Basic Shapes",
            ShapeGroup::Rectangles => "Rectangles",
            ShapeGroup::Flowchart => "Flowchart",
            ShapeGroup::Arrows => "Block Arrows",
            ShapeGroup::Stars => "Stars",
            ShapeGroup::Callouts => "Callouts",
        }
    }
}

/// A shape of the gallery: its name for `insert.shape` (a [`ShapeKind`]'s own, or a DrawingML
/// preset's), what it is called, and its group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GalleryShape {
    pub name: &'static str,
    pub label: &'static str,
    pub group: ShapeGroup,
}

const fn g(name: &'static str, label: &'static str, group: ShapeGroup) -> GalleryShape {
    GalleryShape { name, label, group }
}

/// The shapes of the gallery, in the order shown. Names in camel case with a lower first letter
/// that a [`ShapeKind`] has (`rectangle`, `ellipse`…) are kinds; the others are preset names.
pub const LIBRARY: &[GalleryShape] = &[
    g("line", "Line", ShapeGroup::Lines),
    g("rectangle", "Rectangle", ShapeGroup::Rectangles),
    g("roundedRectangle", "Rounded Rectangle", ShapeGroup::Rectangles),
    g("snip1Rect", "Snip Single Corner Rectangle", ShapeGroup::Rectangles),
    g("snip2SameRect", "Snip Same Side Corner Rectangle", ShapeGroup::Rectangles),
    g("snip2DiagRect", "Snip Diagonal Corner Rectangle", ShapeGroup::Rectangles),
    g("snipRoundRect", "Snip and Round Single Corner Rectangle", ShapeGroup::Rectangles),
    g("round1Rect", "Round Single Corner Rectangle", ShapeGroup::Rectangles),
    g("round2SameRect", "Round Same Side Corner Rectangle", ShapeGroup::Rectangles),
    g("round2DiagRect", "Round Diagonal Corner Rectangle", ShapeGroup::Rectangles),
    g("ellipse", "Oval", ShapeGroup::Basic),
    g("triangle", "Triangle", ShapeGroup::Basic),
    g("rightTriangle", "Right Triangle", ShapeGroup::Basic),
    g("parallelogram", "Parallelogram", ShapeGroup::Basic),
    g("trapezoid", "Trapezoid", ShapeGroup::Basic),
    g("diamond", "Diamond", ShapeGroup::Basic),
    g("pentagon", "Pentagon", ShapeGroup::Basic),
    g("hexagon", "Hexagon", ShapeGroup::Basic),
    g("heptagon", "Heptagon", ShapeGroup::Basic),
    g("octagon", "Octagon", ShapeGroup::Basic),
    g("decagon", "Decagon", ShapeGroup::Basic),
    g("dodecagon", "Dodecagon", ShapeGroup::Basic),
    g("plus", "Cross", ShapeGroup::Basic),
    g("frame", "Frame", ShapeGroup::Basic),
    g("donut", "Ring", ShapeGroup::Basic),
    g("corner", "L-Shape", ShapeGroup::Basic),
    g("cloud", "Cloud", ShapeGroup::Basic),
    g("bracketPair", "Double Bracket", ShapeGroup::Basic),
    g("bracePair", "Double Brace", ShapeGroup::Basic),
    g("leftBracket", "Left Bracket", ShapeGroup::Basic),
    g("rightBracket", "Right Bracket", ShapeGroup::Basic),
    g("leftBrace", "Left Brace", ShapeGroup::Basic),
    g("rightBrace", "Right Brace", ShapeGroup::Basic),
    g("diagStripe", "Diagonal Stripe", ShapeGroup::Basic),
    g("can", "Cylinder", ShapeGroup::Basic),
    g("cube", "Cube", ShapeGroup::Basic),
    g("foldedCorner", "Folded Corner", ShapeGroup::Basic),
    g("heart", "Heart", ShapeGroup::Basic),
    g("arrow", "Right Arrow", ShapeGroup::Arrows),
    g("arrowLeft", "Left Arrow", ShapeGroup::Arrows),
    g("arrowUp", "Up Arrow", ShapeGroup::Arrows),
    g("arrowDown", "Down Arrow", ShapeGroup::Arrows),
    g("leftRightArrow", "Left-Right Arrow", ShapeGroup::Arrows),
    g("upDownArrow", "Up-Down Arrow", ShapeGroup::Arrows),
    g("homePlate", "Pentagon Arrow", ShapeGroup::Arrows),
    g("chevron", "Chevron", ShapeGroup::Arrows),
    g("curvedRightArrow", "Curved Right Arrow", ShapeGroup::Arrows),
    g("curvedLeftArrow", "Curved Left Arrow", ShapeGroup::Arrows),
    g("curvedUpArrow", "Curved Up Arrow", ShapeGroup::Arrows),
    g("curvedDownArrow", "Curved Down Arrow", ShapeGroup::Arrows),
    g("flowChartProcess", "Process", ShapeGroup::Flowchart),
    g("flowChartAlternateProcess", "Alternate Process", ShapeGroup::Flowchart),
    g("flowChartDecision", "Decision", ShapeGroup::Flowchart),
    g("flowChartInputOutput", "Data", ShapeGroup::Flowchart),
    g("flowChartPredefinedProcess", "Predefined Process", ShapeGroup::Flowchart),
    g("flowChartInternalStorage", "Internal Storage", ShapeGroup::Flowchart),
    g("flowChartDocument", "Document", ShapeGroup::Flowchart),
    g("flowChartTerminator", "Terminator", ShapeGroup::Flowchart),
    g("flowChartPreparation", "Preparation", ShapeGroup::Flowchart),
    g("flowChartManualInput", "Manual Input", ShapeGroup::Flowchart),
    g("flowChartManualOperation", "Manual Operation", ShapeGroup::Flowchart),
    g("flowChartConnector", "Connector", ShapeGroup::Flowchart),
    g("flowChartOffpageConnector", "Off-page Connector", ShapeGroup::Flowchart),
    g("flowChartPunchedCard", "Card", ShapeGroup::Flowchart),
    g("flowChartSummingJunction", "Summing Junction", ShapeGroup::Flowchart),
    g("flowChartOr", "Or", ShapeGroup::Flowchart),
    g("flowChartCollate", "Collate", ShapeGroup::Flowchart),
    g("flowChartSort", "Sort", ShapeGroup::Flowchart),
    g("flowChartExtract", "Extract", ShapeGroup::Flowchart),
    g("flowChartMerge", "Merge", ShapeGroup::Flowchart),
    g("flowChartOnlineStorage", "Stored Data", ShapeGroup::Flowchart),
    g("flowChartDelay", "Delay", ShapeGroup::Flowchart),
    g("flowChartMagneticDisk", "Magnetic Disk", ShapeGroup::Flowchart),
    g("flowChartMagneticDrum", "Direct Access Storage", ShapeGroup::Flowchart),
    g("flowChartDisplay", "Display", ShapeGroup::Flowchart),
    g("star4", "4-Point Star", ShapeGroup::Stars),
    g("star", "5-Point Star", ShapeGroup::Stars),
    g("star6", "6-Point Star", ShapeGroup::Stars),
    g("star8", "8-Point Star", ShapeGroup::Stars),
    g("star12", "12-Point Star", ShapeGroup::Stars),
    g("star16", "16-Point Star", ShapeGroup::Stars),
    g("star24", "24-Point Star", ShapeGroup::Stars),
    g("star32", "32-Point Star", ShapeGroup::Stars),
    g("ribbon2", "Up Ribbon", ShapeGroup::Stars),
    g("ribbon", "Down Ribbon", ShapeGroup::Stars),
    g("wedgeRectCallout", "Rectangular Callout", ShapeGroup::Callouts),
    g("wedgeRoundRectCallout", "Rounded Rectangular Callout", ShapeGroup::Callouts),
    g("wedgeEllipseCallout", "Oval Callout", ShapeGroup::Callouts),
];

/// The gallery shape named `name`.
pub fn gallery_shape(name: &str) -> Option<&'static GalleryShape> {
    LIBRARY.iter().find(|s| s.name == name)
}

/// Largest frame side (points) an outline is worked out for.
const MAX_SIDE: f32 = 1.0e6;
/// A whole side, in adjust-value units.
const FULL: f32 = 100_000.0;

/// A path being drawn: where it is now, so arcs can start there.
struct Pen {
    segs: Vec<PathSeg>,
    at: (f32, f32),
}

impl Pen {
    fn new() -> Pen {
        Pen { segs: Vec::with_capacity(24), at: (0.0, 0.0) }
    }
    fn m(&mut self, x: f32, y: f32) -> &mut Pen {
        self.segs.push(PathSeg::Move(x, y));
        self.at = (x, y);
        self
    }
    fn l(&mut self, x: f32, y: f32) -> &mut Pen {
        self.segs.push(PathSeg::Line(x, y));
        self.at = (x, y);
        self
    }
    fn c(&mut self, c1: (f32, f32), c2: (f32, f32), to: (f32, f32)) -> &mut Pen {
        self.segs.push(PathSeg::Cubic(c1.0, c1.1, c2.0, c2.1, to.0, to.1));
        self.at = to;
        self
    }
    fn z(&mut self) -> &mut Pen {
        self.segs.push(PathSeg::Close);
        self
    }
    /// A closed polygon through `pts`.
    fn poly(&mut self, pts: &[(f32, f32)]) -> &mut Pen {
        for (i, &(x, y)) in pts.iter().enumerate() {
            if i == 0 {
                self.m(x, y);
            } else {
                self.l(x, y);
            }
        }
        self.z()
    }
    /// A line of its own from `a` to `b`: drawn, and enclosing nothing.
    fn line(&mut self, a: (f32, f32), b: (f32, f32)) -> &mut Pen {
        self.m(a.0, a.1).l(b.0, b.1)
    }
    /// An arc of an ellipse with radii `rx`, `ry` from where the pen is, which is the ellipse's
    /// point at angle `start` (degrees, clockwise from the x axis as seen on the page), on
    /// through `sweep` degrees (negative: anticlockwise). DrawingML's `arcTo`.
    fn arc(&mut self, rx: f32, ry: f32, start: f32, sweep: f32) -> &mut Pen {
        let (cx, cy) = (self.at.0 - rx * start.to_radians().cos(), self.at.1 - ry * start.to_radians().sin());
        let pieces = (sweep.abs() / 90.0).ceil().clamp(1.0, 8.0);
        let step = (sweep / pieces).to_radians();
        let k = 4.0 / 3.0 * (step / 4.0).tan();
        let mut a = start.to_radians();
        for _ in 0..pieces as usize {
            let b = a + step;
            let (p0, p3) = ((cx + rx * a.cos(), cy + ry * a.sin()), (cx + rx * b.cos(), cy + ry * b.sin()));
            let c1 = (p0.0 - k * rx * a.sin(), p0.1 + k * ry * a.cos());
            let c2 = (p3.0 + k * rx * b.sin(), p3.1 - k * ry * b.cos());
            self.c(c1, c2, p3);
            a = b;
        }
        self
    }
    /// [`Pen::arc`] with its angles as DrawingML gives them for an ellipse that isn't a circle:
    /// the directions of the arc's ends as seen from the ellipse's middle, not positions round
    /// a circle that was then stretched. The two agree at every quarter turn, and nowhere else.
    fn arc_seen(&mut self, rx: f32, ry: f32, start: f32, sweep: f32) -> &mut Pen {
        let round = |seen: f32| {
            if rx <= 0.0 || ry <= 0.0 {
                return seen;
            }
            let a = seen.to_radians();
            let turn = ((rx * a.sin()).atan2(ry * a.cos()) - a.sin().atan2(a.cos())).to_degrees();
            // The correction is never more than a quarter turn either way.
            seen + (turn + 180.0).rem_euclid(360.0) - 180.0
        };
        let from = round(start);
        self.arc(rx, ry, from, round(start + sweep) - from)
    }
    /// A whole ellipse in the box (`x`, `y`, `w`, `h`), clockwise or not.
    fn ellipse(&mut self, x: f32, y: f32, w: f32, h: f32, clockwise: bool) -> &mut Pen {
        self.m(x + w, y + h / 2.0).arc(w / 2.0, h / 2.0, 0.0, if clockwise { 360.0 } else { -360.0 }).z()
    }
}

/// A regular polygon of `n` corners, the first at `first` degrees (clockwise from the x axis),
/// stretched to fill the `w` × `h` frame: DrawingML's pentagon, heptagon, decagon and dodecagon.
fn regular(n: usize, first: f32, w: f32, h: f32) -> Vec<(f32, f32)> {
    let unit: Vec<(f32, f32)> = (0..n).map(|i| (first + 360.0 * i as f32 / n as f32).to_radians()).map(|a| (a.cos(), a.sin())).collect();
    let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for &(x, y) in &unit {
        (x0, x1, y0, y1) = (x0.min(x), x1.max(x), y0.min(y), y1.max(y));
    }
    let (dx, dy) = ((x1 - x0).max(1e-6), (y1 - y0).max(1e-6));
    unit.into_iter().map(|(x, y)| ((x - x0) / dx * w, (y - y0) / dy * h)).collect()
}

/// A star of `n` points in the `w` × `h` frame, its inner corners at `inner` of the way out.
fn star(n: usize, inner: f32, w: f32, h: f32) -> Vec<(f32, f32)> {
    let (cx, cy) = (w / 2.0, h / 2.0);
    (0..n * 2)
        .map(|i| {
            let a = (-90.0 + 180.0 * i as f32 / n as f32).to_radians();
            let r = if i % 2 == 0 { 1.0 } else { inner };
            (cx + cx * r * a.cos(), cy + cy * r * a.sin())
        })
        .collect()
}

/// A rectangle whose corners (top-left, top-right, bottom-right, bottom-left) are each cut
/// straight (`Some(false)`), rounded (`Some(true)`) or square, by the size given for each.
fn cornered(pen: &mut Pen, w: f32, h: f32, corners: [(Option<bool>, f32); 4]) {
    let size = |i: usize| corners.get(i).map_or(0.0, |c| if c.0.is_some() { c.1.max(0.0) } else { 0.0 });
    // A corner of no size is square: no arc of no radius is drawn for it.
    let round = |i: usize| corners.get(i).is_some_and(|c| c.0 == Some(true) && c.1 > 0.0);
    let (tl, tr, br, bl) = (size(0), size(1), size(2), size(3));
    pen.m(tl, 0.0).l(w - tr, 0.0);
    if round(1) {
        pen.arc(tr, tr, 270.0, 90.0);
    } else {
        pen.l(w, tr);
    }
    pen.l(w, h - br);
    if round(2) {
        pen.arc(br, br, 0.0, 90.0);
    } else {
        pen.l(w - br, h);
    }
    pen.l(bl, h);
    if round(3) {
        pen.arc(bl, bl, 90.0, 90.0);
    } else {
        pen.l(0.0, h - bl);
    }
    pen.l(0.0, tl);
    if round(0) {
        pen.arc(tl, tl, 180.0, 90.0);
    } else {
        pen.l(tl, 0.0);
    }
    pen.z();
}

/// A callout's tail: from the point of the body's edge nearest the tip, `(a, b)` on that edge.
struct Tail {
    /// The tip, in the frame.
    tip: (f32, f32),
    /// Which edge the tail leaves: 0 top, 1 right, 2 bottom, 3 left.
    edge: u8,
    /// Whether the tail's foot is in the later part of its edge (right or lower).
    far: bool,
}

/// Where a rectangular callout's tail goes, from its tip adjust values (the tip's place from the
/// frame's middle, as shares of the width and height).
fn tail(preset: &PresetGeom, w: f32, h: f32) -> Tail {
    let (dx, dy) = (w * value(preset, "adj1", -20_833.0) / FULL, h * value(preset, "adj2", 62_500.0) / FULL);
    let tip = (w / 2.0 + dx, h / 2.0 + dy);
    // The edge the tip is beyond, by which way it is farther out in the frame's own proportions.
    let upright = if w > 0.0 { dy.abs() > (dx * h / w).abs() } else { true };
    let edge = match (upright, dx > 0.0, dy > 0.0) {
        (true, _, false) => 0,
        (true, _, true) => 2,
        (false, true, _) => 1,
        (false, false, _) => 3,
    };
    Tail { tip, edge, far: if upright { dx > 0.0 } else { dy > 0.0 } }
}

/// The path mirrored: left to right across a frame `across` wide, top to bottom down one `down`
/// tall.
fn mirror(segs: &mut [PathSeg], across: Option<f32>, down: Option<f32>) {
    let fx = |x: f32| across.map_or(x, |w| w - x);
    let fy = |y: f32| down.map_or(y, |h| h - y);
    for seg in segs {
        *seg = match *seg {
            PathSeg::Move(x, y) => PathSeg::Move(fx(x), fy(y)),
            PathSeg::Line(x, y) => PathSeg::Line(fx(x), fy(y)),
            PathSeg::Cubic(a, b, c, d, x, y) => PathSeg::Cubic(fx(a), fy(b), fx(c), fy(d), fx(x), fy(y)),
            PathSeg::Close => PathSeg::Close,
        };
    }
}

/// DrawingML's cloud: eleven arcs around, and the short arcs drawn on it. The standard gives it
/// on a 43200 × 43200 grid, with angles in 60000ths of a degree.
fn cloud(p: &mut Pen, w: f32, h: f32) {
    const AROUND: [(f32, f32, f32, f32); 11] = [
        (6753.0, 9190.0, -11_429_249.0, 7_426_832.0),
        (5333.0, 7267.0, -8_646_143.0, 5_396_714.0),
        (4365.0, 5945.0, -8_748_475.0, 5_983_381.0),
        (4857.0, 6595.0, -7_859_164.0, 7_034_504.0),
        (5333.0, 7273.0, -4_722_533.0, 6_541_615.0),
        (6775.0, 9220.0, -2_776_035.0, 7_816_140.0),
        (5785.0, 7867.0, 37_501.0, 6_842_000.0),
        (6752.0, 9215.0, 1_347_096.0, 6_910_353.0),
        (7720.0, 10_543.0, 3_974_558.0, 4_542_661.0),
        (4360.0, 5918.0, -16_496_525.0, 8_804_134.0),
        (4345.0, 5945.0, -14_809_710.0, 9_151_131.0),
    ];
    // Where each starts, then its arc.
    const ON_IT: [((f32, f32), (f32, f32, f32, f32)); 11] = [
        ((4693.0, 26_177.0), (4345.0, 5945.0, 5_204_520.0, 1_585_770.0)),
        ((6928.0, 34_899.0), (4360.0, 5918.0, 4_416_628.0, 686_848.0)),
        ((16_478.0, 39_090.0), (6752.0, 9215.0, 8_257_449.0, 844_866.0)),
        ((28_827.0, 34_751.0), (6752.0, 9215.0, 387_196.0, 959_901.0)),
        ((34_129.0, 22_954.0), (5785.0, 7867.0, -4_217_541.0, 4_255_042.0)),
        ((41_798.0, 15_354.0), (5333.0, 7273.0, 1_819_082.0, 1_665_090.0)),
        ((38_324.0, 5426.0), (4857.0, 6595.0, -824_660.0, 891_534.0)),
        ((29_078.0, 3952.0), (4857.0, 6595.0, -8_950_887.0, 1_091_722.0)),
        ((22_141.0, 4720.0), (4365.0, 5945.0, -9_809_656.0, 1_061_181.0)),
        ((14_000.0, 5192.0), (6753.0, 9190.0, -4_002_417.0, 739_161.0)),
        ((4127.0, 15_789.0), (6753.0, 9190.0, 9_459_261.0, 711_490.0)),
    ];
    // Drawn on the standard's grid, where its angles are meant, then stretched to the frame.
    let mut grid = Pen::new();
    grid.m(3900.0, 14_370.0);
    for (rx, ry, start, sweep) in AROUND {
        grid.arc_seen(rx, ry, start / 60_000.0, sweep / 60_000.0);
    }
    // The last arc ends a hair from where the first began: the closing line is that hair.
    grid.z();
    for ((x, y), (rx, ry, start, sweep)) in ON_IT {
        // There and back along the same arc: drawn, and enclosing nothing.
        let (start, sweep) = (start / 60_000.0, sweep / 60_000.0);
        grid.m(x, y).arc_seen(rx, ry, start, sweep).arc_seen(rx, ry, start + sweep, -sweep);
    }
    let (sx, sy) = (w / 43_200.0, h / 43_200.0);
    p.segs.extend(grid.segs.into_iter().map(|seg| match seg {
        PathSeg::Move(x, y) => PathSeg::Move(x * sx, y * sy),
        PathSeg::Line(x, y) => PathSeg::Line(x * sx, y * sy),
        PathSeg::Cubic(a, b, c, d, x, y) => PathSeg::Cubic(a * sx, b * sy, c * sx, d * sy, x * sx, y * sy),
        PathSeg::Close => PathSeg::Close,
    }));
}

/// DrawingML's `curvedRightArrow` in a `w` × `h` frame: a band that sweeps round the left of the
/// frame from its top right, ending in an arrowhead that points right at the bottom. First the
/// band's front with the head, then the part of the band seen from behind.
fn curved_arrow(p: &mut Pen, preset: &PresetGeom, w: f32, h: f32) {
    let ss = w.min(h);
    // The head's width, the band's width (no wider than the head), the head's length.
    let a2 = share(preset, "adj2", 50_000.0, 50_000.0 * h / ss);
    let a1 = share(preset, "adj1", 25_000.0, a2 * FULL);
    let (th, aw) = (ss * a1, ss * a2);
    // The arcs' upright radius: both are quarters of ellipses as wide as the frame.
    let hr = (h / 2.0 - (th + aw) / 4.0).max(h * 0.01);
    let idx = ((hr * 2.0).powi(2) - th * th).max(0.0).sqrt() * w / (hr * 2.0);
    let ah = ss * share(preset, "adj3", 25_000.0, FULL * idx / ss);
    let y3 = hr + th;
    let dy = (w * w - ah * ah).max(0.0).sqrt() * hr / w;
    let (y5, y7, dh) = (hr + dy, y3 + dy, (aw - th) / 2.0);
    let x1 = w - ah;
    let sweep = dy.atan2(ah).to_degrees();
    let behind = (th / 2.0).atan2(idx).to_degrees();
    p.m(0.0, hr).arc_seen(w, hr, 180.0, -sweep).l(x1, y5 - dh).l(w, h - aw / 2.0).l(x1, y7 + dh).l(x1, y7).arc_seen(w, hr, 180.0 - sweep, sweep).z();
    p.m(w, th).arc_seen(w, hr, 270.0, behind - 90.0).arc_seen(w, hr, 180.0 - behind, 90.0 + behind).z();
}

/// The adjust value `name` of `preset`, or `default`.
fn value(preset: &PresetGeom, name: &str, default: f32) -> f32 {
    preset.adjust.iter().find(|(n, _)| n == name).map_or(default, |(_, v)| *v as f32)
}

/// The adjust value `name` as a share, kept within `0..=max` (shares of 100000).
fn share(preset: &PresetGeom, name: &str, default: f32, max: f32) -> f32 {
    let max = if max.is_finite() { max.max(0.0) } else { 0.0 };
    value(preset, name, default).clamp(0.0, max) / FULL
}

/// The path `preset` draws in a `w` × `h` frame at the origin: closed outlines first (filled and
/// stroked), then any lines drawn on the shape, each a subpath of its own that encloses nothing
/// outside the outline. Brackets and braces are open curves instead: a fill joins their ends,
/// the outline doesn't. `None` for a preset this library has no outline for (the shape is then
/// drawn as its kind), and for a frame with no area.
pub fn outline(preset: &PresetGeom, w: f32, h: f32) -> Option<Vec<PathSeg>> {
    let ok = |v: f32| v.is_finite() && v > 0.0;
    if !ok(w) || !ok(h) {
        return None;
    }
    let (w, h) = (w.min(MAX_SIDE), h.min(MAX_SIDE));
    let ss = w.min(h);
    let (hc, vc) = (w / 2.0, h / 2.0);
    let mut p = Pen::new();
    let adj = |name: &str, default: f32, max: f32| share(preset, name, default, max);
    match preset.name.as_str() {
        "parallelogram" => {
            let x = ss * adj("adj", 25_000.0, FULL * w / ss);
            p.poly(&[(0.0, h), (x, 0.0), (w, 0.0), (w - x, h)]);
        }
        "flowChartInputOutput" => {
            p.poly(&[(0.0, h), (w / 5.0, 0.0), (w, 0.0), (w * 0.8, h)]);
        }
        "trapezoid" => {
            let x = ss * adj("adj", 25_000.0, 50_000.0 * w / ss);
            p.poly(&[(0.0, h), (x, 0.0), (w - x, 0.0), (w, h)]);
        }
        "flowChartManualOperation" => {
            p.poly(&[(0.0, 0.0), (w, 0.0), (w * 0.8, h), (w * 0.2, h)]);
        }
        "triangle" => {
            let x = w * adj("adj", 50_000.0, FULL);
            p.poly(&[(0.0, h), (x, 0.0), (w, h)]);
        }
        "flowChartMerge" => {
            p.poly(&[(0.0, 0.0), (w, 0.0), (hc, h)]);
        }
        "hexagon" => {
            let x = ss * adj("adj", 25_000.0, 50_000.0 * w / ss);
            // The slanted sides' height: a regular hexagon's at the default.
            let dy = (vc * value(preset, "vf", 115_470.0) / FULL * 60f32.to_radians().sin()).clamp(0.0, vc);
            p.poly(&[(0.0, vc), (x, vc - dy), (w - x, vc - dy), (w, vc), (w - x, vc + dy), (x, vc + dy)]);
        }
        "flowChartPreparation" => {
            p.poly(&[(0.0, vc), (w * 0.2, 0.0), (w * 0.8, 0.0), (w, vc), (w * 0.8, h), (w * 0.2, h)]);
        }
        "octagon" => {
            let x = ss * adj("adj", 29_289.0, 50_000.0);
            p.poly(&[(0.0, x), (x, 0.0), (w - x, 0.0), (w, x), (w, h - x), (w - x, h), (x, h), (0.0, h - x)]);
        }
        "pentagon" => {
            p.poly(&regular(5, -90.0, w, h));
        }
        "heptagon" => {
            p.poly(&regular(7, -90.0, w, h));
        }
        "decagon" => {
            p.poly(&regular(10, 0.0, w, h));
        }
        "dodecagon" => {
            p.poly(&regular(12, 15.0, w, h));
        }
        "plus" => {
            let x = ss * adj("adj", 25_000.0, 50_000.0);
            let (x2, y2) = (w - x, h - x);
            p.poly(&[(0.0, x), (x, x), (x, 0.0), (x2, 0.0), (x2, x), (w, x), (w, y2), (x2, y2), (x2, h), (x, h), (x, y2), (0.0, y2)]);
        }
        "frame" => {
            let x = ss * adj("adj1", 12_500.0, 50_000.0);
            p.poly(&[(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)]);
            // The opening runs the other way round, so it is left unfilled.
            p.poly(&[(x, x), (x, h - x), (w - x, h - x), (w - x, x)]);
        }
        "donut" => {
            let d = ss * adj("adj", 25_000.0, 50_000.0);
            p.ellipse(0.0, 0.0, w, h, true);
            if w - 2.0 * d > 0.01 && h - 2.0 * d > 0.01 {
                p.ellipse(d, d, w - 2.0 * d, h - 2.0 * d, false);
            }
        }
        "corner" => {
            let (x, dy) = (ss * adj("adj2", 50_000.0, FULL * w / ss), ss * adj("adj1", 50_000.0, FULL * h / ss));
            p.poly(&[(0.0, 0.0), (x, 0.0), (x, h - dy), (w, h - dy), (w, h), (0.0, h)]);
        }
        "diagStripe" => {
            let a = adj("adj", 50_000.0, FULL);
            p.poly(&[(0.0, h * a), (w * a, 0.0), (w, 0.0), (0.0, h)]);
        }
        "chevron" => {
            let x = ss * adj("adj", 50_000.0, FULL * w / ss);
            p.poly(&[(0.0, 0.0), (w - x, 0.0), (w, vc), (w - x, h), (0.0, h), (x, vc)]);
        }
        "homePlate" => {
            let x = w - ss * adj("adj", 50_000.0, FULL * w / ss);
            p.poly(&[(0.0, 0.0), (x, 0.0), (w, vc), (x, h), (0.0, h)]);
        }
        "can" | "flowChartMagneticDisk" => {
            // Half the height of the lid's ellipse.
            let ry = if preset.name == "can" { ss * adj("adj", 25_000.0, 50_000.0 * h / ss) / 2.0 } else { h / 6.0 };
            p.m(0.0, ry).arc(hc, ry, 180.0, 180.0).l(w, h - ry).arc(hc, ry, 0.0, 180.0).z();
            // The lid's front edge.
            p.m(w, ry).arc(hc, ry, 0.0, 180.0);
        }
        "flowChartMagneticDrum" => {
            let rx = w / 6.0;
            p.m(rx, 0.0).l(w - rx, 0.0).arc(rx, vc, 270.0, 180.0).l(rx, h).arc(rx, vc, 90.0, 180.0).z();
            p.m(w - rx, h).arc(rx, vc, 90.0, 180.0);
        }
        "cube" => {
            let d = ss * adj("adj", 25_000.0, FULL);
            p.poly(&[(0.0, d), (d, 0.0), (w, 0.0), (w, h - d), (w - d, h), (0.0, h)]);
            p.line((0.0, d), (w - d, d)).line((w - d, d), (w, 0.0)).line((w - d, d), (w - d, h));
        }
        "foldedCorner" => {
            let d = ss * adj("adj", 16_667.0, 50_000.0);
            p.poly(&[(0.0, 0.0), (w, 0.0), (w, h - d), (w - d, h), (0.0, h)]);
            let fold = (w - d + d / 5.0, h - d + d / 5.0);
            p.line((w - d, h), fold).line(fold, (w, h - d));
        }
        "flowChartTerminator" => {
            let rx = w * 3475.0 / 21_600.0;
            p.m(rx, 0.0).l(w - rx, 0.0).arc(rx, vc, 270.0, 180.0).l(rx, h).arc(rx, vc, 90.0, 180.0).z();
        }
        "flowChartPredefinedProcess" => {
            p.poly(&[(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)]);
            p.line((w / 8.0, 0.0), (w / 8.0, h)).line((w * 7.0 / 8.0, 0.0), (w * 7.0 / 8.0, h));
        }
        "flowChartInternalStorage" => {
            p.poly(&[(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)]);
            p.line((w / 8.0, 0.0), (w / 8.0, h)).line((0.0, h / 8.0), (w, h / 8.0));
        }
        "flowChartDocument" => {
            let y = |v: f32| h * v / 21_600.0;
            p.m(0.0, 0.0).l(w, 0.0).l(w, y(17_322.0)).c((hc, y(17_322.0)), (hc, y(23_922.0)), (0.0, y(20_172.0))).z();
        }
        "flowChartManualInput" => {
            p.poly(&[(0.0, h / 5.0), (w, 0.0), (w, h), (0.0, h)]);
        }
        "flowChartOffpageConnector" => {
            p.poly(&[(0.0, 0.0), (w, 0.0), (w, h * 0.8), (hc, h), (0.0, h * 0.8)]);
        }
        "flowChartPunchedCard" => {
            p.poly(&[(0.0, h / 5.0), (w / 5.0, 0.0), (w, 0.0), (w, h), (0.0, h)]);
        }
        "flowChartSummingJunction" => {
            p.ellipse(0.0, 0.0, w, h, true);
            let (dx, dy) = (hc * std::f32::consts::FRAC_1_SQRT_2, vc * std::f32::consts::FRAC_1_SQRT_2);
            p.line((hc - dx, vc - dy), (hc + dx, vc + dy)).line((hc + dx, vc - dy), (hc - dx, vc + dy));
        }
        "flowChartCollate" => {
            p.poly(&[(0.0, 0.0), (w, 0.0), (hc, vc), (w, h), (0.0, h), (hc, vc)]);
        }
        "flowChartSort" => {
            p.poly(&[(0.0, vc), (hc, 0.0), (w, vc), (hc, h)]);
            p.line((0.0, vc), (w, vc));
        }
        "flowChartOnlineStorage" => {
            let rx = w / 6.0;
            p.m(rx, 0.0).l(w, 0.0).arc(rx, vc, 270.0, -180.0).l(rx, h).arc(rx, vc, 90.0, 180.0).z();
        }
        "flowChartDelay" => {
            p.m(0.0, 0.0).l(hc, 0.0).arc(hc, vc, 270.0, 180.0).l(0.0, h).z();
        }
        "cloud" => cloud(&mut p, w, h),
        "leftBracket" | "rightBracket" => {
            // One open curve: its two ends are joined only by the fill.
            let y1 = ss * adj("adj", 8_333.0, 50_000.0 * h / ss);
            p.m(w, h).arc(w, y1, 90.0, 90.0).l(0.0, y1).arc(w, y1, 180.0, 90.0);
            if preset.name == "rightBracket" {
                mirror(&mut p.segs, Some(w), None);
            }
        }
        "leftBrace" | "rightBrace" => {
            // Where the point is, down the brace; the curls are no taller than half of the
            // shorter stretch beside it.
            let a2 = adj("adj2", 50_000.0, FULL);
            let y1 = ss * adj("adj1", 8_333.0, (1.0 - a2).min(a2) / 2.0 * FULL * h / ss);
            let y3 = h * a2;
            p.m(w, h).arc(hc, y1, 90.0, 90.0).l(hc, y3 + y1).arc(hc, y1, 0.0, -90.0).arc(hc, y1, 90.0, -90.0).l(hc, y1).arc(hc, y1, 180.0, 90.0);
            if preset.name == "rightBrace" {
                mirror(&mut p.segs, Some(w), None);
            }
        }
        "bracketPair" => {
            let x1 = ss * adj("adj", 16_667.0, 50_000.0);
            p.m(x1, h).arc(x1, x1, 90.0, 90.0).l(0.0, x1).arc(x1, x1, 180.0, 90.0);
            p.m(w - x1, 0.0).arc(x1, x1, 270.0, 90.0).l(w, h - x1).arc(x1, x1, 0.0, 90.0);
        }
        "bracePair" => {
            let x1 = ss * adj("adj", 8_333.0, 25_000.0);
            p.m(x1 * 2.0, h)
                .arc(x1, x1, 90.0, 90.0)
                .l(x1, vc + x1)
                .arc(x1, x1, 0.0, -90.0)
                .arc(x1, x1, 90.0, -90.0)
                .l(x1, x1)
                .arc(x1, x1, 180.0, 90.0);
            let x4 = w - x1;
            p.m(w - x1 * 2.0, 0.0)
                .arc(x1, x1, 270.0, 90.0)
                .l(x4, vc - x1)
                .arc(x1, x1, 180.0, -90.0)
                .arc(x1, x1, 270.0, -90.0)
                .l(x4, h - x1)
                .arc(x1, x1, 0.0, 90.0);
        }
        "curvedRightArrow" | "curvedLeftArrow" => {
            curved_arrow(&mut p, preset, w, h);
            // The left one is the right one seen in a mirror.
            if preset.name == "curvedLeftArrow" {
                mirror(&mut p.segs, Some(w), None);
            }
        }
        "curvedDownArrow" | "curvedUpArrow" => {
            // Drawn as the arrow that curves right, in a frame lying on its side.
            curved_arrow(&mut p, preset, h, w);
            for seg in &mut p.segs {
                *seg = match *seg {
                    PathSeg::Move(x, y) => PathSeg::Move(y, x),
                    PathSeg::Line(x, y) => PathSeg::Line(y, x),
                    PathSeg::Cubic(a, b, c, d, x, y) => PathSeg::Cubic(b, a, d, c, y, x),
                    PathSeg::Close => PathSeg::Close,
                };
            }
            if preset.name == "curvedUpArrow" {
                mirror(&mut p.segs, None, Some(h));
            }
        }
        "ribbon" | "ribbon2" => {
            let a1 = adj("adj1", 16_667.0, 33_333.0);
            let a2 = value(preset, "adj2", 50_000.0).clamp(25_000.0, 75_000.0) / FULL;
            let (e, s) = (w / 8.0, w / 32.0);
            let (x2, x9) = (hc - w * a2 / 2.0, hc + w * a2 / 2.0);
            let (x3, x8, x5, x6) = (x2 + s, x9 - s, x2 + e, x9 - e);
            let (y1, y2, r) = (h * a1 / 2.0, h * a1, h * a1 / 4.0);
            let y4 = h - y2;
            p.m(0.0, 0.0).l(x5 - s, 0.0).arc(s, r, 270.0, 180.0).l(x3, y1).arc(s, r, 270.0, -180.0).l(x8, y2).arc(s, r, 90.0, -180.0);
            p.l(x6 + s, y1).arc(s, r, 90.0, 180.0).l(w, 0.0).l(w - e, y4 / 2.0).l(w, y4).l(x9, y4).l(x9, h - r).arc(s, r, 0.0, 90.0);
            p.l(x3, h).arc(s, r, 90.0, 90.0).l(x2, y4).l(0.0, y4).l(e, y4 / 2.0).z();
            // The back of each roll, seen between the tail and the front panel.
            p.m(x5, r).arc(s, r, 0.0, 90.0).l(x3, y1).arc(s, r, 270.0, -180.0).l(x5, y2).z();
            p.m(x6, r).arc(s, r, 180.0, -90.0).l(x8, y1).arc(s, r, 270.0, 180.0).l(x6, y2).z();
            // The folds behind the panel.
            p.line((x2, y4), (x2, y2 - r)).line((x9, y2 - r), (x9, y4));
            // The other ribbon is this one upside down.
            if preset.name == "ribbon2" {
                mirror(&mut p.segs, None, Some(h));
            }
        }
        "flowChartDisplay" => {
            let rx = w / 6.0;
            p.m(0.0, vc).l(rx, 0.0).l(w - rx, 0.0).arc(rx, vc, 270.0, 180.0).l(rx, h).z();
        }
        "star4" => {
            p.poly(&star(4, adj("adj", 12_500.0, 50_000.0) * 2.0, w, h));
        }
        "star6" => {
            // Its points touch the frame's sides: the outer corners lie on a wider ellipse.
            let a = adj("adj", 28_868.0, 50_000.0) * 2.0;
            let (iw, ih) = (hc * value(preset, "hf", 115_470.0) / FULL * a, vc * a);
            let dy = ih * 60f32.to_radians().sin();
            p.poly(&[
                (0.0, h / 4.0),
                (hc - iw / 2.0, vc - dy),
                (hc, 0.0),
                (hc + iw / 2.0, vc - dy),
                (w, h / 4.0),
                (hc + iw, vc),
                (w, h * 0.75),
                (hc + iw / 2.0, vc + dy),
                (hc, h),
                (hc - iw / 2.0, vc + dy),
                (0.0, h * 0.75),
                (hc - iw, vc),
            ]);
        }
        "star8" | "star12" | "star16" | "star24" | "star32" => {
            let n = preset.name.get(4..).and_then(|d| d.parse::<usize>().ok()).unwrap_or(8).clamp(4, 32);
            p.poly(&star(n, adj("adj", 37_500.0, 50_000.0) * 2.0, w, h));
        }
        "rightArrow" | "leftArrow" | "upArrow" | "downArrow" => {
            // Drawn pointing right in a frame `len` long and `thick` thick, then turned.
            let upright = matches!(preset.name.as_str(), "upArrow" | "downArrow");
            let (len, thick) = if upright { (h, w) } else { (w, h) };
            let (shaft, head) = (thick * adj("adj1", 50_000.0, FULL) / 2.0, ss * adj("adj2", 50_000.0, FULL * len / ss));
            let (x, m) = (len - head, thick / 2.0);
            let right = [(0.0, m - shaft), (x, m - shaft), (x, 0.0), (len, m), (x, thick), (x, m + shaft), (0.0, m + shaft)];
            let pts: Vec<(f32, f32)> = right
                .iter()
                .map(|&(u, v)| match preset.name.as_str() {
                    "leftArrow" => (len - u, v),
                    "upArrow" => (v, len - u),
                    "downArrow" => (v, u),
                    _ => (u, v),
                })
                .collect();
            p.poly(&pts);
        }
        "leftRightArrow" => {
            let (x, dy) = (ss * adj("adj2", 50_000.0, 50_000.0 * w / ss), h * adj("adj1", 50_000.0, FULL) / 2.0);
            let (y1, y2, x3) = (vc - dy, vc + dy, w - x);
            p.poly(&[(0.0, vc), (x, 0.0), (x, y1), (x3, y1), (x3, 0.0), (w, vc), (x3, h), (x3, y2), (x, y2), (x, h)]);
        }
        "upDownArrow" => {
            let (y, dx) = (ss * adj("adj2", 50_000.0, 50_000.0 * h / ss), w * adj("adj1", 50_000.0, FULL) / 2.0);
            let (x1, x2, y3) = (hc - dx, hc + dx, h - y);
            p.poly(&[(hc, 0.0), (w, y), (x2, y), (x2, y3), (w, y3), (hc, h), (0.0, y3), (x1, y3), (x1, y), (0.0, y)]);
        }
        "snip1Rect" => {
            let d = ss * adj("adj", 16_667.0, 50_000.0);
            cornered(&mut p, w, h, [(None, 0.0), (Some(false), d), (None, 0.0), (None, 0.0)]);
        }
        "snip2SameRect" | "round2SameRect" => {
            let round = Some(preset.name == "round2SameRect");
            let (top, bottom) = (ss * adj("adj1", 16_667.0, 50_000.0), ss * adj("adj2", 0.0, 50_000.0));
            cornered(&mut p, w, h, [(round, top), (round, top), (round, bottom), (round, bottom)]);
        }
        "snip2DiagRect" | "round2DiagRect" => {
            let round = preset.name == "round2DiagRect";
            // The rounded one's first value is the top-left and bottom-right corners; the cut
            // one's is too, and it starts at none.
            let defaults = if round { (16_667.0, 0.0) } else { (0.0, 16_667.0) };
            let (a, b) = (ss * adj("adj1", defaults.0, 50_000.0), ss * adj("adj2", defaults.1, 50_000.0));
            cornered(&mut p, w, h, [(Some(round), a), (Some(round), b), (Some(round), a), (Some(round), b)]);
        }
        "round1Rect" => {
            let d = ss * adj("adj", 16_667.0, 50_000.0);
            cornered(&mut p, w, h, [(None, 0.0), (Some(true), d), (None, 0.0), (None, 0.0)]);
        }
        "snipRoundRect" => {
            let (round, cut) = (ss * adj("adj1", 16_667.0, 50_000.0), ss * adj("adj2", 16_667.0, 50_000.0));
            cornered(&mut p, w, h, [(Some(true), round), (Some(false), cut), (None, 0.0), (None, 0.0)]);
        }
        "wedgeRectCallout" | "wedgeRoundRectCallout" => {
            let t = tail(preset, w, h);
            let r = if preset.name == "wedgeRoundRectCallout" { ss * adj("adj3", 16_667.0, 50_000.0) } else { 0.0 };
            // The tail's foot: between the marks 2/12 and 5/12, or 7/12 and 10/12, of its edge.
            let (a, b) = if t.far { (7.0 / 12.0, 10.0 / 12.0) } else { (2.0 / 12.0, 5.0 / 12.0) };
            let foot = |edge: u8, pen: &mut Pen| {
                if t.edge != edge {
                    return;
                }
                let (from, to) = match edge {
                    0 => ((w * a, 0.0), (w * b, 0.0)),
                    1 => ((w, h * a), (w, h * b)),
                    2 => ((w * b, h), (w * a, h)),
                    _ => ((0.0, h * b), (0.0, h * a)),
                };
                pen.l(from.0, from.1).l(t.tip.0, t.tip.1).l(to.0, to.1);
            };
            p.m(r, 0.0);
            foot(0, &mut p);
            p.l(w - r, 0.0);
            if r > 0.0 {
                p.arc(r, r, 270.0, 90.0);
            }
            foot(1, &mut p);
            p.l(w, h - r);
            if r > 0.0 {
                p.arc(r, r, 0.0, 90.0);
            }
            foot(2, &mut p);
            p.l(r, h);
            if r > 0.0 {
                p.arc(r, r, 90.0, 90.0);
            }
            foot(3, &mut p);
            p.l(0.0, r);
            if r > 0.0 {
                p.arc(r, r, 180.0, 90.0);
            }
            p.z();
        }
        "wedgeEllipseCallout" => {
            let t = tail(preset, w, h);
            // The tail's foot covers 22° of the ellipse, about the direction of the tip.
            let towards = ((t.tip.1 - vc) * hc).atan2((t.tip.0 - hc) * vc).to_degrees();
            let from = (towards + 11.0).to_radians();
            p.m(t.tip.0, t.tip.1).l(hc + hc * from.cos(), vc + vc * from.sin());
            p.arc(hc, vc, towards + 11.0, 338.0).z();
        }
        _ => return None,
    }
    let fin = p.segs.iter().all(|s| match *s {
        PathSeg::Move(x, y) | PathSeg::Line(x, y) => x.is_finite() && y.is_finite(),
        PathSeg::Cubic(a, b, c, d, e, f) => [a, b, c, d, e, f].into_iter().all(f32::is_finite),
        PathSeg::Close => true,
    });
    fin.then_some(p.segs)
}

/// Whether [`outline`] draws `preset` itself (rather than leaving it to the shape's kind).
pub fn has_outline(preset: &PresetGeom) -> bool {
    outline(preset, 100.0, 100.0).is_some()
}

/// The outline of `preset` in a `w` × `h` frame as a freeform's paths (curves flattened): for a
/// format with no name for the preset, which can still save what it looks like.
pub fn freeform(preset: &PresetGeom, w: f32, h: f32) -> Option<crate::freeform::Freeform> {
    use crate::freeform::{FreePath, Freeform};
    let segs = outline(preset, w, h)?;
    let mut paths: Vec<FreePath> = Vec::new();
    let mut pts: Vec<[f32; 2]> = Vec::new();
    let mut end = |pts: &mut Vec<[f32; 2]>, closed: bool| {
        if pts.len() >= 2 {
            paths.push(FreePath { pts: std::mem::take(pts), closed });
        }
        pts.clear();
    };
    for s in segs {
        match s {
            PathSeg::Move(x, y) => {
                end(&mut pts, false);
                pts.push([x, y]);
            }
            PathSeg::Line(x, y) => pts.push([x, y]),
            PathSeg::Cubic(a, b, c, d, e, f) => {
                let Some(&[x0, y0]) = pts.last() else { continue };
                for k in 1..=8 {
                    let t = k as f32 / 8.0;
                    let u = 1.0 - t;
                    let at = |p0: f32, p1: f32, p2: f32, p3: f32| u * u * u * p0 + 3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t * p3;
                    pts.push([at(x0, a, c, e), at(y0, b, d, f)]);
                }
            }
            PathSeg::Close => end(&mut pts, true),
        }
    }
    end(&mut pts, false);
    (!paths.is_empty()).then(|| Freeform { w, h, paths, ..Default::default() })
}

/// The first closed outline of `segs`, flattened to straight lines.
fn first_polygon(segs: &[PathSeg]) -> Vec<(f32, f32)> {
    let mut pts: Vec<(f32, f32)> = Vec::new();
    for s in segs {
        match *s {
            PathSeg::Move(x, y) => {
                if !pts.is_empty() {
                    break;
                }
                pts.push((x, y));
            }
            PathSeg::Line(x, y) => pts.push((x, y)),
            PathSeg::Cubic(a, b, c, d, e, f) => {
                let Some(&(x0, y0)) = pts.last() else { continue };
                for k in 1..=8 {
                    let t = k as f32 / 8.0;
                    let u = 1.0 - t;
                    let at = |p0: f32, p1: f32, p2: f32, p3: f32| u * u * u * p0 + 3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t * p3;
                    pts.push((at(x0, a, c, e), at(y0, b, d, f)));
                }
            }
            PathSeg::Close => break,
        }
    }
    pts
}

/// The outline of `preset` in unit coordinates (0..1 across its `w` × `h` frame, a callout's tail
/// cut off at the frame), for text to wrap around: [`crate::wrap::shape_outline`] for a preset.
pub fn unit_outline(preset: &PresetGeom, w: f32, h: f32) -> Option<Vec<(f32, f32)>> {
    let pts = first_polygon(&outline(preset, w, h)?);
    (pts.len() >= 3).then(|| pts.into_iter().map(|(x, y)| ((x / w).clamp(0.0, 1.0), (y / h).clamp(0.0, 1.0))).collect())
}

/// Whether point `p` is inside the polygon `poly` (even-odd).
fn inside(poly: &[(f32, f32)], p: (f32, f32)) -> bool {
    let mut odd = false;
    let n = poly.len();
    for i in 0..n {
        let (Some(&(x1, y1)), Some(&(x2, y2))) = (poly.get(i), poly.get((i + 1) % n)) else { continue };
        if (y1 > p.1) != (y2 > p.1) && p.0 < (x2 - x1) * (p.1 - y1) / (y2 - y1) + x1 {
            odd = !odd;
        }
    }
    odd
}

/// The room `preset` gives its text in a `w` × `h` frame: how far inside the frame the text
/// rectangle lies, as (left, top, right, bottom) in points ([`crate::wrap::text_insets`] for a
/// preset). `None` for a preset with no outline here.
///
/// A box with lines on it keeps its text clear of them; any other shape gets the largest
/// rectangle of the frame's own proportions, about the frame's middle, that its outline holds.
pub fn text_insets(preset: &PresetGeom, w: f32, h: f32) -> Option<[f32; 4]> {
    let segs = outline(preset, w, h)?;
    let ss = w.min(h);
    let shares: Option<[f32; 4]> = match preset.name.as_str() {
        "flowChartPredefinedProcess" => Some([0.125, 0.0, 0.125, 0.0]),
        "flowChartInternalStorage" => Some([0.125, 0.125, 0.0, 0.0]),
        "flowChartDocument" => Some([0.0, 0.0, 0.0, 0.2]),
        "flowChartMagneticDisk" => Some([0.0, 1.0 / 3.0, 0.0, 1.0 / 6.0]),
        "flowChartMagneticDrum" => Some([1.0 / 6.0, 0.0, 1.0 / 3.0, 0.0]),
        "can" => {
            let lid = ss * share(preset, "adj", 25_000.0, 50_000.0 * h / ss) / h;
            Some([0.0, lid, 0.0, lid / 2.0])
        }
        "cube" => {
            let d = ss * share(preset, "adj", 25_000.0, FULL);
            Some([0.0, d / h, d / w, 0.0])
        }
        // The lower arm.
        "corner" => Some([0.0, 1.0 - ss * share(preset, "adj1", 50_000.0, FULL * h / ss) / h, 0.0, 0.0]),
        // The text lies across the opening, the waist, the stripe.
        "frame" | "donut" | "diagStripe" => Some([0.0; 4]),
        "flowChartCollate" => Some([0.25; 4]),
        // The standard's text rectangle for the cloud.
        "cloud" => Some([2977.0 / 21_600.0, 3262.0 / 21_600.0, 1.0 - 17_087.0 / 21_600.0, 1.0 - 17_337.0 / 21_600.0]),
        // Between the brackets, beside the brace, across the arrow.
        "bracketPair" | "bracePair" | "leftBracket" | "rightBracket" | "leftBrace" | "rightBrace" => Some([0.0; 4]),
        "curvedRightArrow" | "curvedLeftArrow" | "curvedUpArrow" | "curvedDownArrow" => Some([0.0; 4]),
        // The ribbon's front panel.
        "ribbon" | "ribbon2" => {
            let fold = share(preset, "adj1", 16_667.0, 33_333.0);
            let side = 0.5 - value(preset, "adj2", 50_000.0).clamp(25_000.0, 75_000.0) / FULL / 2.0;
            // Clear of the panel's rounded corners.
            let side = side + 1.0 / 32.0;
            Some(if preset.name == "ribbon" { [side, fold, side, 0.0] } else { [side, 0.0, side, fold] })
        }
        _ => None,
    };
    if let Some([l, t, r, b]) = shares {
        return Some([l * w, t * h, r * w, b * h]);
    }
    let poly = first_polygon(&segs);
    if poly.len() < 3 {
        return Some([0.0; 4]);
    }
    // Shrink the frame about its middle until its corners and sides are inside the outline.
    let (cx, cy) = (w / 2.0, h / 2.0);
    let fits = |s: f32| {
        let (dx, dy) = (cx * s, cy * s);
        (0..4).all(|side| {
            (0..=4).all(|k| {
                let t = k as f32 / 4.0 * 2.0 - 1.0;
                let pt = match side {
                    0 => (cx + t * dx, cy - dy),
                    1 => (cx + dx, cy + t * dy),
                    2 => (cx + t * dx, cy + dy),
                    _ => (cx - dx, cy + t * dy),
                };
                inside(&poly, pt)
            })
        })
    };
    let scale = (1..=50).rev().map(|i| i as f32 / 50.0 - 0.001).find(|s| fits(*s)).unwrap_or(0.0);
    if scale <= 0.1 {
        return Some([0.0; 4]);
    }
    let (ix, iy) = (cx * (1.0 - scale), cy * (1.0 - scale));
    Some([ix, iy, ix, iy])
}

/// Where connectors glue to `preset` in a `w` × `h` frame: the points of its outline straight
/// above, left of, below and right of the frame's middle, in that order (the order of a
/// rectangle's sites). `None` for a preset with no outline here.
pub fn side_sites(preset: &PresetGeom, w: f32, h: f32) -> Option<Vec<(f32, f32)>> {
    let poly = first_polygon(&outline(preset, w, h)?);
    if poly.len() < 3 {
        return None;
    }
    let (cx, cy) = (w / 2.0, h / 2.0);
    // Where the outline crosses the upright (or level) line through the middle.
    let crossings = |upright: bool| -> Vec<f32> {
        let mut at = Vec::new();
        let n = poly.len();
        for i in 0..n {
            let (Some(&a), Some(&b)) = (poly.get(i), poly.get((i + 1) % n)) else { continue };
            let ((a0, a1), (b0, b1), c) = if upright { (a, b, cx) } else { ((a.1, a.0), (b.1, b.0), cy) };
            if (a0 > c) != (b0 > c) {
                at.push(a1 + (b1 - a1) * (c - a0) / (b0 - a0));
            }
        }
        at
    };
    let (ys, xs) = (crossings(true), crossings(false));
    let min = |v: &[f32], or: f32| v.iter().copied().fold(None, |m: Option<f32>, x| Some(m.map_or(x, |m| m.min(x)))).unwrap_or(or);
    let max = |v: &[f32], or: f32| v.iter().copied().fold(None, |m: Option<f32>, x| Some(m.map_or(x, |m| m.max(x)))).unwrap_or(or);
    let keep = |v: f32, side: f32| v.clamp(0.0, side);
    Some(vec![(cx, keep(min(&ys, 0.0), h)), (keep(min(&xs, 0.0), w), cy), (cx, keep(max(&ys, h), h)), (keep(max(&xs, w), w), cy)])
}

/// How one adjust value follows a handle: the value (100000ths) is `scale` times how far the
/// pointer is from `origin` along the frame's x axis (or its y axis), kept within `min..=max`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Follows {
    name: &'static str,
    along_x: bool,
    origin: f32,
    scale: f32,
    min: f32,
    max: f32,
}

/// A handle on a shape that changes its proportions: where it is in the frame, and the adjust
/// values it sets (one, or a callout tip's two).
#[derive(Clone, Debug, PartialEq)]
struct Handle {
    at: (f32, f32),
    sets: Vec<Follows>,
}

/// The preset a shape of `kind` with no kept preset is, where that preset has handles here, and
/// the adjust values that draw it as the kind is drawn (a block arrow's shaft is 40% thick).
fn own_preset(kind: ShapeKind) -> Option<PresetGeom> {
    let (name, adjust): (&str, &[(&str, i32)]) = match kind {
        ShapeKind::Triangle => ("triangle", &[("adj", 50_000)]),
        ShapeKind::Arrow => ("rightArrow", &[("adj1", 40_000), ("adj2", 50_000)]),
        ShapeKind::ArrowLeft => ("leftArrow", &[("adj1", 40_000), ("adj2", 50_000)]),
        ShapeKind::ArrowUp => ("upArrow", &[("adj1", 40_000), ("adj2", 50_000)]),
        ShapeKind::ArrowDown => ("downArrow", &[("adj1", 40_000), ("adj2", 50_000)]),
        _ => return None,
    };
    PresetGeom::new(name, adjust.iter().map(|(n, v)| (n.to_string(), *v)))
}

/// The handles of `preset` in a `w` × `h` frame: where the standard's preset definitions put
/// them (on the outline, at the corner or edge each value moves).
fn handles_of(preset: &PresetGeom, w: f32, h: f32) -> Vec<Handle> {
    let ok = |v: f32| v.is_finite() && v > 0.0;
    if !ok(w) || !ok(h) {
        return Vec::new();
    }
    let (w, h) = (w.min(MAX_SIDE), h.min(MAX_SIDE));
    let ss = w.min(h);
    let (hc, vc) = (w / 2.0, h / 2.0);
    let adj = |name: &str, default: f32, max: f32| share(preset, name, default, max);
    // A value measured along x (or y) from `origin`, `per` points to the whole.
    let x = |name: &'static str, origin: f32, per: f32, max: f32| Follows { name, along_x: true, origin, scale: FULL / per, min: 0.0, max };
    let y = |name: &'static str, origin: f32, per: f32, max: f32| Follows { name, along_x: false, origin, scale: FULL / per, min: 0.0, max };
    let one = |at: (f32, f32), f: Follows| Handle { at, sets: vec![f] };
    match preset.name.as_str() {
        "parallelogram" => {
            let max = FULL * w / ss;
            vec![one((ss * adj("adj", 25_000.0, max), 0.0), x("adj", 0.0, ss, max))]
        }
        "trapezoid" | "hexagon" => {
            let max = 50_000.0 * w / ss;
            vec![one((ss * adj("adj", 25_000.0, max), 0.0), x("adj", 0.0, ss, max))]
        }
        "triangle" => vec![one((w * adj("adj", 50_000.0, FULL), 0.0), x("adj", 0.0, w, FULL))],
        "octagon" => vec![one((ss * adj("adj", 29_289.0, 50_000.0), 0.0), x("adj", 0.0, ss, 50_000.0))],
        "plus" => vec![one((ss * adj("adj", 25_000.0, 50_000.0), 0.0), x("adj", 0.0, ss, 50_000.0))],
        "frame" => vec![one((ss * adj("adj1", 12_500.0, 50_000.0), 0.0), x("adj1", 0.0, ss, 50_000.0))],
        "donut" => vec![one((ss * adj("adj", 25_000.0, 50_000.0), vc), x("adj", 0.0, ss, 50_000.0))],
        "corner" => {
            let (max_x, max_y) = (FULL * w / ss, FULL * h / ss);
            vec![
                one((ss * adj("adj2", 50_000.0, max_x), 0.0), x("adj2", 0.0, ss, max_x)),
                one((w, h - ss * adj("adj1", 50_000.0, max_y)), y("adj1", h, -ss, max_y)),
            ]
        }
        "diagStripe" => vec![one((0.0, h * adj("adj", 50_000.0, FULL)), y("adj", 0.0, h, FULL))],
        "chevron" | "homePlate" => {
            let max = FULL * w / ss;
            vec![one((w - ss * adj("adj", 50_000.0, max), 0.0), x("adj", w, -ss, max))]
        }
        "can" => {
            let max = 50_000.0 * h / ss;
            vec![one((hc, ss * adj("adj", 25_000.0, max)), y("adj", 0.0, ss, max))]
        }
        "cube" => vec![one((0.0, ss * adj("adj", 25_000.0, FULL)), y("adj", 0.0, ss, FULL))],
        "foldedCorner" => vec![one((w - ss * adj("adj", 16_667.0, 50_000.0), h), x("adj", w, -ss, 50_000.0))],
        "star4" | "star6" | "star8" | "star12" | "star16" | "star24" | "star32" => {
            let n = preset.name.get(4..).and_then(|d| d.parse::<usize>().ok()).unwrap_or(8).clamp(4, 32);
            let default = match n {
                4 => 12_500.0,
                6 => 28_868.0,
                _ => 37_500.0,
            };
            // On the inner corner left of the top point: half a point's angle from straight up,
            // as far out as the inner corners are. (A 6-point star's is a little nearer the
            // middle sideways: its points touch the frame's sides.)
            let half = std::f32::consts::PI / n as f32;
            let inner = adj("adj", default, 50_000.0) * 2.0;
            let wide = if n == 6 { value(preset, "hf", 115_470.0) / FULL * 0.5 } else { half.sin() };
            vec![one((hc - hc * inner * wide, vc - vc * inner * half.cos()), y("adj", vc, -2.0 * vc * half.cos(), 50_000.0))]
        }
        "rightArrow" | "leftArrow" | "upArrow" | "downArrow" => {
            let upright = matches!(preset.name.as_str(), "upArrow" | "downArrow");
            let (len, thick) = if upright { (h, w) } else { (w, h) };
            let max = FULL * len / ss;
            let (shaft, head) = (thick * adj("adj1", 50_000.0, FULL) / 2.0, ss * adj("adj2", 50_000.0, max));
            // Pointing right: the head's inner corner at (len - head, 0), the shaft's edge at
            // (0, thick / 2 - shaft); turned with the arrow.
            match preset.name.as_str() {
                "leftArrow" => vec![one((head, 0.0), x("adj2", 0.0, ss, max)), one((w, vc - shaft), y("adj1", vc, -thick / 2.0, FULL))],
                "upArrow" => vec![one((0.0, head), y("adj2", 0.0, ss, max)), one((hc - shaft, h), x("adj1", hc, -thick / 2.0, FULL))],
                "downArrow" => vec![one((0.0, h - head), y("adj2", h, -ss, max)), one((hc - shaft, 0.0), x("adj1", hc, -thick / 2.0, FULL))],
                _ => vec![one((w - head, 0.0), x("adj2", w, -ss, max)), one((0.0, vc - shaft), y("adj1", vc, -thick / 2.0, FULL))],
            }
        }
        "leftRightArrow" => {
            let max = 50_000.0 * w / ss;
            let head = ss * adj("adj2", 50_000.0, max);
            vec![one((head, 0.0), x("adj2", 0.0, ss, max)), one((head, vc - h * adj("adj1", 50_000.0, FULL) / 2.0), y("adj1", vc, -h / 2.0, FULL))]
        }
        "upDownArrow" => {
            let max = 50_000.0 * h / ss;
            let head = ss * adj("adj2", 50_000.0, max);
            vec![one((0.0, head), y("adj2", 0.0, ss, max)), one((hc - w * adj("adj1", 50_000.0, FULL) / 2.0, head), x("adj1", hc, -w / 2.0, FULL))]
        }
        "snip1Rect" | "round1Rect" => vec![one((w - ss * adj("adj", 16_667.0, 50_000.0), 0.0), x("adj", w, -ss, 50_000.0))],
        "snip2SameRect" | "round2SameRect" => vec![
            one((ss * adj("adj1", 16_667.0, 50_000.0), 0.0), x("adj1", 0.0, ss, 50_000.0)),
            one((ss * adj("adj2", 0.0, 50_000.0), h), x("adj2", 0.0, ss, 50_000.0)),
        ],
        "snip2DiagRect" | "round2DiagRect" => {
            let defaults = if preset.name == "round2DiagRect" { (16_667.0, 0.0) } else { (0.0, 16_667.0) };
            vec![
                one((ss * adj("adj1", defaults.0, 50_000.0), 0.0), x("adj1", 0.0, ss, 50_000.0)),
                one((w - ss * adj("adj2", defaults.1, 50_000.0), 0.0), x("adj2", w, -ss, 50_000.0)),
            ]
        }
        "snipRoundRect" => vec![
            one((ss * adj("adj1", 16_667.0, 50_000.0), 0.0), x("adj1", 0.0, ss, 50_000.0)),
            one((w - ss * adj("adj2", 16_667.0, 50_000.0), 0.0), x("adj2", w, -ss, 50_000.0)),
        ],
        "wedgeRectCallout" | "wedgeRoundRectCallout" | "wedgeEllipseCallout" => {
            // The tip: anywhere, as far as ten frames away.
            let far = 1_000_000.0;
            let tip = tail(preset, w, h).tip;
            let to =
                |name: &'static str, along_x: bool, origin: f32, per: f32| Follows { name, along_x, origin, scale: FULL / per, min: -far, max: far };
            vec![Handle { at: tip, sets: vec![to("adj1", true, hc, w), to("adj2", false, vc, h)] }]
        }
        _ => Vec::new(),
    }
}

/// The preset a shape's handles work on: the one it keeps, else its kind's own where that has
/// handles ([`own_preset`]).
fn handled(kind: ShapeKind, preset: Option<&PresetGeom>) -> Option<PresetGeom> {
    if kind.is_open() || kind == ShapeKind::Freeform {
        return None;
    }
    preset.cloned().or_else(|| own_preset(kind))
}

/// Where the handles that change a shape's proportions are, in its `w` × `h` frame: one for each
/// thing its preset lets be adjusted (a parallelogram's lean, an arrow's head and shaft, a
/// callout's tip), and a rounded rectangle's corner. None for a shape with nothing to adjust.
pub fn shape_handles(kind: ShapeKind, preset: Option<&PresetGeom>, corner: Option<u32>, w: f32, h: f32) -> Vec<(f32, f32)> {
    if kind == ShapeKind::RoundedRectangle && preset.is_none() {
        let ok = w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0;
        return if ok { vec![(crate::connector::corner_radius(corner, w, h), 0.0)] } else { Vec::new() };
    }
    handled(kind, preset).map(|p| handles_of(&p, w, h).into_iter().map(|hd| hd.at).collect()).unwrap_or_default()
}

/// What moving a shape's handle gives: its preset with new adjust values, or a rounded
/// rectangle's new corner.
#[derive(Clone, Debug, PartialEq)]
pub enum Adjusted {
    Preset(PresetGeom),
    Corner(u32),
}

/// A shape's handle `index` (as [`shape_handles`] lists them) moved to the frame point `to`.
/// `None` for a handle the shape doesn't have.
pub fn move_handle(kind: ShapeKind, preset: Option<&PresetGeom>, (w, h): (f32, f32), index: usize, to: (f32, f32)) -> Option<Adjusted> {
    let fin = |v: f32| if v.is_finite() { v.clamp(-1.0e7, 1.0e7) } else { 0.0 };
    let to = (fin(to.0), fin(to.1));
    if kind == ShapeKind::RoundedRectangle && preset.is_none() {
        let ss = w.min(h);
        let ok = index == 0 && ss.is_finite() && ss > 0.0;
        return ok.then(|| Adjusted::Corner((to.0 / ss * FULL).clamp(0.0, crate::connector::MAX_CORNER as f32).round() as u32));
    }
    let mut geom = handled(kind, preset)?;
    let handle = handles_of(&geom, w, h).into_iter().nth(index)?;
    for f in handle.sets {
        let v = ((if f.along_x { to.0 } else { to.1 }) - f.origin) * f.scale;
        let v = if v.is_finite() { v.clamp(f.min, f.max).round() as i32 } else { continue };
        let room = geom.adjust.len() < PresetGeom::MAX_ADJUST;
        match geom.adjust.iter().position(|(n, _)| n == f.name) {
            Some(at) => {
                if let Some(slot) = geom.adjust.get_mut(at) {
                    slot.1 = v;
                }
            }
            None if room => geom.adjust.push((f.name.to_string(), v)),
            None => {}
        }
    }
    Some(Adjusted::Preset(geom))
}

/// Whether `preset` is drawn as open curves, not as a closed outline: the brackets and braces. A
/// new one gets no fill.
pub fn is_open_curve(preset: &PresetGeom) -> bool {
    matches!(preset.name.as_str(), "leftBracket" | "rightBracket" | "leftBrace" | "rightBrace" | "bracketPair" | "bracePair")
}

/// The kind a gallery name stands for when it is a [`ShapeKind`]'s own name.
pub fn kind_of_name(name: &str) -> Option<ShapeKind> {
    serde_json::from_value(serde_json::Value::String(name.to_string())).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset(name: &str) -> PresetGeom {
        PresetGeom::new(name, []).unwrap()
    }

    fn with(name: &str, adjust: &[(&str, i32)]) -> PresetGeom {
        PresetGeom::new(name, adjust.iter().map(|(n, v)| (n.to_string(), *v))).unwrap()
    }

    fn corners(segs: &[PathSeg]) -> Vec<(f32, f32)> {
        segs.iter()
            .filter_map(|s| match *s {
                PathSeg::Move(x, y) | PathSeg::Line(x, y) => Some(((x * 100.0).round() / 100.0, (y * 100.0).round() / 100.0)),
                _ => None,
            })
            .collect()
    }

    /// The presets of the gallery that are not a kind's own name.
    fn presets() -> Vec<&'static str> {
        LIBRARY.iter().map(|s| s.name).filter(|n| kind_of_name(n).is_none()).collect()
    }

    #[test]
    fn every_gallery_shape_is_a_kind_or_a_named_preset_and_is_listed_once() {
        let mut seen = std::collections::HashSet::new();
        for s in LIBRARY {
            assert!(seen.insert(s.name), "{} is listed twice", s.name);
            assert!(kind_of_name(s.name).is_some() || PresetGeom::valid_name(s.name), "{}", s.name);
            assert!(!s.label.is_empty());
            assert!(ShapeGroup::ALL.contains(&s.group));
        }
        for group in ShapeGroup::ALL {
            assert!(LIBRARY.iter().any(|s| s.group == group), "{group:?} is empty");
        }
        assert_eq!(gallery_shape("hexagon").map(|s| s.label), Some("Hexagon"));
        assert_eq!(gallery_shape("nothing"), None);
    }

    /// Presets that are exactly a kind's own shape need no outline; every other one has one.
    #[test]
    fn every_preset_of_the_gallery_has_an_outline_in_its_frame() {
        let as_kind = ["flowChartProcess", "flowChartAlternateProcess", "flowChartDecision", "flowChartConnector", "flowChartOr", "flowChartExtract"];
        for name in presets() {
            let geom = preset(name);
            if as_kind.contains(&name) {
                assert!(!has_outline(&geom), "{name} is drawn as its kind");
                continue;
            }
            for (w, h) in [(120.0, 80.0), (40.0, 200.0), (300.0, 20.0), (1.0, 1.0)] {
                let segs = outline(&geom, w, h).unwrap_or_else(|| panic!("{name} has no outline"));
                assert!(matches!(segs.first(), Some(PathSeg::Move(..))), "{name}");
                // Brackets and braces are open curves: filled between their ends, not outlined there.
                let open = matches!(name, "leftBracket" | "rightBracket" | "leftBrace" | "rightBrace" | "bracketPair" | "bracePair");
                assert_eq!(segs.contains(&PathSeg::Close), !open, "{name} is closed, unless it is a bracket");
                // A callout's tail is the one thing drawn outside the frame.
                if name.starts_with("wedge") {
                    continue;
                }
                let slack = 0.01 * w.max(h);
                for s in &segs {
                    let pts: Vec<(f32, f32)> = match *s {
                        PathSeg::Move(x, y) | PathSeg::Line(x, y) => vec![(x, y)],
                        PathSeg::Cubic(.., x, y) => vec![(x, y)],
                        PathSeg::Close => vec![],
                    };
                    for (x, y) in pts {
                        assert!(x >= -slack && x <= w + slack && y >= -slack && y <= h + slack, "{name} {w}x{h}: ({x}, {y})");
                    }
                }
            }
        }
        assert_eq!(outline(&preset("sun"), 10.0, 10.0), None, "a preset with no outline here is left to its kind");
    }

    #[test]
    fn hostile_frames_and_values_give_a_finite_outline_or_none() {
        for name in presets() {
            for (w, h) in [(0.0, 10.0), (10.0, 0.0), (-5.0, 5.0), (f32::NAN, 5.0), (f32::INFINITY, 5.0)] {
                assert_eq!(outline(&preset(name), w, h), None, "{name} {w}x{h}");
            }
            for v in [i32::MIN, -1, 0, i32::MAX] {
                let wild = with(name, &[("adj", v), ("adj1", v), ("adj2", v), ("adj3", v), ("hf", v), ("vf", v)]);
                for (w, h) in [(120.0, 80.0), (0.001, 5.0e7)] {
                    if let Some(segs) = outline(&wild, w, h) {
                        for s in segs {
                            let nums: Vec<f32> = match s {
                                PathSeg::Move(x, y) | PathSeg::Line(x, y) => vec![x, y],
                                PathSeg::Cubic(a, b, c, d, e, f) => vec![a, b, c, d, e, f],
                                PathSeg::Close => vec![],
                            };
                            assert!(nums.into_iter().all(f32::is_finite), "{name} {v}");
                        }
                    }
                    let _ = (text_insets(&wild, w, h), side_sites(&wild, w, h), unit_outline(&wild, w, h));
                }
            }
        }
    }

    #[test]
    fn the_polygons_are_where_the_standard_puts_them() {
        // A parallelogram and a trapezoid lean by a quarter of the shorter side.
        assert_eq!(corners(&outline(&preset("parallelogram"), 120.0, 80.0).unwrap()), vec![(0.0, 80.0), (20.0, 0.0), (120.0, 0.0), (100.0, 80.0)]);
        assert_eq!(corners(&outline(&preset("trapezoid"), 120.0, 80.0).unwrap()), vec![(0.0, 80.0), (20.0, 0.0), (100.0, 0.0), (120.0, 80.0)]);
        // An adjust value moves the lean.
        let leaning = with("parallelogram", &[("adj", 50_000)]);
        assert_eq!(corners(&outline(&leaning, 120.0, 80.0).unwrap())[1], (40.0, 0.0));
        // A hexagon's slanted sides reach the top and bottom; an octagon cuts 29.289% corners.
        assert_eq!(
            corners(&outline(&preset("hexagon"), 120.0, 80.0).unwrap()),
            vec![(0.0, 40.0), (20.0, 0.0), (100.0, 0.0), (120.0, 40.0), (100.0, 80.0), (20.0, 80.0)]
        );
        assert_eq!(corners(&outline(&preset("octagon"), 100.0, 100.0).unwrap())[..3], [(0.0, 29.29), (29.29, 0.0), (70.71, 0.0)]);
        // The regular polygons fill their frame: a pentagon's point is at the top, its lower
        // corners at the bottom; a dodecagon has a side along each side of the frame.
        let five = corners(&outline(&preset("pentagon"), 100.0, 100.0).unwrap());
        assert_eq!((five[0], five[1].0, five[2].1), ((50.0, 0.0), 100.0, 100.0), "{five:?}");
        assert_eq!(five[1].1, 38.2, "{five:?}");
        let twelve = corners(&outline(&preset("dodecagon"), 21_600.0, 21_600.0).unwrap());
        for want in [2894.0, 7906.0, 13_694.0, 18_706.0] {
            assert!(twelve.iter().any(|p| (p.0 - want).abs() < 1.0), "{want} in {twelve:?}");
        }
        // A cross, a chevron and a pentagon arrow.
        assert_eq!(corners(&outline(&preset("plus"), 100.0, 80.0).unwrap())[..4], [(0.0, 20.0), (20.0, 20.0), (20.0, 0.0), (80.0, 0.0)]);
        assert_eq!(
            corners(&outline(&preset("chevron"), 200.0, 80.0).unwrap()),
            vec![(0.0, 0.0), (160.0, 0.0), (200.0, 40.0), (160.0, 80.0), (0.0, 80.0), (40.0, 40.0)]
        );
        assert_eq!(corners(&outline(&preset("homePlate"), 200.0, 80.0).unwrap())[1..3], [(160.0, 0.0), (200.0, 40.0)]);
        // A four-point star's inner corners are an eighth of the way out; an eight-point star
        // has sixteen corners.
        let four = corners(&outline(&preset("star4"), 100.0, 100.0).unwrap());
        assert_eq!((four[0], four[2]), ((50.0, 0.0), (100.0, 50.0)));
        assert!((four[1].0 - 58.84).abs() < 0.01 && (four[1].1 - 41.16).abs() < 0.01, "{four:?}");
        assert_eq!(corners(&outline(&preset("star8"), 100.0, 100.0).unwrap()).len(), 16);
        let six = corners(&outline(&preset("star6"), 100.0, 100.0).unwrap());
        assert_eq!((six[0], six[2], six[4]), ((0.0, 25.0), (50.0, 0.0), (100.0, 25.0)), "{six:?}");
    }

    /// The handle of each thing a preset lets be adjusted sits on the corner or edge it moves,
    /// and dragging it there and back gives the value it had.
    #[test]
    fn a_handle_sits_where_its_value_puts_the_outline_and_moves_it() {
        use ShapeKind::Rectangle;
        // A parallelogram's lean: on the top edge, where the slanted side meets it.
        let lean = preset("parallelogram");
        assert_eq!(shape_handles(Rectangle, Some(&lean), None, 120.0, 80.0), vec![(20.0, 0.0)]);
        let Some(Adjusted::Preset(moved)) = move_handle(Rectangle, Some(&lean), (120.0, 80.0), 0, (40.0, -7.0)) else { panic!() };
        assert_eq!(moved.adjust, vec![("adj".to_string(), 50_000)]);
        assert_eq!(corners(&outline(&moved, 120.0, 80.0).unwrap())[1], (40.0, 0.0), "the outline follows the handle");
        // Kept within what the shape allows: no farther than the whole width, not before the corner.
        let far = move_handle(Rectangle, Some(&lean), (120.0, 80.0), 0, (500.0, 0.0));
        assert_eq!(far, Some(Adjusted::Preset(with("parallelogram", &[("adj", 150_000)]))));
        let back = move_handle(Rectangle, Some(&lean), (120.0, 80.0), 0, (-30.0, 0.0));
        assert_eq!(back, Some(Adjusted::Preset(with("parallelogram", &[("adj", 0)]))));
        // A callout's tip is its handle, and takes both of its values.
        let callout = preset("wedgeRectCallout");
        let at = shape_handles(Rectangle, Some(&callout), None, 120.0, 60.0);
        assert!(matches!(at.as_slice(), [(x, y)] if (x - 35.0).abs() < 0.01 && (y - 67.5).abs() < 0.01), "{at:?}");
        let Some(Adjusted::Preset(tip)) = move_handle(Rectangle, Some(&callout), (120.0, 60.0), 0, (180.0, 15.0)) else { panic!() };
        assert_eq!(tip.adjust, vec![("adj1".to_string(), 100_000), ("adj2".to_string(), -25_000)]);
        // A chevron's notch, a cylinder's lid, a star's inner corners.
        assert_eq!(shape_handles(Rectangle, Some(&preset("chevron")), None, 200.0, 80.0), vec![(160.0, 0.0)]);
        assert_eq!(shape_handles(Rectangle, Some(&preset("can")), None, 100.0, 80.0), vec![(50.0, 20.0)]);
        // Each handle is on a corner of the outline (a ring's, on its hole).
        for name in presets().into_iter().chain(["triangle", "rightArrow", "leftArrow", "upArrow", "downArrow"]) {
            let geom = preset(name);
            let Some(drawn) = outline(&geom, 120.0, 80.0) else {
                assert!(shape_handles(Rectangle, Some(&geom), None, 120.0, 80.0).is_empty(), "{name} isn't drawn as itself: no handle moves it");
                continue;
            };
            let on = corners(&drawn);
            for at in shape_handles(Rectangle, Some(&geom), None, 120.0, 80.0) {
                let near = on.iter().any(|c| (c.0 - at.0).abs() < 0.01 && (c.1 - at.1).abs() < 0.01);
                assert!(near || matches!(name, "donut" | "frame" | "can"), "{name}: {at:?} is on none of {on:?}");
            }
        }
        // Every handle of every preset: moved to where it is, the values stay as they are.
        for name in presets() {
            let geom = preset(name);
            for (i, at) in shape_handles(Rectangle, Some(&geom), None, 120.0, 80.0).into_iter().enumerate() {
                let Some(Adjusted::Preset(same)) = move_handle(Rectangle, Some(&geom), (120.0, 80.0), i, at) else { panic!("{name} {i}") };
                assert_eq!(outline(&same, 120.0, 80.0), outline(&geom, 120.0, 80.0), "{name}: handle {i} at {at:?}");
            }
        }
    }

    /// The shapes a kind draws take handles too: a block arrow's head and shaft (it becomes the
    /// preset it is, looking as it did), a triangle's point, a rounded rectangle's corner.
    #[test]
    fn kinds_with_something_to_adjust_have_handles() {
        // A right arrow 200 × 80: the head starts 40 from the tip, the shaft is 40% thick.
        assert_eq!(shape_handles(ShapeKind::Arrow, None, None, 200.0, 80.0), vec![(160.0, 0.0), (0.0, 24.0)]);
        let own = own_preset(ShapeKind::Arrow).unwrap();
        let unit: Vec<(f32, f32)> = corners(&outline(&own, 200.0, 80.0).unwrap()).into_iter().map(|(x, y)| (x / 200.0, y / 80.0)).collect();
        let drawn = crate::wrap::arrow_outline(ShapeKind::Arrow, 200.0, 80.0);
        for (a, b) in unit.iter().zip(&drawn) {
            assert!((a.0 - b.0).abs() < 1e-4 && (a.1 - b.1).abs() < 1e-4, "{unit:?} is not {drawn:?}");
        }
        let Some(Adjusted::Preset(longer)) = move_handle(ShapeKind::Arrow, None, (200.0, 80.0), 0, (120.0, 0.0)) else { panic!() };
        assert_eq!((longer.name.as_str(), longer.adjust.clone()), ("rightArrow", vec![("adj1".to_string(), 40_000), ("adj2".to_string(), 100_000)]));
        for kind in [ShapeKind::ArrowLeft, ShapeKind::ArrowUp, ShapeKind::ArrowDown, ShapeKind::Triangle] {
            let at = shape_handles(kind, None, None, 120.0, 90.0);
            assert!(!at.is_empty(), "{kind:?}");
            for (i, p) in at.iter().enumerate() {
                assert!(move_handle(kind, None, (120.0, 90.0), i, *p).is_some(), "{kind:?} {i}");
            }
        }
        // A rounded rectangle: its corner, on the top edge.
        assert_eq!(shape_handles(ShapeKind::RoundedRectangle, None, Some(25_000), 120.0, 80.0), vec![(20.0, 0.0)]);
        assert_eq!(move_handle(ShapeKind::RoundedRectangle, None, (120.0, 80.0), 0, (30.0, 9.0)), Some(Adjusted::Corner(37_500)));
        assert_eq!(move_handle(ShapeKind::RoundedRectangle, None, (120.0, 80.0), 0, (900.0, 0.0)), Some(Adjusted::Corner(50_000)));
        // Nothing to adjust, a handle a shape doesn't have, and frames and points that are none.
        assert!(shape_handles(ShapeKind::Rectangle, None, None, 120.0, 80.0).is_empty());
        assert!(shape_handles(ShapeKind::ElbowConnector, None, None, 120.0, 80.0).is_empty());
        assert_eq!(move_handle(ShapeKind::Arrow, None, (200.0, 80.0), 7, (0.0, 0.0)), None);
        assert!(shape_handles(ShapeKind::Arrow, None, None, f32::NAN, 80.0).is_empty());
        assert!(shape_handles(ShapeKind::RoundedRectangle, None, None, 0.0, 80.0).is_empty());
        let wild = move_handle(ShapeKind::Arrow, None, (200.0, 80.0), 0, (f32::NAN, f32::INFINITY));
        assert!(matches!(wild, Some(Adjusted::Preset(p)) if p.adjust.iter().all(|(_, v)| (0..=250_000).contains(v))));
    }

    /// The cloud, the brackets and braces, the curved arrows and the ribbons are drawn as
    /// themselves, from the standard's definitions: they were an oval or a rectangle before.
    #[test]
    fn clouds_brackets_curved_arrows_and_ribbons_have_their_own_outlines() {
        let near = |a: (f32, f32), b: (f32, f32)| (a.0 - b.0).abs() < 0.05 && (a.1 - b.1).abs() < 0.05;
        // Every point a path goes through, its curves' ends too.
        let ends = |segs: &[PathSeg]| -> Vec<(f32, f32)> {
            segs.iter()
                .filter_map(|s| match *s {
                    PathSeg::Move(x, y) | PathSeg::Line(x, y) | PathSeg::Cubic(_, _, _, _, x, y) => Some((x, y)),
                    PathSeg::Close => None,
                })
                .collect()
        };
        let closes = |segs: &[PathSeg]| segs.iter().filter(|s| matches!(s, PathSeg::Close)).count();
        let moves = |segs: &[PathSeg]| segs.iter().filter(|s| matches!(s, PathSeg::Move(..))).count();
        // A left bracket 40 × 120: from the bottom right, round to the left side, up it, round
        // to the top right. Open: nothing closes it. The right one is its mirror image.
        let left = outline(&preset("leftBracket"), 40.0, 120.0).unwrap();
        let at = ends(&left);
        let y1 = 40.0 * 0.08333;
        assert!(near(at[0], (40.0, 120.0)) && near(at[1], (0.0, 120.0 - y1)) && near(at[2], (0.0, y1)) && near(at[3], (40.0, 0.0)), "{at:?}");
        assert_eq!((closes(&left), moves(&left)), (0, 1));
        let right = ends(&outline(&preset("rightBracket"), 40.0, 120.0).unwrap());
        assert!(right.iter().zip(&at).all(|(r, l)| near(*r, (40.0 - l.0, l.1))), "{right:?}");
        // A left brace: its point is on the left side, halfway down by default, and where
        // `adj2` puts it.
        let brace = ends(&outline(&preset("leftBrace"), 40.0, 120.0).unwrap());
        assert!(
            brace.iter().any(|p| near(*p, (0.0, 60.0))) && near(brace[0], (40.0, 120.0)) && near(*brace.last().unwrap(), (40.0, 0.0)),
            "{brace:?}"
        );
        let low = ends(&outline(&with("leftBrace", &[("adj2", 75_000)]), 40.0, 120.0).unwrap());
        assert!(low.iter().any(|p| near(*p, (0.0, 90.0))), "{low:?}");
        let rb = ends(&outline(&preset("rightBrace"), 40.0, 120.0).unwrap());
        assert!(rb.iter().any(|p| near(*p, (40.0, 60.0))) && near(rb[0], (0.0, 120.0)), "{rb:?}");
        // The pairs: two open curves, one at each side.
        for (name, edge) in [("bracketPair", 0.0), ("bracePair", 0.0)] {
            let pair = outline(&preset(name), 200.0, 90.0).unwrap();
            assert_eq!((closes(&pair), moves(&pair)), (0, 2), "{name}");
            let at = ends(&pair);
            let (xs_min, xs_max) = at.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p.0), hi.max(p.0)));
            assert!(near((xs_min, 0.0), (edge, 0.0)) && near((xs_max, 0.0), (200.0, 0.0)), "{name}: {at:?}");
        }
        // The cloud: one closed outline that reaches all four sides of its frame (to within the
        // standard's own slack), and eleven short arcs on it that enclose nothing.
        let cloud = outline(&preset("cloud"), 216.0, 216.0).unwrap();
        assert_eq!((closes(&cloud), moves(&cloud)), (1, 12));
        let poly = first_polygon(&cloud);
        let (mut x0, mut x1, mut y0, mut y1) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
        for &(x, y) in &poly {
            (x0, x1, y0, y1) = (x0.min(x), x1.max(x), y0.min(y), y1.max(y));
        }
        assert!(x0.abs() < 2.0 && (x1 - 216.0).abs() < 2.0 && y0.abs() < 2.0 && (y1 - 216.0).abs() < 2.0, "{x0} {x1} {y0} {y1}");
        let first = cloud.iter().position(|s| matches!(s, PathSeg::Close)).unwrap();
        let (began, ended) = (ends(&cloud)[0], *ends(&cloud[..first]).last().unwrap());
        assert!((began.0 - ended.0).abs() < 1.0 && (began.1 - ended.1).abs() < 1.0, "the arcs around meet where they began: {began:?} {ended:?}");
        // The second arc starts where the standard starts one of the short arcs drawn on it.
        assert!(ends(&cloud).iter().take(4).any(|p| (p.0 - 70.0).abs() < 0.3 && (p.1 - 25.96).abs() < 0.3), "{:?}", &ends(&cloud)[..4]);
        // The arrow that curves right, 100 × 100: its tip is on the right side, three quarters
        // down (half the head's width from the bottom); the band starts at the left, a quarter
        // down. Left is its mirror image; down is it on its side, tip on the bottom; up, on top.
        let tip = |name: &str, at: (f32, f32)| {
            let segs = outline(&preset(name), 100.0, 100.0).unwrap();
            assert_eq!((closes(&segs), moves(&segs)), (2, 2), "{name}");
            assert!(ends(&segs).iter().any(|p| near(*p, at)), "{name}: no corner at {at:?} in {:?}", ends(&segs));
            for (x, y) in ends(&segs) {
                assert!((-0.05..=100.05).contains(&x) && (-0.05..=100.05).contains(&y), "{name}: ({x}, {y}) is outside the frame");
            }
        };
        tip("curvedRightArrow", (100.0, 75.0));
        tip("curvedRightArrow", (0.0, 31.25));
        tip("curvedLeftArrow", (0.0, 75.0));
        tip("curvedDownArrow", (75.0, 100.0));
        tip("curvedUpArrow", (75.0, 0.0));
        // A ribbon 320 × 60: its ends are notched an eighth of the width in, its front panel is
        // the middle half, and it hangs a sixth below the ends' top. The other is upside down.
        let ribbon = outline(&preset("ribbon"), 320.0, 60.0).unwrap();
        assert_eq!((closes(&ribbon), moves(&ribbon)), (3, 5), "the outline, the backs of its two rolls, and two folds");
        let at = ends(&ribbon);
        for want in [(0.0, 0.0), (40.0, 25.0), (0.0, 50.0), (320.0, 0.0), (280.0, 25.0), (320.0, 50.0), (80.0, 50.0), (240.0, 50.0)] {
            assert!(at.iter().any(|p| near(*p, want)), "no corner at {want:?} in {at:?}");
        }
        let up = ends(&outline(&preset("ribbon2"), 320.0, 60.0).unwrap());
        assert!(up.iter().zip(&at).all(|(u, d)| near(*u, (d.0, 60.0 - d.1))));
        assert_eq!(text_insets(&preset("ribbon"), 320.0, 60.0), Some([90.0, 60.0 * 0.16667, 90.0, 0.0]));
        // Values out of range, and frames too thin for the shape, still draw finite paths.
        for name in ["cloud", "leftBracket", "rightBrace", "bracePair", "bracketPair", "curvedRightArrow", "curvedUpArrow", "ribbon", "ribbon2"] {
            for adjust in [i32::MIN, -1, 0, 1, 99_999, i32::MAX] {
                let geom = with(name, &[("adj", adjust), ("adj1", adjust), ("adj2", adjust), ("adj3", adjust)]);
                for (w, h) in [(1.0, 1000.0), (1000.0, 1.0), (0.01, 0.01), (50.0, 50.0)] {
                    let segs = outline(&geom, w, h).unwrap();
                    assert!(ends(&segs).iter().all(|p| p.0.is_finite() && p.1.is_finite()), "{name} {adjust} {w}×{h}");
                }
            }
        }
    }

    #[test]
    fn a_presets_outline_is_also_a_freeforms_paths() {
        let f = freeform(&preset("cube"), 100.0, 80.0).unwrap();
        assert_eq!((f.w, f.h, f.paths.len()), (100.0, 80.0, 4), "the cube and its three inner edges");
        assert_eq!(f.paths.iter().map(|p| p.closed).collect::<Vec<_>>(), vec![true, false, false, false]);
        assert_eq!(f.paths[0].pts, vec![[0.0, 20.0], [20.0, 0.0], [100.0, 0.0], [100.0, 60.0], [80.0, 80.0], [0.0, 80.0]]);
        // Curves become points along them.
        let oval = freeform(&preset("donut"), 100.0, 80.0).unwrap();
        assert_eq!(oval.paths.len(), 2);
        assert!(oval.paths[0].pts.len() > 30 && oval.paths.iter().all(|p| p.closed));
        assert_eq!(freeform(&preset("sun"), 100.0, 80.0), None);
    }

    #[test]
    fn flowchart_boxes_carry_their_lines() {
        let moves = |name: &str| outline(&preset(name), 80.0, 40.0).unwrap().iter().filter(|s| matches!(s, PathSeg::Move(..))).count();
        assert_eq!(moves("flowChartPredefinedProcess"), 3, "the box and a line inside each end");
        assert_eq!(moves("flowChartInternalStorage"), 3);
        assert_eq!(moves("flowChartSort"), 2);
        assert_eq!(moves("flowChartSummingJunction"), 3);
        assert_eq!(moves("cube"), 4);
        assert_eq!(moves("can"), 2);
        // A terminator's ends are half ellipses 3475/21600 of the width deep.
        let ends = outline(&preset("flowChartTerminator"), 216.0, 100.0).unwrap();
        assert_eq!(ends[..2], [PathSeg::Move(34.75, 0.0), PathSeg::Line(181.25, 0.0)]);
        assert!(matches!(ends[3], PathSeg::Cubic(.., x, y) if (x - 181.25).abs() < 0.01 && (y - 100.0).abs() < 0.01), "{ends:?}");
        // A document's lower edge is a wave ending above the frame's bottom on the left.
        let doc = outline(&preset("flowChartDocument"), 216.0, 216.0).unwrap();
        assert!(matches!(doc[3], PathSeg::Cubic(_, _, _, c2y, x, y) if x == 0.0 && (y - 201.72).abs() < 0.01 && c2y > 216.0), "{doc:?}");
        // Stored data is hollowed on the right, by as much as it bulges on the left.
        let stored = outline(&preset("flowChartOnlineStorage"), 60.0, 40.0).unwrap();
        assert!(stored.iter().any(|s| matches!(s, PathSeg::Cubic(.., x, y) if (*x - 50.0).abs() < 0.01 && (*y - 20.0).abs() < 0.01)), "{stored:?}");
    }

    #[test]
    fn a_callouts_tail_leaves_the_edge_its_tip_is_beyond() {
        // The default tip: below the body, left of the middle.
        let c = corners(&outline(&preset("wedgeRectCallout"), 120.0, 60.0).unwrap());
        let tip = (60.0 - 25.0, 30.0 + 37.5);
        let i = c.iter().position(|p| *p == tip).unwrap_or_else(|| panic!("{c:?}"));
        assert_eq!((c[i - 1], c[i + 1]), ((50.0, 60.0), (20.0, 60.0)), "its foot is on the bottom edge, in the left half: {c:?}");
        // A tip to the right: the foot is on the right edge.
        let right = with("wedgeRectCallout", &[("adj1", 90_000), ("adj2", 10_000)]);
        let c = corners(&outline(&right, 120.0, 60.0).unwrap());
        let i = c.iter().position(|p| *p == (168.0, 36.0)).unwrap();
        assert_eq!((c[i - 1], c[i + 1]), ((120.0, 35.0), (120.0, 50.0)), "{c:?}");
        // The oval callout starts at its tip and goes round the oval.
        let oval = outline(&preset("wedgeEllipseCallout"), 120.0, 60.0).unwrap();
        assert!(matches!(oval[0], PathSeg::Move(x, y) if (x - 35.0).abs() < 0.01 && (y - 67.5).abs() < 0.01), "{oval:?}");
        assert!(oval.len() >= 6 && oval.contains(&PathSeg::Close));
    }

    #[test]
    fn text_keeps_inside_the_outline_and_clear_of_a_boxs_lines() {
        for name in presets() {
            let geom = preset(name);
            for (w, h) in [(120.0, 80.0), (60.0, 150.0)] {
                let Some([l, t, r, b]) = text_insets(&geom, w, h) else { continue };
                assert!(l >= 0.0 && t >= 0.0 && r >= 0.0 && b >= 0.0 && l + r < w && t + b < h, "{name}: {l} {t} {r} {b}");
                // No rectangle about the middle fits (a frame's opening, a narrow chevron's
                // notch): the text is laid across the frame, as for a shape with no outline.
                if matches!(name, "flowChartCollate") || [l, t, r, b] == [0.0; 4] {
                    continue;
                }
                let poly = first_polygon(&outline(&geom, w, h).unwrap());
                for pt in [(l + 0.5, t + 0.5), (w - r - 0.5, t + 0.5), (w - r - 0.5, h - b - 0.5), (l + 0.5, h - b - 0.5)] {
                    assert!(inside(&poly, pt), "{name} {w}x{h}: the text corner {pt:?} is outside");
                }
            }
        }
        assert_eq!(text_insets(&preset("flowChartPredefinedProcess"), 80.0, 40.0), Some([10.0, 0.0, 10.0, 0.0]));
        // A hexagon's text keeps clear of its slanted sides; a plain box's fills it.
        let [l, t, ..] = text_insets(&preset("hexagon"), 120.0, 80.0).unwrap();
        assert!(l > 5.0 && t > 5.0, "{l} {t}");
        assert_eq!(text_insets(&preset("sun"), 80.0, 40.0), None);
    }

    #[test]
    fn connectors_glue_to_the_outline_not_to_the_frame() {
        // A parallelogram's left and right sites are on its slanted sides.
        let sites = side_sites(&preset("parallelogram"), 120.0, 80.0).unwrap();
        assert_eq!(sites, vec![(60.0, 0.0), (10.0, 40.0), (60.0, 80.0), (110.0, 40.0)]);
        // A chevron's left site is in its notch; a merge triangle's sides are halfway in.
        assert_eq!(side_sites(&preset("chevron"), 200.0, 80.0).unwrap()[1], (40.0, 40.0));
        assert_eq!(side_sites(&preset("flowChartMerge"), 100.0, 80.0).unwrap(), vec![(50.0, 0.0), (25.0, 40.0), (50.0, 80.0), (75.0, 40.0)]);
        assert_eq!(side_sites(&preset("sun"), 100.0, 80.0), None);
        // And text wraps around the outline.
        let unit = unit_outline(&preset("parallelogram"), 120.0, 80.0).unwrap();
        assert_eq!(unit, vec![(0.0, 1.0), (1.0 / 6.0, 0.0), (1.0, 0.0), (5.0 / 6.0, 1.0)]);
    }
}
