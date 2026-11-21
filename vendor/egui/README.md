# egui crates kept in this repository

`epaint` and `egui-wgpu` are copies of those crates from
[`storytold/egui`](https://github.com/storytold/egui) at commit
`6d518fc799681571963842f95b55a6b68431b15a`, the commit the rest of egui is still taken from
(`[patch.crates-io]` in the workspace `Cargo.toml`). They are licensed `MIT OR Apache-2.0`, like
egui (`LICENSE-MIT`, `LICENSE-APACHE`).

They are kept here so the interface's text can be drawn the way Windows' ClearType draws text
(subpixel edges). Everything else is as it was at that commit, except:

- the manifests name their dependencies' versions instead of egui's workspace (the same
  versions), have no `[lints]` from it, and leave out `epaint`'s benchmarks;
- the changes marked `WordKu:` in the source, listed below.

## Changes

- **ClearType text** (`epaint/src/text/lcd.rs`, new): with `epaint::text::set_lcd_text` on, a
  glyph is rasterized three times as wide, smoothed with FreeType's default LCD filter, and put
  in the atlas as a texel holding a coverage per colour stripe in red, green and blue and the
  least of them in alpha. `font_face.rs` calls it from `rasterize_outlines`; `glyph_atlas.rs`
  puts such texels through the transfer function one stripe at a time.
- **Blending it** (`egui-wgpu/src/egui_lcd.wgsl`, new; `renderer.rs`): `egui_wgpu::want_lcd_text`
  asks for it. A renderer whose device has dual-source blending then turns `set_lcd_text` on
  and draws with a fragment shader whose second output is the coverage of each channel (the
  alpha for every texel but an LCD glyph's, as before), blended with `OneMinusSrc1`. Without
  dual-source blending, or not asked, nothing changes.

To take a newer egui commit, copy the two crates from it again and apply the marked changes.
