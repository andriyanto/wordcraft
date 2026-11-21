//! WordKu: text with subpixel (LCD) edges, as Windows' ClearType draws it.
//!
//! A glyph is rasterized three times as wide, one coverage per colour stripe of the screen's
//! pixels, smoothed with FreeType's default LCD filter, and stored in the glyph atlas as a texel
//! whose red, green and blue are those coverages and whose alpha is the least of them. Ordinary
//! atlas texels (white coverage, colour emoji, the white pixel) are premultiplied, so no colour
//! channel exceeds alpha: a renderer tells the two apart by that, and blends an LCD texel per
//! channel (`egui-wgpu` with dual-source blending). Without such a renderer, leave this off.

use core::sync::atomic::{AtomicU8, Ordering};

use ecolor::Color32;
use emath::vec2;
use vello_cpu::{color, kurbo};

use crate::{ColorImage, FontColorTransferFunction, text::GlyphBitmap};

/// The order of a screen's colour stripes, left to right.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LcdOrder {
    Rgb,
    Bgr,
}

/// 0: grey edges; 1: RGB; 2: BGR.
static LCD_TEXT: AtomicU8 = AtomicU8::new(0);

/// Rasterize text glyphs with subpixel edges in `order` (`None`: grey edges, the default).
///
/// Set it before any text is drawn: glyphs already in the atlas keep their edges. Only for a
/// renderer that blends LCD texels per channel (see the module documentation).
pub fn set_lcd_text(order: Option<LcdOrder>) {
    let v = match order {
        None => 0,
        Some(LcdOrder::Rgb) => 1,
        Some(LcdOrder::Bgr) => 2,
    };
    LCD_TEXT.store(v, Ordering::Relaxed);
}

/// The subpixel order glyphs are rasterized in ([`set_lcd_text`]).
pub fn lcd_text() -> Option<LcdOrder> {
    match LCD_TEXT.load(Ordering::Relaxed) {
        1 => Some(LcdOrder::Rgb),
        2 => Some(LcdOrder::Bgr),
        _ => None,
    }
}

/// FreeType's default LCD filter (`FT_LCD_FILTER_DEFAULT`), in 256ths: each stripe's coverage
/// spread over its neighbours, keeping every colour's ink and leaving edges neutral.
const FILTER: [u32; 5] = [8, 77, 86, 77, 8];

/// Rasterize `path` (physical pixels, from the glyph origin) with subpixel edges in `order`.
///
/// `None` if there is nothing to draw, or it is too wide to rasterize three times as wide.
pub(crate) fn rasterize(path: &kurbo::BezPath, order: LcdOrder) -> Option<GlyphBitmap> {
    let bounds = path.control_box();
    if !(bounds.x0.is_finite() && bounds.x1.is_finite() && bounds.y0.is_finite() && bounds.y1.is_finite()) {
        return None;
    }
    // A pixel more on each side: the filter spreads coverage into it.
    let (x0, x1) = (bounds.x0.floor() - 1.0, bounds.x1.ceil() + 1.0);
    let (y0, y1) = (bounds.y0.floor(), bounds.y1.ceil());
    let (width, height) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let sub = width.checked_mul(3)?;
    if bounds.width() <= 0.0 || height == 0 || u16::try_from(sub).is_err() || u16::try_from(height).is_err() {
        return None;
    }
    let (sub16, height16) = (sub as u16, height as u16);

    let mut ctx = super::font_face::glyph_render_context(sub16, height16);
    ctx.set_transform(kurbo::Affine::scale_non_uniform(3.0, 1.0) * kurbo::Affine::translate((-x0, -y0)));
    ctx.set_paint(color::OpaqueColor::<color::Srgb>::WHITE);
    ctx.fill_path(path);
    let mut dest = vello_cpu::Pixmap::new(sub16, height16);
    let mut resources = vello_cpu::Resources::new();
    ctx.render(&mut dest, &mut resources);

    let coverage: alloc::vec::Vec<u32> = dest.data_as_u8_slice().iter().skip(3).step_by(4).map(|&a| u32::from(a)).collect();
    let pixels = lcd_pixels(&coverage, width, order);
    let image = ColorImage::new([width, height], pixels);
    Some(GlyphBitmap {
        image,
        offset_px: vec2(x0 as f32, y0 as f32),
        is_color: false,
    })
}

/// The texels of a glyph whose stripe coverages (0 to 255, three per pixel, rows `width` pixels
/// wide) are `coverage`: each colour the filtered coverage of its stripe, alpha the least.
pub(crate) fn lcd_pixels(coverage: &[u32], width: usize, order: LcdOrder) -> alloc::vec::Vec<Color32> {
    let sub = width * 3;
    let mut pixels = alloc::vec::Vec::with_capacity(coverage.len() / 3);
    if sub == 0 {
        return pixels;
    }
    for row in coverage.chunks(sub) {
        let filtered = |s: usize| -> u32 {
            let sum: u32 = FILTER
                .iter()
                .enumerate()
                .map(|(k, w)| (s + k).checked_sub(2).and_then(|i| row.get(i)).map_or(0, |c| c * w))
                .sum();
            (sum + 128) >> 8
        };
        for x in 0..width {
            let stripes = [filtered(3 * x), filtered(3 * x + 1), filtered(3 * x + 2)].map(|v| v.min(255) as u8);
            let [r, g, b] = match order {
                LcdOrder::Rgb => stripes,
                LcdOrder::Bgr => [stripes[2], stripes[1], stripes[0]],
            };
            pixels.push(Color32::from_rgba_premultiplied(r, g, b, r.min(g).min(b)));
        }
    }
    pixels
}

/// Is this a texel of an LCD glyph: a colour above its alpha, which no premultiplied colour has?
#[inline]
pub(crate) fn is_lcd_texel(c: Color32) -> bool {
    c.r() > c.a() || c.g() > c.a() || c.b() > c.a()
}

/// An LCD glyph's texel through the atlas' transfer function, which is made for coverage: each
/// stripe's coverage goes through it on its own, and alpha stays the least of them.
pub(crate) fn transfer(transfer: FontColorTransferFunction, c: Color32) -> Color32 {
    let t = |v: u8| transfer.to_atlas_color(Color32::from_white_alpha(v)).a();
    let (r, g, b) = (t(c.r()), t(c.g()), t(c.b()));
    Color32::from_rgba_premultiplied(r, g, b, r.min(g).min(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lcd_texels_colour_the_edges_and_keep_the_ink() {
        // One row, 4 pixels: stripes covered from the middle of pixel 1 to the middle of pixel 2.
        let coverage = [0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0];
        let rgb = lcd_pixels(&coverage, 4, LcdOrder::Rgb);
        assert_eq!(rgb.len(), 4);
        // The ink is the stripes' (the filter only spreads it): four covered stripes.
        let ink: u32 = rgb.iter().map(|p| u32::from(p.r()) + u32::from(p.g()) + u32::from(p.b())).sum();
        assert!(ink.abs_diff(4 * 255) <= 6, "{ink}");
        // The left edge is covered more in blue (its right stripe), the right edge in red.
        assert!(rgb[1].b() > rgb[1].r(), "{:?}", rgb[1]);
        assert!(rgb[2].r() > rgb[2].b(), "{:?}", rgb[2]);
        assert!(rgb.iter().all(|p| p.a() == p.r().min(p.g()).min(p.b())));
        assert!(is_lcd_texel(rgb[1]) && is_lcd_texel(rgb[2]));
        // A BGR screen swaps red and blue.
        let bgr = lcd_pixels(&coverage, 4, LcdOrder::Bgr);
        for (a, b) in rgb.iter().zip(&bgr) {
            assert_eq!((a.r(), a.g(), a.b()), (b.b(), b.g(), b.r()));
        }
        // Ordinary atlas texels are never taken for LCD ones.
        for c in [Color32::WHITE, Color32::TRANSPARENT, Color32::from_white_alpha(128), Color32::from_rgba_premultiplied(200, 10, 30, 200)] {
            assert!(!is_lcd_texel(c), "{c:?}");
        }
    }

    #[test]
    fn lcd_rasterizing_refuses_what_it_cannot_draw() {
        assert!(rasterize(&kurbo::BezPath::new(), LcdOrder::Rgb).is_none());
        let mut huge = kurbo::BezPath::new();
        huge.move_to((0.0, 0.0));
        huge.line_to((40_000.0, 0.0));
        huge.line_to((40_000.0, 10.0));
        huge.close_path();
        assert!(rasterize(&huge, LcdOrder::Rgb).is_none(), "too wide three times over");
        let mut square = kurbo::BezPath::new();
        square.move_to((0.5, 0.0));
        square.line_to((4.5, 0.0));
        square.line_to((4.5, 4.0));
        square.line_to((0.5, 4.0));
        square.close_path();
        let glyph = rasterize(&square, LcdOrder::Rgb).expect("a square is drawn");
        assert_eq!(glyph.offset_px, vec2(-1.0, 0.0));
        assert!(glyph.image.pixels.iter().any(|&p| is_lcd_texel(p)));
    }

    #[test]
    fn the_transfer_function_goes_through_each_stripe() {
        let c = Color32::from_rgba_premultiplied(200, 100, 20, 20);
        let t = transfer(FontColorTransferFunction::TwoCoverageMinusCoverageSq, c);
        assert!(t.r() >= 200 && t.g() >= 100 && t.b() >= 20, "{t:?}");
        assert_eq!(t.a(), t.r().min(t.g()).min(t.b()));
        assert_eq!(transfer(FontColorTransferFunction::Off, c), c);
    }
}
