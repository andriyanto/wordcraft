use std::collections::VecDeque;

use super::cache::{LayoutCache, Totals, env_hash, hash_of};
use super::ctx::{Ctx, Pass, with_pct_base};
use super::floats::{EarlyFloats, WrapArea, early_covered, left_out_objects};
use super::flow::{OutlinePass, place_outline_para, place_para};
use super::headers::{HfLinks, bookmark_pages, headers_footers, headers_footers_refer_to_pages, marks_key, mirror_even_pages, section_side};
use super::notes::{MAX_NOTE_PAGES, NoteSeps, note_info, place_endnotes};
use super::page::{PageBuilder, body_top_for, foot_top_for, item_bottom};
use super::placed::Placed;
use super::resume::{Mark, PassInput, converged, keep};
use super::table_flow::place_table;
use super::{
    Arc, Block, Counters, DocLayout, Document, FieldCtx, HashMap, InlineObject, LayoutOptions, Page, Paragraph, SectionProps, SectionStart, StoryRef,
    TextDirection, ViewMode, balloons, para, vertical,
};
use wordcraft_doc::legacy_form::LegacyForm;

/// How the view lays the document out: `web` for the continuous views, `reflow` for those and
/// the reading view, which both ignore the sections' page geometry.
#[derive(Clone, Copy)]
struct Mode {
    web: bool,
    reflow: bool,
}

impl Mode {
    fn of(view: ViewMode) -> Self {
        let web = matches!(view, ViewMode::Web | ViewMode::Draft | ViewMode::Outline);
        Mode { web, reflow: web || view == ViewMode::Read }
    }
}

/// Where a section's pages start and end: the body below the headers (on the first page, and on
/// the others) and the foot above the footers.
struct Tops {
    body: f32,
    first: f32,
    foot: f32,
    first_foot: f32,
}

/// A section being laid out: its index, its properties as laid out (turned for vertical text),
/// how it starts and its [`Tops`].
struct Sec<'p> {
    si: usize,
    sect: &'p SectionProps,
    page_format: wordcraft_doc::section::NumFormat,
    start: SectionStart,
    tops: Tops,
}

/// Forget cached layouts made for another environment (`env`), and what the last pass used —
/// unless this pass starts part-way (`resuming`): it then uses what the pass before did, too.
fn reset_cache(cache: &mut LayoutCache, env: u64, resuming: bool) {
    if env != cache.env {
        cache.paras.clear();
        cache.measures.clear();
        cache.env = env;
    }
    if !resuming {
        cache.used.clear();
    }
}

/// The walk state at the start of a pass, with the note references the body shows.
fn new_ctx<'a>(
    doc: &'a Document,
    cache: &'a mut LayoutCache,
    opts: &'a LayoutOptions,
    (note_pages, totals): (&HashMap<u32, usize>, &Totals),
) -> Ctx<'a> {
    let notes = note_info(doc, opts.hides_deleted(StoryRef::Body), note_pages);
    let notes_hash = hash_of(&{
        let mut v: Vec<_> = notes.iter().map(|(a, b)| (*a, b.n, b.fmt)).collect();
        v.sort_by_key(|(a, ..)| *a);
        v
    });
    Ctx {
        doc,
        cache,
        opts,
        hide_deleted: opts.hides_deleted(StoryRef::Body),
        counters: Counters::default(),
        fields: FieldCtx {
            page: 1,
            pages: totals.pages,
            section_pages: totals.section(0),
            section: 1,
            notes: Arc::new(notes),
            title: doc.core.title.as_str().into(),
            author: doc.core.creator.as_str().into(),
            filename: opts.filename.clone().unwrap_or_else(|| "".into()),
            filepath: opts.filepath.clone().unwrap_or_else(|| "".into()),
            ..Default::default()
        },
        notes_hash,
        numbers: HashMap::new(),
        eq_count: 0,
        boxes: wordcraft_doc::BoxBudget::default(),
        chains: Default::default(),
        grid: para::Grid::default(),
        uses_totals: false,
        track_totals: true,
        pct_base: None,
        quiet_marks: Vec::new(),
        stacked_marks: Vec::new(),
        mirror: false,
        table_keys: Vec::new(),
        fitted: HashMap::new(),
        header_tables: HashMap::new(),
        footer_wraps: HashMap::new(),
        pin: None,
        floats_above: HashMap::new(),
        hf: Arc::new(HfLinks::new(doc.sections())),
        alphas: Default::default(),
        header_repeats: 0,
        cur_block: 0,
        typing_at: None,
        saved: Vec::new(),
        saved_pages: 0,
        tail: None,
        stopped: false,
    }
}

/// The blocks whose paragraph mark takes no room: the empty paragraph ending a section that a
/// continuous break follows, and empty paragraphs with hidden marks while hidden text is not shown.
fn section_marks(doc: &Document, opts: &LayoutOptions, sections: &[(usize, &SectionProps)]) -> (Vec<usize>, Vec<usize>) {
    let (stacked, mut quiet): (Vec<_>, Vec<_>) = sections
        .windows(2)
        .filter(|w| w[1].1.start == SectionStart::Continuous)
        .map(|w| w[0].0)
        .filter(|end| doc.body.get(*end).and_then(|b| b.as_para()).is_some_and(|p| p.text.is_empty()))
        .partition(|end| {
            // Only a leading run mark is quiet; later marks stack at the page bottom (Word, #3907).
            end.checked_sub(1).and_then(|i| doc.body.get(i)).and_then(|b| b.as_para()).is_some_and(|p| p.section.is_some() && p.text.is_empty())
        });
    if !opts.show_hidden {
        quiet.extend(doc.body.iter().enumerate().filter_map(|(i, b)| {
            b.as_para().filter(|p| p.mark.hidden == Some(true) && p.plain_text().is_empty() && !has_inline_content(doc, opts, p)).map(|_| i)
        }));
    }
    quiet.sort_unstable();
    quiet.dedup();
    (stacked, quiet)
}

/// An in-line picture, shape or chart, a note reference mark, or a field shown as something other
/// than its result, is visible content with no plain text, so its line keeps its room even when
/// the paragraph mark is hidden (#5288). Not one the layout leaves out (a hidden drawing, or one
/// in hidden or deleted text).
fn has_inline_content(doc: &Document, opts: &LayoutOptions, p: &Paragraph) -> bool {
    let left_out = left_out_objects(doc, p, None, opts.show_hidden, opts.hides_deleted(StoryRef::Body));
    // As the paragraph layout shows a field: its code, a legacy check box, a drop-down's entry.
    let shown_field = |o: &InlineObject| match o {
        InlineObject::Field { code, .. } => {
            *code != opts.field_codes
                || o.legacy_form().is_some_and(|f| matches!(f, LegacyForm::CheckBox { .. }) || f.shown_text().is_some_and(|t| !t.is_empty()))
        }
        _ => false,
    };
    p.objects
        .iter()
        .enumerate()
        .any(|(k, o)| !left_out(k) && (matches!(o, InlineObject::NoteRef { .. }) || (o.frame().is_some() && !o.is_floating()) || shown_field(o)))
}

/// The page the continuous views lay out on: the first section's, as wide as the window (and as
/// high in the reading view), with small margins and one column.
fn reflow_sect(first: &SectionProps, opts: &LayoutOptions) -> SectionProps {
    let mut s = first.clone();
    s.page_w = if opts.web_width.is_finite() { opts.web_width.clamp(144.0, 20_000.0) } else { 300.0 };
    if opts.view == ViewMode::Read {
        s.page_h = if opts.read_height.is_finite() { opts.read_height.clamp(144.0, 20_000.0) } else { 500.0 };
        s.margin_bottom = 18.0;
        s.page_borders = None;
        s.line_numbers = None;
    }
    s.margin_left = 18.0;
    s.margin_right = 18.0;
    s.margin_top = 18.0;
    s.gutter = 0.0;
    s.columns = Default::default();
    s.valign = wordcraft_doc::section::PageVAlign::Top;
    s
}

fn new_page_builder(
    sect: &SectionProps,
    page_format: wordcraft_doc::section::NumFormat,
    early: &Arc<EarlyFloats>,
    web: bool,
    settle: bool,
) -> PageBuilder {
    PageBuilder {
        pages: Vec::new(),
        sect: Arc::new(sect.clone()),
        page_format,
        sect_idx: 0,
        col: 0,
        cols: Vec::new(),
        y: 0.0,
        top: 0.0,
        page_top: 0.0,
        bottom: 0.0,
        number: 0,
        web,
        notes: Vec::new(),
        carry: Vec::new(),
        orig_bottom: 0.0,
        prev: None,
        after_break_mark: false,
        next_starts_page: false,
        restart_at: None,
        excl: Vec::new(),
        float_tables: Vec::new(),
        line_no: 0,
        page_items_start: 0,
        block_marks: Vec::new(),
        page_valign: wordcraft_doc::section::PageVAlign::Top,
        seps: NoteSeps::default(),
        para_end: None,
        behind: Vec::new(),
        max_y: 0.0,
        content_bottom: 0.0,
        balance: None,
        col_break_page: None,
        sect_beneath: false,
        beneath: false,
        foot_top: f32::MAX,
        early: early.clone(),
        seeded: 0,
        settle,
        column_lines: None,
    }
}

/// Endnotes in document order, with the section of their reference; placed at the end of a
/// section that keeps them there, else at the end of the document.
fn endnote_queue(ctx: &Ctx) -> VecDeque<(u32, usize)> {
    let mut ids: Vec<(u32, u32, usize)> = ctx
        .fields
        .notes
        .iter()
        .filter(|(id, _)| ctx.doc.parts.get(id).is_some_and(|p| p.kind == wordcraft_doc::PartKind::Endnote))
        .map(|(id, n)| (n.order, *id, n.section))
        .collect();
    ids.sort();
    ids.into_iter().map(|(_, id, sect)| (id, sect)).collect()
}

/// Vertical sections are laid out on their pages turned on their side (see `vertical`).
fn turned_sections(ctx: &mut Ctx, sections: &[(usize, &SectionProps)], web: bool) -> Vec<Option<SectionProps>> {
    sections
        .iter()
        .map(|(_, s)| {
            (!web && s.text_direction.is_turned()).then(|| {
                ctx.fields.page_format = s.page_num_format;
                let top = body_top_for(ctx, s, s.headers.default);
                vertical::unturned_sect(s, top)
            })
        })
        .collect()
}

/// Where section `si`'s pages start and end. Headers and footers left out are linked to the
/// previous section's, and take its room. `fixed`: the margins are the room (continuous views and
/// turned sections).
fn section_tops(ctx: &mut Ctx, hf: &HfLinks, (si, sect): (usize, &SectionProps), fixed: bool) -> Tops {
    ctx.fields.page_format = sect.page_num_format;
    let (heads, feet) = (hf.heads.set(si), hf.feet.set(si));
    let hb = |i: Option<u32>| hf.defining_width(false, si, i);
    let fb = |i: Option<u32>| hf.defining_width(true, si, i);
    let body = if fixed { sect.margin_top } else { with_pct_base(ctx, hb(heads.default), |c| body_top_for(c, sect, heads.default)) };
    let first = if fixed || !sect.title_page { body } else { with_pct_base(ctx, hb(heads.first), |c| body_top_for(c, sect, heads.first)) };
    let foot = if fixed { f32::MAX } else { with_pct_base(ctx, fb(feet.default), |c| foot_top_for(c, sect, feet.default)) };
    let first_foot = if fixed || !sect.title_page { foot } else { with_pct_base(ctx, fb(feet.first), |c| foot_top_for(c, sect, feet.first)) };
    Tops { body, first, foot, first_foot }
}

/// Set up the walk state for a section before its header and footer room is measured: no grid yet
/// (the previous section's must not leak into headers, footers and separators), the field values
/// and the footnote separators.
fn begin_section_state(
    ctx: &mut Ctx,
    pb: &mut PageBuilder,
    (si, sect, page_format): (usize, &SectionProps, wordcraft_doc::section::NumFormat),
    totals: &Totals,
    web: bool,
) {
    let doc = ctx.doc;
    ctx.grid = para::Grid::default();
    pb.sect = Arc::new(sect.clone());
    pb.page_format = page_format;
    ctx.fields.page_format = page_format;
    pb.sect_idx = si;
    pb.sect_beneath = doc.note_options(sect, false).pos == wordcraft_doc::section::NotePos::BeneathText;
    if !web && !doc.footnote_separators.is_empty() {
        pb.seps = NoteSeps::new(ctx, &doc.footnote_separators, sect.text_width());
    }
    ctx.fields.section = u32::try_from(si).unwrap_or(u32::MAX).saturating_add(1);
    ctx.fields.section_pages = totals.section(si);
}

/// The section's own grid and line numbering, once its header and footer room is known.
fn apply_section_grid(ctx: &mut Ctx, pb: &mut PageBuilder, sect: &SectionProps) {
    ctx.grid = para::Grid::of(ctx.doc, sect);
    if sect.line_numbers.as_ref().is_some_and(|l| l.restart == wordcraft_doc::section::LineNumberRestart::Section) {
        pb.line_no = 0;
    }
}

/// The blank page that puts a section on the right side ends the previous section.
fn end_previous_section<'p>(ctx: &mut Ctx, pb: &mut PageBuilder, hf: &HfLinks<'p>, si: usize, block: usize) {
    let Some((_, prev)) = si.checked_sub(1).and_then(|k| hf.sections.get(k)) else { return };
    pb.sect = Arc::new((*prev).clone());
    pb.page_format = prev.page_num_format;
    ctx.fields.page_format = prev.page_num_format;
    pb.sect_idx = si - 1;
    let (ph, pf) = (hf.heads.set(si - 1), hf.feet.set(si - 1));
    let top = body_top_for(ctx, prev, ph.default);
    pb.foot_top = foot_top_for(ctx, prev, pf.default);
    pb.new_page(block, top);
}

/// Start the section on a new page, after the blank one that puts it on the right side if any.
fn start_section_page<'p>(ctx: &mut Ctx, pb: &mut PageBuilder, hf: &HfLinks<'p>, sec: &Sec<'p>, (mode, block): (Mode, usize)) {
    // A page holds text running one way: a section turning the other way starts a page.
    if !mode.web {
        pb.take_back_break_page();
    }
    let s = &ctx.doc.settings;
    let (filler, restart) = section_side(sec.start, sec.sect.page_num_start, pb.number, pb.pages.len(), s.even_odd_headers, s.mirror_margins);
    if filler && !mode.web {
        end_previous_section(ctx, pb, hf, sec.si, block);
        pb.sect = Arc::new(sec.sect.clone());
        pb.page_format = sec.page_format;
        ctx.fields.page_format = sec.page_format;
        pb.sect_idx = sec.si;
    }
    if restart.is_some() {
        pb.restart_at = restart;
    }
    pb.foot_top = sec.tops.first_foot;
    pb.new_page(block, sec.tops.first);
}

/// Continuous: new column layout below the current text (its tallest column).
fn continue_page(pb: &mut PageBuilder, sect: &SectionProps, block: usize) {
    pb.close_column_lines();
    pb.cols = sect.column_boxes();
    pb.col = 0;
    pb.top = pb.text_bottom();
    pb.y = pb.top;
    pb.max_y = pb.top;
    pb.open_column_lines(block);
}

/// Whether the section's last page balances its columns (see [`balance_columns`]), and set what
/// the builder needs to know about the sections after it. Newspaper columns are balanced when the
/// next section starts on the same page (a continuous break), as in Word. That page is laid out
/// again from the start of the block that reaches it, with shorter columns.
fn prepare_section_end(pb: &mut PageBuilder, sections: &[(usize, &SectionProps)], (si, web): (usize, bool)) -> bool {
    let next = sections.get(si + 1).map(|(_, s)| s.start);
    // An ODF section may ask for balanced columns at the end of the document too, or for none.
    let follows = match pb.sect.balance_columns {
        None => next == Some(SectionStart::Continuous),
        Some(true) => next.is_none() || next == Some(SectionStart::Continuous),
        Some(false) => false,
    };
    let balance = !web && pb.cols.len() > 1 && pb.sect.valign == wordcraft_doc::section::PageVAlign::Top && follows;
    pb.col_break_page = None;
    pb.next_starts_page = !web && next.is_some_and(|s| s != SectionStart::Continuous);
    balance
}

fn place_block(ctx: &mut Ctx, pb: &mut PageBuilder, outline: &Option<OutlinePass>, block: usize, body_top: f32) {
    let Some(b) = ctx.doc.body.get(block) else { return };
    match (&**b, outline) {
        (Block::Para(p), Some(o)) => place_outline_para(ctx, pb, p, block, o),
        (Block::Table(_), Some(o)) if o.hidden.get(block).copied().unwrap_or(false) => {}
        (Block::Para(p), None) => place_para(ctx, pb, p, block, body_top),
        (Block::Table(t), _) => place_table(ctx, pb, t, block, body_top),
    }
}

/// What the marks taken in a section say of it ([`Mark`]); `None` where none are taken.
#[derive(Clone, Copy)]
struct Marking {
    si: usize,
    entry_page: u32,
    /// Text boxes thrown away before the section.
    discarded: usize,
}

/// Place the blocks from `block` to `last`, balancing the columns of the last page if asked.
/// Returns the block after them and the text boxes laid out in the balancing attempts thrown away.
fn place_section_blocks(
    ctx: &mut Ctx,
    pb: &mut PageBuilder,
    outline: &Option<OutlinePass>,
    (mut block, last): (usize, usize),
    (balance, body_top): (bool, f32),
    marking: Option<Marking>,
) -> (usize, usize) {
    let mut from = balance.then(|| (block, Snap::take(pb, ctx)));
    // Checking that a snapshot holds all of the walk's state: every block is placed, taken back
    // and placed again, which must leave the pages as placing it once does.
    let replay = crate::verify::replaying();
    let mut replayed_boxes = 0usize;
    while block <= last && ctx.doc.body.get(block).is_some() {
        let before = balance.then(|| (pb.pages.len(), Snap::take(pb, ctx)));
        // In the state the last pass was in here, with the same blocks from here on: the rest
        // is what it was.
        if let Some(m) = marking
            && !balance
            && ctx.tail.is_some()
            && converged(ctx, pb, block, (m.si, m.entry_page, body_top, m.discarded + replayed_boxes))
        {
            ctx.stopped = true;
            break;
        }
        let again = replay.then(|| Checkpoint::take(pb, ctx));
        // A mark at the first block boundary of a page, every few pages, where the walk's state
        // is all a later pass needs to start from here (not while columns are balanced, which goes back
        // over blocks, nor once pictures' outlines have been charged to this pass's budget).
        if let Some(m) = marking
            && !balance
            && block > 0
            && pb.pages.len() >= ctx.saved_pages + crate::resume::pages_between_marks()
            && ctx.alphas.untouched()
        {
            ctx.saved_pages = pb.pages.len();
            ctx.saved.push(Mark {
                block,
                si: m.si,
                entry_page: m.entry_page,
                body_top,
                at: Checkpoint::take(pb, ctx),
                boxes: ctx.boxes.clone(),
                boxes_discarded: m.discarded + replayed_boxes,
                uses_totals: ctx.uses_totals,
            });
        }
        ctx.cur_block = block;
        pb.mark_block();
        place_block(ctx, pb, outline, block, body_top);
        if let Some(saved) = again {
            replayed_boxes += ctx.boxes.total().saturating_sub(saved.snap.boxes);
            saved.restore(pb, ctx);
            pb.mark_block();
            place_block(ctx, pb, outline, block, body_top);
        }
        if let Some((pages, snap)) = before
            && pb.pages.len() != pages
        {
            from = Some((block, snap));
        }
        block += 1;
    }
    ctx.cur_block = 0;
    let discarded = from.map_or(0, |(first, snap)| balance_columns(ctx, pb, (first, block), &snap, body_top));
    (block, discarded + replayed_boxes)
}

/// Endnotes at the end of the section: those referenced so far.
fn section_endnotes(ctx: &mut Ctx, pb: &mut PageBuilder, endnotes: &mut VecDeque<(u32, usize)>, (sect, si, block): (&SectionProps, usize, usize)) {
    if ctx.doc.note_options(sect, true).pos != wordcraft_doc::section::NotePos::SectEnd || pb.pages.is_empty() {
        return;
    }
    let mut here = Vec::new();
    while let Some((id, _)) = endnotes.front().filter(|(_, s)| *s <= si).copied() {
        endnotes.pop_front();
        here.push(id);
    }
    place_endnotes(ctx, pb, here, block.saturating_sub(1));
}

/// Finish the pages: vertical alignment, the footnotes still continuing after the last page
/// (each page takes at least a line of the first note, so this ends; the cap is a backstop), and
/// the turned pages of vertical sections.
fn finish_pages(pb: PageBuilder, doc: &Document, sections: &[(usize, &SectionProps)], web: bool) -> Vec<Page> {
    let mut pb = pb;
    pb.close_column_lines();
    pb.apply_valign();
    pb.flush_notes();
    let last_block = doc.body.len().saturating_sub(1);
    for _ in 0..MAX_NOTE_PAGES {
        if pb.carry.is_empty() {
            break;
        }
        let top = pb.top;
        pb.new_page(last_block, top);
        pb.flush_notes();
    }
    pb.carry.clear();
    let mut pages = pb.pages;
    if !web {
        for p in &mut pages {
            if let Some((_, s)) = sections.get(p.section) {
                vertical::turn_page(p, s.text_direction, s);
            }
        }
    }
    pages
}

/// The web page is one long page as high as its text.
fn fit_web_page(pages: &mut [Page]) {
    let Some(p) = pages.first_mut() else { return };
    let bottom = p
        .items
        .iter()
        .filter_map(|it| if let Placed::Lines { y, para, l0, l1, .. } = it { item_bottom(*y, para, *l0, *l1) } else { None })
        .fold(0.0f32, f32::max);
    p.h = bottom + 36.0;
}

/// The floats found above their anchors, if they aren't all in the `early` floats' wrap areas.
fn uncovered_floats(ctx: &mut Ctx, early: &EarlyFloats) -> Option<EarlyFloats> {
    let mut above: Vec<((usize, usize), usize, WrapArea)> =
        ctx.floats_above.drain().filter_map(|(k, v)| v.map(|(page, area)| (k, page, area))).collect();
    above.sort_by_key(|(k, ..)| *k);
    let mut found = EarlyFloats::new();
    for ((block, oi), page, area) in above {
        found.entry(page).or_default().push((block, oi, area));
    }
    (!early_covered(&found, early)).then_some(found)
}

/// Fill in what depends on the finished pagination: page totals and page references in headers
/// and footers (they don't make the body lay out again), mirrored margins. The totals found if the
/// body must be laid out again for them.
fn fill_headers_footers(
    ctx: &mut Ctx,
    pages: &mut [Page],
    sections: &[(usize, &SectionProps)],
    (totals, may_repeat): (&Totals, bool),
) -> Option<Totals> {
    let found = Totals::of(pages, sections);
    if ctx.uses_totals && may_repeat && found != *totals {
        return Some(found);
    }
    ctx.track_totals = false;
    if headers_footers_refer_to_pages(ctx.doc) {
        let marks = bookmark_pages(ctx.doc, pages, sections);
        ctx.fields.bookmarks_key = marks_key(&marks);
        ctx.fields.bookmark_pages = Arc::new(marks);
    }
    headers_footers(ctx, pages, sections, &found);
    if ctx.doc.settings.mirror_margins {
        mirror_even_pages(pages, sections);
    }
    None
}

/// Evict paragraph layouts that weren't used this pass if the cache grew large.
fn evict_unused(cache: &mut LayoutCache) {
    if cache.paras.len() + cache.measures.len() > 4 * cache.used.len().max(1024) {
        let used = std::mem::take(&mut cache.used);
        cache.paras.retain(|k, _| used.contains(k));
        cache.measures.retain(|k, _| used.contains(k));
        cache.used = used;
    }
}

fn index_lines(pages: &[Page]) -> crate::LineIndex {
    let mut index = crate::LineIndex::default();
    for (pi, p) in pages.iter().enumerate() {
        for (ii, it) in p.items.iter().enumerate() {
            if let Placed::Lines { story, path, .. } = it {
                index.entry((*story, path.clone())).or_default().push((pi, ii));
            }
        }
    }
    index
}

/// One layout pass, numbering footnotes restarted on each page by `note_pages` ([`note_info`]),
/// with `totals` assumed for the page totals in the body. With `may_repeat`, a pass whose body
/// shows totals that turn out different asks to be done again. Pages start with the wrap areas
/// of the `early` floats; with `may_rewrap`, a pass that finds others asks to be done again.
pub(crate) fn layout_pass(
    doc: &Document,
    cache: &mut LayoutCache,
    opts: &LayoutOptions,
    note_pages: &HashMap<u32, usize>,
    totals: &Totals,
    may_repeat: bool,
    (early, may_rewrap): (&Arc<EarlyFloats>, bool),
) -> Pass {
    let mode = Mode::of(opts.view);
    let env = env_hash(doc, opts);
    let settle = !may_rewrap && !mode.reflow;
    let doc_sections = doc.sections();
    let input = PassInput { doc, opts, env, sections: &doc_sections, note_pages, totals, early, settle };
    // Start from where the document changed, if the pass before left what that takes.
    let (mut resume, mut kept) = match cache.kept.take().map(|k| k.resume(&input)) {
        Some(Ok(r)) => (Some(r), None),
        Some(Err(k)) => (None, Some((k, false))),
        None => (None, None),
    };
    let mut ctx = new_ctx(doc, cache, opts, (note_pages, totals));
    if let Some(r) = resume.take_if(|r| !r.same_notes(ctx.notes_hash)) {
        kept = Some((r.give_up(), false));
    }
    // And stop where the walk comes to be as it was last time, with the same blocks after it.
    let stop_from = match (&resume, &kept) {
        (Some(r), _) => r.stoppable(&input, ctx.notes_hash),
        (None, Some((k, _))) => k.stoppable(&input, ctx.notes_hash),
        (None, None) => None,
    };
    if resume.is_none()
        && let (Some(from), Some((k, used))) = (stop_from, kept.as_mut())
    {
        ctx.tail = k.whole_tail(from);
        *used = ctx.tail.is_some();
    }
    reset_cache(ctx.cache, env, resume.is_some() || ctx.tail.is_some());
    let hf = Arc::clone(&ctx.hf);
    let sections = &hf.sections;
    ctx.mirror = doc.settings.mirror_margins && !mode.reflow;
    (ctx.stacked_marks, ctx.quiet_marks) = section_marks(doc, opts, sections);
    let outline = (opts.view == ViewMode::Outline).then(|| OutlinePass::new(doc, opts.outline.clone().unwrap_or_default()));
    let default_sect = SectionProps::default();
    let first_sect = sections.first().map_or(&default_sect, |(_, s)| *s);
    let web_sect = mode.reflow.then(|| reflow_sect(first_sect, opts));
    let sect_ref = web_sect.as_ref().unwrap_or(first_sect);
    let mut pb = new_page_builder(sect_ref, first_sect.page_num_format, early, mode.web, settle);
    // Marks are for walks whose state they hold whole: pages (the long page of the continuous
    // views would be copied at every mark), every block placed (not the outline view), and no
    // text flowing from one box to another, which reads boxes anywhere in the document.
    let marking = !mode.web && outline.is_none() && !doc.parts.values().any(|p| p.body.next.is_some());
    let mut endnotes = endnote_queue(&ctx);
    // Text boxes laid out in balancing attempts that were thrown away.
    let mut boxes_discarded = 0usize;
    let mut block = 0usize;
    let turned = turned_sections(&mut ctx, sections, mode.web);
    let mut prev_turn = TextDirection::Horizontal;
    for (si, (end, sect)) in sections.iter().enumerate() {
        let page_format = sect.page_num_format;
        let turn = if mode.reflow { TextDirection::Horizontal } else { sect.text_direction };
        let sect: &SectionProps = match turned.get(si) {
            Some(Some(t)) => t,
            _ if mode.reflow => sect_ref,
            _ => sect,
        };
        let start = if si == 0 { SectionStart::NextPage } else { sect.start };
        let last = (*end).min(doc.body.len().saturating_sub(1));
        let from = resume.as_ref().and_then(|r| r.mark()).map(|m| (m.si, m.entry_page, m.block, m.body_top));
        if let Some((at, ..)) = from
            && si < at
        {
            // A section before the mark: its pages are kept. Its endnotes were placed at its end.
            if doc.note_options(sect, true).pos == wordcraft_doc::section::NotePos::SectEnd {
                while endnotes.front().is_some_and(|(_, s)| *s <= si) {
                    endnotes.pop_front();
                }
            }
            (block, prev_turn) = (last + 1, turn);
            continue;
        }
        // The page field as the section's header and footer room was measured with.
        let entry_page = from.map_or(ctx.fields.page, |(_, page, ..)| page);
        ctx.fields.page = entry_page;
        if !mode.web && !pb.pages.is_empty() && (start != SectionStart::Continuous || turn != prev_turn) {
            // The page the section before ends on is finished with its own margins and footnote
            // separators, before this section's replace them: the footnotes pending on it
            // belong to the section whose page it is (#5289).
            pb.finish_page();
        }
        // This section's fields and separators, without the previous section's grid, before its
        // header and footer room is measured; its own grid once that room is known (#4501).
        begin_section_state(&mut ctx, &mut pb, (si, sect, page_format), totals, mode.web);
        let tops = section_tops(&mut ctx, &hf, (si, sect), mode.reflow || turn.is_turned());
        // Measuring the room took the (reflow) section's own number format; the body keeps the
        // document section's.
        ctx.fields.page_format = page_format;
        apply_section_grid(&mut ctx, &mut pb, sect);
        let sec = Sec { si, sect, page_format, start, tops };
        let (balance, body_top) = if let (Some((.., at, body_top)), Some(r)) = (from, resume.take()) {
            // The mark's section: go on from the mark, in the state the walk was in there.
            let (was, discarded) = r.start(&mut pb, &mut ctx, stop_from);
            (kept, block, boxes_discarded) = (Some((was, true)), at, discarded);
            (false, body_top)
        } else {
            if mode.web && si > 0 {
                // one long page
            } else if pb.pages.is_empty() || start != SectionStart::Continuous || turn != prev_turn {
                start_section_page(&mut ctx, &mut pb, &hf, &sec, (mode, block));
            } else {
                continue_page(&mut pb, sect, block);
            }
            pb.foot_top = sec.tops.foot;
            (prepare_section_end(&mut pb, sections, (si, mode.web)), sec.tops.body)
        };
        prev_turn = turn;
        let mark = marking.then_some(Marking { si, entry_page, discarded: boxes_discarded });
        let (next, discarded) = place_section_blocks(&mut ctx, &mut pb, &outline, (block, last), (balance, body_top), mark);
        (block, boxes_discarded) = (next, boxes_discarded + discarded);
        if ctx.stopped {
            break;
        }
        section_endnotes(&mut ctx, &mut pb, &mut endnotes, (sect, si, block));
    }
    if let Some(tail) = ctx.tail.take().filter(|_| ctx.stopped) {
        // The walk stopped at a mark of the last pass: the rest is that pass's.
        boxes_discarded = tail.finish(&mut pb, &mut ctx);
    } else {
        if pb.pages.is_empty() {
            pb.new_page(0, sect_ref.margin_top);
        }
        // The rest of the endnotes after the last paragraph.
        let rest: Vec<u32> = endnotes.into_iter().map(|(id, _)| id).collect();
        place_endnotes(&mut ctx, &mut pb, rest, block);
    }
    let kept = keep(&input, &mut ctx, &mut pb, kept, boxes_discarded);
    ctx.cache.kept = kept;
    let mut pages = finish_pages(pb, doc, sections, mode.web);
    if mode.web {
        fit_web_page(&mut pages);
    }
    ctx.grid = para::Grid::default();
    if !mode.reflow
        && may_rewrap
        && let Some(found) = uncovered_floats(&mut ctx, early)
    {
        return Pass::Rewrap(found);
    }
    if !mode.reflow
        && let Some(found) = fill_headers_footers(&mut ctx, &mut pages, sections, (totals, may_repeat))
    {
        return Pass::Again(found);
    }
    evict_unused(ctx.cache);
    let mut lay = DocLayout { index: index_lines(&pages), pages, ms: 0.0, text_boxes: 0 };
    if opts.comment_balloons && !mode.web {
        balloons::add(&mut ctx, &mut lay);
    }
    lay.text_boxes = ctx.boxes.total().saturating_sub(boxes_discarded);
    Pass::Done(lay)
}

/// Pagination state saved before a block, to lay the blocks from there out again (balancing
/// columns): the builder without its pages, how far its last page had got, and the walk state.
struct Snap {
    pb: PageBuilder,
    pages: usize,
    items: usize,
    behind: usize,
    counters: Counters,
    eq_count: u32,
    page_field: u32,
    boxes: usize,
    header_repeats: usize,
}

impl Snap {
    fn take(pb: &mut PageBuilder, ctx: &Ctx) -> Self {
        // The pages stay out of the copy: it would cost the whole document per block.
        let (pages, behind) = (std::mem::take(&mut pb.pages), std::mem::take(&mut pb.behind));
        let copy = pb.clone();
        (pb.pages, pb.behind) = (pages, behind);
        Snap {
            pb: copy,
            pages: pb.pages.len(),
            items: pb.pages.last().map_or(0, |p| p.items.len()),
            behind: pb.behind.last().copied().unwrap_or(0),
            counters: ctx.counters.clone(),
            eq_count: ctx.eq_count,
            page_field: ctx.fields.page,
            boxes: ctx.boxes.total(),
            header_repeats: ctx.header_repeats,
        }
    }

    /// Go back to this state: drop the pages added since and what was added to the last one
    /// (appended, or inserted at the front behind the text).
    fn restore(&self, pb: &mut PageBuilder, ctx: &mut Ctx) {
        let (mut pages, mut behind) = (std::mem::take(&mut pb.pages), std::mem::take(&mut pb.behind));
        pages.truncate(self.pages);
        behind.truncate(self.pages);
        if let Some(pg) = pages.last_mut() {
            let k = behind.last().copied().unwrap_or(0).saturating_sub(self.behind).min(pg.items.len());
            pg.items.drain(..k);
            pg.items.truncate(self.items);
        }
        if let Some(b) = behind.last_mut() {
            *b = self.behind;
        }
        *pb = self.pb.clone();
        (pb.pages, pb.behind) = (pages, behind);
        ctx.counters = self.counters.clone();
        ctx.eq_count = self.eq_count;
        ctx.fields.page = self.page_field;
        ctx.header_repeats = self.header_repeats;
    }
}

/// Pagination state at a block boundary that owns everything it needs: a [`Snap`], and the page
/// being filled as it stood. A `Snap` goes back by taking away what was added to that page, which
/// is enough while the page is still being filled (balancing columns stays on one page). Once a
/// later block has finished the page, its items have been moved (vertical alignment), added to
/// (footnotes, column lines) and listed (floating tables), so going back to before that needs
/// the page itself. Laying out from a `Checkpoint` is what resuming pagination where an edit is
/// will do; [`crate::replay_blocks`] checks that it holds all of the walk's state.
pub(crate) struct Checkpoint {
    snap: Snap,
    /// The page being filled when the checkpoint was taken (`None` before the first page).
    page: Option<Page>,
}

impl Checkpoint {
    pub(crate) fn take(pb: &mut PageBuilder, ctx: &Ctx) -> Self {
        Checkpoint { snap: Snap::take(pb, ctx), page: pb.pages.last().cloned() }
    }

    /// Go back to this state: the pages started since are dropped and the page being filled is
    /// as it was.
    fn restore(&self, pb: &mut PageBuilder, ctx: &mut Ctx) {
        self.snap.restore(pb, ctx);
        if let (Some(now), Some(then)) = (pb.pages.last_mut(), &self.page) {
            *now = then.clone();
        }
    }

    /// How many pages there were (the last being filled).
    pub(crate) fn pages(&self) -> usize {
        self.snap.pages
    }

    /// Whether the walk is in this state now: every value the builder and the walk hold, and
    /// the page being filled. (The values are compared as they print, which every one of them
    /// does in full; the few that tell most walks apart are compared first.)
    pub(crate) fn same(&self, pb: &mut PageBuilder, ctx: &Ctx) -> bool {
        let s = &self.snap;
        if s.pages != pb.pages.len()
            || s.pb.y != pb.y
            || s.pb.number != pb.number
            || s.pb.col != pb.col
            || s.items != pb.pages.last().map_or(0, |p| p.items.len())
            || s.behind != pb.behind.last().copied().unwrap_or(0)
            || s.eq_count != ctx.eq_count
            || s.page_field != ctx.fields.page
            || s.header_repeats != ctx.header_repeats
        {
            return false;
        }
        if format!("{:?}", s.counters) != format!("{:?}", ctx.counters) || format!("{:?}", self.page) != format!("{:?}", pb.pages.last()) {
            return false;
        }
        // The builder without its pages, as the snapshot holds it.
        let (pages, behind) = (std::mem::take(&mut pb.pages), std::mem::take(&mut pb.behind));
        let same = format!("{:?}", s.pb) == format!("{pb:?}");
        (pb.pages, pb.behind) = (pages, behind);
        same
    }

    /// Start a walk in this state, in another pass: `pages` (and what is `behind` the text on
    /// each) are the pages of the walk it was taken in, of which those before the one being
    /// filled are kept.
    pub(crate) fn resume(&self, pb: &mut PageBuilder, ctx: &mut Ctx, pages: Vec<Page>, behind: Vec<usize>) {
        (pb.pages, pb.behind) = (pages, behind);
        self.restore(pb, ctx);
    }
}

/// One balancing pass: blocks `first..end` laid out again from `snap` with the columns on `page`
/// ending `h` below `top`; true if the page's text fits them.
fn balance_pass(
    ctx: &mut Ctx,
    pb: &mut PageBuilder,
    snap: &Snap,
    (first, end): (usize, usize),
    (page, top, h): (usize, f32, f32),
    body_top: f32,
) -> bool {
    snap.restore(pb, ctx);
    pb.balance = Some((page, h));
    pb.cap_columns();
    for block in first..end {
        match ctx.doc.body.get(block).map(|b| &**b) {
            Some(Block::Para(p)) => place_para(ctx, pb, p, block, body_top),
            Some(Block::Table(t)) => place_table(ctx, pb, t, block, body_top),
            None => break,
        }
        // Spilled onto another page: too short (and no need to lay out the rest).
        if pb.pages.len() > page + 1 {
            break;
        }
    }
    pb.balance = None;
    pb.cap_columns();
    pb.pages.len() == page + 1 && pb.content_bottom <= top + h + 0.5
}

/// Balancing lays out at most this many pages again per pass (a block reaching the last page
/// from further back is left unbalanced), so its cost stays a small multiple of one page.
const BALANCE_MAX_PAGES: usize = 16;

/// Passes of the binary search on the column height (enough for well under a point).
const BALANCE_PASSES: usize = 12;

/// Balance the columns on the last page of a section followed by a continuous break, as Word
/// does: lay blocks `first..end` out again from `snap` (taken before `first`) with the shortest
/// columns that still hold that page's text — a binary search on the column height, where a
/// height fits when nothing moves to another page and nothing (an unsplittable row, a line
/// taller than the column) runs past it. Keep-together, keep-with-next and widow/orphan control
/// apply as usual, so the columns may come out a little uneven. Not balanced: a page with an
/// explicit column break, which says where the columns end. Returns the text boxes laid out in the
/// passes thrown away.
fn balance_columns(ctx: &mut Ctx, pb: &mut PageBuilder, blocks: (usize, usize), snap: &Snap, body_top: f32) -> usize {
    let page = pb.pages.len().saturating_sub(1);
    let ncols = pb.cols.len().max(1) as f32;
    let top = pb.top;
    let full = pb.uncapped_bottom() - top;
    let used = pb.content_bottom.min(pb.uncapped_bottom()) - top;
    if pb.col_break_page == Some(page)
        || snap.pages == 0
        || (page + 1).saturating_sub(snap.pages) > BALANCE_MAX_PAGES
        || used <= 1.0
        || full <= 1.0
        || pb.cols.len() < 2
    {
        return 0;
    }
    let pass = |ctx: &mut Ctx, pb: &mut PageBuilder, h: f32| balance_pass(ctx, pb, snap, blocks, (page, top, h), body_top);
    // The text on the page fills `ncols` columns at least `used / ncols` high; the original layout
    // (`full`) fits, and so, nearly always, do columns as high as its tallest one.
    let mut lo = (used / ncols - 1.0).max(0.0);
    let mut hi = full;
    let mut last = None;
    let discard = |ctx: &Ctx| ctx.boxes.total().saturating_sub(snap.boxes);
    let mut thrown = discard(ctx);
    if used < full - 0.5 {
        let ok = pass(ctx, pb, used);
        if ok {
            hi = used;
        }
        last = Some((used, ok));
    }
    for _ in 0..BALANCE_PASSES {
        if hi - lo <= 1.0 {
            break;
        }
        let mid = (lo + hi) / 2.0;
        thrown = discard(ctx);
        let ok = pass(ctx, pb, mid);
        if ok {
            hi = mid;
        } else {
            lo = mid;
        }
        last = Some((mid, ok));
    }
    // End on the best height (the full column, as laid out first, when none was shorter).
    if last != Some((hi, true)) {
        thrown = discard(ctx);
        if !pass(ctx, pb, hi) && hi < full {
            thrown = discard(ctx);
            pass(ctx, pb, full);
        }
    }
    thrown
}

/// Milliseconds since an arbitrary epoch (monotonic on native; the browser's clock on the web).
pub fn now_ms() -> f64 {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        static T0: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        T0.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1000.0
    }
    // `Instant::now()` panics on wasm32-unknown-unknown, so ask the browser's clock.
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        let ms = js_sys::Date::now();
        if ms.is_finite() { ms } else { 0.0 }
    }
}
