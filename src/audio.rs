use crate::pattern::Pattern;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, Source, source::SineWave};
use std::{
    error::Error,
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
    click: Vec<f32>,
    sample_rate: NonZeroU32,
    samples_per_step: f64,
    step: usize,
    phase: f64,
}

impl BeatSource {
    fn new(pattern: &Pattern) -> Self {
        let tone = SineWave::new(1_000.0);
        let sample_rate = tone.sample_rate();
        let click = tone
            .take_duration(Duration::from_millis(15))
            .fade_in(Duration::from_millis(1))
            .fade_out(Duration::from_millis(14))
            .amplify(0.25)
            .collect();
        Self {
            steps: pattern.beats.iter().flatten().copied().collect(),
            click,
            sample_rate,
            samples_per_step: f64::from(sample_rate.get()) * 20.0 / pattern.bpm,
            step: 0,
            phase: 0.0,
        }
    }
}

impl Iterator for BeatSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = if self.steps[self.step] {
            self.click.get(self.phase as usize).copied().unwrap_or(0.0)
        } else {
            0.0
        };
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
        let pattern = Pattern {
            bpm: 120.0,
            beats: vec![[true, false, true], [false; 3]],
        };
        let samples: Vec<_> = BeatSource::new(&pattern).take(96_000).collect();
        for (step, chunk) in samples.chunks_exact(8_000).enumerate() {
            let audible = chunk.iter().any(|sample| sample.abs() > 0.01);
            assert_eq!(audible, matches!(step % 6, 0 | 2), "step {step}");
            assert!(chunk[720..].iter().all(|sample| *sample == 0.0));
        }
    }

    #[test]
    fn fractional_tempo_does_not_accumulate_step_rounding_error() {
        let pattern = Pattern {
            bpm: 137.5,
            beats: vec![[true; 3]],
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
}
