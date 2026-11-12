//! Paginating from where the document changed.
//!
//! A pass walks the body from the first block, though an edit leaves everything before it as it
//! was: the same blocks reach the same pages in the same way. So a pass keeps what it would need
//! to start again part-way — a [`Mark`] every few pages, at the first block boundary on the
//! page, holding the walk's state there — together with what it was laid out from. The next
//! pass compares that with what it is given, finds the last mark the changes cannot have
//! reached, and walks from there: the pages before it are the ones kept.
//!
//! What a mark's state depends on, and so what must be unchanged for it to be used:
//!
//! - the body blocks before it, and the few after it that the walk looks ahead at
//!   ([`LOOK_AHEAD`]): the same blocks at the same places. Blocks are compared by address,
//!   which tells: the blocks laid out are kept (shared) here, and a shared block is never
//!   changed in place, only replaced by a changed copy;
//! - everything a pass is given besides the body ([`Given`]): the options, the styles, the
//!   sections, the other stories, the settings, and the values a pass is done again with.
//!
//! The walk also stops early. Past an edit that leaves the page breaks where they were, it comes
//! to one of the last pass's marks in the very state that pass was in there, with the same
//! blocks from there on: what follows is then what followed last time, and the walk takes the
//! rest of that pass ([`Tail`]) rather than laying it out. That needs more to be unchanged: the
//! number of blocks (a placed item says which block it is of), and the block before the mark,
//! which the walk looks back at ([`LOOK_BACK`]).
//!
//! Anything else (an edit to a footnote, a new section layout) and the pass walks the whole body,
//! as it also does where a mark's state would not be all there is to the walk: documents with
//! linked text boxes, pictures wrapped along their outline, columns being balanced, and the
//! views that lay the document out as one long page.
//!
//! The result must be the pages a walk from the first block gives: the tests lay documents out
//! both ways after every one of a long run of random edits ([`crate::verify_against_full`]), and
//! the crate's own tests can all be run resuming from the middle of each layout
//! (`WORDCRAFT_LAYOUT_RESUME=1`).

use std::cell::Cell;
use std::collections::BTreeMap;

use wordcraft_doc::{NoteSeparators, Numbering, Part, Revision, Settings, StyleSheet};

use super::cache::Totals;
use super::ctx::Ctx;
use super::floats::{EarlyFloats, WrapArea};
use super::page::PageBuilder;
use super::pass::Checkpoint;
use super::{Arc, Block, Blocks, Document, HashMap, LayoutOptions, Page, SectionProps, TypingWord};

/// How many blocks past the one being placed the walk reads: the next one (spacing between
/// paragraphs, joined borders), and for keep with next the chain of paragraphs kept together
/// (eight at most) with the one after it. A little more than that, for good measure.
const LOOK_AHEAD: usize = 12;

/// A pass walks the whole body after this many in a row that didn't: one that resumes doesn't
/// see which paragraph layouts the blocks before its mark use, so the cache keeps everything
/// meanwhile, and a whole walk is what lets it drop what no block uses any more.
const MAX_IN_A_ROW: u32 = 256;

/// How many blocks before the one being placed the walk reads: the one before (whether it ended
/// a section, joined borders, spacing between paragraphs). The walk can stop at a mark only when
/// these are unchanged too, and one more for good measure.
const LOOK_BACK: usize = 2;

/// A mark is taken every so many pages: taking one copies the walk's state, which a pass over
/// a long document would otherwise do thousands of times to save the next pass a few pages.
const PAGES_BETWEEN_MARKS: usize = if cfg!(test) { 1 } else { 8 };

thread_local! {
    static EVERY: Cell<usize> = const { Cell::new(PAGES_BETWEEN_MARKS) };
    static RESUMED: Cell<u64> = const { Cell::new(0) };
    static STOPPED: Cell<u64> = const { Cell::new(0) };
    static OFF: Cell<bool> = const { Cell::new(false) };
    #[cfg(test)]
    static MIDDLE: Cell<bool> = const { Cell::new(false) };
}

/// How many passes on this thread stopped at a mark rather than after the last block.
pub fn stopped_passes() -> u64 {
    STOPPED.with(Cell::get)
}

/// How many passes on this thread started from a mark rather than the first block.
pub fn resumed_passes() -> u64 {
    RESUMED.with(Cell::get)
}

pub(crate) fn pages_between_marks() -> usize {
    EVERY.with(Cell::get).max(1)
}

/// Take a mark every `pages` pages on this thread (tests take one on every page, to start from
/// as many places as there are).
pub fn mark_every(pages: usize) {
    EVERY.with(|v| v.set(pages));
}

/// Turn resuming off (or back on) for this thread: every pass then walks the whole body.
pub fn resume_passes(on: bool) {
    OFF.with(|v| v.set(!on));
}

/// Resume from the middle mark rather than the last, to check marks an edit at the end of the
/// document would never use.
#[cfg(test)]
pub(crate) fn resume_from_the_middle(on: bool) {
    MIDDLE.with(|v| v.set(on));
}

/// The walk's state before a top-level block, the first to start on its page.
pub(crate) struct Mark {
    /// The block about to be placed.
    pub(crate) block: usize,
    /// Its section, and the page field's value when the section was begun (its header and
    /// footer room is measured again on resuming, as it was then).
    pub(crate) si: usize,
    pub(crate) entry_page: u32,
    /// Where the section's pages start below their header.
    pub(crate) body_top: f32,
    pub(crate) at: Checkpoint,
    pub(crate) boxes: wordcraft_doc::BoxBudget,
    pub(crate) boxes_discarded: usize,
    /// Whether text laid out so far shows a page total: the mark then holds for those totals only.
    pub(crate) uses_totals: bool,
}

/// Everything a pass is laid out from besides the body's blocks.
struct Given {
    env: u64,
    /// The options, without the word being typed (it changes with every key, and matters only
    /// where its paragraph is: [`Kept::typing_at`]).
    opts: LayoutOptions,
    sections: Vec<SectionProps>,
    parts: BTreeMap<u32, Part>,
    styles: Arc<StyleSheet>,
    numbering: Arc<Numbering>,
    settings: Settings,
    title: String,
    author: String,
    footnote_separators: NoteSeparators,
    endnote_separators: NoteSeparators,
    media: BTreeMap<String, Arc<Vec<u8>>>,
    revisions: Vec<Revision>,
}

fn same_blocks(a: &Blocks, b: &Blocks) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| Arc::ptr_eq(x, y))
}

fn without_typing(opts: &LayoutOptions) -> LayoutOptions {
    LayoutOptions { typing_word: None, ..opts.clone() }
}

impl Given {
    fn of(doc: &Document, opts: &LayoutOptions, env: u64, sections: &[(usize, &SectionProps)]) -> Self {
        Given {
            env,
            opts: without_typing(opts),
            sections: sections.iter().map(|(_, s)| (*s).clone()).collect(),
            parts: doc.parts.clone(),
            styles: doc.styles.clone(),
            numbering: doc.numbering.clone(),
            settings: doc.settings.clone(),
            title: doc.core.title.clone(),
            author: doc.core.creator.clone(),
            footnote_separators: doc.footnote_separators.clone(),
            endnote_separators: doc.endnote_separators.clone(),
            media: doc.media.clone(),
            revisions: doc.revisions.clone(),
        }
    }

    fn holds(&self, doc: &Document, opts: &LayoutOptions, env: u64, sections: &[(usize, &SectionProps)]) -> bool {
        self.env == env
            && self.opts == without_typing(opts)
            && self.sections.len() == sections.len()
            && self.sections.iter().zip(sections).all(|(a, (_, b))| a == *b)
            && Arc::ptr_eq(&self.styles, &doc.styles)
            && Arc::ptr_eq(&self.numbering, &doc.numbering)
            && self.parts.len() == doc.parts.len()
            && self
                .parts
                .iter()
                .zip(&doc.parts)
                .all(|((i, a), (j, b))| i == j && a.kind == b.kind && a.body == b.body && same_blocks(&a.blocks, &b.blocks))
            && self.media.len() == doc.media.len()
            && self.media.iter().zip(&doc.media).all(|((i, a), (j, b))| i == j && Arc::ptr_eq(a, b))
            && self.settings == doc.settings
            && self.title == doc.core.title
            && self.author == doc.core.creator
            && self.footnote_separators == doc.footnote_separators
            && self.endnote_separators == doc.endnote_separators
            && self.revisions == doc.revisions
    }
}

/// What the last pass left for the next: what it was laid out from, its marks, and its pages as
/// the walk left them (before headers, footers and the rest are added to them).
pub(crate) struct Kept {
    given: Given,
    body: Blocks,
    /// The last block of each section.
    ends: Vec<usize>,
    typing: Option<TypingWord>,
    /// The first block a paragraph with the word being typed was laid out for (0 for one outside
    /// the body).
    typing_at: Option<usize>,
    notes_hash: u64,
    note_pages: HashMap<u32, usize>,
    totals: Totals,
    /// Whether the last pass kept floats clear of the text above them rather than asking for
    /// another pass.
    settle: bool,
    marks: Vec<Mark>,
    pages: Vec<Page>,
    behind: Vec<usize>,
    floats_above: HashMap<(usize, usize), Option<(usize, WrapArea)>>,
    /// The walk's state when it ended.
    end: Option<End>,
    in_a_row: u32,
}

/// The walk's state after the last block (and the endnotes after it).
pub(crate) struct End {
    at: Checkpoint,
    boxes: wordcraft_doc::BoxBudget,
    boxes_discarded: usize,
    uses_totals: bool,
    /// The field values the last section left.
    section: u32,
    page_format: wordcraft_doc::section::NumFormat,
}

/// The rest of the last walk, for a walk that comes to be in the state the last one was in at
/// one of its marks, with the same blocks from there on: what follows is then what followed last
/// time, and the walk stops there and takes it.
pub(crate) struct Tail {
    /// The marks not yet reached, in order.
    marks: std::collections::VecDeque<Mark>,
    /// The last walk's pages from page `base` on, and what is behind the text on each.
    pages: Vec<Page>,
    behind: Vec<usize>,
    base: usize,
    /// The blocks from this one on are the ones laid out last time.
    from: usize,
    end: End,
    floats_above: HashMap<(usize, usize), Option<(usize, WrapArea)>>,
    /// The word being typed last time, and where it was laid out.
    typing: Option<(TypingWord, usize)>,
}

/// A pass about to start from a mark.
pub(crate) struct Resume {
    kept: Box<Kept>,
    /// The mark to start from; those before it are kept.
    pick: usize,
}

/// What a pass is given, for [`Kept::resume`] and [`keep`].
pub(crate) struct PassInput<'a> {
    pub(crate) doc: &'a Document,
    pub(crate) opts: &'a LayoutOptions,
    pub(crate) env: u64,
    pub(crate) sections: &'a [(usize, &'a SectionProps)],
    pub(crate) note_pages: &'a HashMap<u32, usize>,
    pub(crate) totals: &'a Totals,
    pub(crate) early: &'a EarlyFloats,
    pub(crate) settle: bool,
}

/// Whether `block` holds a paragraph of revision `rev` (in its cells, for a table).
fn holds_rev(block: &Block, rev: u64, depth: usize) -> bool {
    match block {
        Block::Para(p) => p.rev == rev,
        Block::Table(t) => depth < 16 && t.rows.iter().flat_map(|r| &r.cells).flat_map(|c| &c.blocks).any(|b| holds_rev(b, rev, depth + 1)),
    }
}

impl Kept {
    /// Whether what the pass is given is what the last one was (but for the body's blocks).
    fn holds(&self, input: &PassInput) -> bool {
        let PassInput { doc, opts, env, sections, note_pages, early, settle, .. } = *input;
        !OFF.with(Cell::get)
            && self.in_a_row < MAX_IN_A_ROW
            && self.settle == settle
            && early.is_empty()
            && self.note_pages == *note_pages
            && self.given.holds(doc, opts, env, sections)
    }

    /// Whether this pass can stop at a mark of the last one ([`Tail`]), and from which block on
    /// the body is the same: the same number of blocks in the same sections (an item says
    /// which block it is of), the same footnote numbers and page totals, and the word being
    /// typed not among the blocks that are the same.
    pub(crate) fn stoppable(&self, input: &PassInput, notes_hash: u64) -> Option<usize> {
        let PassInput { doc, opts, sections, totals, .. } = *input;
        let end = self.end.as_ref()?;
        if !self.holds(input)
            || self.notes_hash != notes_hash
            || self.body.len() != doc.body.len()
            || self.ends.len() != sections.len()
            || self.ends.iter().zip(sections).any(|(was, (now, _))| was != now)
            || (end.uses_totals && self.totals != *totals)
        {
            return None;
        }
        let same = self.body.iter().rev().zip(doc.body.iter().rev()).take_while(|(a, b)| Arc::ptr_eq(a, b)).count();
        let from = doc.body.len() - same;
        if self.typing != opts.typing_word {
            let before = |at: usize| at < from;
            if self.typing.is_some() && !self.typing_at.is_none_or(before) {
                return None;
            }
            if let Some(w) = opts.typing_word
                && !doc.body.iter().take(from).rev().take(4).any(|b| holds_rev(b, w.rev, 0))
            {
                return None;
            }
        }
        Some(from)
    }

    /// Take the rest of the last walk after mark `pick` (all of it for `None`), for a walk
    /// whose blocks from `from` on are the last one's.
    fn tail(&mut self, pick: Option<usize>, from: usize) -> Option<Box<Tail>> {
        let end = self.end.take()?;
        let after = pick.map_or(0, |i| i + 1);
        let base = pick.and_then(|i| self.marks.get(i)).map_or(0, |m| m.at.pages().saturating_sub(1));
        if base > self.pages.len() || base > self.behind.len() {
            return None;
        }
        let marks = self.marks.drain(after.min(self.marks.len())..).filter(|m| m.block >= from.saturating_add(LOOK_BACK)).collect();
        let (pages, behind) = (self.pages.split_off(base), self.behind.split_off(base));
        // The page the mark is on is the first of the rest, and the last of what is kept.
        self.pages.extend(pages.first().cloned());
        self.behind.extend(behind.first().copied());
        let typing = self.typing.zip(self.typing_at);
        Some(Box::new(Tail { marks, pages, behind, base, from, end, floats_above: self.floats_above.clone(), typing }))
    }

    /// For a pass that walks from the first block: the last walk as one it may stop in.
    pub(crate) fn whole_tail(&mut self, from: usize) -> Option<Box<Tail>> {
        self.tail(None, from)
    }

    /// The mark to start this pass from, if what the pass is given leaves one usable. Else the
    /// kept state is handed back, for what it was laid out from ([`keep`]).
    pub(crate) fn resume(self: Box<Self>, input: &PassInput) -> Result<Resume, Box<Kept>> {
        match self.usable(input) {
            Some(pick) => Ok(Resume { kept: self, pick }),
            None => Err(self),
        }
    }

    fn usable(&self, input: &PassInput) -> Option<usize> {
        let PassInput { doc, opts, sections, totals, .. } = *input;
        if self.marks.is_empty() || !self.holds(input) {
            return None;
        }
        // The blocks up to here are the ones laid out last time.
        let same = self.body.iter().zip(&doc.body).take_while(|(a, b)| Arc::ptr_eq(a, b)).count();
        let mut upto = if same == self.body.len() && same == doc.body.len() { usize::MAX } else { same };
        // A section that ends elsewhere: nothing at or after the nearer end.
        for (was, (now, _)) in self.ends.iter().zip(sections) {
            if was != now {
                upto = upto.min(*was.min(now));
            }
        }
        // The word being typed is drawn differently (no squiggle) in a paragraph that is
        // otherwise the same: nothing at or after where it was, and its paragraph now must be
        // among the changed blocks (not one laid out before).
        if self.typing != opts.typing_word {
            if self.typing.is_some() {
                upto = upto.min(self.typing_at.unwrap_or(usize::MAX));
            }
            if let Some(w) = opts.typing_word
                && !doc.body.iter().skip(same).take(4).any(|b| holds_rev(b, w.rev, 0))
            {
                return None;
            }
        }
        let ok =
            |m: &Mark| m.block.saturating_add(LOOK_AHEAD) <= upto && (!m.uses_totals || self.totals == *totals) && m.at.pages() <= self.pages.len();
        // Marks are in block order, and one that holds is after others that do (but for totals,
        // which hold for all or from the first that uses them on).
        let n = self.marks.partition_point(|m| m.block.saturating_add(LOOK_AHEAD) <= upto);
        let usable: Vec<usize> = (0..n).filter(|&i| self.marks.get(i).is_some_and(ok)).collect();
        #[cfg(test)]
        if MIDDLE.with(Cell::get) {
            return usable.get(usable.len() / 2).copied();
        }
        usable.last().copied()
    }
}

impl Resume {
    pub(crate) fn mark(&self) -> Option<&Mark> {
        self.kept.marks.get(self.pick)
    }

    /// Whether the pass's footnote numbering is the kept one's (known once the pass has its own).
    pub(crate) fn same_notes(&self, notes_hash: u64) -> bool {
        self.kept.notes_hash == notes_hash
    }

    /// [`Kept::stoppable`], of the state the mark is from.
    pub(crate) fn stoppable(&self, input: &PassInput, notes_hash: u64) -> Option<usize> {
        self.kept.stoppable(input, notes_hash)
    }

    /// Give the kept state back without using it.
    pub(crate) fn give_up(self) -> Box<Kept> {
        self.kept
    }

    /// Put the walk in the mark's state: the pages before it, the builder and the counters.
    /// `from`: the walk may stop at a later mark ([`Kept::stoppable`]). Returns the kept state
    /// (for [`keep`]) and the text boxes thrown away before the mark.
    pub(crate) fn start(mut self, pb: &mut PageBuilder, ctx: &mut Ctx, from: Option<usize>) -> (Box<Kept>, usize) {
        ctx.tail = from.and_then(|from| self.kept.tail(Some(self.pick), from));
        self.kept.marks.truncate(self.pick + 1);
        let (pages, behind) = (std::mem::take(&mut self.kept.pages), std::mem::take(&mut self.kept.behind));
        let mut discarded = 0;
        if let Some(m) = self.kept.marks.last() {
            m.at.resume(pb, ctx, pages, behind);
            ctx.boxes = m.boxes.clone();
            ctx.uses_totals = m.uses_totals;
            ctx.floats_above = self.kept.floats_above.iter().filter(|((block, _), _)| *block < m.block).map(|(k, v)| (*k, v.clone())).collect();
            ctx.saved_pages = pb.pages.len();
            discarded = m.boxes_discarded;
        }
        ctx.saved = std::mem::take(&mut self.kept.marks);
        if self.kept.typing == ctx.opts.typing_word {
            ctx.typing_at = self.kept.typing_at;
        }
        RESUMED.with(|n| n.set(n.get() + 1));
        (self.kept, discarded)
    }
}

/// Whether the walk, about to place `block`, is in the state the last walk was in at its mark
/// there, with the same blocks around and after it: it can stop.
pub(crate) fn converged(ctx: &mut Ctx, pb: &mut PageBuilder, block: usize, at: (usize, u32, f32, usize)) -> bool {
    let Some(mut tail) = ctx.tail.take() else { return false };
    while tail.marks.front().is_some_and(|m| m.block < block) {
        tail.marks.pop_front();
    }
    let (si, entry_page, body_top, discarded) = at;
    let same = tail.marks.front().is_some_and(|m| {
        m.block == block
            && block >= tail.from.saturating_add(LOOK_BACK)
            && m.si == si
            && m.entry_page == entry_page
            && m.body_top == body_top
            && m.boxes_discarded == discarded
            && m.uses_totals == ctx.uses_totals
            && m.boxes == ctx.boxes
            && ctx.alphas.untouched()
            && m.at.pages() > tail.base
            && m.at.same(pb, ctx)
    });
    // Once the marks are behind the walk there is nothing left to stop at.
    if same || !tail.marks.is_empty() {
        ctx.tail = Some(tail);
    }
    same
}

impl Tail {
    /// The walk stopped at the first of the marks left: the pages from the mark's on are the
    /// last walk's, and so is the state the walk ends in. Returns the text boxes thrown away.
    pub(crate) fn finish(mut self: Box<Self>, pb: &mut PageBuilder, ctx: &mut Ctx) -> usize {
        let block = self.marks.front().map_or(0, |m| m.block);
        // The page being filled is the mark's (the states are the same): the last walk's page
        // there holds it, and what came after.
        let n = pb.pages.len().saturating_sub(1);
        let (mut pages, mut behind) = (std::mem::take(&mut pb.pages), std::mem::take(&mut pb.behind));
        pages.truncate(n);
        behind.truncate(n);
        let skip = n.saturating_sub(self.base);
        pages.extend(self.pages.drain(skip.min(self.pages.len())..));
        behind.extend(self.behind.drain(skip.min(self.behind.len())..));
        self.end.at.resume(pb, ctx, pages, behind);
        ctx.boxes = self.end.boxes.clone();
        ctx.uses_totals = self.end.uses_totals;
        ctx.fields.section = self.end.section;
        ctx.fields.page_format = self.end.page_format;
        ctx.floats_above.extend(self.floats_above.drain().filter(|((b, _), _)| *b >= block));
        if let Some((_, at)) = self.typing.filter(|(word, _)| ctx.opts.typing_word == Some(*word)) {
            ctx.typing_at = Some(ctx.typing_at.map_or(at, |now| now.min(at)));
        }
        ctx.saved.extend(self.marks);
        STOPPED.with(|n| n.set(n.get() + 1));
        self.end.boxes_discarded
    }
}

/// Keep what the next pass needs to resume this one, as the walk ends (`pb`'s pages are the
/// walk's own, before anything is added to them). `before`: the state this pass was given, with
/// whether the pass used it (started from a mark of it, or could have stopped at one);
/// `boxes_discarded`: the text boxes laid out in attempts thrown away.
pub(crate) fn keep(
    input: &PassInput,
    ctx: &mut Ctx,
    pb: &mut PageBuilder,
    before: Option<(Box<Kept>, bool)>,
    boxes_discarded: usize,
) -> Option<Box<Kept>> {
    let marks = std::mem::take(&mut ctx.saved);
    if marks.is_empty() {
        return None;
    }
    let PassInput { doc, opts, env, sections, note_pages, totals, settle, .. } = *input;
    // What the pass was laid out from is kept as it is while it still holds.
    let (given, in_a_row) = match before {
        Some((kept, true)) => (kept.given, kept.in_a_row + 1),
        Some((kept, false)) if kept.given.holds(doc, opts, env, sections) => (kept.given, 0),
        _ => (Given::of(doc, opts, env, sections), 0),
    };
    Some(Box::new(Kept {
        given,
        body: doc.body.clone(),
        ends: sections.iter().map(|(end, _)| *end).collect(),
        typing: opts.typing_word,
        typing_at: ctx.typing_at,
        notes_hash: ctx.notes_hash,
        note_pages: note_pages.clone(),
        totals: totals.clone(),
        settle,
        marks,
        pages: pb.pages.clone(),
        behind: pb.behind.clone(),
        floats_above: ctx.floats_above.clone(),
        end: Some(End {
            at: Checkpoint::take(pb, ctx),
            boxes: ctx.boxes.clone(),
            boxes_discarded,
            uses_totals: ctx.uses_totals,
            section: ctx.fields.section,
            page_format: ctx.fields.page_format,
        }),
        in_a_row,
    }))
}
