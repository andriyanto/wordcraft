use super::*;
use wordcraft_doc::connector::ShapeExtra;
use wordcraft_doc::freeform::{FreePath, Freeform, InkTool};

#[test]
fn freeform_fill_has_own_alpha() {
    let rect = Rect::new(4.0, 4.0, 24.0, 24.0);
    for transparency in [0.0, 50.0, 100.0] {
        for closed in [false, true] {
            let freeform = Freeform {
                w: 24.0,
                h: 24.0,
                paths: vec![FreePath { pts: vec![[0.0, 0.0], [24.0, 0.0], [24.0, 24.0]], closed }],
                alpha: 0.25,
                ..Default::default()
            };
            let mut out = Vec::new();
            shape_draws(
                rect,
                &ShapeLook {
                    kind: ShapeKind::Freeform,
                    fill: Some(&wordcraft_doc::fill::Fill::Solid { color: Rgb::BLACK, transparency }),
                    stroke: Some(Rgb::WHITE),
                    stroke_width: 2.0,
                    effects: Default::default(),
                    freeform: Some(&freeform),
                    extra: &ShapeExtra::default(),
                },
                &DisplayOptions::default(),
                &mut out,
            );
            let Draw::Path { fill, stroke, stroke_width, .. } = &out[0] else { panic!("expected freeform path") };
            match fill {
                Some(Paint::Solid(color, alpha)) if closed => {
                    assert_eq!(*color, Rgb::BLACK);
                    assert_eq!(*alpha, 1.0 - transparency / 100.0);
                }
                None if !closed => {}
                _ => panic!("unexpected freeform paint: {fill:?}"),
            }
            assert_eq!((*stroke, *stroke_width), (Some(Rgb::WHITE), 2.0));
        }
    }
    let ink = Freeform::ink(InkTool::Highlighter, 24.0, 24.0, vec![[0.0, 0.0], [24.0, 24.0]]);
    let mut out = Vec::new();
    shape_draws(
        rect,
        &ShapeLook {
            kind: ShapeKind::Freeform,
            fill: Some(&wordcraft_doc::fill::Fill::Solid { color: Rgb::BLACK, transparency: 100.0 }),
            stroke: Some(Rgb::WHITE),
            stroke_width: 2.0,
            effects: Default::default(),
            freeform: Some(&ink),
            extra: &ShapeExtra::default(),
        },
        &DisplayOptions::default(),
        &mut out,
    );
    assert!(matches!(out.as_slice(), [Draw::Ink { alpha, .. }] if *alpha == 0.5));
}

#[test]
fn other_path_alpha_defaults() {
    let rect = Rect::new(4.0, 4.0, 24.0, 24.0);
    let graphic = Graphic {
        items: vec![GraphicItem::Path {
            segs: vec![PathSeg::Move(0.0, 0.0), PathSeg::Line(24.0, 24.0)],
            fill: Some(Rgb::BLACK),
            stroke: None,
            stroke_width: 0.0,
        }],
        ..Default::default()
    };
    let mut out = graphic_draws(&Document::new(), &graphic, rect, 1.0);
    out.push(path_arrowhead(&[(4.0, 4.0), (28.0, 28.0)], Rgb::BLACK, 2.0, ArrowStyle::default()).unwrap());
    let extra = ShapeExtra { arrow_start: true, arrow_end: true, ..Default::default() };
    line_draws(rect, ShapeKind::ElbowConnector, Some(Rgb::BLACK), 2.0, Default::default(), &extra, &mut out);
    assert_eq!(out.len(), 5);
    assert!(out.iter().all(
        |item| matches!(item, Draw::Path { fill, .. } if fill.as_ref().is_none_or(|paint| matches!(paint, Paint::Solid(_, alpha) if *alpha == 1.0)))
    ));
}

/// A connector is drawn as the preset it keeps, and its arrowheads point along its first and
/// last runs. Every bent connector was the three-run one before, its heads level: a one-corner
/// elbow from a file was drawn with a run it doesn't have, and a head that arrives downwards
/// pointed sideways.
#[test]
fn a_connector_is_drawn_as_its_preset_with_heads_along_its_runs() {
    use wordcraft_doc::connector::PresetGeom;
    let rect = Rect::new(10.0, 20.0, 100.0, 40.0);
    let draws = |preset: Option<PresetGeom>| {
        let extra = ShapeExtra { arrow_end: true, preset: preset.map(std::sync::Arc::new), ..Default::default() };
        let mut out = Vec::new();
        line_draws(rect, ShapeKind::ElbowConnector, Some(Rgb::BLACK), 1.0, Default::default(), &extra, &mut out);
        let [Draw::Path { segs: line, .. }, Draw::Path { segs: head, .. }] = out.as_slice() else { panic!("{out:?}") };
        (line.clone(), head.clone())
    };
    use PathSeg::{Line, Move};
    // The plain elbow: out, down the middle, in; the head points right.
    let (line, head) = draws(None);
    assert_eq!(line, vec![Move(10.0, 20.0), Line(60.0, 20.0), Line(60.0, 60.0), Line(110.0, 60.0)]);
    assert!(matches!(head.as_slice(), [Move(x, y), Line(bx, _), ..] if (*x, *y) == (110.0, 60.0) && *bx < 110.0), "{head:?}");
    // One corner: the head points down.
    let (line, head) = draws(PresetGeom::new("bentConnector2", []));
    assert_eq!(line, vec![Move(10.0, 20.0), Line(110.0, 20.0), Line(110.0, 60.0)]);
    assert!(matches!(head.as_slice(), [Move(x, y), Line(_, by), ..] if (*x, *y) == (110.0, 60.0) && *by < 60.0), "{head:?}");
    // A run past the end: the connector comes back to it, and the head points back.
    let (line, head) = draws(PresetGeom::new("bentConnector3", [("adj1".to_string(), 120_000)]));
    assert_eq!(line, vec![Move(10.0, 20.0), Line(130.0, 20.0), Line(130.0, 60.0), Line(110.0, 60.0)]);
    assert!(matches!(head.as_slice(), [Move(x, y), Line(bx, _), ..] if (*x, *y) == (110.0, 60.0) && *bx > 110.0), "{head:?}");
}
