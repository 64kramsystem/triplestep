use serde::{Deserialize, Serialize};
use std::{error::Error, fs, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sound {
    Clap,
    Snare,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pattern {
    pub bpm: f64,
    pub beats: Vec<[bool; 3]>,
    pub sound: Sound,
}

impl Default for Pattern {
    fn default() -> Self {
        Self {
            bpm: 120.0,
            beats: vec![[true, false, false]],
            sound: Sound::Clap,
        }
    }
}

impl Pattern {
    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let pattern: Self = serde_json::from_slice(&fs::read(path)?)?;
        if !(1.0..=999.0).contains(&pattern.bpm) {
            return Err("BPM must be between 1 and 999.".into());
        }
        if pattern.beats.is_empty() {
            return Err("A pattern needs at least one beat.".into());
        }
        Ok(pattern)
    }

    pub fn save(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_patterns_restore_tempo_and_every_toggle() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pattern.json");
        let pattern = Pattern {
            bpm: 137.5,
            beats: vec![[true, false, true], [false; 3], [false, true, false]],
            sound: Sound::Snare,
        };
        pattern.save(&path).unwrap();
        assert_eq!(Pattern::load(&path).unwrap(), pattern);
    }

    #[test]
    fn malformed_files_and_invalid_patterns_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pattern.json");
        for data in [
            "not json",
            r#"{"bpm":0,"beats":[[true,false,false]],"sound":"clap"}"#,
            r#"{"bpm":1000,"beats":[[true,false,false]],"sound":"clap"}"#,
            r#"{"bpm":120,"beats":[],"sound":"clap"}"#,
            r#"{"bpm":120,"beats":[[true,false]],"sound":"clap"}"#,
            r#"{"bpm":120,"beats":[[true,1,false]],"sound":"clap"}"#,
            r#"{"bpm":120,"beats":[[true,false,false]],"sound":"unknown"}"#,
        ] {
            fs::write(&path, data).unwrap();
            assert!(Pattern::load(&path).is_err(), "accepted {data}");
        }
    }
}
