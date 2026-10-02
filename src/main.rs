mod audio;
mod pattern;

use audio::Audio;
use eframe::egui::{self, Color32, RichText, Stroke};
use pattern::{Pattern, Sound};
use std::{path::PathBuf, time::Duration};

const BACKGROUND: Color32 = Color32::from_rgb(14, 17, 22);
const PANEL: Color32 = Color32::from_rgb(21, 26, 33);
const PAD: Color32 = Color32::from_rgb(33, 40, 49);
const BORDER: Color32 = Color32::from_rgb(49, 59, 71);
const MUTED: Color32 = Color32::from_rgb(141, 155, 172);
const ACCENT: Color32 = Color32::from_rgb(192, 239, 103);
const PLAYHEAD: Color32 = Color32::from_rgb(255, 189, 105);
const ROW_HEIGHT: f32 = 36.0;

fn configure_style(ctx: &egui::Context) {
    let mut style = egui::Style::default();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = BACKGROUND;
    style.visuals.extreme_bg_color = BACKGROUND;
    style.visuals.weak_text_color = Some(MUTED);
    style.visuals.selection.bg_fill = ACCENT;
    style.visuals.selection.stroke = Stroke::new(1.0, BACKGROUND);
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
    ] {
        widget.corner_radius = egui::CornerRadius::same(7);
        widget.bg_stroke = Stroke::new(1.0, BORDER);
    }
    style.visuals.widgets.inactive.bg_fill = PAD;
    style.visuals.widgets.inactive.weak_bg_fill = PAD;
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(49, 60, 72);
    style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(49, 60, 72);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, MUTED);
    ctx.set_theme(egui::Theme::Dark);
    ctx.set_global_style(style);
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TripleStep")
            .with_inner_size([500.0, 740.0])
            .with_min_inner_size([440.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "TripleStep",
        options,
        Box::new(|cc| {
            configure_style(&cc.egui_ctx);
            Ok(Box::new(TripleStep::new(
                dirs::config_dir().map(|dir| dir.join("triplestep/config.json")),
            )))
        }),
    )
}

struct TripleStep {
    pattern: Pattern,
    bpm_text: String,
    audio: Option<Audio>,
    message: String,
    config_path: Option<PathBuf>,
}

impl Default for TripleStep {
    fn default() -> Self {
        Self {
            pattern: Pattern::default(),
            bpm_text: "120".into(),
            audio: None,
            message: String::new(),
            config_path: None,
        }
    }
}

impl TripleStep {
    fn new(config_path: Option<PathBuf>) -> Self {
        let mut app = Self {
            config_path,
            ..Self::default()
        };
        if app.config_path.as_ref().is_some_and(|path| path.exists()) {
            app.load();
        }
        app
    }

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
        if let Some(audio) = &mut self.audio
            && audio.playing()
        {
            audio.start(&self.pattern);
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
        let Some(path) = &self.config_path else {
            self.message = "Cannot locate the configuration folder.".into();
            return;
        };
        self.message = match self.pattern.save(path) {
            Ok(()) => "Settings saved.".into(),
            Err(error) => format!("Cannot save settings: {error}"),
        };
    }

    fn load(&mut self) {
        let Some(path) = &self.config_path else {
            self.message = "Cannot locate the configuration folder.".into();
            return;
        };
        match Pattern::load(path) {
            Ok(pattern) => {
                self.stop();
                self.bpm_text = pattern.bpm.to_string();
                self.pattern = pattern;
                self.message = "Settings loaded.".into();
            }
            Err(error) => self.message = format!("Cannot load settings: {error}"),
        }
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        let playing = self.playing();
        ui.horizontal(|ui| {
            ui.label(RichText::new("TripleStep").size(24.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(if playing { "PLAYING" } else { "READY" })
                        .size(11.0)
                        .color(if playing { ACCENT } else { MUTED }),
                );
                let (rect, _) = ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
                ui.painter().circle_filled(
                    rect.center(),
                    3.0,
                    if playing { ACCENT } else { MUTED },
                );
            });
        });
        ui.add_space(18.0);
        ui.label(RichText::new("TEMPO / BPM").size(11.0).color(MUTED));
        ui.horizontal(|ui| {
            let decrease = ui.add_sized([36.0, 54.0], egui::Button::new("<")).clicked();
            let response = ui.add_sized(
                [112.0, 54.0],
                egui::TextEdit::singleline(&mut self.bpm_text)
                    .font(egui::FontId::monospace(30.0))
                    .text_color(ACCENT)
                    .horizontal_align(egui::Align::Center)
                    .vertical_align(egui::Align::Center),
            );
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, "BPM"));
            if response.lost_focus()
                || (response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
            {
                self.commit_bpm();
            }
            let increase = ui.add_sized([36.0, 54.0], egui::Button::new(">")).clicked();
            if (decrease || increase) && self.commit_bpm() {
                self.pattern.bpm =
                    (self.pattern.bpm + if increase { 5.0 } else { -5.0 }).clamp(1.0, 999.0);
                self.bpm_text = self.pattern.bpm.to_string();
                self.restart();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_sized(
                        [112.0, 54.0],
                        egui::Button::new(
                            RichText::new(if playing { "Stop" } else { "Start" })
                                .size(17.0)
                                .strong()
                                .color(BACKGROUND),
                        )
                        .fill(if playing { PLAYHEAD } else { ACCENT })
                        .stroke(Stroke::NONE),
                    )
                    .clicked()
                {
                    if playing {
                        self.stop();
                    } else {
                        self.start();
                    }
                }
            });
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("SOUND").size(11.0).color(MUTED));
            let clap = ui.selectable_value(&mut self.pattern.sound, Sound::Clap, "TR-808 Clap");
            let snare = ui.selectable_value(&mut self.pattern.sound, Sound::Snare, "TR-707 Snare");
            if clap.changed() || snare.changed() {
                self.restart();
            }
        });
        ui.add_space(16.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("PATTERN").size(11.0).color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} beats / 3 steps", self.pattern.beats.len()))
                        .size(11.0)
                        .color(MUTED),
                );
            });
        });

        let current_step = self
            .audio
            .as_ref()
            .and_then(|audio| audio.current_step(&self.pattern));
        let mut changed = false;
        let mut insert = None;
        let mut remove = None;
        let beat_count = self.pattern.beats.len();
        let eight_rows_height = 8.0 * (ROW_HEIGHT + ui.spacing().item_spacing.y);
        egui::Frame::new()
            .fill(PANEL)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(12)
            .inner_margin(12)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(eight_rows_height.min((ui.available_height() - 100.0).max(50.0)))
                    .show(ui, |ui| {
                        let pad_width = (ui.available_width() - 32.0 - 56.0 - 5.0 * 8.0) / 3.0;
                        for (row, beat) in self.pattern.beats.iter_mut().enumerate() {
                            ui.push_id(row, |ui| {
                                ui.horizontal(|ui| {
                                    ui.add_sized(
                                        [32.0, ROW_HEIGHT],
                                        egui::Label::new(
                                            RichText::new(format!("{:02}", row + 1))
                                                .monospace()
                                                .color(
                                                    if current_step
                                                        .is_some_and(|step| step / 3 == row)
                                                    {
                                                        PLAYHEAD
                                                    } else {
                                                        MUTED
                                                    },
                                                ),
                                        ),
                                    );
                                    for (third, enabled) in beat.iter_mut().enumerate() {
                                        let active = current_step == Some(row * 3 + third);
                                        let button = egui::Button::new(
                                            RichText::new((third + 1).to_string())
                                                .size(16.0)
                                                .strong()
                                                .color(if active || *enabled {
                                                    BACKGROUND
                                                } else {
                                                    MUTED
                                                }),
                                        )
                                        .fill(if active {
                                            PLAYHEAD
                                        } else if *enabled {
                                            ACCENT
                                        } else {
                                            PAD
                                        })
                                        .stroke(Stroke::new(
                                            1.0,
                                            if active {
                                                PLAYHEAD
                                            } else if *enabled {
                                                ACCENT
                                            } else {
                                                BORDER
                                            },
                                        ));
                                        let response =
                                            ui.add_sized([pad_width, ROW_HEIGHT], button);
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
                                        .add_sized(
                                            [28.0, ROW_HEIGHT],
                                            egui::Button::new("+").frame(false),
                                        )
                                        .on_hover_text("Add an empty beat below")
                                        .clicked()
                                    {
                                        insert = Some(row + 1);
                                    }
                                    if ui
                                        .add_enabled_ui(beat_count > 1, |ui| {
                                            ui.add_sized(
                                                [28.0, ROW_HEIGHT],
                                                egui::Button::new("−").frame(false),
                                            )
                                        })
                                        .inner
                                        .on_hover_text("Remove this beat")
                                        .clicked()
                                    {
                                        remove = Some(row);
                                    }
                                });
                            });
                        }
                    });
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
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui
                .button("Load")
                .on_hover_text("Restore saved settings")
                .clicked()
            {
                self.load();
            }
            if ui
                .button("Save")
                .on_hover_text("Save tempo, steps, and sound")
                .clicked()
            {
                self.save();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Reset").clicked() {
                    self.stop();
                    self.pattern = Pattern::default();
                    self.bpm_text = self.pattern.bpm.to_string();
                    self.message.clear();
                }
            });
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new("Three equal steps per beat. Edits restart the loop.")
                .size(11.0)
                .color(MUTED),
        );
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
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BACKGROUND).inner_margin(20))
            .show(ui, |ui| self.controls(ui));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    fn harness() -> Harness<'static, TripleStep> {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(500.0, 740.0))
            .build_ui_state(
                |ui, app: &mut TripleStep| app.controls(ui),
                TripleStep::default(),
            );
        configure_style(&harness.ctx);
        harness.run();
        harness
    }

    #[test]
    fn beat_controls_toggle_insert_remove_and_reset() {
        let mut harness = harness();
        assert_eq!(harness.state().pattern, Pattern::default());
        harness.get_by_label("TR-707 Snare").click();
        harness.run();
        assert_eq!(harness.state().pattern.sound, Sound::Snare);
        harness.get_by_label("Beat 1, step 2").click();
        harness.run();
        assert_eq!(
            harness.state().pattern.beats,
            vec![
                [true, true, false],
                [true, false, false],
                [true, false, false],
                [true, false, false]
            ]
        );
        harness.get_all_by_label("+").next().unwrap().click();
        harness.run();
        assert_eq!(
            harness.state().pattern.beats,
            vec![
                [true, true, false],
                [false; 3],
                [true, false, false],
                [true, false, false],
                [true, false, false]
            ]
        );
        harness.get_by_label("Beat 2, step 3").click();
        harness.run();
        harness.get_all_by_label("−").next().unwrap().click();
        harness.run();
        assert_eq!(
            harness.state().pattern.beats,
            vec![
                [false, false, true],
                [true, false, false],
                [true, false, false],
                [true, false, false]
            ]
        );
        while harness.state().pattern.beats.len() > 1 {
            harness.get_all_by_label("−").last().unwrap().click();
            harness.run();
        }
        harness.get_by_label("−").click();
        harness.run();
        assert_eq!(harness.state().pattern.beats.len(), 1);
        harness.get_by_label("Reset").click();
        harness.run();
        assert_eq!(harness.state().pattern, Pattern::default());
    }

    #[test]
    fn save_and_load_restore_all_settings() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("triplestep/config.json");
        let mut harness = harness();
        harness.state_mut().config_path = Some(path.clone());
        harness.get_by_label(">").click();
        harness.run();
        harness.get_by_label("TR-707 Snare").click();
        harness.run();
        harness.get_by_label("Beat 1, step 3").click();
        harness.run();
        harness.get_all_by_label("+").last().unwrap().click();
        harness.run();
        let expected = harness.state().pattern.clone();
        harness.get_by_label("Save").click();
        harness.run();
        assert_eq!(Pattern::load(&path).unwrap(), expected);
        assert_eq!(TripleStep::new(Some(path.clone())).pattern, expected);
        harness.get_by_label("Reset").click();
        harness.run();
        assert_eq!(harness.state().pattern, Pattern::default());
        assert_eq!(Pattern::load(&path).unwrap(), expected);
        harness.get_by_label("Load").click();
        harness.run();
        assert_eq!(harness.state().pattern, expected);
        assert_eq!(harness.state().bpm_text, "125");
        harness.state_mut().bpm_text = "0".into();
        harness.get_by_label("Save").click();
        harness.run();
        assert_eq!(Pattern::load(&path).unwrap(), expected);
        std::fs::write(&path, "invalid json").unwrap();
        harness.get_by_label("Load").click();
        harness.run();
        assert_eq!(harness.state().pattern, expected);
        assert!(harness.state().message.starts_with("Cannot load settings:"));
    }

    #[test]
    fn eight_rows_can_be_edited_without_scrolling() {
        let mut harness = harness();
        for _ in 0..4 {
            harness.get_all_by_label("+").last().unwrap().click();
            harness.run();
        }
        harness.get_by_label("Beat 8, step 3").click();
        harness.run();
        assert_eq!(harness.state().pattern.beats[7], [false, false, true]);
        harness.get_by_label("Beat 1, step 3").click();
        harness.run();
        assert_eq!(harness.state().pattern.beats[0], [true, false, true]);
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
        harness
            .get_by_role_and_label(egui::accesskit::Role::TextInput, "BPM")
            .click();
        harness.run();
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        harness.event(egui::Event::Text("137.5".into()));
        harness.key_press(egui::Key::Enter);
        harness.run();
        assert_eq!(harness.state().pattern.bpm, 137.5);
        harness
            .get_by_role_and_label(egui::accesskit::Role::TextInput, "BPM")
            .click();
        harness.run();
        harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        harness.event(egui::Event::Text("0".into()));
        harness.key_press(egui::Key::Enter);
        harness.run();
        assert_eq!(harness.state().pattern.bpm, 137.5);
        assert_eq!(harness.state().message, "BPM must be between 1 and 999.");
    }
}
