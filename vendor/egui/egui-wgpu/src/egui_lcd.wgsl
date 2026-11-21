// WordKu: ClearType text. Appended to `egui.wgsl` (with `enable dual_source_blending;` before
// it) when the device has dual-source blending and `epaint` rasterizes glyphs with subpixel
// edges (`epaint::text::set_lcd_text`).
//
// An LCD glyph's texel holds a coverage per colour stripe in red, green and blue, and the least
// of them in alpha, so one of its colours is above its alpha, which no premultiplied texel has.
// It is blended per channel: the second output is how much of each channel of what is behind is
// covered. Every other texel is blended as `fs_main_*` blends it: each channel covered by alpha.

struct LcdOutput {
    @location(0) @blend_src(0) color: vec4<f32>,
    @location(0) @blend_src(1) coverage: vec4<f32>,
};

fn lcd_output(in: VertexOutput, linear_framebuffer: bool) -> LcdOutput {
    let tex = sample_texture(in);
    var color = in.color * tex;
    var coverage = vec3<f32>(color.a);
    if max(tex.r, max(tex.g, tex.b)) > tex.a + 0.5 / 255.0 {
        // `in.color` is premultiplied: each channel covered by its own stripe.
        color = vec4<f32>(in.color.rgb * tex.rgb, in.color.a * tex.a);
        coverage = in.color.a * tex.rgb;
    }
    if r_locals.dithering == 1 {
        color = vec4<f32>(dither_interleaved(color.rgb, 256.0, in.position), color.a);
    }
    if linear_framebuffer {
        color = vec4<f32>(linear_from_gamma_rgb(color.rgb), color.a);
    }
    var out: LcdOutput;
    out.color = color;
    out.coverage = vec4<f32>(coverage, color.a);
    return out;
}

@fragment
fn fs_main_lcd_linear_framebuffer(in: VertexOutput) -> LcdOutput {
    return lcd_output(in, true);
}

@fragment
fn fs_main_lcd_gamma_framebuffer(in: VertexOutput) -> LcdOutput {
    return lcd_output(in, false);
}
