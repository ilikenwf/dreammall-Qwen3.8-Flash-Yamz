//! Procedural audio: vaporwave muzak loops + SFX, rendered to WAV bytes.
//! Pure std; the binary feeds these to bevy AudioSource.

const SR: usize = 44_100;

// ---- tiny deterministic rng ----
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 11
    }
    pub fn f(&mut self) -> f32 {
        (self.next() % 100_000) as f32 / 100_000.0
    }
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.f() * (hi - lo)
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next() % xs.len() as u64) as usize]
    }
}

fn midi_f(n: f32) -> f32 {
    440.0 * (2.0f32).powf((n - 69.0) / 12.0)
}

// ---- wav encode ----
pub fn wav16(samples: &[f32], channels: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(44 + samples.len() * 2);
    let data_len = (samples.len() * 2) as u32;
    let byte_rate = SR as u32 * channels as u32 * 2;
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&(SR as u32).to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32000.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

// ---- synth voices (add into stereo f32 buffer) ----
/// one plucky sine-ish tone with harmonics + exponential decay
fn tone(buf: &mut [f32], start: usize, dur: f32, freq: f32, amp: f32, bright: f32) {
    let n = (dur * SR as f32) as usize;
    let s0 = start.min(buf.len() / 2);
    let atk = (0.008 * SR as f32) as usize;
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let env = if i < atk {
            i as f32 / atk as f32
        } else {
            (-(t - 0.008) * (2.5 / dur).max(0.6)).exp()
        };
        let mut v = (std::f32::consts::TAU * freq * t).sin();
        v += bright * 0.5 * (std::f32::consts::TAU * freq * 2.0 * t).sin();
        v += bright * bright * 0.25 * (std::f32::consts::TAU * freq * 3.0 * t).sin();
        // gentle vibrato for the dream
        v *= 1.0 + 0.004 * (std::f32::consts::TAU * 5.3 * t).sin();
        let j = (s0 + i) * 2;
        if j + 1 < buf.len() {
            buf[j] += v * env * amp;
            buf[j + 1] += v * env * amp * 0.92; // slight stereo imbalance
        }
    }
}

fn noise_burst(buf: &mut [f32], start: usize, dur: f32, amp: f32, rng: &mut Rng, lp: f32, pan: f32) {
    let n = (dur * SR as f32) as usize;
    let s0 = start.min(buf.len() / 2);
    let mut z = 0.0f32;
    for i in 0..n {
        let t = i as f32 / (dur * SR as f32);
        let env = (1.0 - t).powi(2);
        z += lp * (rng.range(-1.0, 1.0) - z);
        let j = (s0 + i) * 2;
        if j + 1 < buf.len() {
            buf[j] += z * env * amp * (1.0 - pan);
            buf[j + 1] += z * env * amp * pan;
        }
    }
}

fn sine_sweep(buf: &mut [f32], start: usize, dur: f32, f0: f32, f1: f32, amp: f32) {
    let n = (dur * SR as f32) as usize;
    let s0 = start.min(buf.len() / 2);
    let mut phase = 0.0f32;
    for i in 0..n {
        let t = i as f32 / n as f32;
        let f = f0 + (f1 - f0) * t;
        phase += std::f32::consts::TAU * f / SR as f32;
        let env = (t * 6.0).min(1.0) * (1.0 - t).min(1.0);
        let v = phase.sin() * env * amp;
        let j = (s0 + i) * 2;
        if j + 1 < buf.len() {
            buf[j] += v;
            buf[j + 1] += v;
        }
    }
}

/// Schroeder-ish reverb: two combs + one allpass, wet mixed back in.
fn reverb(buf: &mut [f32], wet: f32) {
    const C1: usize = 1557;
    const C2: usize = 1617;
    const A1: usize = 225;
    let mut b1 = vec![0.0f32; C1];
    let mut b2 = vec![0.0f32; C2];
    let mut a1 = vec![0.0f32; A1];
    let fb = 0.78f32;
    let afb = 0.5f32;
    for ch in 0..2 {
        for i in 0..buf.len() / 2 {
            let j = i * 2 + ch;
            let x = buf[j];
            let y = b1[i % C1] * fb + b2[i % C2] * fb;
            b1[i % C1] = x + b1[i % C1] * fb;
            b2[i % C2] = x + b2[i % C2] * fb * 0.97;
            let out_pre = y;
            let aout = a1[i % A1] * afb + out_pre;
            a1[i % A1] = out_pre - a1[i % A1] * afb;
            buf[j] += aout * wet;
        }
    }
}

fn soft_clip(buf: &mut [f32]) {
    for s in buf.iter_mut() {
        *s = s.tanh();
    }
}

// ---- muzak ----
/// one chord = midi notes (root + triad/7th)
const PROG_BASE: [[f32; 4]; 4] = [
    [53.0, 57.0, 60.0, 64.0], // Fmaj7
    [50.0, 53.0, 57.0, 60.0], // Dm7
    [55.0, 58.0, 62.0, 65.0], // Gm7
    [48.0, 55.0, 60.0, 64.0], // C9-ish
];

pub struct MuzakParams {
    pub seed: u64,
    pub key_shift: f32,
    pub speed: f32,   // bar duration scale
    pub detune: f32,  // cents
    pub bright: f32,
    pub slow: bool, // slow/dreamy variant
}

/// Render one seamless muzak loop (4 bars, ~16-18s). Returns stereo WAV bytes.
pub fn muzak_loop(p: &MuzakParams) -> Vec<u8> {
    let bar = 4.5 * p.speed;
    let total = bar * 4.0;
    let n = (total * SR as f32) as usize;
    let mut buf = vec![0.0f32; n * 2];
    let mut rng = Rng(p.seed);
    let dt = p.detune / 1200.0;
    // chords: pad each bar (soft, slow-ish swell approximated by long tone)
    for b in 0..4 {
        let chord = PROG_BASE[b];
        let t0 = (b as f32 * bar * SR as f32) as usize;
        for (ci, &note) in chord.iter().enumerate() {
            let f = midi_f(note + p.key_shift + dt);
            let amp = if p.slow { 0.06 } else { 0.05 } * (1.0 - ci as f32 * 0.12);
            tone(&mut buf, t0, bar * 0.98, f, amp, p.bright * 0.4);
        }
        // bass on beats 1 and 3 (2 hits per bar)
        let root = midi_f(chord[0] - 12.0 + p.key_shift + dt);
        tone(&mut buf, t0, bar * 0.4, root, 0.16, 0.15);
        tone(&mut buf, t0 + (bar * 0.5 * SR as f32) as usize, bar * 0.4, root, 0.11, 0.15);
        // kick soft on 1
        sine_sweep(&mut buf, t0, 0.18, 62.0, 38.0, 0.14);
        // hats on off-eighths
        for h in 0..8 {
            if h % 2 == 1 {
                let t = t0 + (h as f32 * bar / 8.0 * SR as f32) as usize;
                noise_burst(&mut buf, t, 0.05, 0.05, &mut rng, 0.25, 0.5);
            }
        }
        // melody: 8th notes from chord tones + neighbor, sparse
        let steps = 8;
        for s in 0..steps {
            if rng.f() < (if p.slow { 0.45 } else { 0.6 }) {
                let base = chord[(s % 4) as usize] + if rng.f() < 0.3 { 12.0 } else { 0.0 };
                let nte = base + p.key_shift + dt + if rng.f() < 0.25 { 2.0 } else { 0.0 };
                let t = t0 + (s as f32 * bar / steps as f32 * SR as f32) as usize;
                let amp = 0.09 + rng.f() * 0.05;
                tone(&mut buf, t, bar / steps as f32 * 1.8, midi_f(nte + 12.0), amp, p.bright);
            }
        }
    }
    reverb(&mut buf, 0.35);
    // crossfade tail into head for seamlessness
    let fade = (1.2 * SR as f32) as usize;
    for i in 0..fade {
        let t = i as f32 / fade as f32;
        let head = (i * 2, i * 2 + 1);
        let tail = ((n - fade + i) * 2, (n - fade + i) * 2 + 1);
        buf[tail.0] = buf[tail.0] * (1.0 - t) + buf[head.0] * t * 0.5;
        buf[tail.1] = buf[tail.1] * (1.0 - t) + buf[head.1] * t * 0.5;
    }
    soft_clip(&mut buf);
    wav16(&buf, 2)
}

/// The set: 4 bright variants + 2 slow variants. Returns Vec<WAV bytes>.
pub fn muzak_set() -> Vec<Vec<u8>> {
    let specs: [(f32, f32, f32, f32, bool); 6] = [
        (0.0, 1.0, 0.0, 0.6, false),
        (2.0, 1.08, 4.0, 0.5, false),
        (-3.0, 0.94, -5.0, 0.7, false),
        (5.0, 1.02, 2.0, 0.45, false),
        (0.0, 1.35, -8.0, 0.3, true),
        (-5.0, 1.42, 6.0, 0.25, true),
    ];
    specs
        .iter()
        .enumerate()
        .map(|(i, &(k, sp, dt, br, slow))| {
            muzak_loop(&MuzakParams {
                seed: 0xDEA17 ^ (i as u64).wrapping_mul(7919),
                key_shift: k,
                speed: sp,
                detune: dt,
                bright: br,
                slow,
            })
        })
        .collect()
}

// ---- SFX ----
pub fn sfx_footstep(left: bool) -> Vec<u8> {
    let mut rng = Rng(0xFEED + left as u64);
    let n = (0.13 * SR as f32) as usize;
    let mut buf = vec![0.0f32; n * 2];
    noise_burst(&mut buf, 0, 0.11, 0.30, &mut rng, 0.18, if left { 0.42 } else { 0.58 });
    tone(&mut buf, 0, 0.05, 90.0, 0.10, 0.0);
    soft_clip(&mut buf);
    wav16(&buf, 2)
}

pub fn sfx_pickup() -> Vec<u8> {
    let n = (0.5 * SR as f32) as usize;
    let mut buf = vec![0.0f32; n * 2];
    tone(&mut buf, 0, 0.18, midi_f(84.0), 0.25, 0.6);
    tone(&mut buf, (0.09 * SR as f32) as usize, 0.2, midi_f(91.0), 0.25, 0.6);
    tone(&mut buf, (0.18 * SR as f32) as usize, 0.3, midi_f(96.0), 0.22, 0.6);
    reverb(&mut buf, 0.3);
    soft_clip(&mut buf);
    wav16(&buf, 2)
}

pub fn sfx_throw() -> Vec<u8> {
    let n = (0.6 * SR as f32) as usize;
    let mut buf = vec![0.0f32; n * 2];
    let mut rng = Rng(0xBADC0DE);
    noise_burst(&mut buf, 0, 0.55, 0.25, &mut rng, 0.5, 0.5);
    sine_sweep(&mut buf, 0, 0.5, 200.0, 700.0, 0.10);
    soft_clip(&mut buf);
    wav16(&buf, 2)
}

pub fn sfx_win_chime() -> Vec<u8> {
    let n = (2.6 * SR as f32) as usize;
    let mut buf = vec![0.0f32; n * 2];
    for (i, &nte) in [72.0f32, 76.0, 79.0, 84.0, 88.0].iter().enumerate() {
        tone(&mut buf, (i as f32 * 0.16 * SR as f32) as usize, 1.8, midi_f(nte), 0.22, 0.5);
    }
    reverb(&mut buf, 0.5);
    soft_clip(&mut buf);
    wav16(&buf, 2)
}

pub fn sfx_munch() -> Vec<u8> {
    let mut rng = Rng(0xC0FFEE);
    let n = (0.45 * SR as f32) as usize;
    let mut buf = vec![0.0f32; n * 2];
    for i in 0..4 {
        let t = (i as f32 * 0.1 * SR as f32) as usize;
        noise_burst(&mut buf, t, 0.09, 0.35, &mut rng, 0.6, 0.5);
    }
    soft_clip(&mut buf);
    wav16(&buf, 2)
}

pub fn sfx_riser() -> Vec<u8> {
    let n = (1.6 * SR as f32) as usize;
    let mut buf = vec![0.0f32; n * 2];
    sine_sweep(&mut buf, 0, 1.5, 110.0, 880.0, 0.18);
    reverb(&mut buf, 0.4);
    soft_clip(&mut buf);
    wav16(&buf, 2)
}

pub fn sfx_glitch() -> Vec<u8> {
    let mut rng = Rng(0x61746368);
    let n = (0.35 * SR as f32) as usize;
    let mut buf = vec![0.0f32; n * 2];
    for i in 0..6 {
        let t = (i as f32 * 0.05 * SR as f32) as usize;
        let f = rng.range(200.0, 1800.0);
        tone(&mut buf, t, 0.04, f, 0.12, 0.9);
    }
    noise_burst(&mut buf, 0, 0.3, 0.12, &mut rng, 0.8, 0.5);
    soft_clip(&mut buf);
    wav16(&buf, 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header() {
        let w = sfx_pickup();
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[8..12], b"WAVE");
        let dlen = u32::from_le_bytes(w[40..44].try_into().unwrap()) as usize;
        assert_eq!(dlen, w.len() - 44);
    }

    #[test]
    fn muzak_not_silent() {
        let set = muzak_set();
        assert_eq!(set.len(), 6);
        for w in &set {
            assert_eq!(&w[0..4], b"RIFF");
            // parse samples roughly: rms above threshold
            let mut acc = 0.0f64;
            let mut cnt = 0;
            for chunk in w[44..].chunks(4) {
                let v = i16::from_le_bytes(chunk.try_into().unwrap()) as f64 / 32000.0;
                acc += v * v;
                cnt += 1;
            }
            let rms = (acc / cnt as f64).sqrt();
            assert!(rms > 0.01, "rms {rms}");
        }
    }

    #[test]
    fn loop_is_about_18s() {
        let w = muzak_loop(&MuzakParams {
            seed: 1,
            key_shift: 0.0,
            speed: 1.0,
            detune: 0.0,
            bright: 0.5,
            slow: false,
        });
        let secs = (w.len() - 44) as f32 / (SR as f32 * 2.0 * 2.0);
        assert!(secs > 15.0 && secs < 20.0, "{secs}");
    }
}
