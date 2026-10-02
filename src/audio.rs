use crate::pattern::{Pattern, Sound};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use std::{
    error::Error,
    io::Cursor,
    num::{NonZeroU16, NonZeroU32},
    time::Duration,
};

pub struct Audio {
    device: MixerDeviceSink,
    player: Option<Player>,
}

impl Audio {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            device: DeviceSinkBuilder::open_default_sink()?,
            player: None,
        })
    }

    pub fn start(&mut self, pattern: &Pattern) {
        self.stop();
        let player = Player::connect_new(self.device.mixer());
        player.append(BeatSource::new(pattern));
        self.player = Some(player);
    }

    pub fn stop(&mut self) {
        self.player = None;
    }

    pub fn playing(&self) -> bool {
        self.player.is_some()
    }

    pub fn current_step(&self, pattern: &Pattern) -> Option<usize> {
        self.player.as_ref().map(|player| {
            (player.get_pos().as_secs_f64() * pattern.bpm / 20.0) as usize
                % (pattern.beats.len() * 3)
        })
    }
}

// Generate steps on the audio thread, so GUI redraws cannot disturb the rhythm.
struct BeatSource {
    steps: Vec<bool>,
    sample: Vec<f32>,
    voices: Vec<usize>,
    sample_rate: NonZeroU32,
    samples_per_step: f64,
    step: usize,
    phase: f64,
}

impl BeatSource {
    fn new(pattern: &Pattern) -> Self {
        let bytes: &[u8] = match pattern.sound {
            Sound::Clap => include_bytes!("../assets/tr808-clap.wav"),
            Sound::Snare => include_bytes!("../assets/tr707-snare.wav"),
        };
        let decoder =
            Decoder::try_from(Cursor::new(bytes)).expect("bundled drum sample must be valid");
        let sample_rate = decoder.sample_rate();
        let sample: Vec<_> = decoder.amplify(0.4).collect();
        let samples_per_step = f64::from(sample_rate.get()) * 20.0 / pattern.bpm;
        let voices = Vec::with_capacity((sample.len() as f64 / samples_per_step).ceil() as usize);
        Self {
            steps: pattern.beats.iter().flatten().copied().collect(),
            sample,
            voices,
            sample_rate,
            samples_per_step,
            step: 0,
            phase: 0.0,
        }
    }
}

impl Iterator for BeatSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.phase < 1.0 && self.steps[self.step] {
            self.voices.push(0);
        }
        // Let earlier hits finish even when the next step has started.
        let sample = self
            .voices
            .iter_mut()
            .map(|position| {
                let sample = self.sample[*position];
                *position += 1;
                sample
            })
            .sum();
        self.voices.retain(|position| *position < self.sample.len());
        self.phase += 1.0;
        if self.phase >= self.samples_per_step {
            // Keep the fractional remainder to avoid drift at non-integral tempos.
            self.phase -= self.samples_per_step;
            self.step = (self.step + 1) % self.steps.len();
        }
        Some(sample)
    }
}

impl Source for BeatSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> NonZeroU16 {
        NonZeroU16::new(1).unwrap()
    }

    fn sample_rate(&self) -> NonZeroU32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicks_follow_enabled_thirds_and_loop_over_silent_beats() {
        for sound in [Sound::Clap, Sound::Snare] {
            let pattern = Pattern {
                bpm: 120.0,
                beats: vec![[true, false, true], [false; 3]],
                sound,
            };
            let source = BeatSource::new(&pattern);
            let hit = source.sample.clone();
            let samples: Vec<_> = source.take(96_000).collect();
            let mut expected = vec![0.0; 96_000];
            for start in [0, 16_000, 48_000, 64_000] {
                for (index, sample) in hit.iter().enumerate() {
                    expected[start + index] += sample;
                }
            }
            assert!(samples.iter().any(|sample| sample.abs() > 0.01));
            assert!(
                samples
                    .iter()
                    .zip(expected)
                    .all(|(actual, expected)| (actual - expected).abs() < 1e-6)
            );
        }
    }

    #[test]
    fn fractional_tempo_does_not_accumulate_step_rounding_error() {
        let pattern = Pattern {
            bpm: 137.5,
            beats: vec![[true; 3]],
            sound: Sound::Clap,
        };
        let mut source = BeatSource::new(&pattern);
        let frames = 480_000;
        for _ in 0..frames {
            source.next();
        }
        let elapsed_steps = frames as f64 / (48_000.0 * 20.0 / pattern.bpm);
        assert_eq!(source.step, elapsed_steps.floor() as usize % 3);
        assert!((source.phase / source.samples_per_step - elapsed_steps.fract()).abs() < 1e-9);
    }

    #[test]
    fn fast_subdivisions_preserve_overlapping_sample_tails() {
        for sound in [Sound::Clap, Sound::Snare] {
            let pattern = Pattern {
                bpm: 999.0,
                beats: vec![[true, true, false]],
                sound,
            };
            let source = BeatSource::new(&pattern);
            let hit = source.sample.clone();
            let samples: Vec<_> = source.take(48_000).collect();
            for (frame, sample) in samples.iter().take(2_880).enumerate() {
                let previous = frame.checked_sub(961).map(|i| hit[i]).unwrap_or(0.0);
                assert!((sample - hit[frame] - previous).abs() < 1e-6);
            }
            assert!(samples.iter().all(|sample| sample.abs() <= 1.0));
        }
    }
}
