//! A layout that reuses earlier work must be the layout from scratch.
//!
//! Scripts of random edits, with every layout checked against the document laid out with
//! nothing to reuse ([`wordcraft_layout::verify_against_full`]). Whatever a layout comes to
//! reuse (paragraph layouts today, pages once pagination resumes from where an edit is) is
//! covered by the same scripts: a difference names the edit after which reuse went wrong.

use serde_json::{Value, json};
use wordcraft_doc::{Document, Pos, StoryRef};

use crate::Session;

/// A small deterministic generator (xorshift): the scripts must be the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    fn pick<'a>(&mut self, of: &'a [&'a str]) -> &'a str {
        of[self.below(of.len() as u64) as usize]
    }
}

const WORDS: &[&str] =
    &["lorem", "ipsum", "dolor", "sit", "amet,", "consectetur", "adipiscing", "elit.", "Sed", "do", "eiusmod", "tempor", "漢字", "42"];

/// One random edit: the command and its parameters.
fn step(r: &mut Rng) -> (&'static str, Value) {
    let text = |r: &mut Rng, n: u64| (0..=r.below(n)).map(|_| r.pick(WORDS)).collect::<Vec<_>>().join(" ") + " ";
    match r.below(100) {
        0..=27 => ("text.insert", json!({"text": text(r, 12)})),
        28..=33 => ("text.insert", json!({"text": text(r, 90)})),
        34..=41 => ("text.newParagraph", json!({})),
        42..=45 => ("text.backspace", json!({})),
        46..=48 => ("text.delete", json!({})),
        49 => ("text.deleteWordBack", json!({})),
        50..=52 => ("caret.up", json!({})),
        53..=55 => ("caret.down", json!({})),
        56 => ("caret.left", json!({})),
        57 => ("caret.right", json!({})),
        58 => ("caret.wordLeft", json!({})),
        59 => ("caret.paraUp", json!({})),
        60 => ("caret.paraDown", json!({})),
        61 => ("caret.docStart", json!({})),
        62..=63 => ("caret.docEnd", json!({})),
        64 => ("caret.pageUp", json!({})),
        65 => ("caret.pageDown", json!({})),
        66..=67 => ("para.numbering", json!({})),
        68 => ("para.bullets", json!({})),
        69 => ("para.indent", json!({})),
        70 => ("para.alignCenter", json!({})),
        71 => ("para.keepNext", json!({})),
        72 => ("para.keepLines", json!({})),
        73 => ("para.pageBreakBefore", json!({})),
        74 => ("para.borders", json!({"preset": "box"})),
        75 => ("para.shading", json!({"color": "FFFF00"})),
        76 => ("para.spacing", json!({"before": 18.0, "after": 6.0})),
        77 => ("para.lineSpacing", json!({"value": 1.5})),
        78 => ("styles.apply", json!({"style": "Heading 1"})),
        79 => ("styles.apply", json!({"style": "Normal"})),
        80 => ("format.bold", json!({})),
        81 => ("format.size", json!({"size": 16.0})),
        82 => ("select.paragraph", json!({})),
        83 => ("select.word", json!({})),
        84..=85 => ("insert.table", json!({"rows": 1 + r.below(4), "cols": 1 + r.below(3)})),
        86 => ("table.insertRowBelow", json!({})),
        87 => ("references.footnote", json!({})),
        88 => ("references.endnote", json!({})),
        89 => ("insert.pageBreak", json!({})),
        90 => ("text.lineBreak", json!({})),
        91 => ("text.tab", json!({})),
        92 => ("insert.horizontalLine", json!({})),
        93 => ("layout.columns", json!({"count": 1 + r.below(2)})),
        94 => ("layout.lineNumbers", json!({})),
        95..=97 => ("edit.undo", json!({})),
        98 => ("edit.redo", json!({})),
        _ => ("text.insert", json!({"text": text(r, 300)})),
    }
}

/// Run `steps` random edits from `seed`, laying the document out after each with the check on.
/// Returns how many of the edits the document took (a command that isn't available where the
/// caret is changes nothing) and how many pages it ended with.
fn script(seed: u64, steps: usize, replay: bool) -> (usize, usize) {
    let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let mut s = Session::new(Document::new());
    wordcraft_layout::verify_against_full(true);
    // A mark on every page, so that a pass starts wherever one can.
    wordcraft_layout::mark_every(1);
    wordcraft_layout::replay_blocks(replay);
    let _ = wordcraft_layout::take_mismatch();
    let mut done: Vec<&'static str> = Vec::new();
    let mut pages = 0;
    for n in 0..steps {
        let (id, params) = step(&mut r);
        if s.run(id, &params).is_ok() {
            done.push(id);
        }
        pages = s.layout().pages.len();
        if let Some(why) = wordcraft_layout::take_mismatch() {
            wordcraft_layout::verify_against_full(false);
            wordcraft_layout::replay_blocks(false);
            let last: Vec<_> = done.iter().rev().take(8).rev().collect();
            panic!(
                "seed {seed}, step {n} ({id} {params}): the layout differs from the document laid out from scratch\n{why}\nlast edits taken: {last:?}"
            );
        }
    }
    wordcraft_layout::verify_against_full(false);
    wordcraft_layout::replay_blocks(false);
    (done.len(), pages)
}

#[test]
fn layouts_after_random_edits_are_the_layouts_from_scratch() {
    let (mut taken, mut most_pages) = (0, 0);
    for seed in 1..=12 {
        let (done, pages) = script(seed, 220, false);
        taken += done;
        most_pages = most_pages.max(pages);
    }
    // The scripts must really edit and really paginate, or the check checks nothing.
    assert!(taken > 1500, "only {taken} edits were taken");
    assert!(most_pages >= 3, "the longest document was {most_pages} page(s)");
}

/// The same scripts with every block placed, taken back to the checkpoint before it, and placed
/// again ([`wordcraft_layout::replay_blocks`]): the pages must still be the pages of the plain
/// walk. A checkpoint that misses any of pagination's state fails here, which is what resuming
/// pagination from a saved checkpoint depends on.
#[test]
fn pagination_resumed_from_a_checkpoint_before_every_block_gives_the_same_pages() {
    let mut most_pages = 0;
    for seed in 1..=12 {
        most_pages = most_pages.max(script(seed, 220, true).1);
    }
    assert!(most_pages >= 3, "the longest document was {most_pages} page(s)");
}

/// A document of several pages, edited mostly the way people write: typing where the caret is,
/// which is often near the end. Pagination then starts from a mark near the edit rather than
/// from the first block ([`wordcraft_layout::resumed_passes`]), and the pages must still be the
/// pages of the document laid out from scratch.
#[test]
fn pagination_started_near_an_edit_gives_the_pages_from_scratch() {
    wordcraft_layout::mark_every(1);
    for seed in 1..=2u64 {
        let mut r = Rng(seed.wrapping_mul(0xD1B5_4A32_D192_ED03) | 1);
        let mut s = Session::new(Document::new());
        for _ in 0..160 {
            let words = (0..=r.below(40)).map(|_| r.pick(WORDS)).collect::<Vec<_>>().join(" ");
            s.run("text.insert", &json!({"text": words})).unwrap();
            s.run("text.newParagraph", &json!({})).unwrap();
        }
        assert!(s.layout().pages.len() >= 6, "seed {seed}: {} page(s)", s.layout().pages.len());
        wordcraft_layout::verify_against_full(true);
        let _ = wordcraft_layout::take_mismatch();
        let before = wordcraft_layout::resumed_passes();
        for n in 0..300 {
            let (id, params) = match r.below(100) {
                0..=54 => ("text.insert", json!({"text": r.pick(WORDS).to_string() + " "})),
                55..=59 => ("caret.docEnd", json!({})),
                60..=64 => ("caret.pageUp", json!({})),
                65..=67 => ("caret.pageDown", json!({})),
                _ => step(&mut r),
            };
            // Back to the body from a note, most of the time: near the end, or anywhere.
            if s.sel.focus.story != StoryRef::Body && r.below(4) != 0 {
                let blocks = s.doc.body.len() as u64;
                let block = if r.below(5) == 0 { r.below(blocks) } else { blocks - 1 - r.below(blocks.min(30)) };
                let _ = s.run("caret.set", &json!({"pos": Pos::body(block as usize, 0)}));
            }
            let _ = s.run(id, &params);
            s.layout();
            if let Some(why) = wordcraft_layout::take_mismatch() {
                wordcraft_layout::verify_against_full(false);
                panic!("seed {seed}, step {n} ({id} {params}): the layout differs from the document laid out from scratch\n{why}");
            }
        }
        wordcraft_layout::verify_against_full(false);
        let resumed = wordcraft_layout::resumed_passes() - before;
        assert!(resumed >= 100, "seed {seed}: only {resumed} of the passes started from a mark");
    }
}
