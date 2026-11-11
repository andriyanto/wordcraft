//! The document's word and character counts, kept up to date at the cost of what changed.
//!
//! The status bar shows the counts after every keystroke. Counting the whole document each time
//! (and building all of its text as one string for the character counts) took a quarter of a
//! second a key in a document of a few thousand pages. [`CountCache`] remembers each top-level
//! block's counts instead: a document's blocks are shared and copied when changed, so a block at
//! the same address is the same block, and only the blocks an edit touched are counted again.
//!
//! The totals are exactly [`Document::story_word_count`]'s and those of
//! [`count_chars`](crate::count_chars) and [`count_chars_no_spaces`](crate::count_chars_no_spaces)
//! over [`Document::plain_text`], story by story.

use std::collections::HashMap;
use std::sync::{Arc, Weak};

use crate::block::{Block, Blocks};
use crate::text_walk::{TextMode, story_word_count_of, text_of};
use crate::{Document, Numbering, StoryRef, numbering, styles};

/// A document's counts ([`CountCache::counts`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DocCounts {
    /// Words, as the status bar counts them (generated list numbers included).
    pub words: usize,
    /// Characters, paragraph breaks left out.
    pub chars: usize,
    /// Characters that aren't whitespace.
    pub chars_no_spaces: usize,
}

/// One block's counts.
struct Entry {
    /// Keeps the block's allocation, so no other block can come to have its address while the
    /// entry is kept; and a block something weakly points at is copied when changed, never
    /// changed in place.
    block: Weak<Block>,
    /// Words in the block's own text, when none of its paragraphs is in a list: a numbered
    /// paragraph's label depends on the paragraphs before it, so those blocks' words are counted
    /// on every pass (`None`).
    words: Option<usize>,
    chars: usize,
    chars_no_spaces: usize,
    /// The pass that last used the entry.
    pass: u64,
}

/// Each block's counts as last counted (see the module).
#[derive(Default)]
pub struct CountCache {
    blocks: HashMap<usize, Entry>,
    /// The styles and numbering the entries were counted with: whether a paragraph is in a list
    /// depends on them, so entries don't outlive them. Held, so their addresses aren't reused.
    env: Option<(Arc<styles::StyleSheet>, Arc<Numbering>)>,
    pass: u64,
}

impl CountCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// The counts of `stories` in `doc`, added up.
    pub fn counts(&mut self, doc: &Document, stories: &[StoryRef]) -> DocCounts {
        let same_env = self.env.as_ref().is_some_and(|(s, n)| Arc::ptr_eq(s, &doc.styles) && Arc::ptr_eq(n, &doc.numbering));
        if !same_env {
            self.blocks.clear();
            self.env = Some((doc.styles.clone(), doc.numbering.clone()));
        }
        self.pass = self.pass.wrapping_add(1);
        let mut total = DocCounts::default();
        for story in stories {
            let Some(blocks) = doc.story(*story) else { continue };
            // List numbering starts again in each story, as `Document::story_word_count` has it.
            let mut counters = numbering::Counters::default();
            for block in blocks {
                let (words, chars, chars_no_spaces) = self.block(doc, block, &mut counters);
                total.words = total.words.saturating_add(words);
                total.chars = total.chars.saturating_add(chars);
                total.chars_no_spaces = total.chars_no_spaces.saturating_add(chars_no_spaces);
            }
        }
        // Blocks no longer in the document (or not counted this time) are forgotten.
        let pass = self.pass;
        self.blocks.retain(|_, e| e.pass == pass);
        total
    }

    /// One top-level block's (words, characters, characters without spaces).
    fn block(&mut self, doc: &Document, block: &Arc<Block>, counters: &mut numbering::Counters) -> (usize, usize, usize) {
        let key = Arc::as_ptr(block) as usize;
        let alone = |b: &Arc<Block>| -> Blocks { vec![b.clone()] };
        if let Some(e) = self.blocks.get_mut(&key)
            && std::ptr::eq(e.block.as_ptr(), Arc::as_ptr(block))
        {
            e.pass = self.pass;
            let words = match e.words {
                Some(w) => w,
                None => story_word_count_of(&alone(block), &doc.numbering, &doc.styles, counters, 0),
            };
            return (words, e.chars, e.chars_no_spaces);
        }
        let one = alone(block);
        let mut text = String::new();
        text_of(&one, &mut text, 0, TextMode::Plain);
        let (chars, chars_no_spaces) = (crate::count_chars(&text), crate::count_chars_no_spaces(&text));
        let words = story_word_count_of(&one, &doc.numbering, &doc.styles, counters, 0);
        let listed = in_a_list(&one, &doc.styles, 0);
        self.blocks.insert(key, Entry { block: Arc::downgrade(block), words: (!listed).then_some(words), chars, chars_no_spaces, pass: self.pass });
        (words, chars, chars_no_spaces)
    }
}

/// Whether any paragraph in `blocks` is in a list (directly or by its style), as
/// [`story_word_count_of`] finds them: such a paragraph moves the list counters. Past the depth
/// that walk stops at nothing is counted, so nothing there is in a list either.
fn in_a_list(blocks: &Blocks, styles: &styles::StyleSheet, depth: usize) -> bool {
    if depth > 16 {
        return false;
    }
    blocks.iter().any(|b| match &**b {
        Block::Para(p) => p.props.numbering.or_else(|| styles.resolve_para(&p.props).numbering).is_some_and(|n| n.num != 0),
        Block::Table(t) => t.rows.iter().any(|r| r.cells.iter().any(|c| in_a_list(&c.blocks, styles, depth + 1))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CharProps, Paragraph, para_block};

    /// The counts the status bar computed before the cache: the whole document, every time.
    fn whole(doc: &Document, stories: &[StoryRef]) -> DocCounts {
        let text = stories.iter().map(|s| doc.plain_text(*s)).collect::<Vec<_>>().join("\n");
        DocCounts {
            words: stories.iter().map(|s| doc.story_word_count(*s)).sum(),
            chars: crate::count_chars(&text),
            chars_no_spaces: crate::count_chars_no_spaces(&text),
        }
    }

    fn para(text: &str) -> Arc<Block> {
        para_block(Paragraph::with_text(text, CharProps::default()))
    }

    #[test]
    fn the_cached_counts_are_the_whole_documents_counts() {
        let mut doc = Document::from_text("Hello, world!\nThis is — a test 42.\n\n漢字 かな tab\there");
        let table = crate::table::Table::new(2, 3, 300.0);
        doc.body.insert(1, Arc::new(Block::Table(table)));
        if let Some(Block::Table(t)) = doc.body.get_mut(1).map(Arc::make_mut) {
            t.rows[0].cells[0].blocks = vec![para("one two"), para(""), para("three")];
            t.rows[1].cells[2].blocks = vec![para("four"), para("")];
        }
        let stories = [StoryRef::Body];
        let mut cache = CountCache::new();
        assert_eq!(cache.counts(&doc, &stories), whole(&doc, &stories));
        // Again from the cache, and after each kind of edit.
        assert_eq!(cache.counts(&doc, &stories), whole(&doc, &stories));
        doc.body.push(para("more words here"));
        assert_eq!(cache.counts(&doc, &stories), whole(&doc, &stories));
        doc.body.remove(0);
        assert_eq!(cache.counts(&doc, &stories), whole(&doc, &stories));
        if let Some(Block::Para(p)) = doc.body.last_mut().map(Arc::make_mut) {
            *p = Paragraph::with_text("changed in place", CharProps::default());
        }
        assert_eq!(cache.counts(&doc, &stories), whole(&doc, &stories));
        assert_eq!(cache.blocks.len(), doc.body.len(), "blocks that left the document are forgotten");
    }

    /// A block changed while only the cache points at it is not changed in place: the cache's weak
    /// pointer makes the change a copy, at another address, so its old counts are not used.
    #[test]
    fn a_block_only_the_cache_knows_is_counted_again_when_it_changes() {
        let mut doc = Document::from_text("one two three");
        let stories = [StoryRef::Body];
        let mut cache = CountCache::new();
        assert_eq!(cache.counts(&doc, &stories).words, 3);
        let before = Arc::as_ptr(&doc.body[0]);
        if let Some(Block::Para(p)) = doc.body.first_mut().map(Arc::make_mut) {
            *p = Paragraph::with_text("one", CharProps::default());
        }
        assert_ne!(before, Arc::as_ptr(&doc.body[0]));
        assert_eq!(cache.counts(&doc, &stories), whole(&doc, &stories));
        assert_eq!(cache.counts(&doc, &stories).words, 1);
    }

    /// Numbered paragraphs' labels are words too and depend on the paragraphs before them, so
    /// they are counted on every pass; the totals still match after a paragraph joins the list.
    #[test]
    fn list_labels_are_counted_as_the_whole_count_counts_them() {
        let mut doc = Document::from_text("first\nsecond\nthird");
        let num = Arc::make_mut(&mut doc.numbering).add_list(crate::numbering::ListKind::Numbered);
        let stories = [StoryRef::Body];
        let mut cache = CountCache::new();
        assert_eq!(cache.counts(&doc, &stories), whole(&doc, &stories));
        for at in [0, 2, 1] {
            if let Some(Block::Para(p)) = doc.body.get_mut(at).map(Arc::make_mut) {
                p.props.numbering = Some(crate::props::NumRef { num, level: 0 });
            }
            assert_eq!(cache.counts(&doc, &stories), whole(&doc, &stories), "after numbering paragraph {at}");
        }
        assert!(cache.counts(&doc, &stories).words > 3, "the labels are words");
    }
}
