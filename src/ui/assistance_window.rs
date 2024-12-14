use crate::llm::LLMSelector;
use crate::llm::LLMType;
use crate::ui::answer_analyser::analyse_answer;
use crate::ui::template_editor::TemplateMap;
use plugovr_types::Screenshots;

use crate::ui::diff_view::display_diff;
use crate::ui::show_form_fields::FormFieldsOverlay;

use crate::window_handling;
use crate::window_handling::ActiveWindow;
use arboard::Clipboard;
use egui::Layout;
use egui::ScrollArea;
use egui::scroll_area::ScrollBarVisibility;
use itertools::Itertools;
use screenshots::Screen;
use std::collections::BTreeSet;

use std::sync::{Arc, Mutex};
#[derive(Debug, Clone, Copy, PartialEq)]

pub enum AiResponseAction {
    Replace,
    Extend,
    Ignore,
}
#[derive(PartialEq)]
pub enum DisplayMode {
    Normal,
    Diff,
}

pub struct AssistanceWindow {
    pub show: bool,
    pub text_entry: Arc<Mutex<bool>>,
    pub text_entryfield_position: Arc<Mutex<(i32, i32)>>,
    pub ai_context: Arc<Mutex<String>>,
    pub screenshots: Screenshots,
    pub text_entry_changed: bool,
    pub shortcut_clicked: bool,
    pub small_window: bool,
    prompt_templates: TemplateMap,
    spinner: Arc<Mutex<bool>>,
    ai_answer: Arc<Mutex<String>>,
    max_tokens_reached: Arc<Mutex<bool>>,
    llm_selector: Arc<Mutex<LLMSelector>>,
    pub ai_response_action: AiResponseAction,
    display_mode: DisplayMode,
    last_analyzed_answer: String,

    form_fields_overlay: FormFieldsOverlay,

    pub screenshot_mode: bool,

    pub screenshot_start: Option<egui::Pos2>,

    pub screenshot_end: Option<egui::Pos2>,
    pub screen_width: u16,
    pub screen_height: u16,
    pub scale: f32,
    pub text: String,
    pub clipboard: Clipboard,
    pub active_window: Arc<Mutex<ActiveWindow>>,
}

impl AssistanceWindow {
    pub fn new(
        active_window: Arc<Mutex<ActiveWindow>>,
        text_entry: Arc<Mutex<bool>>,
        text_entryfield_position: Arc<Mutex<(i32, i32)>>,
        ai_context: Arc<Mutex<String>>,
        prompt_templates: TemplateMap,
        mouse_position: Arc<Mutex<(i32, i32)>>,
        ai_answer: Arc<Mutex<String>>,
        screen_width: u16,
        screen_height: u16,
        llm_selector: Arc<Mutex<LLMSelector>>,
    ) -> Self {
        Self {
            show: false,
            text_entry,
            text_entryfield_position,
            ai_context,
            screenshots: Vec::new(),
            text_entry_changed: false,
            shortcut_clicked: false,
            small_window: false,
            prompt_templates,
            spinner: Arc::new(Mutex::new(false)),

            ai_answer,
            max_tokens_reached: Arc::new(Mutex::new(false)),
            llm_selector,
            ai_response_action: AiResponseAction::Replace,
            display_mode: DisplayMode::Normal,
            last_analyzed_answer: String::new(),
            form_fields_overlay: FormFieldsOverlay::new(mouse_position.clone()),
            screenshot_mode: false,

            screenshot_start: None,
            screenshot_end: None,
            screen_width,
            screen_height,
            scale: 1.0,
            text: String::new(),
            clipboard: Clipboard::new().unwrap(),
            active_window,
        }
    }
    pub fn show(&mut self, egui_context: &egui::Context, scale: f32) {
        self.show = *self
            .text_entry
            .lock()
            .expect("Failed to lock text_entry POISON");

        let text_entryfield_position = *self
            .text_entryfield_position
            .lock()
            .expect("Failed to lock text_entryfield_position POISON");
        let x = text_entryfield_position.0 as f32 / scale;
        let y = text_entryfield_position.1 as f32 / scale;
        let mut window = egui::Window::new("PlugOvr Assistant")
            .movable(true)
            .drag_to_scroll(true)
            .interactable(true)
            .title_bar(true)
            .open(&mut self.show)
            .collapsible(false);

        if self.text_entry_changed || self.shortcut_clicked {
            window = window.current_pos(egui::pos2(x, y)).max_width(400.0);
            self.screenshots = Vec::new();
        }
        if self.text_entry_changed && !self.shortcut_clicked {
            self.small_window = false;
            self.ai_answer
                .lock()
                .expect("Failed to lock ai_answer POISON")
                .clear();
        }
        let mut run_llm = false;
        window.show(egui_context, |ui| {
            ui.group(|ui| {
                if !self.small_window {
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::left_to_right(egui::Align::TOP), |ui| {
                            ui.vertical(|ui| {
                                ui.label("AI Context:");
                                if ui.button("Add Screenshot").clicked() {
                                    self.screenshot_mode = true;
                                    *self.text_entry.lock().expect("Failed to lock text_entry POISON") = false;
                                }
                                if ui.button("Clear").clicked() {
                                    self.ai_context.lock().expect("Failed to lock ai_context POISON").clear();
                                    self.screenshots = Vec::new();
                                }

                            });
                            ScrollArea::vertical()
                                .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                                .show(ui, |ui| {
                                    ui.with_layout(
                                        egui::Layout::left_to_right(egui::Align::TOP)
                                            .with_main_justify(true)
                                            .with_main_wrap(true),
                                        |ui| {
                                            ui.vertical(|ui| {
                                                if !self.screenshots.is_empty() {
                                                    ui.horizontal(|ui| {
                                                        for (i, screenshot) in self.screenshots.iter().enumerate() {
                                                            let max_width = 100.0f32;
                                                            let max_height = 100.0f32;

                                                            let scale = (max_width / screenshot.0.width() as f32)
                                                                .min(max_height / screenshot.0.height() as f32)
                                                                .min(1.0f32);

                                                            let new_width = (screenshot.0.width() as f32 * scale) as u32;
                                                            let new_height = (screenshot.0.height() as f32 * scale) as u32;

                                                            let screenshot_small = image_24::imageops::resize(
                                                                &screenshot.0,
                                                                new_width,
                                                                new_height,
                                                                image_24::imageops::FilterType::Triangle,
                                                            );

                                                            let size = [new_width as _, new_height as _];
                                                            let image = egui::ColorImage::from_rgba_unmultiplied(
                                                                size,
                                                                screenshot_small.as_flat_samples().as_slice(),
                                                            );
                                                            let texture = egui_context.load_texture(
                                                                format!("screenshot_thumb_{}", i),
                                                                image,
                                                                Default::default(),
                                                            );

                                                            let response = ui.image(&texture);

                                                            if response.hovered() {
                                                                let full_size = [screenshot.0.width() as _, screenshot.0.height() as _];
                                                                let full_image = egui::ColorImage::from_rgba_unmultiplied(
                                                                    full_size,
                                                                    screenshot.0.as_flat_samples().as_slice(),
                                                                );
                                                                let full_texture = egui_context.load_texture(
                                                                    format!("screenshot_full_{}", i),
                                                                    full_image,
                                                                    Default::default(),
                                                                );

                                                                egui::show_tooltip(
                                                                    ui.ctx(),
                                                                    ui.layer_id(),
                                                                    egui::Id::new(format!("full_screenshot_{}", i)),
                                                                    |ui| {
                                                                        ui.image(&full_texture);
                                                                    },
                                                                );
                                                            }
                                                        }
                                                });
                                                }
                                                ui.label(self.ai_context.lock().expect("Failed to lock ai_context POISON").as_str());
                                            });
                                        },
                                    )
                                });
                        });
                    });
                    ui.add_space(10.0); // Add bottom margin
                    let mut shortcut_clicked = false;
                    ui.horizontal(|ui| {    // Add buttons for each template shortcut
                        ui.label("Shortcut:");
                        let templates = self.prompt_templates.lock().expect("Failed to lock prompt_templates POISON");

                        for (key, _) in templates.iter().filter(|(_, (_, _, is_shortcut))| *is_shortcut).sorted_by(|a, b| a.0.cmp(b.0)) {
                            if ui.button(key).clicked() {
                                self.text = key.clone();
                                shortcut_clicked=true;
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        let inputs = self
                            .prompt_templates
                            .lock()
                            .expect("Failed to lock prompt_templates POISON")
                            .keys()
                            .cloned()
                            .collect::<BTreeSet<_>>();

                        let resp = ui.add(
                            egui_autocomplete::AutoCompleteTextEdit::new(&mut self.text, inputs)
                                .set_text_edit_properties(|text_edit: egui::TextEdit<'_>| {
                                    text_edit
                                        .hint_text("enter your instructions to the AI")
                                        .frame(true)
                                }),
                        );
                        if ui.memory(|mem| !mem.has_focus(resp.id)) && self.text_entry_changed {
                            self.text = "".to_string();
                            resp.request_focus();
                            self.text_entry_changed = false;
                        }
                        let resp_submit = ui.add(egui::Button::new("Submit"));
                        if resp.lost_focus() {
                            resp_submit.request_focus();
                        }
                        if resp_submit.clicked() || shortcut_clicked {
                            self.ai_answer.lock().expect("Failed to lock ai_answer POISON").clear();
                            run_llm=true;
                        }



                        ui.add(egui::Label::new(" "))
                    });
                } else if self.shortcut_clicked {
                    run_llm=true;
                }

                if run_llm && !*self.spinner.lock().expect("Failed to lock spinner POISON") {
                    self.shortcut_clicked = false;
                    let mut ai_instruction = self.text.clone();
                    let mut llm_from_template: Option<LLMType> = None;

                    for (template, replacement) in self.prompt_templates.lock().expect("Failed to lock prompt_templates POISON").iter() {
                        if ai_instruction.contains(template) {
                            ai_instruction = ai_instruction.replace(template, replacement.0.as_str());
                            llm_from_template = replacement.1.clone();
                            break;
                        }
                    }

                    let prompt = format!(
                        "context: {} instruction: {}",
                        self.ai_context.lock().expect("Failed to lock ai_context POISON"),
                        ai_instruction
                    );

                    let spinner_clone = self.spinner.clone();
                    let ai_answer_clone = self.ai_answer.clone();
                    let max_tokens_reached_clone = self.max_tokens_reached.clone();

                    let _ = self.llm_selector.lock().expect("Failed to lock llm_selector POISON").process_input(
                        prompt,
                        self.ai_context.lock().expect("Failed to lock ai_context POISON").clone(),
                        self.screenshots.clone(),
                        ai_instruction,
                        ai_answer_clone,
                        max_tokens_reached_clone,
                        spinner_clone,
                        llm_from_template,
                    );
                }
                ui.vertical(|ui| {
                    if *self.max_tokens_reached.lock().expect("Failed to lock max_tokens_reached POISON") {
                        let colored_label = egui::RichText::new(
                            "Warning: Max tokens reached. Your answer may be incomplete.",
                        )
                        .color(egui::Color32::from_rgb(255, 0, 0)); // RGB for red color

                        ui.label(colored_label);
                    }
                    ui.horizontal(|ui| {
                        ui.label("Action:");
                        ui.radio_value(
                            &mut self.ai_response_action,
                            AiResponseAction::Replace,
                            "Replace",
                        )
                        .on_hover_text("Replace the selected text with the AI answer");
                        ui.radio_value(
                            &mut self.ai_response_action,
                            AiResponseAction::Extend,
                            "Extend",
                        )
                        .on_hover_text(
                            "Extend the text with the AI answer at the cursor position",
                        );
                        ui.radio_value(
                            &mut self.ai_response_action,
                            AiResponseAction::Ignore,
                            "Ignore",
                        )
                        .on_hover_text(
                            "Does nothing with the AI answer. Closes windows on accept.",
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("AI Answer:");

                        if ui.add(egui::Button::new("Accept")).clicked() {
                            *self.text_entry.lock().expect("Failed to lock text_entry POISON") = false;

                            let active_window = self.active_window.clone();
                            let ai_context = self.ai_context.clone();
                            let ai_answer = self.ai_answer.clone();
                            let ai_resonde_action = self.ai_response_action;
                            std::thread::spawn(move || {
                                if let Err(e) = window_handling::send_results(
                                    active_window,
                                    ai_context.clone(),
                                    ai_answer.clone(),
                                    ai_resonde_action,
                                ) {
                                    eprintln!("Error sending result: {:?}", e);
                                }
                                *ai_answer.lock().expect("Failed to lock ai_answer POISON") = "".to_string();
                            });
                        }
                        if ui.add(egui::Button::new("Reject")).clicked() {
                            *self.ai_answer.lock().expect("Failed to lock ai_answer POISON") = "".to_string();
                            self.text = "".to_string();
                            *self.text_entry.lock().expect("Failed to lock text_entry POISON") = false;
                        }
                        if ui.add(egui::Button::new("Copy to Clipboard")).clicked() {
                            let _ = self.clipboard
                                .set_text(self.ai_answer.lock().expect("Failed to lock ai_answer POISON").to_owned());
                        }
                        ui.add(egui::Label::new(" "))
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Display mode:");
                        ui.radio_value(&mut self.display_mode, DisplayMode::Normal, "Normal");
                        ui.radio_value(&mut self.display_mode, DisplayMode::Diff, "Diff");
                    });
                    ScrollArea::vertical().max_height(500.0)
                        .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                        .show(ui, |ui| {
                            let ai_answer = self.ai_answer.lock().expect("Failed to lock ai_answer POISON");
                            if self.display_mode == DisplayMode::Normal {
                                ui.label(ai_answer.as_str());
                            } else {
                                // Compute and display diff
                                let original_text = self.ai_context.lock().expect("Failed to lock ai_context POISON");
                                if !ai_answer.is_empty() {
                                    display_diff(ui, &original_text, &ai_answer);
                                } else {
                                    ui.label(ai_answer.as_str());
                                }
                            }
                        if *self.spinner.lock().expect("Failed to lock spinner POISON") {
                            ui.add(egui::Spinner::new());
                        }
                            // Only analyze if the answer has changed
                            if ai_answer.as_str() != self.last_analyzed_answer.as_str() {
                                if let Some(form_fields) = analyse_answer(&ai_answer) {
                                    self.form_fields_overlay.form_fields = Some(form_fields);
                                    self.form_fields_overlay.hidden_fields.lock().expect("Failed to lock hidden_fields POISON").clear();
                                }
                                self.last_analyzed_answer = ai_answer.clone();
                            }
                        });
                    ui.label("")
                })
                .inner
            })
            .inner
        });
        if self.screenshot_mode {
            // Change cursor to crosshair
            egui_context.output_mut(|o| o.cursor_icon = egui::CursorIcon::Crosshair);

            let screen_rect = egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(self.screen_width as f32, self.screen_height as f32),
            );
            let scale = self.scale;
            egui::Area::new(egui::Id::new("screenshot_overlay"))
                .fixed_pos(egui::pos2(0.0, 0.0))
                .show(egui_context, |ui| {
                    let response = ui.allocate_rect(screen_rect, egui::Sense::drag());

                    if response.drag_started() {
                        let pos = response.hover_pos().unwrap();
                        self.screenshot_start = Some(egui::Pos2::new(pos.x * scale, pos.y * scale));
                    }

                    if let Some(hover_pos) = response.hover_pos() {
                        self.screenshot_end =
                            Some(egui::Pos2::new(hover_pos.x * scale, hover_pos.y * scale));

                        if let Some(start) = self.screenshot_start {
                            let rect = egui::Rect::from_two_pos(
                                egui::Pos2::new(start.x / scale, start.y / scale),
                                hover_pos,
                            );
                            ui.painter().rect_stroke(
                                rect,
                                0.0,
                                (2.0, egui::Color32::RED),
                                egui::StrokeKind::Inside,
                            );
                        }
                    }

                    if response.drag_stopped() {
                        self.take_screenshot();
                        *self.text_entry.lock().unwrap() = true;
                        self.show = true;
                    }

                    // Check for Escape key press
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.exit_screenshot_mode();
                    }
                });
        } else {
            // Reset cursor to default when not in screenshot mode
            egui_context.output_mut(|o| o.cursor_icon = egui::CursorIcon::Default);
        }

        self.form_fields_overlay
            .show(egui_context, self.screenshots.first().map(|(_, pos)| pos));
        if !self.show {
            *self
                .text_entry
                .lock()
                .expect("Failed to lock text_entry POISON") = self.show;
        }
    }
    fn take_screenshot(&mut self) {
        if let (Some(start), Some(end)) = (self.screenshot_start, self.screenshot_end) {
            let x = start.x.min(end.x) as i32;
            let y = start.y.min(end.y) as i32;
            let width = (start.x - end.x).abs() as u32;
            let height = (start.y - end.y).abs() as u32;

            let screens = Screen::all().unwrap();
            let mut selected_screen = &screens[0];
            let mut adjusted_x = x;

            // Find the correct screen and adjust x coordinate
            for screen in &screens {
                if x >= screen.display_info.x
                    && x < screen.display_info.x + screen.display_info.width as i32
                {
                    selected_screen = screen;
                    adjusted_x = x - screen.display_info.x;
                    break;
                }
            }

            let display_scale = selected_screen.display_info.scale_factor;

            if let Ok(image) = selected_screen.capture_area(
                (adjusted_x as f32 + display_scale) as i32,
                (y as f32 + display_scale) as i32,
                (width as f32 - 2.0 * display_scale) as u32,
                (height as f32 - 2.0 * display_scale) as u32,
            ) {
                if display_scale > 1.0 {
                    let scale_factor = 1.0 / display_scale;
                    let new_width = (image.width() as f32 * scale_factor) as u32;
                    let new_height = (image.height() as f32 * scale_factor) as u32;
                    let image_resized = image_24::imageops::resize(
                        &image,
                        new_width,
                        new_height,
                        image_24::imageops::FilterType::Lanczos3,
                    );
                    self.screenshots.push((image_resized, start));
                } else if self.scale != 1.0 {
                    let scale_factor = self.scale;
                    let new_width = (image.width() as f32 / scale_factor) as u32;
                    let new_height = (image.height() as f32 / scale_factor) as u32;
                    let image_resized = image_24::imageops::resize(
                        &image,
                        new_width,
                        new_height,
                        image_24::imageops::FilterType::Lanczos3,
                    );
                    self.screenshots.push((image_resized, start));
                } else {
                    self.screenshots.push((image, start));
                }
            }
        }

        // Reset screenshot mode and coordinates
        self.screenshot_mode = false;
        self.screenshot_start = None;
        self.screenshot_end = None;
    }
    fn exit_screenshot_mode(&mut self) {
        self.screenshot_mode = false;
        self.screenshot_start = None;
        self.screenshot_end = None;
        *self.text_entry.lock().unwrap() = true;
    }
}
