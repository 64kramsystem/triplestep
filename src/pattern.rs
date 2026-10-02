use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error::Error, fs, path::Path};

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
            beats: vec![[true, false, false]; 4],
            sound: Sound::Clap,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub selected: Option<String>,
    pub presets: BTreeMap<String, Pattern>,
}

impl Settings {
    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let settings: Self = serde_json::from_slice(&fs::read(path)?)?;
        if settings
            .selected
            .as_ref()
            .is_some_and(|name| !settings.presets.contains_key(name))
        {
            return Err("The selected beat is missing.".into());
        }
        for (name, pattern) in &settings.presets {
            if name.trim().is_empty() {
                return Err("Saved beats need a name.".into());
            }
            if !(1.0..=999.0).contains(&pattern.bpm) {
                return Err("BPM must be between 1 and 999.".into());
            }
            if pattern.beats.is_empty() {
                return Err("A pattern needs at least one beat.".into());
            }
        }
        Ok(settings)
    }

    pub fn save(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_presets_restore_names_tempo_steps_and_sound() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("triplestep/config.json");
        let settings = Settings {
            selected: Some("Snare groove".into()),
            presets: BTreeMap::from([
                ("Clap".into(), Pattern::default()),
                (
                    "Snare groove".into(),
                    Pattern {
                        bpm: 137.5,
                        beats: vec![[true, false, true], [false; 3], [false, true, false]],
                        sound: Sound::Snare,
                    },
                ),
            ]),
        };
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path).unwrap(), settings);
    }

    #[test]
    fn malformed_files_and_invalid_patterns_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pattern.json");
        for data in [
            r#"{"bpm":0,"beats":[[true,false,false]],"sound":"clap"}"#,
            r#"{"bpm":1000,"beats":[[true,false,false]],"sound":"clap"}"#,
            r#"{"bpm":120,"beats":[],"sound":"clap"}"#,
            r#"{"bpm":120,"beats":[[true,false]],"sound":"clap"}"#,
            r#"{"bpm":120,"beats":[[true,1,false]],"sound":"clap"}"#,
            r#"{"bpm":120,"beats":[[true,false,false]],"sound":"unknown"}"#,
        ] {
            let data = format!(r#"{{"selected":"Beat","presets":{{"Beat":{data}}}}}"#);
            fs::write(&path, &data).unwrap();
            assert!(Settings::load(&path).is_err(), "accepted {data}");
        }
        for data in [
            "not json",
            r#"{"selected":"Missing","presets":{}}"#,
            r#"{"selected":" ","presets":{" ":{"bpm":120,"beats":[[true,false,false]],"sound":"clap"}}}"#,
        ] {
            fs::write(&path, data).unwrap();
            assert!(Settings::load(&path).is_err(), "accepted {data}");
        }
    }
}
