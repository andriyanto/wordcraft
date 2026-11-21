//! Subpixel (LCD) text, as ClearType draws it on Windows: each of a pixel's red, green and blue
//! stripes is covered by the text on its own, so small text is sharper sideways than with grey
//! edges.
//!
//! vello_cpu fills paths with one coverage per pixel, so the page is drawn three ways:
//! 1. as always ([`Pass::All`]),
//! 2. without the text that takes subpixel edges ([`Pass::NoText`]), what lies under it,
//! 3. that text alone, as white coverage, three times as wide ([`Pass::Mask`]): a coverage for
//!    every stripe, smoothed with FreeType's default LCD filter so the edges don't turn into
//!    coloured fringes.
//!
//! Where (1) and (2) differ, the text shows: the difference divided by the pixel's coverage is
//! what full coverage adds there (the text colour over what is under it, faded with its layer),
//! so each channel gets that times its own stripe's coverage. Text covered by something drawn
//! over it (a shape in front) makes no difference and stays as it is.

use std::cell::Cell;

use vello_cpu::kurbo::Affine;
use wordcraft_doc::Document;
use wordcraft_layout::Page;

use crate::RenderOptions;

/// The order of a screen's colour stripes, left to right: what Windows calls the ClearType
/// "RGB" or "BGR" orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SubpixelOrder {
    Rgb,
    Bgr,
}

/// What a page render draws (see the module documentation).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pass {
    /// Everything.
    All,
    /// Everything but the text drawn with subpixel edges.
    NoText,
    /// That text alone, white, on nothing.
    Mask,
}

thread_local! {
    // The drawing code reads it rather than taking it through every call: a page render draws
    // on the thread that asked for it (vello_cpu's worker threads only rasterise).
    static PASS: Cell<Pass> = const { Cell::new(Pass::All) };
    // Whether the text mask being drawn has any text in it.
    static MASKED: Cell<bool> = const { Cell::new(false) };
}

/// Note that the text mask has text in it ([`Pass::Mask`] draws some).
pub(crate) fn note_masked() {
    MASKED.with(|m| m.set(true));
}

/// Puts back the pass that was current before [`Pass::set`].
pub(crate) struct PassGuard(Pass);

impl Drop for PassGuard {
    fn drop(&mut self) {
        PASS.with(|p| p.set(self.0));
    }
}

impl Pass {
    pub(crate) fn current() -> Pass {
        PASS.with(Cell::get)
    }

    /// Draw `pass` until the guard goes.
    #[must_use]
    pub(crate) fn set(pass: Pass) -> PassGuard {
        PassGuard(PASS.with(|p| p.replace(pass)))
    }
}

/// FreeType's default LCD filter (`FT_LCD_FILTER_DEFAULT`): each stripe's coverage spread over
/// its neighbours, in 256ths. It keeps the ink of every colour the same and its edges neutral.
const FILTER: [u32; 5] = [8, 77, 86, 77, 8];

/// A fully covered stripe after the filter: 255 in 256ths.
const FULL: u32 = 255 * 256;

/// Most subpixels a text mask may hold (128 MB of RGBA): a larger raster keeps grey edges.
const MAX_MASK_SUBPIXELS: u64 = 32_000_000;

/// Least coverage, in thirds of a pixel's 765, of the pixel the text colour is read from: below
/// it the colour difference is too faint to divide by.
const MIN_SOURCE_COVERAGE: u32 = 115;

/// Whether a `w`×`h` raster can take subpixel text: its mask is three times as wide.
pub(crate) fn fits(w: u32, h: u32) -> bool {
    let wide = u64::from(w) * 3;
    wide <= u64::from(u16::MAX) && wide * u64::from(h) <= MAX_MASK_SUBPIXELS
}

/// `shown` (the page drawn as always, `w`×`h` with `view`) with its upright text's edges drawn
/// per stripe in `order`.
pub(crate) fn compose(
    doc: &Document,
    page: &Page,
    (w, h): (u16, u16),
    view: Affine,
    opts: &RenderOptions,
    order: SubpixelOrder,
    shown: Vec<u8>,
) -> Vec<u8> {
    let wide = w.saturating_mul(3);
    let mut fresh = Scratch::default();
    // The kept buffers, unless a render on this thread has them (none does: only a page render
    // composes, and it draws no page inside).
    SCRATCH.with(|kept| match kept.try_borrow_mut() {
        Ok(mut kept) => compose_with(&mut kept, doc, page, (w, h, wide), view, opts, order, shown),
        Err(_) => compose_with(&mut fresh, doc, page, (w, h, wide), view, opts, order, shown),
    })
}

/// The text mask and the page without its text, kept between renders on a thread.
#[derive(Default)]
struct Scratch {
    mask: Vec<u8>,
    bare: Vec<u8>,
}

thread_local! {
    static SCRATCH: std::cell::RefCell<Scratch> = std::cell::RefCell::new(Scratch::default());
}

/// Largest buffers kept between renders (a full page at 2 px per point is about 31 MB in all):
/// a larger raster's are let go.
const MAX_KEPT_BYTES: usize = 48 << 20;

#[allow(clippy::too_many_arguments)]
fn compose_with(
    scratch: &mut Scratch,
    doc: &Document,
    page: &Page,
    (w, h, wide): (u16, u16, u16),
    view: Affine,
    opts: &RenderOptions,
    order: SubpixelOrder,
    mut shown: Vec<u8>,
) -> Vec<u8> {
    MASKED.with(|m| m.set(false));
    crate::paint_page_into(&mut scratch.mask, doc, page, (wide, h), Affine::scale_non_uniform(3.0, 1.0) * view, opts, Pass::Mask);
    // Some such text on this raster: draw its edges per stripe.
    if MASKED.with(Cell::get) {
        crate::paint_page_into(&mut scratch.bare, doc, page, (w, h), view, opts, Pass::NoText);
        blend(&mut shown, &scratch.bare, &scratch.mask, usize::from(w), order);
    }
    if scratch.mask.capacity() + scratch.bare.capacity() > MAX_KEPT_BYTES {
        *scratch = Scratch::default();
    }
    shown
}

/// The blend described in the module documentation, into `shown`. Leaves it as it is if the
/// rasters' sizes don't agree, and says whether it blended.
pub(crate) fn blend(shown: &mut [u8], bare: &[u8], mask: &[u8], w: usize, order: SubpixelOrder) -> bool {
    let (Some(sub), Some(row), Some(mask_len)) = (w.checked_mul(3), w.checked_mul(4), shown.len().checked_mul(3)) else { return false };
    if w == 0 || shown.len() != bare.len() || !shown.len().is_multiple_of(row) || mask.len() != mask_len {
        return false;
    }
    // Bands of rows on the rasteriser's worker threads: each row stands alone.
    let rows = shown.len() / row;
    let threads = usize::from(crate::default_threads()).clamp(1, 8);
    let band = rows.div_ceil(threads).max(1);
    let bands = shown.chunks_mut(band * row).zip(bare.chunks(band * row)).zip(mask.chunks(band * sub * 4));
    std::thread::scope(|scope| {
        for ((shown, bare), mask) in bands {
            scope.spawn(move || blend_rows(shown, bare, mask, w, order));
        }
    });
    true
}

/// [`blend`] over whole rows, `w` pixels wide.
fn blend_rows(px: &mut [u8], bare: &[u8], mask: &[u8], w: usize, order: SubpixelOrder) {
    let (sub, row) = (w * 3, w * 4);
    // A row's stripe coverages, with two empty stripes at each end for the filter.
    let mut cov = vec![0u32; sub + 4];
    // Each stripe's coverage after the filter, in 256ths of 255.
    let mut filtered = vec![0u32; sub];
    // Each pixel's coverage, the sum of its three stripes (0 to 765).
    let mut sums = vec![0u32; w];
    // The row as first drawn: the text colour is read from it while the row is rewritten.
    let mut shown = vec![0u8; row];
    let rows = px.chunks_exact_mut(row).zip(bare.chunks_exact(row)).zip(mask.chunks_exact(sub * 4));
    for ((out, bare), mask) in rows {
        let mut any = 0u8;
        for (c, m) in cov[2..].iter_mut().zip(mask.chunks_exact(4)) {
            any |= m[3];
            *c = u32::from(m[3]);
        }
        if any == 0 {
            continue;
        }
        shown.copy_from_slice(out);
        for (f, k) in filtered.iter_mut().zip(cov.windows(5)) {
            *f = k[0] * FILTER[0] + k[1] * FILTER[1] + k[2] * FILTER[2] + k[3] * FILTER[3] + k[4] * FILTER[4];
        }
        for (s, three) in sums.iter_mut().zip(cov[2..].chunks_exact(3)) {
            *s = three[0] + three[1] + three[2];
        }
        for (x, f) in filtered.chunks_exact(3).enumerate() {
            if f[0] | f[1] | f[2] == 0 {
                continue;
            }
            // Fully covered, with every stripe nearby: the text colour as drawn.
            if f[0] & f[1] & f[2] == FULL && sums[x] == 765 {
                continue;
            }
            // The text colour is read where the text covers most: this pixel, else the nearest
            // neighbour that covers more (the filter spreads coverage a pixel or so past the text).
            let mut src = x;
            for i in [x.wrapping_sub(1), x + 1, x.wrapping_sub(2), x + 2] {
                if i < w && sums[i] > sums[src] {
                    src = i;
                }
            }
            let coverage = sums[src];
            let (o, so) = (4 * x, 4 * src);
            if coverage < MIN_SOURCE_COVERAGE || shown[o + 3] != 255 || bare[o + 3] != 255 || shown[so + 3] != 255 || bare[so + 3] != 255 {
                continue;
            }
            let per = 765.0 / (coverage as f32 * FULL as f32);
            for ch in 0..3 {
                let stripe = match order {
                    SubpixelOrder::Rgb => f[ch],
                    SubpixelOrder::Bgr => f[2 - ch],
                };
                // What full coverage adds to this channel there, then this stripe's share of it.
                let full = f32::from(shown[so + ch]) - f32::from(bare[so + ch]);
                let v = f32::from(bare[o + ch]) + full * stripe as f32 * per;
                // Rounded by adding a half before the cast (which truncates).
                out[o + ch] = (v.clamp(0.0, 255.0) + 0.5) as u8;
            }
        }
    }
}
