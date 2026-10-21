//! Headless logic/asset verification: layout math, texturegen, synth, font.
//! Runs without GPU/audio: `cargo run --bin simcheck`.

use dreammall::game::*;
use dreammall::synth::{self, MuzakParams};
use dreammall::texturegen;

fn ok(name: &str, cond: bool) -> bool {
    println!("{} {}", if cond { "PASS" } else { "FAIL" }, name);
    cond
}

fn main() {
    let mut all = true;
    all &= ok("section table: 2 fountains, 2 food courts, 1 atrium", {
        let (mut f, mut fc, mut at) = (0, 0, 0);
        for i in 0..N_SEC {
            match section_kind(i) {
                SectionKind::Fountain => f += 1,
                SectionKind::FoodCourt => fc += 1,
                SectionKind::Atrium => at += 1,
                _ => {}
            }
        }
        f == 2 && fc == 2 && at == 1
    });
    all &= ok("sections tile 360 deg exactly", {
        (N_SEC as f32 * SEC_DEG - 360.0).abs() < 1e-3
    });
    all &= ok("walk forward wraps ring to start", {
        // integrate 6 deg steps along tangent from section 0 center
        let mut th = section_center(0);
        for _ in 0..60 {
            th = wrap_angle(th + 6.0);
        }
        (angle_delta(section_center(0), th)).abs() < 1e-3
    });
    all &= ok("arc_point radius constant", {
        (0..360).step_by(17).all(|d| {
            let (x, z) = arc_point(d as f32, R_MID);
            (x * x + z * z - R_MID * R_MID).abs() < 1.0
        })
    });
    all &= ok("corridor fits R_IN<R_MID<R_OUT with margin", R_IN + WALK_HALF < R_MID && R_MID + WALK_HALF < R_OUT);

    // ---- determinism ----
    all &= ok("hash2 deterministic", hash2(42, 7) == hash2(42, 7) && hash2(42, 7) != hash2(43, 7));
    all &= ok("fhash in [0,1)", (0..1000).all(|i| {
        let v = fhash(9, i);
        v >= 0.0 && v < 1.0
    }));
    all &= ok("band smooth in [0,1]", (0..500).all(|i| {
        let v = band(i as f32 * 0.13, 3);
        v >= 0.0 && v <= 1.0
    }));

    // ---- textures ----
    let tex: [(&str, texturegen::Rgba); 7] = [
        ("floor_tile", texturegen::floor_tile(1)),
        ("wall_tile", texturegen::wall_tile(1, 0.1)),
        ("ceil_tile", texturegen::ceil_tile(1)),
        ("sunset", texturegen::sunset(1)),
        ("brick", texturegen::brick(1)),
        ("nyancat", texturegen::nyancat()),
        ("water", texturegen::water(1)),
    ];
    for (name, buf) in &tex {
        all &= ok(&format!("texture {name} rgba-sized"), !buf.is_empty() && buf.len() % 4 == 0);
        all &= ok(&format!("texture {name} has visible pixels"), buf.chunks(4).any(|p| p[3] > 100));
    }
    all &= ok("ad_poster renders", texturegen::ad_poster(2, "MOON MART", "YOU HAVE BEEN HERE").len() % 4 == 0);
    all &= ok("sign_tex renders", texturegen::sign_tex(2, "MOON MART").len() % 4 == 0);
    all &= ok("texturegen seed-stable", texturegen::floor_tile(5) == texturegen::floor_tile(5) && texturegen::floor_tile(5) != texturegen::floor_tile(6));

    // ---- font ----
    let mut fb = vec![0u8; 512 * 64 * 4];
    all &= ok("font draws full phrase", text_to_pixels("FOOD COURT 87 YOU'VE BEEN HERE? -1:2,3", 2, [255, 255, 255], [0, 0, 0, 255], 512, 64, &mut fb));
    all &= ok("font rejects unknown glyph", !text_to_pixels("bad \x07", 2, [0, 0, 0], [0, 0, 0, 255], 256, 64, &mut fb));

    // ---- audio ----
    let set = synth::muzak_set_for(0xC0FFEE);
    all &= ok("muzak set has variants", set.len() == 6);
    let set2 = synth::muzak_set_for(0xBEEF);
    all &= ok(
        "muzak differs per seed",
        set.iter().zip(&set2).all(|(a, b)| a != b),
    );
    for (i, wav) in set.iter().enumerate() {
        all &= ok(&format!("muzak {i} valid wav"), wav.len() > 1000 && &wav[0..4] == b"RIFF" && &wav[8..12] == b"WAVE");
        // scan PCM16 body for sanity: any real signal, nothing pinned at full scale
        let body = &wav[44..];
        let (mut peak, mut rms_sum, mut n) = (0i16, 0.0f64, 0usize);
        for c in body.chunks(2) {
            let s = i16::from_le_bytes([c[0], c[1]]);
            peak = peak.max(s.abs());
            rms_sum += (s as f64) * (s as f64);
            n += 1;
        }
        let rms = (rms_sum / n as f64).sqrt() / 32768.0;
        all &= ok(&format!("muzak {i} audible, not clipping"), peak < 32767 && rms > 0.02);
    }
    // event streams behind each loop variant must be non-empty and musical
    for i in 0..set.len() {
        let song = synth::song_for_variant(i);
        all &= ok(
            &format!("muzak {i} event stream non-empty"),
            !song.is_empty()
                && song.events.iter().any(|e| e.prog == synth::Prog::Rhodes)
                && song.events.iter().any(|e| e.prog == synth::Prog::Bass),
        );
    }
    let lp = synth::muzak_loop(&MuzakParams { seed: 1, key_shift: 0.0, speed: 1.0, detune: 0.0, bright: 0.5, slow: true });
    all &= ok("slow variant renders", lp.len() > 1000);
    for (name, wav) in [
        ("footstep_l", synth::sfx_footstep(true)),
        ("footstep_r", synth::sfx_footstep(false)),
        ("pickup", synth::sfx_pickup()),
        ("throw", synth::sfx_throw()),
        ("chime", synth::sfx_win_chime()),
        ("munch", synth::sfx_munch()),
        ("riser", synth::sfx_riser()),
        ("glitch", synth::sfx_glitch()),
    ] {
        all &= ok(&format!("sfx {name} valid wav"), wav.len() > 200 && &wav[0..4] == b"RIFF");
    }

    // ---- wav16 header correctness ----
    let w16 = synth::wav16(&[0.0, 0.5, -0.5], 1);
    all &= ok("wav16 header sizes consistent", {
        let dlen = u32::from_le_bytes([w16[4], w16[5], w16[6], w16[7]]) as usize;
        dlen + 8 == w16.len()
    });

    if all {
        println!("\nSIMCHECK: all {} checks passed", "✔");
        std::process::exit(0);
    } else {
        println!("\nSIMCHECK: FAILURES PRESENT");
        std::process::exit(1);
    }
}
