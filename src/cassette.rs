use std::ops::Range;

use anyhow::{bail, ensure, Result};
use bitvec::{order::Msb0, vec::BitVec, view::BitView};

/// Distance away from zero to consider a crossing.
/// This is used to reduce the impact of noise on the signal.
pub const CROSS_THRESHOLD: f32 = 0.1;

// The length of each pulse type in seconds.
pub const PULSE_ONE: Range<f32> = (15.0 / 44100.0)..(20.0 / 44100.0);
pub const PULSE_ZERO: Range<f32> = (35.0 / 44100.0)..(39.0 / 44100.0);
pub const PULSE_START: Range<f32> = (41.0 / 44100.0)..(46.0 / 44100.0);
pub const PULSE_END: f32 = 20000.0 / 44100.0;

/// The start sequence is 01111111.
const START_SEQUENCE: u8 = 0x7F;
const INT_CROSS_THRESHOLD: i32 = (CROSS_THRESHOLD * i16::MAX as f32) as i32;

#[derive(Debug)]
enum Pulse {
    Start,
    Zero,
    One,
}

pub struct Spec {
    sample_rate: u32,
    channels: u16,
}

pub fn decode(samples: &[i32], spec: Spec) -> Result<Vec<BitVec<u8, Msb0>>> {
    let mut intersections = Vec::new();
    let mut last = (0_i32, 0_usize);
    for (i, sample) in samples.into_iter().enumerate() {
        if i % spec.channels as usize != 0 {
            continue;
        }

        if sample.abs() > INT_CROSS_THRESHOLD {
            if last.0.signum() != sample.signum() && last.0.signum() == -1 {
                intersections.push(i);
            }
            last = (*sample, i);
        }
    }

    let mut sections = Vec::new();
    let mut dat = Vec::new();
    for i in 0..intersections.len() - 1 {
        let diff = (intersections[i + 1] - intersections[i]) as f32 / spec.sample_rate as f32;
        if PULSE_ONE.contains(&diff) {
            dat.push(Pulse::One);
        } else if PULSE_ZERO.contains(&diff) {
            dat.push(Pulse::Zero);
        } else if PULSE_START.contains(&diff) {
            dat.push(Pulse::Start);
        } else if diff > PULSE_END {
            sections.push(dat);
            dat = Default::default();
        } else {
            bail!("Invalid pulse length: {}", diff);
        }
    }

    if !dat.is_empty() {
        sections.push(dat);
    }

    let mut raw_sections = Vec::new();
    let mut dat = BitVec::<u8, Msb0>::new();
    for section in sections.iter_mut() {
        let mut active = false;
        for pulse in section {
            match pulse {
                Pulse::Zero => dat.push(false),
                Pulse::One => dat.push(true),
                Pulse::Start if active => ensure!(dat.len() % 8 == 0, "Invalid start pulse"),
                Pulse::Start => dat.push(false),
            }

            if !active
                && dat.len() >= 8
                && &dat[dat.len() - 8..] == START_SEQUENCE.view_bits::<Msb0>()
            {
                active = true;
                dat.clear();
            }
        }

        ensure!(active, "Didn't find start sequence");
        raw_sections.push(dat);
        dat = Default::default();
    }

    Ok(raw_sections)
}

impl From<hound::WavSpec> for Spec {
    fn from(spec: hound::WavSpec) -> Self {
        Self {
            sample_rate: spec.sample_rate,
            channels: spec.channels,
        }
    }
}

impl From<cpal::SupportedStreamConfig> for Spec {
    fn from(spec: cpal::SupportedStreamConfig) -> Self {
        Self {
            sample_rate: spec.sample_rate().0,
            channels: spec.channels(),
        }
    }
}
