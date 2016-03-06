extern crate byteorder;

use std::collections::HashMap;
use std::io::Write;
use byteorder::{LittleEndian, WriteBytesExt};

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct Frac(u64, u64);

type Memory = HashMap<Frac, f64>;

fn simplify(Frac(a, b): Frac) -> Frac {
    fn gcd(x: u64, y: u64) -> u64 {
        if y == 0 {
            x
        } else {
            gcd(y, x % y)
        }
    }

    let d = gcd(a, b);
    Frac(a/d, b/d)
}

/// judge a set of notes based on harmony.
/// range: floats in [0, 1] and lower is better.
fn judge_harmony(noteset: &[Frac], memory: &Memory) -> f64 {
    let mut harmony_sum = 0_f64;

    for &Frac(a1, b1) in noteset {
        for (&Frac(a2, b2), &familiarity) in memory.iter() {
            let Frac(a3, b3) = simplify(Frac(a1*b2, a2*b1));
            harmony_sum += familiarity * (a3 as f64) * (b3 as f64);
        }
    }
    let iterations = noteset.len()*memory.len();
    let avg_harmony = (harmony_sum as f64)/(iterations as f64);

    (1_f64 - 1_f64/(avg_harmony/5_f64).exp()).max(0_f64).min(1_f64)
}

/// judge a set of notes based on familiarity & novelty balance.
/// range: floats in [0, 1] and lower is better.
fn judge_novelty(noteset: &[Frac], memory: &Memory) -> f64 {
    if noteset.len() < 1 {
        panic!("judge_novelty: need at least 1 note");
    }

    let mut familiarity_sum = 0_f64;
    for note in noteset {
        let &familiarity = memory.get(note).unwrap_or(&0_f64);
        familiarity_sum += familiarity;
    }

    let avg_familiarity = familiarity_sum / (noteset.len() as f64);
    let target_familiarity = 0.1_f64;
    let disparity = (target_familiarity - avg_familiarity).abs();

    (1_f64 - 1_f64/disparity.exp()).max(0_f64).min(1_f64)
}

/// judge a set of notes.
/// range: floats in [0, 1] and lower is better.
fn judge(noteset: &[Frac], memory: &Memory) -> f64 {
    (judge_harmony(noteset, memory) + judge_novelty(noteset, memory))/2_f64
}

fn forget(memory: &mut Memory) {
    for (n, val) in memory.iter_mut() {
        *val *= 0.75;
    }
}

fn remember(note_set: &[Frac], memory: &mut Memory) {
    let increase = 0.1_f64;
    for note in note_set {
        let val = match memory.get(note) {
            Some(v) => v + increase,
            None => increase,
        };
        memory.insert(note.clone(), val);
    }
}

/// step to a set of notes that minimizes the judge function.
fn step_notes(note_set: &[Frac], memory: &Memory) -> Vec<Frac> {
    let mut best: Vec<Frac> = note_set.to_owned();
    let mut best_score = 1_f64;
    for i in 0..note_set.len() {
        for a in 1..12 {
            for b in 1..12 {
                let possibility = simplify(Frac(a, b));
                if (note_set.contains(&possibility)) {
                    continue;
                }
                let note_set2: Vec<Frac> = note_set[0..i].iter()
                                                         .chain(note_set[i+1..note_set.len()].iter())
                                                         .chain([possibility].iter())
                                                         .map(|n| n.clone())
                                                         .collect();
                let score = judge(&note_set2, memory);
                if score < best_score {
                    best = note_set2;
                    best_score = score;
                }
            }
        }
    }

    best
}

type PCM_Sample = i16;
static PCM_HZ: u64 = 44100_u64;
static STEPS_PER_SEC: u64 = 4;
static BASE_NOTE: f64 = 250_f64;
type Endianness = LittleEndian;

fn sine_wave(freq: f64, step: u64) -> f64 {
    (2.0*std::f64::consts::PI*(step as f64)*freq/(PCM_HZ as f64)).sin()
}

fn sine_waves(base_note: f64, fractions: &[Frac], step: u64) -> f64 {
    let mut sum = 0_f64;
    for &Frac(a, b) in fractions {
        let freq = (base_note / (b as f64)) * (a as f64);
        sum += sine_wave(freq, step);
    }

    sum / (fractions.len() as f64)
}

fn linear_envelope(sample: f64, duration: u64, progress: u64) -> f64 {
    sample * (progress as f64) / (duration as f64)
}

fn output_pcm() {
    let mut notes = vec![Frac(1, 2), Frac(1, 1), Frac(1, 3), Frac(1, 5), Frac(1, 7)];
    let mut memory = Memory::new();

    let mut j=0;
    for i in (0_u64..u64::max_value()).cycle() {
        let sample = sine_waves(BASE_NOTE, &notes, i) *
                     (PCM_Sample::max_value() as f64);

        let enveloped = linear_envelope(sample, j, PCM_HZ/STEPS_PER_SEC);

        let bounded = enveloped.min(PCM_Sample::max_value() as f64 - 1_f64)
                               .max(PCM_Sample::min_value() as f64 + 1_f64);

        let as_sample: PCM_Sample = bounded as PCM_Sample;
        std::io::stdout().write_i16::<Endianness>(as_sample).unwrap();

        j += 1;
        if j == PCM_HZ/STEPS_PER_SEC {
            j = 0;
            forget(&mut memory);
            notes = step_notes(&notes, &memory);
            remember(&notes, &mut memory);
        }
    }
}

fn main() {
    output_pcm();
}
