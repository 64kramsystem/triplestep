mod audio;
mod pattern;

use audio::Audio;
use eframe::egui::{self, Color32, RichText, Stroke};
use pattern::Pattern;
use std::time::Duration;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TripleStep")
            .with_inner_size([460.0, 420.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };
    eframe::run_native(
        "TripleStep",
        options,
        Box::new(|_| Ok(Box::new(TripleStep::default()))),
    )
}

struct TripleStep {
    pattern: Pattern,
    bpm_text: String,
    audio: Option<Audio>,
    message: String,
}

impl Default for TripleStep {
    fn default() -> Self {
        Self {
            pattern: Pattern::default(),
            bpm_text: "120".into(),
            audio: None,
            message: String::new(),
        }
    }
}

impl TripleStep {
    fn playing(&self) -> bool {
        self.audio.as_ref().is_some_and(Audio::playing)
    }

    fn commit_bpm(&mut self) -> bool {
        match self.bpm_text.trim().parse::<f64>() {
            Ok(bpm) if (1.0..=999.0).contains(&bpm) => {
                let changed = self.pattern.bpm != bpm;
                self.pattern.bpm = bpm;
                self.message.clear();
                if changed {
                    self.restart();
                }
                true
            }
            _ => {
                self.message = "BPM must be between 1 and 999.".into();
                false
            }
        }
    }

    fn restart(&mut self) {
        if let Some(audio) = &mut self.audio {
            if audio.playing() {
                audio.start(&self.pattern);
            }
        }
    }

    fn stop(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.stop();
        }
    }

    fn start(&mut self) {
        if !self.commit_bpm() {
            return;
        }
        if self.audio.is_none() {
            match Audio::new() {
                Ok(audio) => self.audio = Some(audio),
                Err(error) => {
                    self.message = format!("Cannot open audio output: {error}");
                    return;
                }
            }
        }
        self.audio.as_mut().unwrap().start(&self.pattern);
    }

    fn save(&mut self) {
        if !self.commit_bpm() {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Save TripleStep pattern")
            .add_filter("TripleStep pattern", &["json"])
            .set_file_name("triplestep.json")
            .save_file()
        {
            self.message = match self.pattern.save(&path) {
                Ok(()) => "Pattern saved.".into(),
                Err(error) => format!("Cannot save pattern: {error}"),
            };
        }
    }

    fn load(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Load TripleStep pattern")
            .add_filter("TripleStep pattern", &["json"])
            .pick_file()
        {
            match Pattern::load(&path) {
                Ok(pattern) => {
                    self.stop();
                    self.bpm_text = pattern.bpm.to_string();
                    self.pattern = pattern;
                    self.message = "Pattern loaded.".into();
                }
                Err(error) => self.message = format!("Cannot load pattern: {error}"),
            }
        }
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("BPM");
        ui.horizontal(|ui| {
            let decrease = ui.add_sized([44.0, 38.0], egui::Button::new("<")).clicked();
            let response = ui.add_sized(
                [100.0, 38.0],
                egui::TextEdit::singleline(&mut self.bpm_text)
                    .font(egui::TextStyle::Heading)
                    .horizontal_align(egui::Align::Center),
            );
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "BPM"));
            if response.lost_focus()
                || (response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                self.commit_bpm();
            }
            let increase = ui.add_sized([44.0, 38.0], egui::Button::new(">")).clicked();
            if decrease || increase {
                if self.commit_bpm() {
                    self.pattern.bpm =
                        (self.pattern.bpm + if increase { 5.0 } else { -5.0 }).clamp(1.0, 999.0);
                    self.bpm_text = self.pattern.bpm.to_string();
                    self.restart();
                }
            }
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui
                .add_sized(
                    [100.0, 32.0],
                    egui::Button::new(if self.playing() { "Stop" } else { "Start" }),
                )
                .clicked()
            {
                if self.playing() {
                    self.stop();
                } else {
                    self.start();
                }
            }
            ui.weak("Three equal steps per beat.");
        });
        ui.add_space(10.0);
        ui.separator();

        let current_step = self
            .audio
            .as_ref()
            .and_then(|audio| audio.current_step(&self.pattern));
        let mut changed = false;
        let mut insert = None;
        let mut remove = None;
        let beat_count = self.pattern.beats.len();
        egui::ScrollArea::vertical()
            .max_height((ui.available_height() - 85.0).max(50.0))
            .show(ui, |ui| {
                for (row, beat) in self.pattern.beats.iter_mut().enumerate() {
                    ui.push_id(row, |ui| {
                        ui.horizontal(|ui| {
                            ui.add_sized([46.0, 40.0], egui::Label::new(format!("{:02}", row + 1)));
                            for (third, enabled) in beat.iter_mut().enumerate() {
                                let active = current_step == Some(row * 3 + third);
                                let mut button = egui::Button::new(
                                    RichText::new((third + 1).to_string()).size(18.0),
                                )
                                .selected(*enabled);
                                if active {
                                    button = button
                                        .stroke(Stroke::new(2.0, Color32::from_rgb(235, 160, 45)));
                                }
                                let response = ui.add_sized([64.0, 40.0], button);
                                response.widget_info(|| {
                                    egui::WidgetInfo::selected(
                                        egui::WidgetType::SelectableLabel,
                                        true,
                                        *enabled,
                                        format!("Beat {}, step {}", row + 1, third + 1),
                                    )
                                });
                                if response.clicked() {
                                    *enabled = !*enabled;
                                    changed = true;
                                }
                            }
                            if ui
                                .button("+")
                                .on_hover_text("Add an empty beat below")
                                .clicked()
                            {
                                insert = Some(row + 1);
                            }
                            if ui
                                .add_enabled(beat_count > 1, egui::Button::new("−"))
                                .on_hover_text("Remove this beat")
                                .clicked()
                            {
                                remove = Some(row);
                            }
                        });
                    });
                }
            });
        if let Some(row) = insert {
            self.pattern.beats.insert(row, [false; 3]);
            changed = true;
        }
        if let Some(row) = remove {
            self.pattern.beats.remove(row);
            changed = true;
        }
        if changed {
            self.restart();
        }
        ui.add_space(10.0);
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Load…").clicked() {
                self.load();
            }
            if ui.button("Save…").clicked() {
                self.save();
            }
            if ui.button("Reset").clicked() {
                self.stop();
                self.pattern = Pattern::default();
                self.bpm_text = self.pattern.bpm.to_string();
                self.message.clear();
            }
        });
        ui.weak("Edits restart the loop.");
        if !self.message.is_empty() {
            ui.label(&self.message);
        }
        if self.playing() {
            ui.ctx().request_repaint_after(Duration::from_millis(16));
        }
    }
}

impl eframe::App for TripleStep {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| self.controls(ui));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    fn harness() -> Harness<'static, TripleStep> {
        Harness::builder()
            .with_size(egui::vec2(460.0, 420.0))
            .build_ui_state(
                |ui, app: &mut TripleStep| app.controls(ui),
                TripleStep::default(),
            )
    }

    #[test]
    fn beat_controls_toggle_insert_remove_and_reset() {
        let mut harness = harness();
        assert_eq!(harness.state().pattern, Pattern::default());
        harness.get_by_label("Beat 1, step 2").click();
        harness.run();
        assert_eq!(harness.state().pattern.beats, vec![[true, true, false]]);
        harness.get_by_label("+").click();
        harness.run();
        assert_eq!(
            harness.state().pattern.beats,
            vec![[true, true, false], [false; 3]]
        );
        harness.get_by_label("Beat 2, step 3").click();
        harness.run();
        harness.get_all_by_label("−").next().unwrap().click();
        harness.run();
        assert_eq!(harness.state().pattern.beats, vec![[false, false, true]]);
        harness.get_by_label("−").click();
        harness.run();
        assert_eq!(harness.state().pattern.beats.len(), 1);
        harness.get_by_label("Reset").click();
        harness.run();
        assert_eq!(harness.state().pattern, Pattern::default());
    }

    #[test]
    fn bpm_accepts_typing_steps_by_five_and_rejects_invalid_values() {
        let mut harness = harness();
        harness.get_by_label(">").click();
        harness.run();
        assert_eq!(harness.state().pattern.bpm, 125.0);
        harness.get_by_label("<").click();
        harness.run();
        assert_eq!(harness.state().pattern.bpm, 120.0);
        harness.get_by_label("BPM").click();
        harness.run();
        harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::A);
        harness.event(egui::Event::Text("137.5".into()));
        harness.key_press(egui::Key::Enter);
        harness.run();
        assert_eq!(harness.state().pattern.bpm, 137.5);
        harness.get_by_label("BPM").click();
        harness.run();
        harness.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::A);
        harness.event(egui::Event::Text("0".into()));
        harness.key_press(egui::Key::Enter);
        harness.run();
        assert_eq!(harness.state().pattern.bpm, 137.5);
        assert_eq!(harness.state().message, "BPM must be between 1 and 999.");
    }
}
