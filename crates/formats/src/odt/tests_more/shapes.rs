//! Drawing shapes (ODF 1.3 §10.3: `draw:rect`, `draw:ellipse`, `draw:line`, and
//! `draw:custom-shape` with its `draw:enhanced-geometry` type): opened as WordCraft shapes with
//! their size, placement, fill, line and text, and written back as the same elements.

use crate::odt::tests::{NS_T, content_of, package};
use crate::odt::{export, import};
use wordcraft_doc::para::{InlineObject, OBJ, ShapeKind, Wrap};
use wordcraft_doc::props::Rgb;
use wordcraft_doc::{Document, Paragraph, para_block};

const NS_D: &str =
    r#"xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0" xmlns:svg="urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0""#;

/// A shape as read: (kind, size, fill, line colour and width, floats, drawn bottom-left to top-right, its text).
type Seen = (ShapeKind, (f32, f32), Option<Rgb>, Option<(Rgb, f32)>, bool, bool, String);

fn shapes(d: &Document) -> Vec<Seen> {
    let mut out = Vec::new();
    for b in d.body.iter() {
        let Some(p) = b.as_para() else { continue };
        for o in &p.objects {
            if let InlineObject::Shape { kind, w, h, fill, stroke, stroke_width, float, story, .. } = o {
                let text = story
                    .and_then(|id| d.parts.get(&id))
                    .map(|part| part.blocks.iter().filter_map(|b| b.as_para()).map(|p| p.text.clone()).collect::<Vec<_>>().join("|"))
                    .unwrap_or_default();
                let size = ((*w * 10.0).round() / 10.0, (*h * 10.0).round() / 10.0);
                let line = stroke.map(|c| (c, (*stroke_width * 100.0).round() / 100.0));
                out.push((*kind, size, fill.as_ref().map(|f| f.main_color()), line, float.wrap != Wrap::Inline, float.flip_h != float.flip_v, text));
            }
        }
    }
    out
}

fn open() -> Document {
    let styles = format!(
        r##"<office:document-styles {NS_T} {NS_D}><office:styles><style:default-style style:family="graphic"><style:graphic-properties svg:stroke-color="#3465a4" draw:fill-color="#729fcf"/></style:default-style></office:styles></office:document-styles>"##
    );
    let content = format!(
        r##"<office:document-content {NS_T} {NS_D}><office:automatic-styles>
<style:style style:name="gr1" style:family="graphic"><style:graphic-properties svg:stroke-width="0.5pt" svg:stroke-color="#0000ff" draw:fill="solid" draw:fill-color="#ffcc00"/></style:style>
<style:style style:name="gr2" style:family="graphic"><style:graphic-properties draw:fill="solid" draw:fill-color="#00aa00"/></style:style>
<style:style style:name="gr3" style:family="graphic"><style:graphic-properties svg:stroke-color="#000080" draw:fill-color="#99ccff" style:wrap="parallel" style:horizontal-pos="from-left" style:horizontal-rel="paragraph" style:vertical-pos="from-top" style:vertical-rel="paragraph"/></style:style>
<style:style style:name="gr4" style:family="graphic"><style:graphic-properties svg:stroke-width="2pt" svg:stroke-color="#000000" style:wrap="run-through" style:run-through="background"/></style:style>
<style:style style:name="gr5" style:family="graphic"><style:graphic-properties draw:stroke="none" draw:fill="none"/></style:style>
</office:automatic-styles><office:body><office:text>
<text:p>A <draw:rect text:anchor-type="as-char" draw:style-name="gr1" svg:width="1in" svg:height="0.5in"><text:p/></draw:rect> b <draw:ellipse text:anchor-type="as-char" draw:style-name="gr2" svg:width="0.5in" svg:height="0.5in"/> c.</text:p>
<text:p><draw:custom-shape text:anchor-type="paragraph" draw:style-name="gr3" svg:width="2in" svg:height="1in" svg:x="1in" svg:y="0in"><text:p>Shape text</text:p><draw:enhanced-geometry svg:viewBox="0 0 21600 21600" draw:type="round-rectangle" draw:modifiers="3600"><draw:equation draw:name="f0" draw:formula="45"/></draw:enhanced-geometry></draw:custom-shape>Beside.</text:p>
<text:p><draw:line text:anchor-type="paragraph" draw:style-name="gr4" svg:x1="3in" svg:y1="0in" svg:x2="1in" svg:y2="1in"><text:p/></draw:line><draw:custom-shape text:anchor-type="paragraph" draw:style-name="gr5" svg:width="1in" svg:height="1in"><draw:enhanced-geometry draw:type="ooxml-heart"/></draw:custom-shape><draw:custom-shape text:anchor-type="paragraph" draw:style-name="gr5" svg:width="1in" svg:height="1in"><draw:enhanced-geometry draw:type="mso-spt100" draw:enhanced-path="M 0 0 L 21600 21600 N"/></draw:custom-shape>Under.</text:p>
</office:text></office:body></office:document-content>"##
    );
    import(&package(&content, Some(&styles))).unwrap()
}

#[test]
fn shapes_open_with_their_kind_size_look_and_text() {
    let d = open();
    // A line width no style states (the standard gives none): WordCraft's 0.25 pt.
    let blue = Some((Rgb(0x34, 0x65, 0xa4), 0.25));
    assert_eq!(
        shapes(&d),
        [
            (ShapeKind::Rectangle, (72.0, 36.0), Some(Rgb(255, 204, 0)), Some((Rgb(0, 0, 255), 0.5)), false, false, String::new()),
            // No line stated: the default graphic style's, a hairline.
            (ShapeKind::Ellipse, (36.0, 36.0), Some(Rgb(0, 170, 0)), blue, false, false, String::new()),
            // No fill stated: a solid fill in the style's fill colour.
            (
                ShapeKind::RoundedRectangle,
                (144.0, 72.0),
                Some(Rgb(0x99, 0xcc, 0xff)),
                Some((Rgb(0, 0, 0x80), 0.25)),
                true,
                false,
                "Shape text".into()
            ),
            // From (3in, 0) to (1in, 1in): 2in × 1in, rising from its bottom left.
            (ShapeKind::Line, (144.0, 72.0), None, Some((Rgb(0, 0, 0), 2.0)), true, true, String::new()),
            (ShapeKind::Heart, (72.0, 72.0), None, None, true, false, String::new()),
            // A geometry WordCraft has no shape for: drawn from its own path.
            (ShapeKind::Freeform, (72.0, 72.0), None, None, true, false, String::new()),
        ]
    );
    // The shape's text isn't the paragraph's.
    assert_eq!(d.body[1].as_para().unwrap().text.replace(OBJ, "#"), "#Beside.");
    assert_eq!(d.body[0].as_para().unwrap().text.replace(OBJ, "#"), "A # b # c.");
    // The line starts at its leftmost point.
    let Some(InlineObject::Shape { float, .. }) = d.body[2].as_para().unwrap().objects.first() else { panic!() };
    assert_eq!((float.x, float.y), (72.0, 0.0));
    assert_eq!(float.wrap, Wrap::BehindText);
}

#[test]
fn shapes_round_trip() {
    let d = open();
    let bytes = crate::odt::tests::export_model(&d).unwrap();
    let c = content_of(&bytes);
    for el in ["<draw:rect ", "<draw:ellipse ", "<draw:line ", r#"draw:type="round-rectangle""#, r#"draw:type="heart""#] {
        assert!(c.contains(el), "{el} in {c}");
    }
    let back = import(&bytes).unwrap();
    assert_eq!(shapes(&back), shapes(&d));
    assert_eq!(back.body[1].as_para().unwrap().text, d.body[1].as_para().unwrap().text);
}

#[test]
fn every_wordcraft_shape_kind_is_written_and_read_back() {
    use ShapeKind::*;
    let kinds = [
        Rectangle,
        RoundedRectangle,
        Ellipse,
        FlowchartOr,
        Triangle,
        RightTriangle,
        Diamond,
        Line,
        Arrow,
        ArrowLeft,
        ArrowUp,
        ArrowDown,
        Star,
        Heart,
    ];
    let mut p = Paragraph::new();
    for k in kinds {
        p.text.push(OBJ);
        p.objects.push(InlineObject::Shape {
            kind: k,
            w: 50.0,
            h: 40.0,
            fill: Some(wordcraft_doc::fill::Fill::solid(Rgb(1, 2, 3))),
            stroke: Some(Rgb(4, 5, 6)),
            stroke_width: 1.5,
            float: Default::default(),
            story: None,
            freeform: None,
            effects: Default::default(),
            extra: Default::default(),
        });
    }
    p.runs = vec![wordcraft_doc::para::Run { len: p.text.len(), props: Default::default() }];
    let mut d = Document::new();
    d.body = vec![para_block(p)];
    let back = import(&export(&d).unwrap()).unwrap();
    let fill = |k| if k == Line { None } else { Some(Rgb(1, 2, 3)) };
    assert_eq!(shapes(&back), kinds.map(|k| (k, (50.0, 40.0), fill(k), Some((Rgb(4, 5, 6), 1.5)), false, false, String::new())).to_vec());
}

#[test]
fn hostile_shapes_do_not_panic() {
    let deep = r#"<draw:custom-shape svg:width="1in" svg:height="1in"><text:p>"#.repeat(100);
    for inner in [
        deep.as_str(),
        r#"<draw:rect svg:width="-5in" svg:height="NaNin"><text:p>unclosed"#,
        r#"<draw:line svg:x1="1e30in" svg:y1="0" svg:x2="-1e30in"/>"#,
        r#"<draw:rect svg:width="1in" svg:height="1in"><text:p>a<text:note><text:note-body><text:p>n</text:p></text:note-body></text:note></text:p></draw:rect>"#,
        r#"<text:note><text:note-body><text:p><draw:ellipse svg:width="1in" svg:height="1in"><text:p>in a note</text:p></draw:ellipse></text:p></text:note-body></text:note>"#,
    ] {
        let content = format!(
            r#"<office:document-content {NS_T} {NS_D}><office:body><office:text><text:p>x{inner}</text:p></office:text></office:body></office:document-content>"#
        );
        let d = import(&package(&content, None)).unwrap();
        let _ = export(&d).unwrap();
    }
}

#[test]
fn a_shape_keeps_its_size_unless_it_grows_to_its_text() {
    let content = format!(
        r#"<office:document-content {NS_T} {NS_D}><office:automatic-styles><style:style style:name="g" style:family="graphic"><style:graphic-properties draw:auto-grow-height="true"/></style:style></office:automatic-styles><office:body><office:text><text:p><draw:rect svg:width="1in" svg:height="1in"><text:p>fixed</text:p></draw:rect><draw:rect draw:style-name="g" svg:width="1in" svg:height="1in"><text:p>grows</text:p></draw:rect></text:p></office:text></office:body></office:document-content>"#
    );
    let d = import(&package(&content, None)).unwrap();
    let fits = |d: &Document| -> Vec<bool> {
        d.body[0].as_para().unwrap().objects.iter().filter_map(|o| o.text_box()).map(|id| d.parts.get(&id).unwrap().body.auto_fit).collect()
    };
    assert_eq!(fits(&d), [false, true]);
    let bytes = crate::odt::tests::export_model(&d).unwrap();
    // LibreOffice resizes a shape with text to it unless told not to.
    assert!(content_of(&bytes).contains(r#"draw:auto-grow-height="false" draw:auto-grow-width="false""#));
    assert_eq!(fits(&import(&bytes).unwrap()), [false, true]);
}

#[test]
fn a_shape_with_only_an_empty_paragraph_has_no_text() {
    // LibreOffice writes `<text:p/>` in every shape; only the round rectangle holds text.
    let d = open();
    let stories: Vec<bool> = d.body.iter().filter_map(|b| b.as_para()).flat_map(|p| p.objects.iter()).map(|o| o.text_box().is_some()).collect();
    assert_eq!(stories, [false, false, true, false, false, false]);
}

/// A shape's text keeps where it sits in the shape (top, middle, bottom) and the colour the
/// shape gives it (white on a dark shape), and a connector's label stays its connector's. None
/// of the three was written: reopened, the text was at the top in black, and the label was an
/// ordinary text box the connector left behind.
#[test]
fn a_shapes_text_keeps_its_place_and_colour_and_a_label_its_connector() {
    use wordcraft_doc::connector::ShapeExtra;
    use wordcraft_doc::para::Float;
    use wordcraft_doc::props::VAlign;
    let mut d = Document::new();
    let story = |d: &mut Document, text: &str, anchor: VAlign, color: Option<Rgb>| -> u32 {
        let mut p = Paragraph::new();
        p.text.push_str(text);
        p.runs = vec![wordcraft_doc::para::Run { len: p.text.len(), props: Default::default() }];
        let id = d.try_add_part(wordcraft_doc::PartKind::TextBox, vec![para_block(p)]).unwrap();
        let part = d.parts.get_mut(&id).unwrap();
        part.body.anchor = anchor;
        part.body.auto_color = color;
        id
    };
    let shape = |kind: ShapeKind, x: f32, story: Option<u32>, extra: ShapeExtra| InlineObject::Shape {
        kind,
        w: 90.0,
        h: 60.0,
        fill: (!kind.is_open() && kind != ShapeKind::TextBox).then(|| wordcraft_doc::fill::Fill::solid(Rgb(0x15, 0x60, 0x82))),
        stroke: (kind != ShapeKind::TextBox).then_some(Rgb(4, 5, 6)),
        stroke_width: 1.0,
        float: Float { wrap: Wrap::InFrontOfText, x, y: 20.0, ..Default::default() },
        story,
        freeform: None,
        effects: Default::default(),
        extra,
    };
    let white = Some(Rgb(255, 255, 255));
    let stories = [
        story(&mut d, "Middle", VAlign::Center, white),
        story(&mut d, "Bottom", VAlign::Bottom, None),
        story(&mut d, "Top", VAlign::Top, Some(Rgb(0xAA, 0x11, 0x22))),
        story(&mut d, "Label", VAlign::Center, None),
    ];
    let mut p = Paragraph::new();
    let objects = [
        shape(ShapeKind::Rectangle, 0.0, Some(stories[0]), ShapeExtra { id: 7, ..Default::default() }),
        shape(ShapeKind::Ellipse, 200.0, Some(stories[1]), ShapeExtra { id: 9, ..Default::default() }),
        shape(ShapeKind::TextBox, 100.0, Some(stories[2]), Default::default()),
        shape(
            ShapeKind::StraightConnector,
            90.0,
            None,
            ShapeExtra {
                id: 12,
                start: Some(wordcraft_doc::connector::ConnEnd { id: 7, site: 3 }),
                end: Some(wordcraft_doc::connector::ConnEnd { id: 9, site: 1 }),
                ..Default::default()
            },
        ),
        shape(ShapeKind::TextBox, 110.0, Some(stories[3]), ShapeExtra { label_of: Some(12), ..Default::default() }),
    ];
    for o in objects {
        p.text.push(OBJ);
        p.objects.push(o);
    }
    p.runs = vec![wordcraft_doc::para::Run { len: p.text.len(), props: Default::default() }];
    d.body = vec![para_block(p)];

    /// Each shape with text: its text, where it sits, the colour the shape gives it; then each
    /// connector's drawing id, and the connector each label names.
    type Kept = (Vec<(String, VAlign, Option<Rgb>)>, Vec<u32>, Vec<u32>);
    fn kept(d: &Document) -> Kept {
        let (mut texts, mut connectors, mut labels) = (Vec::new(), Vec::new(), Vec::new());
        for o in d.body.iter().filter_map(|b| b.as_para()).flat_map(|p| &p.objects) {
            let InlineObject::Shape { kind, story, extra, .. } = o else { continue };
            if let Some(part) = story.and_then(|id| d.parts.get(&id)) {
                let text = part.blocks.iter().filter_map(|b| b.as_para()).map(|p| p.text.clone()).collect::<Vec<_>>().join("|");
                texts.push((text, part.body.anchor, part.body.auto_color));
            }
            if kind.is_connector() {
                connectors.push(extra.id);
            }
            labels.extend(extra.label_of);
        }
        (texts, connectors, labels)
    }
    let file = export(&d).unwrap();
    let out = content_of(&file);
    // As ODF says them: the text area's alignment, the shape style's text colour, and the
    // label's name after its connector's id.
    assert!(out.contains(r#"draw:textarea-vertical-align="middle""#) && out.contains(r#"draw:textarea-vertical-align="bottom""#), "{out}");
    assert!(out.contains(r##"<style:text-properties fo:color="#ffffff"/></style:style>"##), "{out}");
    assert!(out.contains(r##"<style:text-properties fo:color="#aa1122"/></style:style>"##), "{out}");
    assert!(out.contains(r#"draw:name="Connector Label shape12""#) && out.contains(r#"xml:id="shape12""#), "{out}");
    assert!(!out.contains('\u{1}'), "the marker is not written");
    let back = import(&file).unwrap();
    let (texts, connectors, labels) = kept(&back);
    assert_eq!(
        texts,
        [
            ("Middle".to_string(), VAlign::Center, white),
            ("Bottom".to_string(), VAlign::Bottom, None),
            ("Top".to_string(), VAlign::Top, Some(Rgb(0xAA, 0x11, 0x22))),
            ("Label".to_string(), VAlign::Center, None),
        ]
    );
    assert_eq!(connectors.len(), 1);
    assert_eq!(labels, connectors, "the label names the connector, whatever its id is now");
    assert_ne!(connectors, [0]);
    // And again: nothing drifts on a second save.
    assert_eq!(kept(&import(&export(&back).unwrap()).unwrap()), kept(&back));
    // A label naming a connector that isn't there, and names with nothing or junk after the
    // words, are plain text boxes.
    for name in ["Connector Label ", "Connector Label", "Connector Label \u{0}\u{1}"] {
        let content = format!(
            r#"<office:document-content {NS_T} {NS_D}><office:body><office:text><text:p><draw:frame draw:name="{name}" svg:width="1in" svg:height="1in"><draw:text-box><text:p>x</text:p></draw:text-box></draw:frame></text:p></office:text></office:body></office:document-content>"#,
            name = name.replace('\u{0}', "&#x9;").replace('\u{1}', "&#xA;")
        );
        let d = import(&package(&content, None)).unwrap();
        let (texts, _, labels) = kept(&d);
        assert_eq!((texts.len(), labels.len()), (1, 0), "{name:?}");
    }
}
