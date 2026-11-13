//! A long unbroken word after a tab.

use crate::*;
use wordcraft_doc::props::{TabAlign, TabStop};
use wordcraft_doc::{Pos, StoryRef};

fn lines_of(text: &str) -> Vec<(usize, usize)> {
    crate::test_fonts();
    let mut d = Document::from_text(text);
    let at = Pos::body(0, 0);
    d.format_paragraphs(&at, &at, &|p| {
        p.tabs = Some(vec![TabStop { pos: 100.0, align: TabAlign::Left, leader: Default::default() }]);
        p.indent_left = Some(100.0);
        p.indent_first = Some(-100.0);
    })
    .unwrap();
    let mut c = LayoutCache::new();
    let l = layout(&d, &mut c, &LayoutOptions::default());
    for p in &l.pages {
        for it in &p.items {
            if let Placed::Lines { story: StoryRef::Body, para, .. } = it {
                return para.lines.iter().map(|l| (l.c0, l.c1)).collect();
            }
        }
    }
    panic!("no paragraph laid out");
}

/// A word too long for any line is split wherever it starts, so it starts after the tab rather
/// than leaving the first line with only its label.
#[test]
fn a_word_too_long_for_a_line_starts_after_the_tab() {
    let lines = lines_of(&format!("Name\t{}", "s".repeat(400)));
    assert!(lines.len() > 2, "the word takes several lines: {lines:?}");
    let (c0, c1) = lines[0];
    assert!(c1 - c0 > 20, "the first line holds the label, the tab and the word's start: {lines:?}");
}

/// A word that fits a line of its own still moves there whole.
#[test]
fn a_word_that_fits_the_next_line_still_wraps_whole() {
    let lead = "word ".repeat(12);
    let lines = lines_of(&format!("Name\t{lead}\t{}", "s".repeat(30)));
    assert!(lines.len() >= 2, "{lines:?}");
    let last = lines[lines.len() - 1];
    assert_eq!(last.1 - last.0, 30, "the word is whole on the last line: {lines:?}");
}
