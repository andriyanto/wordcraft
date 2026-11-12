//! Paginating from where the document changed ([`crate::resume`]): the pages are those of a walk
//! from the first block, whatever was kept.

use super::*;
use crate::{TypingWord, resumed_passes, same_layout};

/// Twenty-odd pages of paragraphs of a few lines each.
fn long_doc() -> Document {
    let text: Vec<String> = (0..400).map(|i| format!("Paragraph {i} {}", "lorem ipsum dolor sit amet ".repeat(1 + i % 9))).collect();
    pagination_fixture_document(&text.join("\n"))
}

fn retype(d: &mut Document, block: usize, text: &str) -> u64 {
    let p = d.para_mut(StoryRef::Body, &Path::top(block)).unwrap();
    p.text.push_str(text);
    p.touch();
    p.rev
}

/// `got` passes started from a mark, where `want` should have: not counted when every layout is
/// made twice (`WORDCRAFT_LAYOUT_RESUME=1`), which starts passes of its own from marks.
#[track_caller]
fn started(got: u64, want: u64) {
    if std::env::var_os("WORDCRAFT_LAYOUT_RESUME").is_none() {
        assert_eq!(got, want, "passes started from a mark");
    }
}

/// The layout with what `cache` keeps, checked against the layout from scratch; how many passes
/// started from a mark.
fn relay(d: &Document, cache: &mut LayoutCache, opts: &LayoutOptions) -> (DocLayout, u64) {
    let before = resumed_passes();
    let l = layout(d, cache, opts);
    let resumed = resumed_passes() - before;
    crate::resume_passes(false);
    let scratch = layout(d, &mut LayoutCache::new(), opts);
    crate::resume_passes(true);
    if let Err(why) = same_layout(&l, &scratch) {
        panic!("not the layout from scratch: {why}");
    }
    (l, resumed)
}

#[test]
fn an_edit_is_laid_out_from_the_mark_before_it() {
    let mut d = long_doc();
    let (mut cache, opts) = (LayoutCache::new(), LayoutOptions::default());
    let (first, resumed) = relay(&d, &mut cache, &opts);
    assert!(first.pages.len() > 20, "{} pages", first.pages.len());
    started(resumed, 0);
    // Near the end, in the middle, and growing by enough lines to move every page after it.
    for (block, text) in [(399, " more"), (398, " and more"), (200, " words"), (200, &*" many more words".repeat(40)), (60, " early")] {
        retype(&mut d, block, text);
        let (l, resumed) = relay(&d, &mut cache, &opts);
        started(resumed, 1);
        // The pages before the mark are the kept ones themselves, not copies.
        let kept = first.pages.iter().zip(&l.pages).take_while(|(a, b)| crate::Items::shared(&a.items, &b.items)).count();
        assert!(block < 100 || kept > 3, "block {block}: {kept} pages kept");
    }
    // Nothing changed: the last mark still holds.
    started(relay(&d, &mut cache, &opts).1, 1);
    // The first paragraph is before every mark.
    retype(&mut d, 0, " start");
    started(relay(&d, &mut cache, &opts).1, 0);
}

#[test]
fn paragraphs_added_and_removed_are_laid_out_from_the_mark_before_them() {
    let mut d = long_doc();
    let (mut cache, opts) = (LayoutCache::new(), LayoutOptions::default());
    let pages = relay(&d, &mut cache, &opts).0.pages.len();
    d.insert_block(StoryRef::Body, &Path::top(300), Block::Para(Paragraph::with_text(&"inserted text ".repeat(300), Default::default()))).unwrap();
    let (l, resumed) = relay(&d, &mut cache, &opts);
    started(resumed, 1);
    assert!(l.pages.len() > pages);
    d.body.remove(300);
    d.body.remove(250);
    let (l, resumed) = relay(&d, &mut cache, &opts);
    started(resumed, 1);
    assert!(l.pages.len() <= pages);
    d.body.truncate(120);
    started(relay(&d, &mut cache, &opts).1, 1);
}

#[test]
fn what_a_pass_is_given_besides_the_body_must_be_the_same_to_start_from_a_mark() {
    let mut d = long_doc();
    let id = d.add_part(wordcraft_doc::PartKind::Footnote, vec![Arc::new(Block::Para(Paragraph::with_text("A note", Default::default())))]);
    let note = InlineObject::NoteRef { kind: wordcraft_doc::para::NoteKind::Footnote, id, custom: String::new() };
    d.insert_object(&Pos::body(390, 3), note, &Default::default()).unwrap();
    let (mut cache, mut opts) = (LayoutCache::new(), LayoutOptions::default());
    relay(&d, &mut cache, &opts);
    retype(&mut d, 395, " body text, with the note kept");
    started(relay(&d, &mut cache, &opts).1, 1);
    // The note's own text: no block of the body changed, but the page it is on did.
    let p = d.para_mut(StoryRef::Part(id), &Path::top(0)).unwrap();
    p.text.push_str(&" that grows by several lines".repeat(30));
    p.touch();
    started(relay(&d, &mut cache, &opts).1, 0);
    // A style, the page size, an option.
    Arc::make_mut(&mut d.styles).default_chr.size = Some(11.0);
    started(relay(&d, &mut cache, &opts).1, 0);
    d.last_section.page_h -= 72.0;
    started(relay(&d, &mut cache, &opts).1, 0);
    opts.show_hidden = true;
    started(relay(&d, &mut cache, &opts).1, 0);
    started(relay(&d, &mut cache, &opts).1, 1);
}

/// The word being typed is drawn without a spelling squiggle: a paragraph that held it is laid
/// out again when the word moves on, though the paragraph itself is as it was.
#[test]
fn the_paragraph_the_typed_word_was_in_is_laid_out_again() {
    let mut d = long_doc();
    let (mut cache, mut opts) = (LayoutCache::new(), LayoutOptions { proofing: true, ..Default::default() });
    relay(&d, &mut cache, &opts);
    let rev = retype(&mut d, 100, " mispeled");
    let len = d.para(StoryRef::Body, &Path::top(100)).unwrap().text.len();
    opts.typing_word = Some(TypingWord { rev, start: len - 8, end: len });
    started(relay(&d, &mut cache, &opts).1, 1);
    // Typing goes on further down: the word in block 100 is no longer being typed.
    let rev = retype(&mut d, 300, " anuther");
    let len = d.para(StoryRef::Body, &Path::top(300)).unwrap().text.len();
    opts.typing_word = Some(TypingWord { rev, start: len - 7, end: len });
    let (_, resumed) = relay(&d, &mut cache, &opts);
    started(resumed, 1);
    // The caret leaves, and nothing else changes.
    opts.typing_word = None;
    relay(&d, &mut cache, &opts);
    // A word in a paragraph that did not change (laid out before any mark that could be used).
    let p = d.para(StoryRef::Body, &Path::top(50)).unwrap();
    opts.typing_word = Some(TypingWord { rev: p.rev, start: 0, end: 9 });
    started(relay(&d, &mut cache, &opts).1, 0);
}

#[test]
fn sections_page_breaks_and_tables_before_the_edit_are_kept() {
    let mut d = long_doc();
    let cols = SectionProps { columns: wordcraft_doc::section::Columns { count: 2, ..Default::default() }, ..Default::default() };
    d.para_mut(StoryRef::Body, &Path::top(60)).unwrap().section = Some(Box::new(SectionProps::default()));
    d.para_mut(StoryRef::Body, &Path::top(140)).unwrap().section = Some(Box::new(cols));
    d.last_section.start = wordcraft_doc::section::SectionStart::Continuous;
    d.para_mut(StoryRef::Body, &Path::top(180)).unwrap().props.page_break_before = Some(true);
    d.para_mut(StoryRef::Body, &Path::top(181)).unwrap().props.keep_next = Some(true);
    d.insert_block(StoryRef::Body, &Path::top(220), Block::Table(wordcraft_doc::Table::new(30, 3, 400.0))).unwrap();
    let (mut cache, opts) = (LayoutCache::new(), LayoutOptions::default());
    relay(&d, &mut cache, &opts);
    for block in [400, 399, 300, 230, 219, 182, 150, 100, 61, 59, 400] {
        retype(&mut d, block, " a few more words here");
        relay(&d, &mut cache, &opts);
    }
    // A section's page layout changes: every section is given again.
    d.para_mut(StoryRef::Body, &Path::top(60)).unwrap().section.as_mut().unwrap().margin_left += 36.0;
    started(relay(&d, &mut cache, &opts).1, 0);
}

/// Where a paragraph kept with the next goes depends on the paragraphs after it: a chain of
/// them stays on the page only if the paragraph the chain ends at can start there too. A mark
/// taken after the chain's first paragraph holds only while the whole chain, and the paragraph
/// after it, are unchanged.
#[test]
fn a_mark_does_not_hold_when_a_paragraph_it_looked_ahead_at_changes() {
    let chr = |size: f32| wordcraft_doc::CharProps { size: Some(size), font: Some(WRAP_TEST_FONT.into()), ..Default::default() };
    let build = |filler: usize, last: f32| {
        let mut d = pagination_fixture_document(&vec!["Filler line"; filler].join("\n"));
        for name in ["Heading one", "Heading two", "Heading three"] {
            let mut p = Paragraph::with_text(name, chr(12.0));
            p.props.keep_next = Some(true);
            d.body.push(Arc::new(Block::Para(p)));
        }
        d.body.push(Arc::new(Block::Para(Paragraph::with_text("The text the headings are kept with", chr(last)))));
        for _ in 0..40 {
            d.body.push(Arc::new(Block::Para(Paragraph::with_text("More text after it", chr(12.0)))));
        }
        d
    };
    let page_of = |l: &DocLayout, block: usize| l.caret(&Pos::body(block, 0)).map(|c| c.page);
    // As much filler as leaves room for the three headings and a line of small text, but not
    // for a line of large text: the headings then go to the next page with it.
    let filler = (1..80)
        .find(|&n| page_of(&lay(&build(n, 12.0)), n) == Some(0) && page_of(&lay(&build(n, 60.0)), n) == Some(1))
        .expect("a page the headings fit on only with small text after them");
    let mut d = build(filler, 60.0);
    let (mut cache, opts) = (LayoutCache::new(), LayoutOptions::default());
    let (l, _) = relay(&d, &mut cache, &opts);
    assert_eq!(page_of(&l, filler), Some(1));
    // The large text becomes small: three blocks after the first heading, which now stays.
    let small = Paragraph::with_text("The text the headings are kept with", chr(12.0));
    d.body[filler + 3] = Arc::new(Block::Para(small));
    let (l, _) = relay(&d, &mut cache, &opts);
    assert_eq!(page_of(&l, filler), Some(0));
}
