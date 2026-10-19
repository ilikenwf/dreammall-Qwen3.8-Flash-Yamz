//! Pure constants, layout math, name pools, and the 5x7 bitmap font.
//! No bevy imports: simcheck can use this module.

// ---- ring layout ----
pub const R_IN: f32 = 40.0; // inner wall radius (void side)
pub const R_OUT: f32 = 46.0; // outer wall radius (storefront side)
pub const R_MID: f32 = 43.0; // corridor center line
pub const N_SEC: usize = 10; // corridor sections around the ring
pub const SEC_DEG: f32 = 360.0 / N_SEC as f32; // 36 deg per section
pub const CEIL_BASE: f32 = 4.6; // base ceiling height
pub const CEIL_VAR: f32 = 1.2; // ceiling height variation
pub const FLOOR_Y: f32 = 0.0;
pub const EYE_H: f32 = 1.62;
pub const WALK_HALF: f32 = 0.45; // collision margin from walls

// section role table (mutates only in appearance, structure stays "recognizable")
// 0 fountain atrium, 1 shops, 2 food court, 3 shops, 4 fountain atrium,
// 5 shops, 6 food court, 7 shops, 8 shops, 9 atrium
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SectionKind {
    Shops,
    FoodCourt,
    Fountain,
    Atrium,
}

pub const fn section_kind(i: usize) -> SectionKind {
    match i {
        0 | 4 => SectionKind::Fountain,
        2 | 6 => SectionKind::FoodCourt,
        9 => SectionKind::Atrium,
        _ => SectionKind::Shops,
    }
}

/// section centers (deg)
pub fn section_center(i: usize) -> f32 {
    i as f32 * SEC_DEG + SEC_DEG * 0.5
}

/// world position of ring angle theta (degrees), radius r. CCW increasing theta walks forward.
/// p(theta) = (r sin t, y, r cos t); tangent = (cos t, 0, -sin t).
pub fn arc_point(deg: f32, r: f32) -> (f32, f32) {
    let t = deg.to_radians();
    (r * t.sin(), r * t.cos())
}

pub fn wrap_angle(deg: f32) -> f32 {
    let mut d = deg % 360.0;
    if d < 0.0 {
        d += 360.0;
    }
    d
}

/// signed shortest delta a->b in (-180, 180]
pub fn angle_delta(a: f32, b: f32) -> f32 {
    let mut d = wrap_angle(b - a);
    if d > 180.0 {
        d -= 360.0;
    }
    d
}

// ---- hashing / deterministic noise ----
pub fn hash2(seed: u64, a: u64) -> u64 {
    let mut x = seed
        .wrapping_mul(0x9E3779B97F4A7C15)
        .wrapping_add(a.wrapping_mul(0xC2B2AE3D27D4EB4F));
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

/// deterministic smooth [0,1] noise over integer lattice + smoothstep interp, keyed by f32 band index
pub fn band(x: f32, seed: u64) -> f32 {
    let i0 = x.floor();
    let f = x - i0;
    let h0 = (hash2(seed, i0 as i64 as u64) % 10000) as f32 / 10000.0;
    let h1 = (hash2(seed, (i0 as i64 + 1) as u64) % 10000) as f32 / 10000.0;
    let s = f * f * (3.0 - 2.0 * f);
    h0 + (h1 - h0) * s
}

/// hashed [0,1) for a pair
pub fn fhash(seed: u64, a: u64) -> f32 {
    (hash2(seed, a) % 1_000_000) as f32 / 1_000_000.0
}

// ---- name pools ----
pub const SHOP_NAMES: [&str; 12] = [
    "SUNSET SOFT SERVE",
    "DREAM STOP",
    "NEON NOODLE",
    "MOON MART",
    "ECHO AUDIO",
    "LILAC OPTICAL",
    "VELVET 7",
    "HUSH SHOES",
    "STAR PIZZA",
    "MIRA BOOKS",
    "TELEPHONE",
    "SKY GARDEN",
];

pub const SLOGANS: [&str; 8] = [
    "TASTE THE LOOP",
    "NOW FOREVER",
    "OPEN ALWAYS",
    "WALK ON",
    "WE REMEMBER YOU",
    "SINCE TOMORROW",
    "STEP RIGHT UP",
    "PLEASE SMILE",
];

pub const TOASTS: [&str; 6] = [
    "THE MALL REARRANGES ITSELF.",
    "YOU HAVE BEEN HERE BEFORE.",
    "A FOUNTAIN REMEMBERED YOU.",
    "SOMEONE WAVE BACK?",
    "THE LIGHTS HUM A NAME.",
    "KEEP WALKING.",
];

// ---- 5x7 bitmap font: A-Z 0-9 space . ' ? - : ! , and degree-free glyphs ----
// Each glyph: 7 rows of 5 bits, bit4 = leftmost.
pub fn glyph(c: char) -> Option<[u8; 7]> {
    let g: [u8; 7] = match c {
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1C, 0x12, 0x11, 0x11, 0x11, 0x12, 0x1C],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        'J' => [0x07, 0x02, 0x02, 0x02, 0x02, 0x12, 0x0C],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1B, 0x11],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        ' ' => [0, 0, 0, 0, 0, 0, 0],
        '.' => [0, 0, 0, 0, 0, 0x0C, 0x0C],
        ',' => [0, 0, 0, 0, 0x0C, 0x04, 0x08],
        '\'' => [0x04, 0x04, 0, 0, 0, 0, 0],
        '-' => [0, 0, 0, 0x1F, 0, 0, 0],
        ':' => [0, 0x0C, 0x0C, 0, 0x0C, 0x0C, 0],
        '!' => [0x04, 0x04, 0x04, 0x04, 0x04, 0, 0x04],
        '?' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0, 0x04],
        '/' => [0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10],
        '(' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        ')' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        '+' => [0, 0x04, 0x04, 0x1F, 0x04, 0x04, 0],
        _ => return None,
    };
    Some(g)
}

/// Draw centered text into an RGBA buffer. Returns false if a glyph is missing.
/// scale: px per bitmap pixel. Centered horizontally, vertical center at h/2.
pub fn text_to_pixels(text: &str, scale: u32, fg: [u8; 3], bg: [u8; 4], w: u32, h: u32, buf: &mut [u8]) -> bool {
    // fill bg
    for px in buf.chunks_exact_mut(4) {
        px.copy_from_slice(&bg);
    }
    let chars: Vec<char> = text.chars().collect();
    let gw = (chars.len() as u32) * 6 * scale - scale; // 5px glyph + 1 gap
    let gh = 7 * scale;
    if gw > w || gh > h {
        return false;
    }
    let x0 = (w - gw) / 2;
    let y0 = (h - gh) / 2;
    let mut ok = true;
    for (ci, &c) in chars.iter().enumerate() {
        let up = c.to_ascii_uppercase();
        let Some(g) = glyph(up) else {
            ok = false;
            continue;
        };
        let gx = x0 + (ci as u32) * 6 * scale;
        for (r, bits) in g.iter().enumerate() {
            for b in 0..5u32 {
                if bits & (1u8 << (4 - b)) != 0 {
                    let px = gx + b * scale;
                    let py = y0 + (r as u32) * scale;
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let x = px + dx;
                            let y = py + dy;
                            if x < w && y < h {
                                let i = ((y * w + x) * 4) as usize;
                                buf[i] = fg[0];
                                buf[i + 1] = fg[1];
                                buf[i + 2] = fg[2];
                                buf[i + 3] = 255;
                            }
                        }
                    }
                }
            }
        }
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pools_cover_font() {
        let mut all = String::new();
        for s in SHOP_NAMES {
            all.push_str(s);
            all.push(' ');
        }
        for s in SLOGANS {
            all.push_str(s);
            all.push(' ');
        }
        for s in TOASTS {
            all.push_str(s);
            all.push(' ');
        }
        for c in all.chars() {
            assert!(glyph(c.to_ascii_uppercase()).is_some(), "missing glyph {c:?}");
        }
    }

    #[test]
    fn ring_math() {
        for i in 0..N_SEC {
            let c = section_center(i);
            assert!((c - 18.0 - i as f32 * 36.0).abs() < 1e-4);
        }
        let (x, z) = arc_point(0.0, 1.0);
        assert!(x.abs() < 1e-6 && (z - 1.0).abs() < 1e-6);
        assert!((angle_delta(350.0, 10.0) - 20.0).abs() < 1e-5);
        assert!((angle_delta(10.0, 350.0) + 20.0).abs() < 1e-5);
    }
}
