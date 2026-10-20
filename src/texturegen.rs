//! Procedural texture generation: pure RGBA byte buffers (no bevy, no file IO).
//! The binary uploads these as Images. Textures drawn with the game::font.

use crate::game::{band, fhash, text_to_pixels};

pub type Rgba = Vec<u8>;

fn put(buf: &mut Rgba, w: u32, x: u32, y: u32, c: [u8; 4]) {
    let i = ((y * w + x) * 4) as usize;
    buf[i] = c[0];
    buf[i + 1] = c[1];
    buf[i + 2] = c[2];
    buf[i + 3] = c[3];
}

fn blend(buf: &mut Rgba, w: u32, x: u32, y: u32, c: [u8; 4]) {
    let i = ((y * w + x) * 4) as usize;
    let a = c[3] as u32;
    buf[i] = ((c[0] as u32 * a + buf[i] as u32 * (255 - a)) / 255) as u8;
    buf[i + 1] = ((c[1] as u32 * a + buf[i + 1] as u32 * (255 - a)) / 255) as u8;
    buf[i + 2] = ((c[2] as u32 * a + buf[i + 2] as u32 * (255 - a)) / 255) as u8;
    buf[i + 3] = 255;
}

fn new_buf(w: u32, h: u32, c: [u8; 4]) -> Rgba {
    let mut b = vec![0u8; (w * h * 4) as usize];
    for px in b.chunks_exact_mut(4) {
        px.copy_from_slice(&c);
    }
    b
}

// ---- checkerboard floor: pink/cream marble ----
pub fn floor_tile(seed: u64) -> Rgba {
    let w = 256u32;
    let h = 256u32;
    let mut b = new_buf(w, h, [255, 255, 255, 255]);
    for y in 0..h {
        for x in 0..w {
            let cell = ((x / 128) + (y / 128)) % 2;
            let base: [f32; 3] = if cell == 0 {
                [235.0, 195.0, 205.0] // pink
            } else {
                [240.0, 232.0, 215.0] // cream
            };
            // marble veining
            let v = band(x as f32 * 0.05 + y as f32 * 0.02, seed + (cell as u64) * 77)
                + 0.5 * band(x as f32 * 0.11 - y as f32 * 0.07, seed + 999 + cell as u64);
            let v = (v - 0.75).max(0.0) * 60.0;
            let n = fhash(seed + (y * w + x) as u64, 3) * 14.0 - 7.0;
            let g = 215.0 + fhash(seed + (y * w + x) as u64, 5) * 18.0;
            let c = [
                (base[0] - v + n).clamp(0.0, 255.0) as u8,
                (base[1] - v + n).clamp(0.0, 255.0) as u8,
                (base[2] - v * 0.8 + n).clamp(0.0, 255.0) as u8,
                g as u8,
            ];
            put(&mut b, w, x, y, [c[0], c[1], c[2], 255]);
        }
    }
    // grout lines
    for y in 0..h {
        for x in 0..w {
            if x % 128 < 2 || y % 128 < 2 {
                let sh = 0.82 + fhash(seed + (y * w + x) as u64, 11) * 0.08;
                put(
                    &mut b,
                    w,
                    x,
                    y,
                    [(170.0 * sh) as u8, (160.0 * sh) as u8, (155.0 * sh) as u8, 255],
                );
            }
        }
    }
    b
}

// ---- wall panels: pastel tile with wainscot line ----
pub fn wall_tile(seed: u64, hue_shift: f32) -> Rgba {
    let w = 256u32;
    let h = 256u32;
    let mut b = new_buf(w, h, [255, 255, 255, 255]);
    let top = hsv(hue_shift, 0.14, 0.92);
    let bot = hsv(hue_shift + 0.03, 0.22, 0.72);
    for y in 0..h {
        for x in 0..w {
            let t = y as f32 / h as f32;
            let mut c = [
                top[0] as f32 * (1.0 - t) + bot[0] as f32 * t,
                top[1] as f32 * (1.0 - t) + bot[1] as f32 * t,
                top[2] as f32 * (1.0 - t) + bot[2] as f32 * t,
            ];
            // vertical panel seams
            if x % 64 < 2 {
                c[0] *= 0.88;
                c[1] *= 0.88;
                c[2] *= 0.88;
            }
            // wainscot rail at y=170
            let dy = y.abs_diff(170);
            if dy < 3 {
                let s = 1.06f32.min(1.0 + (3 - dy) as f32 * 0.04);
                c[0] *= s;
                c[1] *= s;
                c[2] *= s;
            }
            let n = fhash(seed + (y * w + x) as u64, 13) * 10.0 - 5.0;
            put(
                &mut b,
                w,
                x,
                y,
                [
                    (c[0] + n).clamp(0.0, 255.0) as u8,
                    (c[1] + n).clamp(0.0, 255.0) as u8,
                    (c[2] + n).clamp(0.0, 255.0) as u8,
                    255,
                ],
            );
        }
    }
    b
}

// ---- ceiling: light grid tiles ----
pub fn ceil_tile(seed: u64) -> Rgba {
    let w = 256u32;
    let h = 256u32;
    let mut b = new_buf(w, h, [246, 240, 244, 255]);
    for y in 0..h {
        for x in 0..w {
            let seam = x % 128 < 3 || y % 128 < 3;
            let n = fhash(seed + (y * w + x) as u64, 17) * 12.0 - 6.0;
            let v = if seam { 190.0 } else { 244.0 };
            let c = (v + n).clamp(0.0, 255.0) as u8;
            put(&mut b, w, x, y, [c, (c as f32 * 0.98) as u8, (c as f32 * 1.0) as u8, 255]);
        }
    }
    b
}

// ---- storefront window: vaporwave sunset (also used on mirror material) ----
pub fn sunset(seed: u64) -> Rgba {
    let w = 256u32;
    let h = 256u32;
    let mut b = new_buf(w, h, [0, 0, 0, 255]);
    let horizon = h as f32 * 0.62;
    // sun bands
    let cx = w as f32 * 0.5;
    let cy = horizon - 34.0;
    let r = 46.0f32;
    for y in 0..h {
        for x in 0..w {
            let c = if (y as f32) < horizon {
                // sky: deep purple -> magenta -> orange at horizon
                let k = (y as f32 / horizon).powi(2);
                let r0 = 30.0 + 225.0 * k;
                let g0 = 10.0 + 110.0 * k;
                let b0 = 90.0 + 70.0 * (1.0 - k);
                let flick = 1.0 + 0.03 * band(y as f32 * 0.35, seed);
                [
                    (r0 * flick).clamp(0.0, 255.0) as u8,
                    (g0 * flick).clamp(0.0, 255.0) as u8,
                    (b0 * flick).clamp(0.0, 255.0) as u8,
                    255,
                ]
            } else {
                // ground: retro grid, dark magenta base
                let k = ((y as f32 - horizon) / (h as f32 - horizon)).powi(2);
                let r0 = 60.0 + 120.0 * k;
                let g0 = 10.0 + 30.0 * k;
                let b0 = 90.0 + 40.0 * k;
                [r0 as u8, g0 as u8, b0 as u8, 255]
            };
            put(&mut b, w, x, y, c);
        }
    }
    // sun disc with horizontal slice bands
    for y in (horizon as u32).saturating_sub(r as u32 + 4)..=(horizon as u32).min(h - 1) {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            if d < r {
                let band_ok = (cy - y as f32) % 9.0 > 3.5; // slice gaps lower half
                if band_ok {
                    let k = 1.0 - (cy - y as f32) / r;
                    put(
                        &mut b,
                        w,
                        x,
                        y,
                        [255, (170.0 - 110.0 * k) as u8, (90.0 - 60.0 * k) as u8, 255],
                    );
                }
            }
        }
    }
    // grid lines on ground
    for y in (horizon as u32 + 1)..h {
        let gy = (y as f32 - horizon) / (h as f32 - horizon);
        let period = 4.0 + gy * 26.0;
        if ((y as f32 - horizon) % period) < 1.6 {
            for x in 0..w {
                blend(&mut b, w, x, y, [255, 90, 220, 190]);
            }
        }
    }
    for x in 0..w {
        let fx = x as f32 / w as f32 - 0.5;
        for y in (horizon as u32 + 1)..h {
            let gyy = (y as f32 - horizon) / (h as f32 - horizon);
            let px = fx / (0.06 + gyy * 0.94);
            if (px * 10.0).fract().abs() < 0.045 {
                blend(&mut b, w, x, y, [255, 90, 220, 150]);
            }
        }
    }
    // horizon glow line
    for x in 0..w {
        blend(&mut b, w, x, horizon as u32, [255, 230, 255, 230]);
    }
    let _ = t_unused();
    b
}

fn t_unused() -> f32 {
    0.0
}

// ---- ad poster: gradient + slogan ----
pub fn ad_poster(seed: u64, name: &str, slogan: &str) -> Rgba {
    let w = 256u32;
    let h = 256u32;
    let mut b = new_buf(w, h, [0, 0, 0, 255]);
    let h0 = fhash(seed, 31);
    for y in 0..h {
        let t = y as f32 / h as f32;
        let c = hsv(h0 + 0.12 * t, 0.55 - 0.2 * t, 0.95 - 0.25 * t);
        for x in 0..w {
            put(&mut b, w, x, y, [c[0], c[1], c[2], 255]);
        }
    }
    // sun circle deco
    let cx = w as f32 / 2.0;
    let cy = h as f32 * 0.30;
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let d = (dx * dx + dy * dy).sqrt();
            if d < 40.0 && (cy - y as f32) % 7.0 > 2.5 {
                put(&mut b, w, x, y, [255, 220, 120, 255]);
            }
        }
    }
    let mut tmp = vec![0u8; (w * h * 4) as usize];
    // name band
    for y in 96..132 {
        for x in 0..w {
            put(&mut b, w, x, y, [12, 6, 24, 255]);
        }
    }
    text_to_pixels(name, 2, [255, 120, 220], [12, 6, 24, 255], w, 36, &mut tmp[..(w * 36 * 4) as usize]);
    blit_band(&mut b, w, &tmp, w, 36, 96);
    text_to_pixels(slogan, 2, [140, 230, 255], [0, 0, 0, 0], w, 30, &mut tmp[..(w * 30 * 4) as usize]);
    blit_band_alpha(&mut b, w, &tmp, w, 30, 150);
    b
}

fn blit_band(dst: &mut Rgba, w: u32, src: &Rgba, sw: u32, sh: u32, y0: u32) {
    for y in 0..sh {
        for x in 0..sw {
            let i = ((y * sw + x) * 4) as usize;
            let j = (((y0 + y) * w + x) * 4) as usize;
            dst[j..j + 4].copy_from_slice(&src[i..i + 4]);
        }
    }
}

fn blit_band_alpha(dst: &mut Rgba, w: u32, src: &Rgba, sw: u32, sh: u32, y0: u32) {
    for y in 0..sh {
        for x in 0..sw {
            let i = ((y * sw + x) * 4) as usize;
            if src[i + 3] == 0 {
                continue;
            }
            let j = (((y0 + y) * w + x) * 4) as usize;
            dst[j..j + 4].copy_from_slice(&src[i..i + 4]);
        }
    }
}

// ---- store sign: name on dark slab ----
pub fn sign_tex(seed: u64, name: &str) -> Rgba {
    let w = 256u32;
    let h = 64u32;
    let mut b = new_buf(w, h, [10, 6, 20, 255]);
    let h0 = fhash(seed, 41);
    let c = hsv(h0, 0.5, 1.0);
    // border
    for x in 0..w {
        for y in 0..h {
            if x < 3 || y < 3 || x >= w - 3 || y >= h - 3 {
                put(&mut b, w, x, y, [c[0], c[1], c[2], 255]);
            }
        }
    }
    let scale = if name.len() > 13 { 2 } else { 3 };
    text_to_pixels(name, scale, [255, 255, 255], [10, 6, 20, 255], w, h, &mut b);
    b
}

// ---- brick (fountain / planter) ----
pub fn brick(seed: u64) -> Rgba {
    let w = 256u32;
    let h = 256u32;
    let mut b = new_buf(w, h, [210, 170, 160, 255]);
    let bh = 16u32;
    let bw = 32u32;
    for y in 0..h {
        let row = y / bh;
        let off = if row % 2 == 0 { 0 } else { bw / 2 };
        for x in 0..w {
            let mortar = (y % bh) < 2 || ((x + off) % bw) < 2;
            let n = fhash(seed + (y * w + x) as u64, 23) * 24.0 - 12.0;
            let c = if mortar {
                [
                    (228.0 + n * 0.4) as u8,
                    (222.0 + n * 0.4) as u8,
                    (214.0 + n * 0.4) as u8,
                    255,
                ]
            } else {
                let tint = fhash(seed + ((y / bh) * 31 + (x + off) / bw) as u64, 29);
                [
                    (196.0 + tint * 40.0 + n) as u8,
                    (140.0 + tint * 40.0 + n) as u8,
                    (130.0 + tint * 30.0 + n) as u8,
                    255,
                ]
            };
            put(&mut b, w, x, y, c);
        }
    }
    b
}

// ---- nyancat: 120x40 real Nyan Cat 8-bit pixel art (RGBA, transparent elsewhere) ----
// Source: official nyan.cat thumbnail frame 1 (Chris Torres / sarajay), native indexed
// sprite at 2x screen pixels; decoded to its 51x20 logical grid, palette below.
// Rendered x2 nearest-neighbor at x=0..102, y=0..40; trail runs flush to the left edge.
const NYAN_ROWS: [&str; 20] = [
    "bbbbbbbb........bbbbbbbb...ccccccccccccccccc.......",
    "bbbbbbbbbbbbbbbbbbbbbbbbbbcdddddddddddddddddc......",
    "bbbbbbbbbbbbbbbbbbbbbbbbbcdddeeeeeeeeeeeeedddc.....",
    "ffffffffbbbbbbbbffffffffbcddeeeeeegeegeeeeeddc.....",
    "fffffffffffffffffffffffffcdeegeeeeeeeeeeeeeedc.....",
    "fffffffffffffffffffffffffcdeeeeeeeeeecceegeedc.cc..",
    "hhhhhhhhffffffffhhhhhhhhfcdeeeeeeeeeciiceeeedcciic.",
    "hhhhhhhhhhhhhhhhhhhcccchhcdeeeeeegeeciiiceeedciiic.",
    "hhhhhhhhhhhhhhhhhhhciicchcdeeeeeeeeeciiiicccciiiic.",
    "jjjjjjjjhhhhhhhhjjjcciicccdeeegeeeeeciiiiiiiiiiiic.",
    "jjjjjjjjjjjjjjjjjjjjcciiccdeeeeeeegciiiiiiiiiiiiiic",
    "jjjjjjjjjjjjjjjjjjjjjcciicdegeeeeeeciiikciiiiikciic",
    "lllllllljjjjjjjjllllllccccdeeeeeeeeciiicciiicicciic",
    "llllllllllllllllllllllllccdeeeeegeecimmiiiiiiiiimmc",
    "lllllllllllllllllllllllllcddegeeeeecimmiciiciicimmc",
    "nnnnnnnnllllllllnnnnnnnnlcdddeeeeeeeciiiccccccciic.",
    "nnnnnnnnnnnnnnnnnnnnnnnncccddddddddddciiiiiiiiiic..",
    "nnnnnnnnnnnnnnnnnnnnnnnciiiccccccccccccccccccccc...",
    "........nnnnnnnn.......ciiccnciic.....ciic.ciic....",
    ".......................cccc..ccc.......ccc..cc.....",
];
// b..n = red, black outline, waffle crust, pink frosting, orange, sprinkle magenta,
// yellow, gray, green, eye-cyan, blue, cheek pink, purple
const NYAN_PAL: [[u8; 3]; 13] = [
    [255, 0, 0],
    [0, 0, 0],
    [255, 204, 153],
    [255, 153, 255],
    [255, 153, 0],
    [255, 51, 153],
    [255, 255, 0],
    [153, 153, 153],
    [51, 255, 0],
    [189, 255, 247],
    [0, 153, 255],
    [255, 153, 153],
    [102, 51, 255],
];
pub fn nyancat() -> Rgba {
    let w = 120u32;
    let h = 40u32;
    let mut b = new_buf(w, h, [0, 0, 0, 0]);
    for (gy, row) in NYAN_ROWS.iter().enumerate() {
        for (gx, ch) in row.bytes().enumerate() {
            if ch == b'.' {
                continue;
            }
            let c = NYAN_PAL[(ch - b'b') as usize];
            let x = (gx as u32) * 2;
            let y = (gy as u32) * 2;
            // x2 nearest-neighbor: replicate each logical pixel into a 2x2 block
            for dy in 0..2u32 {
                for dx in 0..2u32 {
                    put(&mut b, w, x + dx, y + dy, [c[0], c[1], c[2], 255]);
                }
            }
        }
    }
    b
}

// ---- radial vignette: transparent center -> dark edges (UI overlay) ----
pub fn vignette() -> Rgba {
    let w = 256u32;
    let h = 256u32;
    let mut b = new_buf(w, h, [0, 0, 0, 0]);
    for y in 0..h {
        for x in 0..w {
            let dx = (x as f32 - w as f32 / 2.0) / (w as f32 / 2.0);
            let dy = (y as f32 - h as f32 / 2.0) / (h as f32 / 2.0);
            let d = (dx * dx + dy * dy).sqrt();
            let a = ((d - 0.55) / 0.65).clamp(0.0, 1.0);
            put(&mut b, w, x, y, [10, 4, 18, (a * a * 200.0) as u8]);
        }
    }
    b
}

// ---- water: teal ripple ----
pub fn water(seed: u64) -> Rgba {
    let w = 128u32;
    let h = 128u32;
    let mut b = new_buf(w, h, [0, 0, 0, 255]);
    for y in 0..h {
        for x in 0..w {
            let r = (((x as f32 - 64.0).powi(2) + (y as f32 - 64.0).powi(2)).sqrt()) / 64.0;
            let rip = 0.5 + 0.5 * (r * 18.0 + band(x as f32 * 0.1, seed) * 2.0).sin();
            let n = fhash(seed + (y * w + x) as u64, 37) * 10.0;
            put(
                &mut b,
                w,
                x,
                y,
                [
                    (40.0 + 60.0 * rip + n) as u8,
                    (150.0 + 60.0 * rip + n) as u8,
                    (170.0 + 60.0 * rip + n) as u8,
                    255,
                ],
            );
        }
    }
    b
}

pub fn hsv(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = (h - h.floor()) * 6.0;
    let i = h.floor();
    let f = h - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i as u32 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

/// Void backdrop frame: equirectangular panorama for the singularity sphere.
/// Texture rows map sphere latitude top-down: row 0 = zenith (starfield),
/// row h-1 = nadir (vortex center, straight below the mall). `t` is loop
/// phase in [0,1); every temporal term completes an integer number of cycles
/// per loop so cycling frames never pops.
pub fn void_frame(seed: u64, t: f32) -> Rgba {
    let w = 512u32;
    let h = 256u32;
    let mut b = new_buf(w, h, [0, 0, 0, 255]);
    let ta = t * std::f32::consts::TAU;
    for y in 0..h {
        // v: 0 at zenith, 1 at nadir; d = angular distance from vortex center
        let v = (y as f32 + 0.5) / h as f32;
        let d = 1.0 - v;
        // vortex glow: tight around the nadir, fading out by the horizon
        let glow = (-d * 3.4).exp();
        // bright event-horizon ring, breathing twice per loop
        let ring = (-((d - 0.16) * 13.0).powi(2)).exp()
            * (0.6 + 0.4 * (ta * 2.0 + d * 9.0).sin());
        // spiral value ramp + additive weight; whole spiral rotates once per loop
        let val = (glow * 1.25 + ring * 0.55).clamp(0.0, 1.0);
        let add = (glow * 1.4 + ring * 0.5).min(1.0);
        for x in 0..w {
            let u = (x as f32 + 0.5) / w as f32;
            // 3 rainbow arms around the pole, 4 hue bands pole-to-horizon; hue is
            // frame-invariant so frame swaps never pop the bands (the swirl comes
            // from the smooth sphere rotation; only pulse/twinkle animate below)
            let hue = (3.0 * u + 4.0 * d).fract();
            let [vr, vg, vb] = hsv(hue, 0.85, val);
            // dark purple-black void, slightly brighter toward the vortex
            let mut r = 6.0 + 20.0 * glow * glow;
            let mut g = 3.0 + 4.0 * glow * glow;
            let mut bl = 14.0 + 26.0 * glow * glow;
            r += vr as f32 * add;
            g += vg as f32 * add;
            bl += vb as f32 * add;
            // twinkling starfield in the upper void, fading below the horizon
            let up = (1.0 - v * 2.2).max(0.0);
            if up > 0.05 {
                let cx = (u * 96.0) as u64;
                let cy = (v * 48.0) as u64;
                if fhash(seed + 3, cx.wrapping_mul(73856093) ^ cy.wrapping_mul(19349663)) > 0.982
                {
                    let ph = fhash(seed + 5, cx.wrapping_mul(91) ^ cy.wrapping_mul(31));
                    let tw =
                        (0.45 + 0.55 * (ta * 3.0 + ph * std::f32::consts::TAU).sin()).max(0.0);
                    let s = 170.0 * tw * up;
                    r += s * 0.85;
                    g += s * 0.8;
                    bl += s;
                }
            }
            // faint grain to break banding
            let n = (fhash(seed + 9, (y as u64) << 16 | x as u64) - 0.5) * 5.0;
            put(
                &mut b,
                w,
                x,
                y,
                [
                    (r + n).clamp(0.0, 255.0) as u8,
                    (g + n).clamp(0.0, 255.0) as u8,
                    (bl + n).clamp(0.0, 255.0) as u8,
                    255,
                ],
            );
        }
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffers_sized() {
        assert_eq!(floor_tile(1).len(), 256 * 256 * 4);
        assert_eq!(wall_tile(1, 0.9).len(), 256 * 256 * 4);
        assert_eq!(ceil_tile(1).len(), 256 * 256 * 4);
        assert_eq!(sunset(1).len(), 256 * 256 * 4);
        assert_eq!(ad_poster(1, "MOON MART", "WALK ON").len(), 256 * 256 * 4);
        assert_eq!(sign_tex(1, "MOON MART").len(), 256 * 64 * 4);
        assert_eq!(brick(1).len(), 256 * 256 * 4);
        assert_eq!(nyancat().len(), 120 * 40 * 4);
        assert_eq!(vignette().len(), 256 * 256 * 4);
        assert_eq!(water(1).len(), 128 * 128 * 4);
    }

    #[test]
    fn font_draws_on_ad() {
        let b = ad_poster(2, "STAR PIZZA", "STEP RIGHT UP");
        // some white-ish text pixels exist in the name band
        let mut hits = 0;
        for y in 96..132u32 {
            for x in 0..256u32 {
                let i = ((y * 256 + x) * 4) as usize;
                if b[i] > 200 && b[i + 1] > 80 && b[i + 1] < 180 {
                    hits += 1;
                }
            }
        }
        assert!(hits > 200, "expected magenta text pixels, got {hits}");
    }
    #[test]
    fn void_frame_layout() {
        let b = void_frame(1, 0.0);
        assert_eq!(b.len(), 512 * 256 * 4);
        let lum = |x: u32, y: u32| -> u32 {
            let i = ((y * 512 + x) * 4) as usize;
            b[i] as u32 + b[i + 1] as u32 + b[i + 2] as u32
        };
        // vortex (nadir, last row) must dominate the zenith (first row)
        let mut bot = 0u32;
        let mut top = 0u32;
        for x in 0..512u32 {
            bot += lum(x, 255);
            top += lum(x, 0);
        }
        assert!(bot > top * 4, "vortex {bot} vs zenith {top}");
    }

}
