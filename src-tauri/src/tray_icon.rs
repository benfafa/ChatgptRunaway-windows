//! Dynamic tray icon renderer.
//!
//! Draws a 32x32 RGBA PNG that combines:
//! - a circular ring showing the primary quota usage (0..100%)
//! - a 1–3 digit number in the center (the percentage, rounded)
//!
//! Color thresholds (matching the popover gauge):
//!   0..70  → green
//!   70..90 → amber
//!   90..   → red
//!
//! The output is suitable for Tauri 2's `TrayIcon::set_icon(Some(Image::from_bytes))`.
//! We deliberately keep the renderer dependency-free (just `image`) so it
//! works on any platform without bundling a font — the digit is rasterized as
//! a hand-drawn 3x5 bitmap font. That keeps the tray icon sharp at the
//! 32x32 sizes Windows uses.

use image::{ImageBuffer, Rgba, RgbaImage};

pub const TRAY_ICON_SIZE: u32 = 32;
const RING_INNER: i32 = 9;
const RING_OUTER: i32 = 14;
const CENTER: (i32, i32) = (16, 16);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Ok,
    Warn,
    Danger,
}

impl Severity {
    pub fn from_percent(pct: f32) -> Self {
        if pct >= 90.0 {
            Severity::Danger
        } else if pct >= 70.0 {
            Severity::Warn
        } else {
            Severity::Ok
        }
    }
}

pub fn severity_color(sev: Severity) -> [u8; 4] {
    match sev {
        // Solid, slightly translucent so it reads on the dark Windows 11
        // taskbar and on light Win10 themes.
        Severity::Ok => [52, 199, 89, 255],      // #34c759
        Severity::Warn => [255, 179, 64, 255],    // #ffb340
        Severity::Danger => [255, 59, 48, 255],   // #ff3b30
    }
}

/// Build a tray icon image for the given usage percent (0..=100). Values
/// outside the range are clamped.
pub fn render(used_percent: f32) -> RgbaImage {
    let mut img: RgbaImage = ImageBuffer::from_pixel(TRAY_ICON_SIZE, TRAY_ICON_SIZE, Rgba([0, 0, 0, 0]));
    let pct = used_percent.clamp(0.0, 100.0);
    let severity = Severity::from_percent(pct);
    let color = severity_color(severity);

    // Background disk: very light so the ring is visible on both light and
    // dark taskbars. 0 alpha outside the disk; opaque inside.
    draw_disk(&mut img, CENTER, 15, [255, 255, 255, 230]);

    // Empty ring background (light grey).
    draw_ring(&mut img, CENTER, RING_INNER, RING_OUTER, 0.0, 360.0, [200, 200, 200, 220]);

    // Active arc.
    let sweep = (pct as f64 / 100.0) * 360.0;
    if sweep > 0.0 {
        draw_ring(&mut img, CENTER, RING_INNER, RING_OUTER, 0.0, sweep, color);
    }

    // Center label — short text showing the rounded percent.
    let label = pct.round().clamp(0.0, 100.0) as u8;
    draw_label(&mut img, label, [40, 40, 40, 255]);

    img
}

/// Same as `render`, but returns the bytes. Convenient for `set_icon`.
pub fn render_png(used_percent: f32) -> Vec<u8> {
    let img = render(used_percent);
    let mut out = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut out);
        // Unwrap: writing to a Vec<u8> cannot fail.
        img.write_to(&mut cursor, image::ImageFormat::Png).expect("encode png");
    }
    out
}

// ---------------------------------------------------------------------------
// Drawing primitives
// ---------------------------------------------------------------------------

fn draw_disk(img: &mut RgbaImage, center: (i32, i32), radius: i32, color: [u8; 4]) {
    let (cx, cy) = center;
    let r2 = (radius as i32) * (radius as i32);
    for y in 0..TRAY_ICON_SIZE as i32 {
        for x in 0..TRAY_ICON_SIZE as i32 {
            let dx = x - cx;
            let dy = y - cy;
            if dx * dx + dy * dy <= r2 {
                blend_pixel(img, x as u32, y as u32, color);
            }
        }
    }
}

fn draw_ring(
    img: &mut RgbaImage,
    center: (i32, i32),
    inner: i32,
    outer: i32,
    start_deg: f64,
    end_deg: f64,
    color: [u8; 4],
) {
    let (cx, cy) = center;
    let inner2 = inner * inner;
    let outer2 = outer * outer;
    // We use a small angular step. 1° is plenty at 32px.
    let steps = ((end_deg - start_deg).max(0.0) as i32).max(1);
    for y in 0..TRAY_ICON_SIZE as i32 {
        for x in 0..TRAY_ICON_SIZE as i32 {
            let dx = x - cx;
            let dy = y - cy;
            let r2 = dx * dx + dy * dy;
            if r2 < inner2 || r2 > outer2 {
                continue;
            }
            // angle: 0 = +X (3 o'clock), counter-clockwise.
            let mut deg = (dy as f64).atan2(dx as f64).to_degrees();
            if deg < 0.0 {
                deg += 360.0;
            }
            // shift so 0° is at 12 o'clock
            deg = (deg + 90.0) % 360.0;
            let progress = deg;
            if progress >= start_deg && progress <= end_deg {
                blend_pixel(img, x as u32, y as u32, color);
            }
            let _ = steps; // kept for future per-step rendering
        }
    }
}

/// Tiny 3x5 bitmap font for digits 0..=9, plus '%'. Each glyph is 3 px wide,
/// 5 px tall. Stored as 5 rows of 3 bits per row, MSB left.
const GLYPHS: &[(u8, [u8; 5])] = &[
    (b'0', [0b111, 0b101, 0b101, 0b101, 0b111]),
    (b'1', [0b010, 0b110, 0b010, 0b010, 0b111]),
    (b'2', [0b111, 0b001, 0b111, 0b100, 0b111]),
    (b'3', [0b111, 0b001, 0b111, 0b001, 0b111]),
    (b'4', [0b101, 0b101, 0b111, 0b001, 0b001]),
    (b'5', [0b111, 0b100, 0b111, 0b001, 0b111]),
    (b'6', [0b111, 0b100, 0b111, 0b101, 0b111]),
    (b'7', [0b111, 0b001, 0b010, 0b010, 0b010]),
    (b'8', [0b111, 0b101, 0b111, 0b101, 0b111]),
    (b'9', [0b111, 0b101, 0b111, 0b001, 0b111]),
    (b'%', [0b101, 0b101, 0b010, 0b101, 0b101]),
];

fn glyph(c: u8) -> Option<[u8; 5]> {
    GLYPHS.iter().find(|(k, _)| *k == c).map(|(_, g)| *g)
}

fn draw_label(img: &mut RgbaImage, percent: u8, color: [u8; 4]) {
    // Decide the label: 1, 2, or 3 chars.
    // < 10: single digit. 10..=99: two digits. 100: "100".
    let text: [u8; 3] = if percent >= 100 {
        [b'1', b'0', b'0']
    } else if percent >= 10 {
        [b'0' + (percent / 10), b'0' + (percent % 10), 0]
    } else {
        [b'0' + percent, 0, 0]
    };
    let visible: Vec<u8> = text.iter().copied().filter(|c| *c != 0).collect();
    let char_w = 3i32;
    let char_h = 5i32;
    let spacing = 1i32;
    let total_w = visible.len() as i32 * char_w + (visible.len() as i32 - 1).max(0) * spacing;
    let start_x = CENTER.0 - total_w / 2;
    let start_y = CENTER.1 - char_h / 2;
    for (i, c) in visible.iter().enumerate() {
        let Some(glyph) = glyph(*c) else { continue };
        let x0 = start_x + i as i32 * (char_w + spacing);
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..char_w {
                let on = (bits >> (char_w - 1 - col)) & 1 == 1;
                if on {
                    let x = x0 + col;
                    let y = start_y + row as i32;
                    if x >= 0 && y >= 0 && (x as u32) < TRAY_ICON_SIZE && (y as u32) < TRAY_ICON_SIZE {
                        blend_pixel(img, x as u32, y as u32, color);
                    }
                }
            }
        }
    }
}

fn blend_pixel(img: &mut RgbaImage, x: u32, y: u32, color: [u8; 4]) {
    let dst = img.get_pixel(x, y);
    // Standard "source over" alpha compositing.
    let sa = color[3] as u32;
    let da = dst[3] as u32;
    let out_a = sa + da * (255 - sa) / 255;
    if out_a == 0 {
        return;
    }
    let mut out = [0u8; 4];
    for i in 0..3 {
        let sc = color[i] as u32;
        let dc = dst[i] as u32;
        let blended = (sc * sa + dc * da * (255 - sa) / 255) / out_a;
        out[i] = blended as u8;
    }
    out[3] = out_a as u8;
    img.put_pixel(x, y, Rgba(out));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_thresholds() {
        assert_eq!(Severity::from_percent(0.0), Severity::Ok);
        assert_eq!(Severity::from_percent(50.0), Severity::Ok);
        assert_eq!(Severity::from_percent(70.0), Severity::Warn);
        assert_eq!(Severity::from_percent(89.9), Severity::Warn);
        assert_eq!(Severity::from_percent(90.0), Severity::Danger);
        assert_eq!(Severity::from_percent(100.0), Severity::Danger);
    }

    #[test]
    fn render_produces_32x32() {
        let img = render(42.0);
        assert_eq!(img.width(), 32);
        assert_eq!(img.height(), 32);
    }

    #[test]
    fn render_is_pure_alpha_outside_disk() {
        // 0x0 corner of the 32x32 image is well outside the 15-radius disk.
        let img = render(0.0);
        let corner = img.get_pixel(0, 0);
        assert_eq!(corner[3], 0, "expected alpha 0 at corner, got {:?}", corner);
    }

    #[test]
    fn render_zero_percent_has_no_active_arc() {
        // At 0% the ring should still be drawn (background), but no green
        // pixels should appear in the upper half (12 o'clock side).
        let img = render(0.0);
        let mut found_green = false;
        for y in 0..6 {
            for x in 0..TRAY_ICON_SIZE {
                let p = img.get_pixel(x, y);
                let is_green = p[0] < 100 && p[1] > 150 && p[2] < 150 && p[3] > 0;
                if is_green {
                    found_green = true;
                }
            }
        }
        assert!(!found_green, "0% should not paint green pixels in the top band");
    }

    #[test]
    fn render_png_is_valid_png() {
        let bytes = render_png(50.0);
        // PNG signature
        assert_eq!(&bytes[0..8], &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        assert!(bytes.len() > 100, "png body suspiciously small");
    }

    #[test]
    fn render_clamps_out_of_range() {
        // Should not panic and should produce a valid 32x32.
        let _ = render(-10.0);
        let _ = render(200.0);
    }

    #[test]
    fn full_ring_uses_danger_color() {
        let img = render(100.0);
        // 100% should put red pixels in the upper region of the ring.
        let mut found_red = false;
        for y in 0..TRAY_ICON_SIZE {
            for x in 0..TRAY_ICON_SIZE {
                let p = img.get_pixel(x, y);
                if p[0] > 200 && p[1] < 100 && p[2] < 100 && p[3] > 0 {
                    found_red = true;
                }
            }
        }
        assert!(found_red, "expected danger-color pixels at 100%");
    }
}
