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
        lo + self.f() * (hi - lo).max(1e-9)
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[(self.next() % xs.len() as u64) as usize]
    }
    pub fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }
    /// inclusive integer range
    pub fn irange(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next() % ((hi - lo + 1).max(1) as u64)) as i32
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

/// Rhodes-ish electric piano: fast-decaying tine fundamental + mellow reedy
/// body partials + slow tremolo + mild detune. `vel` 0..1 drives brightness.
fn rhodes(buf: &mut [f32], start: usize, dur: f32, freq: f32, amp: f32, vel: f32, detune: f32) {
    let n = (dur * SR as f32) as usize;
    let s0 = start.min(buf.len() / 2);
    let d = 2.0f32.powf(detune / 1200.0);
    let atk = (0.004 * SR as f32) as usize;
    let tau1 = 0.55 + 90.0 / freq; // tine: fast on highs, long on lows
    let tau2 = 1.6 + 160.0 / freq; // reed body: slower decay
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let a = if i < atk { i as f32 / atk as f32 } else { 1.0 };
        let tine = (-(t - 0.004).max(0.0) / tau1).exp() * (1.0 + vel * 0.6);
        let reed = (-(t - 0.004).max(0.0) / tau2).exp();
        let trem = 1.0 + 0.08 * (std::f32::consts::TAU * 4.7 * t).sin();
        let mut v = tine * (std::f32::consts::TAU * freq * d * t).sin();
        v += 0.35 * reed * (std::f32::consts::TAU * freq * 2.0 / d * t).sin();
        v += 0.14 * reed * (std::f32::consts::TAU * freq * 3.0 * t).sin();
        v += 0.06 * reed * (std::f32::consts::TAU * freq * 4.0 * t).sin();
        v *= a * trem;
        let j = (s0 + i) * 2;
        if j + 1 < buf.len() {
            buf[j] += v * amp;
            buf[j + 1] += v * amp * 0.9;
        }
    }
}

/// warm electric bass: fundamental + octave-down body + tapered decay
fn bass_note(buf: &mut [f32], start: usize, dur: f32, freq: f32, amp: f32) {
    let n = (dur * SR as f32) as usize;
    let s0 = start.min(buf.len() / 2);
    let atk = (0.006 * SR as f32) as usize;
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let a = if i < atk { i as f32 / atk as f32 } else { 1.0 };
        let env = a * (-(t - 0.006).max(0.0) * (2.2 / dur).max(0.7)).exp();
        let mut v = (std::f32::consts::TAU * freq * t).sin();
        v += 0.45 * (std::f32::consts::TAU * freq * 2.0 * t).sin();
        v += 0.25 * (std::f32::consts::TAU * freq * 0.5 * t).sin();
        let j = (s0 + i) * 2;
        if j + 1 < buf.len() {
            buf[j] += v * env * amp;
            buf[j + 1] += v * env * amp * 0.95;
        }
    }
}

/// soft swelling pad voice: detuned dual saw-ish (2 partials) + slow attack
fn pad_voice(buf: &mut [f32], start: usize, dur: f32, freq: f32, amp: f32, detune: f32) {
    let n = (dur * SR as f32) as usize;
    let s0 = start.min(buf.len() / 2);
    let d = 2.0f32.powf(detune / 1200.0);
    let atk = (0.5 * SR as f32) as usize;
    let rel = (0.6 * SR as f32) as usize;
    for i in 0..n {
        let t = i as f32 / SR as f32;
        let a = (i as f32 / atk as f32).min(1.0) * (((n - i) as f32 / rel as f32).min(1.0));
        let a = a.max(0.0);
        let mut v = (std::f32::consts::TAU * freq * d * t).sin();
        v += 0.6 * (std::f32::consts::TAU * freq / d * t).sin();
        v += 0.3 * (std::f32::consts::TAU * freq * 2.0 * t).sin();
        v *= a;
        let j = (s0 + i) * 2;
        if j + 1 < buf.len() {
            buf[j] += v * amp;
            buf[j + 1] += v * amp * 0.9;
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

// =================================================================== muzak ==
// A tiny MIDI-like event engine: songs are written as note_on/note_off events
// on a ppq=48 tick grid with swing, then rendered voice-by-voice into f32.

/// ticks per quarter note
pub const PPQ: i32 = 48;
const BEATS_PER_BAR: i32 = 4;
const TICKS_PER_BAR: i32 = PPQ * BEATS_PER_BAR;

/// synthesizable voice programs
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Prog {
    Rhodes, // melody / chords
    Bass,
    Pad,
    Kick,
    Snare,
    Hats,
}

/// one musical event on the tick grid
#[derive(Clone, Copy, Debug)]
pub struct Ev {
    pub tick: i32,
    pub prog: Prog,
    pub note: u8, // midi note (percussion: fixed ids)
    pub vel: u8,  // 1..127
    pub dur: i32, // ticks (rendered voice length); ignored for drums
}

/// a song: tick-grid event stream + tempo
pub struct Song {
    pub events: Vec<Ev>,
    pub bars: i32,
    pub bpm: f32,
    pub swing: f32, // 0..1: fraction of an eighth-note delayed (0 = straight)
}

impl Song {
    pub fn new(bars: i32, bpm: f32, swing: f32) -> Song {
        Song { events: Vec::new(), bars, bpm, swing }
    }
    pub fn push(&mut self, tick: i32, prog: Prog, note: u8, vel: u8, dur: i32) {
        self.events.push(Ev { tick, prog, note, vel, dur });
    }
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

// chord vocabularies: degrees are semitone offsets from key root; quality
// 0=maj7 [0,4,7,11], 1=min7 [0,3,7,10], 2=dom7 [0,4,7,10].
#[derive(Clone, Copy)]
struct Chord {
    deg: i32,
    q: u8,
}

fn chord_notes(c: Chord) -> [i32; 4] {
    let third = if c.q == 1 { 3 } else { 4 };
    let sev = if c.q == 0 { 11 } else { 10 };
    [c.deg, c.deg + third, c.deg + 7, c.deg + sev]
}

/// vocabulary: name + chord degrees + diatonic scale for melody
struct Vocab {
    #[allow(dead_code)]
    name: &'static str,
    bars: usize,
    chords: &'static [Chord],
    scale: &'static [i32],
}

const fn c(deg: i32, q: u8) -> Chord {
    Chord { deg, q }
}

// ii-V-I jazz turnaround loop (C major: Dm7 G7 | Em7 Am7 | Dm7 G7 | Cmaj7 A7)
const VOCAB_JAZZ: Vocab = Vocab {
    name: "jazz-iiVI",
    bars: 8,
    chords: &[
        c(2, 1),  c(7, 2),  // Dm7 G7
        c(4, 1),  c(9, 1),  // Em7 Am7
        c(2, 1),  c(7, 2),  // Dm7 G7
        c(0, 0),  c(9, 2),  // Cmaj7 A7 turnaround
    ],
    scale: &[0, 2, 4, 5, 7, 9, 11],
};

// 50s doo-wop I-vi-IV-V
const VOCAB_DOOWOP: Vocab = Vocab {
    name: "doo-wop",
    bars: 8,
    chords: &[
        c(0, 0),  c(9, 0),  // Cmaj7 Am7(9)
        c(5, 0),  c(7, 2),  // Fmaj7 G7
        c(0, 0),  c(9, 0),
        c(5, 0),  c(7, 2),
    ],
    scale: &[0, 2, 4, 5, 7, 9, 11],
};

// minor sad loop: Am7 Fmaj7 Cmaj7 G7 (vi-IV-I-V of relative major), 4-bar loop
const VOCAB_SAD: Vocab = Vocab {
    name: "minor-sad",
    bars: 4,
    chords: &[c(9, 1), c(5, 0), c(0, 0), c(7, 2)],
    scale: &[0, 2, 3, 5, 7, 8, 10], // C natural minor
};

// 4-bar doo-wop turnaround variant (used for shorter forms)
const VOCAB_DOOWOP4: Vocab = Vocab {
    name: "doo-wop4",
    bars: 4,
    chords: &[c(0, 0), c(9, 0), c(5, 0), c(7, 2)],
    scale: &[0, 2, 4, 5, 7, 9, 11],
};

// ---- voice-leading melody generator ---------------------------------------
/// Pick a chord tone closest to `cur` on a strong beat.
fn nearest_chord_tone(cur: i32, chord: &[i32; 4], lo: i32, hi: i32) -> i32 {
    let mut best = cur;
    let mut bd = i32::MAX;
    for &t in chord {
        for oct in [-12, 0, 12] {
            let cand = t + oct;
            if cand >= lo && cand <= hi {
                let d = (cand - cur).abs();
                if d < bd {
                    bd = d;
                    best = cand;
                }
            }
        }
    }
    best
}

/// stepwise motion toward `target`, landing exactly on it.
fn step_toward(cur: i32, target: i32, scale: &[i32], rng: &mut Rng) -> i32 {
    if rng.chance(0.15) {
        return cur; // brief repeat (a small rest in motion)
    }
    let dir = if target > cur { 1 } else if target < cur { -1 } else { if rng.chance(0.5) { 1 } else { -1 } };
    // walk the scale one degree in `dir`; big jumps only rarely
    let step = if rng.chance(0.12) { 2 } else { 1 };
    let mut cur = cur;
    for _ in 0..step {
        // next scale degree above/below cur
        let mut cands: Vec<i32> = Vec::new();
        for &s in scale {
            for oct in [-12, 0, 12] {
                let v = s + oct;
                if (dir > 0 && v > cur) || (dir < 0 && v < cur) {
                    cands.push(v);
                }
            }
        }
        if let Some(&next) = cands.iter().min_by_key(|&&v| (v - cur).abs()) {
            cur = next;
        }
    }
    if (cur - target).abs() <= 2 && rng.chance(0.5) {
        cur = target;
    }
    cur
}

/// Compose a call-and-response melody over the whole song. Strong beats get
/// chord tones; weak beats step through the scale toward the next chord tone;
/// phrase ends rest so the next phrase can answer.
fn compose_melody(song: &mut Song, vocab: &Vocab, key: i32, rot: i32, rng: &mut Rng) {
    let lo = key + 16;
    let hi = key + 33; // E4..A5 above the key root
    let mut cur = key + 24; // C5-ish for key C3
    let scale: Vec<i32> = vocab.scale.iter().map(|&s| s + key).collect();
    let mut phrase = 0;
    for bar in 0..vocab.bars as i32 {
        let chord: [i32; 4] = {
            let mut ch = chord_notes(vocab.chords[(bar + rot) as usize % vocab.chords.len()]);
            for x in ch.iter_mut() { *x += key; }
            ch
        };
        let next: [i32; 4] = {
            let mut ch = chord_notes(vocab.chords[((bar + rot) as usize + 1) % vocab.chords.len()]);
            for x in ch.iter_mut() { *x += key; }
            ch
        };
        let t0 = bar * TICKS_PER_BAR;
        // 8 eighth-note slots per bar
        for s in 0..8 {
            let tick = t0 + s * (PPQ / 2);
            let strong = s % 4 == 0;
            let mid = s % 2 == 0;
            // rest density: denser mid-phrase, rests at phrase ends (bars 3,7..)
            let rest_p = if strong { 0.15 } else if s >= 6 { 0.75 } else { 0.45 };
            if rng.chance(rest_p) {
                continue;
            }
            let target = if strong {
                nearest_chord_tone(cur, &chord, lo, hi)
            } else if s == 7 {
                nearest_chord_tone(cur, &next, lo, hi) // approach next bar's chord
            } else {
                cur
            };
            let note = if mid {
                nearest_chord_tone(cur, &chord, lo, hi)
            } else {
                step_toward(cur, target, &scale, rng)
            };
            let note = note.clamp(lo, hi);
            // dynamics: downbeats louder, phrase 2 (call->response) softer echo
            let mut vel = 78 + rng.irange(0, 22);
            if strong {
                vel += 16;
            }
            if phrase % 2 == 1 {
                vel -= 14; // response phrase echoes back
            }
            let dur = if strong { PPQ } else { PPQ / 2 + PPQ / 4 };
            song.push(tick, Prog::Rhodes, note as u8, vel.clamp(40, 127) as u8, dur);
            cur = note;
        }
        phrase += 1;
    }
}

/// Drums: soft jazzy kit. shuffle => swung hats/kick pattern.
fn compose_drums(song: &mut Song, bars: i32, shuffle: bool, rng: &mut Rng) {
    for bar in 0..bars {
        let t0 = bar * TICKS_PER_BAR;
        // kick: 1 and (3, or offbeat pickup when shuffling)
        song.push(t0, Prog::Kick, 36, 96, 0);
        if shuffle {
            song.push(t0 + 3 * PPQ + PPQ / 3, Prog::Kick, 36, 72, 0);
        } else {
            song.push(t0 + 2 * PPQ, Prog::Kick, 36, 78, 0);
        }
        // snare: backbeat 2 & 4 (soft brush feel)
        song.push(t0 + PPQ, Prog::Snare, 38, 64, 0);
        song.push(t0 + 3 * PPQ, Prog::Snare, 38, 70, 0);
        if rng.chance(0.25) {
            song.push(t0 + 4 * PPQ - PPQ / 2, Prog::Snare, 38, 48, 0); // pickup
        }
        // hats: 8ths with swing on offbeats, light velocity variation
        for h in 0..8 {
            let mut tick = t0 + h * (PPQ / 2);
            if h % 2 == 1 {
                tick += (song.swing * (PPQ / 2) as f32) as i32;
            }
            let vel = if h % 2 == 1 { 44 + rng.irange(0, 12) } else { 30 + rng.irange(0, 8) };
            song.push(tick, Prog::Hats, 42, vel as u8, 0);
        }
    }
}

/// Pads: hold each bar's 7th chord at low level.
fn compose_pads(song: &mut Song, vocab: &Vocab, key: i32, rot: i32, amp_vel: u8) {
    for bar in 0..vocab.bars as i32 {
        let chord = chord_notes(vocab.chords[(bar + rot) as usize % vocab.chords.len()]);
        let t0 = bar * TICKS_PER_BAR;
        for (ci, &n) in chord.iter().enumerate() {
            let note = key + 12 + n + if ci == 0 { 12 } else { 0 }; // voicing: root doubled up an octave
            song.push(t0, Prog::Pad, note as u8, amp_vel.saturating_sub(ci as u8 * 6), TICKS_PER_BAR);
        }
    }
}

/// Bass: root on 1, fifth on 3 (or walking shuffle feel).
fn compose_bass(song: &mut Song, vocab: &Vocab, key: i32, rot: i32, shuffle: bool, rng: &mut Rng) {
    for bar in 0..vocab.bars as i32 {
        let chord = chord_notes(vocab.chords[(bar + rot) as usize % vocab.chords.len()]);
        let next = chord_notes(vocab.chords[((bar + rot) as usize + 1) % vocab.chords.len()]);
        let t0 = bar * TICKS_PER_BAR;
        let root = key + chord[0];
        let fifth = root + 7;
        song.push(t0, Prog::Bass, root as u8, 100, PPQ);
        if shuffle {
            song.push(t0 + PPQ + PPQ / 3, Prog::Bass, fifth as u8, 82, PPQ / 2);
            song.push(t0 + 2 * PPQ + PPQ / 3, Prog::Bass, root as u8, 88, PPQ / 2);
            song.push(t0 + 3 * PPQ + PPQ / 3, Prog::Bass, (root + 12) as u8, 70, PPQ / 2);
        } else {
            song.push(t0 + 2 * PPQ, Prog::Bass, fifth as u8, 84, PPQ);
            if rng.chance(0.4) {
                // chromatic approach into next bar's root
                let app = key + next[0] - 1;
                song.push(t0 + 4 * PPQ - PPQ / 2, Prog::Bass, app as u8, 74, PPQ / 2);
            }
        }
    }
}

/// Chords: Rhodes comps the 7th chord on beats 1 and the "and" of 2.
fn compose_comp(song: &mut Song, vocab: &Vocab, key: i32, rot: i32, rng: &mut Rng) {
    for bar in 0..vocab.bars as i32 {
        let chord = chord_notes(vocab.chords[(bar + rot) as usize % vocab.chords.len()]);
        let t0 = bar * TICKS_PER_BAR;
        for (off, vel) in [(0, 62), (PPQ + PPQ / 2, 52)] {
            for (ci, &n) in chord.iter().enumerate() {
                let note = key + 8 + n;
                let v = (vel - ci as u8 * 4 + rng.irange(0, 6) as u8).clamp(30, 90);
                song.push(t0 + off, Prog::Rhodes, note as u8, v, PPQ / 2);
            }
        }
    }
}

// ---- song forms -------------------------------------------------------------
/// Build a song from a vocabulary + feel. `bars_total` may repeat the vocab
/// (16-bar forms = vocab twice with an AABA-ish melodic repeat handled by the
/// melody generator's phrase dynamics).
fn build_song(vocab: &Vocab, key: i32, bpm: f32, swing: f32, drums: bool, comp: bool, bars_total: i32, rot: i32, seed: u64) -> Song {
    let mut rng = Rng(seed);
    let mut song = Song::new(bars_total, bpm, swing);
    // pad + bass + comp follow the vocab, tiled across bars_total
    let reps = (bars_total / vocab.bars as i32).max(1);
    for r in 0..reps {
        let off = r * vocab.bars as i32;
        // tile by temporarily offsetting: easiest is to build a sub-song then shift
        let mut sub = Song::new(vocab.bars as i32, bpm, swing);
        compose_pads(&mut sub, vocab, key, rot, 46);
        compose_bass(&mut sub, vocab, key, rot, swing > 0.1, &mut rng);
        if comp {
            compose_comp(&mut sub, vocab, key, rot, &mut rng);
        }
        for mut e in sub.events.drain(..) {
            e.tick += off * TICKS_PER_BAR;
            song.events.push(e);
        }
    }
    // melody: write per-vocab-rep, AABA: rep0=A, rep1=B(higher register), rep2=A, rep3=A/B
    let mut mel_rng = Rng(seed ^ 0xBEEF);
    for r in 0..reps {
        let off = r * vocab.bars as i32;
        let mut sub = Song::new(vocab.bars as i32, bpm, swing);
        let k = key + if r % 4 == 1 { 12 } else { 0 }; // B section lifts an octave
        compose_melody(&mut sub, vocab, k, rot, &mut mel_rng);
        for mut e in sub.events.drain(..) {
            e.tick += off * TICKS_PER_BAR;
            song.events.push(e);
        }
    }
    if drums {
        compose_drums(&mut song, bars_total, swing > 0.1, &mut rng);
    }
    song.events.sort_by_key(|e| (e.tick, e.prog as u8 as i32));
    song
}

// ---- render pass ------------------------------------------------------------
/// Walk the event stream and render every voice into the stereo f32 buffer.
fn render_song(song: &Song, detune: f32, bright: f32) -> Vec<f32> {
    let beat = 60.0 / song.bpm;
    let tick = beat / PPQ as f32;
    let total = song.bars as f32 * BEATS_PER_BAR as f32 * beat;
    let n = (total * SR as f32) as usize;
    let mut buf = vec![0.0f32; n * 2];
    let mut rng = Rng(0x9E3779 ^ song.events.len() as u64);
    for e in &song.events {
        let start = (e.tick as f32 * tick * SR as f32) as usize;
        let vel = e.vel as f32 / 127.0;
        match e.prog {
            Prog::Rhodes => {
                rhodes(&mut buf, start, e.dur as f32 * tick * 1.6, midi_f(e.note as f32 + detune / 1200.0), 0.10 + 0.16 * vel, vel, detune);
            }
            Prog::Bass => {
                bass_note(&mut buf, start, e.dur as f32 * tick * 1.4, midi_f(e.note as f32 + detune / 1200.0), 0.10 + 0.12 * vel);
            }
            Prog::Pad => {
                pad_voice(&mut buf, start, e.dur as f32 * tick * 1.05, midi_f(e.note as f32 + detune / 1200.0), 0.028 + 0.03 * vel, detune * 1.6);
            }
            Prog::Kick => {
                sine_sweep(&mut buf, start, 0.18, 62.0, 38.0, 0.10 + 0.08 * vel);
            }
            Prog::Snare => {
                noise_burst(&mut buf, start, 0.12, 0.05 + 0.07 * vel, &mut rng, 0.35, 0.5);
            }
            Prog::Hats => {
                noise_burst(&mut buf, start, 0.045, 0.03 + 0.05 * vel * bright, &mut rng, 0.22, 0.55);
            }
        }
    }
    buf
}

/// params for one loop variant
#[derive(Clone)]
pub struct MuzakParams {
    pub seed: u64,
    pub key_shift: f32,
    pub speed: f32,   // tempo scale (higher = faster; legacy name kept)
    pub detune: f32,  // cents
    pub bright: f32,
    pub slow: bool,   // slow/dreamy variant
}

/// Internal recipe selecting vocabulary + form + feel.
#[derive(Clone, Copy)]
struct Recipe {
    vocab: &'static Vocab,
    key: i32,
    bpm: f32,
    swing: f32,
    drums: bool,
    comp: bool,
    bars: i32,
    rot: i32,   // progression rotation (starting degree offset)
}

fn song_from(p: &MuzakParams, r: &Recipe) -> Song {
    // legacy semantics: speed was a bar-duration scale (1.35 = slower)
    let bpm = r.bpm / p.speed.max(0.25);
    let key = r.key + p.key_shift.round() as i32;
    build_song(r.vocab, key, bpm, r.swing, r.drums, r.comp, r.bars, r.rot, p.seed)
}

/// Render one seamless muzak loop (8-16 bars, ~10-20s). Returns stereo WAV bytes.
pub fn muzak_loop(p: &MuzakParams) -> Vec<u8> {
    let r = recipe_for(p);
    let song = song_from(p, &r);
    render_loop_wav(&song, p.detune, p.bright, p.slow)
}

/// Shared render tail: reverb + seamless tail-fold + soft clip + WAV encode.
fn render_loop_wav(song: &Song, detune: f32, bright: f32, slow: bool) -> Vec<u8> {
    let mut buf = render_song(song, detune, bright);
    let n = buf.len() / 2;
    reverb(&mut buf, if slow { 0.45 } else { 0.35 });
    // tail fold: reverb ringing out past the loop point is added back into the
    // head so the seam is continuous (the loop repeats seamlessly).
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

/// The recipe for a given params object (deterministic).
fn recipe_for(p: &MuzakParams) -> Recipe {
    if p.slow {
        Recipe { vocab: &VOCAB_SAD, key: 48, bpm: 60.0, swing: 0.0, drums: false, comp: true, bars: 4, rot: 0 }
    } else {
        // pick among vocabularies by seed so muzak_loop alone still varies
        let opts = [&VOCAB_JAZZ, &VOCAB_DOOWOP, &VOCAB_DOOWOP4];
        let v = opts[(p.seed % 3) as usize];
        // seed-stable tempo so standalone muzak_loop calls vary a little
        let bpm = 97.0 + (p.seed % 5) as f32 * 4.0;
        Recipe { vocab: v, key: 48, bpm, swing: 0.22, drums: true, comp: true, bars: 8, rot: 0 }
    }
}

/// The classic set: fixed recipes, deterministic. Kept for tests/tools.
pub fn muzak_set() -> Vec<Vec<u8>> {
    muzak_set_for(0xDEA17)
}

/// Chord rotation for a vocab: start the progression on a random diatonic
/// degree. Still a valid loop, just re-anchored (ii-V-I -> V-I-ii, etc.).
fn rot_of(v: &Vocab, rng: &mut Rng) -> i32 {
    rng.irange(0, v.chords.len() as i32 - 1)
}

/// Derive one loop recipe from rng state within the style rules:
/// jazz/doo-wop/sad vocabularies, diatonic keys in a comfortable register,
/// tempo + swing bands per family, sad loops never get drums.
fn recipe_from_seed(rng: &mut Rng) -> Recipe {
    match rng.next() % 4 {
        0 => {
            let v = &VOCAB_JAZZ;
            Recipe { vocab: v, key: rng.irange(45, 57), bpm: rng.range(96.0, 122.0), swing: rng.range(0.12, 0.30), drums: true, comp: rng.chance(0.8), bars: 8, rot: rot_of(v, rng) }
        }
        1 => {
            let v = &VOCAB_DOOWOP;
            Recipe { vocab: v, key: rng.irange(45, 57), bpm: rng.range(100.0, 126.0), swing: rng.range(0.0, 0.12), drums: true, comp: rng.chance(0.7), bars: 8, rot: rot_of(v, rng) }
        }
        2 => {
            let v = &VOCAB_DOOWOP4;
            Recipe { vocab: v, key: rng.irange(45, 57), bpm: rng.range(104.0, 130.0), swing: rng.range(0.0, 0.10), drums: true, comp: rng.chance(0.6), bars: 8, rot: rot_of(v, rng) }
        }
        _ => {
            let v = &VOCAB_SAD;
            Recipe { vocab: v, key: rng.irange(43, 55), bpm: rng.range(62.0, 76.0), swing: rng.range(0.0, 0.15), drums: false, comp: true, bars: 4, rot: rot_of(v, rng) }
        }
    }
}

/// Build a per-dream set of 6 distinct loops derived from `seed`: the same
/// style rules every time (jazz/doo-wop/sad vocabularies, tasteful keys and
/// tempi), but a new progression rotation, key, tempo, and feel per listen.
pub fn muzak_set_for(seed: u64) -> Vec<Vec<u8>> {
    let mut out = Vec::with_capacity(6);
    for i in 0..6u64 {
        let mut rng = Rng(seed ^ (i + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let r = recipe_from_seed(&mut rng);
        let slow = r.vocab.name == "minor-sad";
        let p = MuzakParams {
            seed: seed ^ (i + 1).wrapping_mul(7919),
            key_shift: 0.0,
            speed: if slow { rng.range(1.28, 1.45) } else { rng.range(0.94, 1.08) },
            detune: rng.range(-8.0, 8.0),
            bright: rng.range(0.25, 0.7),
            slow,
        };
        let song = song_from(&p, &r);
        out.push(render_loop_wav(&song, p.detune, p.bright, p.slow));
    }
    out
}

/// Per-variant recipes for the in-game set: different key / vocab / form / feel.
const SET_RECIPES: [Recipe; 6] = [
    Recipe { vocab: &VOCAB_JAZZ,    key: 48, bpm: 104.0, swing: 0.28, drums: true,  comp: true,  bars: 8, rot: 0 },  // C swing jazz
    Recipe { vocab: &VOCAB_DOOWOP,  key: 53, bpm: 116.0, swing: 0.0,  drums: true,  comp: true,  bars: 8, rot: 0 },  // F doo-wop
    Recipe { vocab: &VOCAB_DOOWOP4, key: 50, bpm: 126.0, swing: 0.0,  drums: true,  comp: false, bars: 8, rot: 0 },  // Bb up-tempo turnaround
    Recipe { vocab: &VOCAB_JAZZ,    key: 55, bpm: 102.0, swing: 0.18, drums: true,  comp: true,  bars: 8, rot: 0 },  // G laid-back jazz
    Recipe { vocab: &VOCAB_SAD,     key: 48, bpm: 72.0,  swing: 0.0,  drums: false, comp: true,  bars: 4, rot: 0 },  // Am sad, no drums
    Recipe { vocab: &VOCAB_SAD,     key: 43, bpm: 72.0,  swing: 0.12, drums: false, comp: true,  bars: 4, rot: 0 },  // Fm slow shuffle
];

/// Build the event stream for variant `i` of the in-game set (for tests).
pub fn song_for_variant(i: usize) -> Song {
    let specs: [(f32, f32, f32, f32, bool); 6] = [
        (0.0, 1.0, 0.0, 0.6, false),
        (2.0, 1.08, 4.0, 0.5, false),
        (-3.0, 0.94, -5.0, 0.7, false),
        (5.0, 1.02, 2.0, 0.45, false),
        (0.0, 1.35, -8.0, 0.3, true),
        (-5.0, 1.42, 6.0, 0.25, true),
    ];
    let (k, sp, dt, br, slow) = specs[i % specs.len()];
    let p = MuzakParams { seed: 0xDEA17 ^ ((i as u64).wrapping_mul(7919)), key_shift: k, speed: sp, detune: dt, bright: br, slow };
    song_from(&p, &SET_RECIPES[i % SET_RECIPES.len()])
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
            for chunk in w[44..].chunks(2) {
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

    #[test]
    fn event_streams_nonempty() {
        for i in 0..6 {
            let s = song_for_variant(i);
            assert!(!s.is_empty(), "variant {i} has no events");
            assert!(s.events.iter().any(|e| e.prog == Prog::Rhodes));
            assert!(s.events.iter().any(|e| e.prog == Prog::Bass));
        }
    }

    #[test]
    fn muzak_seed_varies() {
        let a = muzak_set_for(0xC0FFEE);
        let b = muzak_set_for(0xBADF00D);
        let a2 = muzak_set_for(0xC0FFEE);
        assert_eq!(a.len(), 6);
        assert_eq!(a, a2, "same seed must render identical bytes");
        assert!(
            a.iter().zip(&b).all(|(x, y)| x != y),
            "different seeds must render different music"
        );
    }
}
