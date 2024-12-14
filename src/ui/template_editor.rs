use crate::llm::CloudModel; // Add this line
use crate::llm::LLMSelector;
use crate::llm::LLMType;
use crate::llm::LocalModel;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use strum::IntoEnumIterator;

pub type TemplateMap = Arc<Mutex<HashMap<String, (String, Option<LLMType>, bool)>>>;

pub struct TemplateEditor {
    pub show: bool,
    prompt_templates: TemplateMap,
    new_template_key: String,
    new_template_value: String,
    reset_templates_confirmation: String,
    llm_selector: Arc<Mutex<LLMSelector>>,
}

impl TemplateEditor {
    pub fn new(prompt_templates: TemplateMap) -> Self {
        Self {
            show: false,
            prompt_templates,
            new_template_key: String::new(),
            new_template_value: String::new(),
            reset_templates_confirmation: String::new(),
            llm_selector: Arc::new(Mutex::new(LLMSelector::new(Arc::new(Mutex::new(None))))),
        }
    }
    pub fn show_template_editor(&mut self, egui_context: &egui::Context) {
        let mut show_window = self.show; // Create local copy
        egui::Window::new("Template Editor")
            .resizable(true)
            .collapsible(false)
            .open(&mut show_window) // Use local copy
            .show(egui_context, |ui| {
                ui.group(|ui| {
                    let mut templates_to_remove = Vec::new();
                    let mut templates_to_add = Vec::new();

                    // Clone the templates to avoid holding the lock
                    let templates_clone = self.prompt_templates.lock().unwrap().clone();
                    // Convert HashMap to Vec and sort
                    let mut templates_vec: Vec<_> = templates_clone.into_iter().collect();
                    templates_vec.sort_by(|a, b| a.0.cmp(&b.0));

                    egui::Grid::new("template_grid")
                        .num_columns(4)
                        .max_col_width(400.0)
                        .min_col_width(50.)
                        .striped(false)
                        .show(ui, |ui| {
                            ui.label("Template Name");
                            ui.label("Instruction");
                            ui.label("AI Model");
                            ui.label("Shortcut");

                            ui.end_row();

                            // Use templates_vec instead of templates_clone in the loop
                            for (key, value) in &templates_vec {
                                let mut local_value = value.clone();
                                ui.label(key);
                                let value_edit = ui.text_edit_singleline(&mut local_value.0);
                                let mut selected_llm = local_value.1.clone();
                                egui::ComboBox::from_id_source(format!("llm_type_{}", key))
                                    .selected_text(if let Some(llm_type) = &local_value.1 {
                                        format!("{:?}", llm_type.description())
                                    } else {
                                        "default".to_string()
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut selected_llm, None, "default");
                                        for cloud_model in CloudModel::iter() {
                                            ui.selectable_value(
                                                &mut selected_llm,
                                                Some(LLMType::Cloud(cloud_model)),
                                                cloud_model.description(),
                                            );
                                        }
                                        for local_model in LocalModel::iter() {
                                            ui.selectable_value(
                                                &mut selected_llm,
                                                Some(LLMType::Local(local_model)),
                                                local_model.description(),
                                            );
                                        }
                                        let ollama_models = self
                                            .llm_selector
                                            .lock()
                                            .unwrap()
                                            .ollama_models
                                            .lock()
                                            .unwrap()
                                            .clone();
                                        if let Some(models) = ollama_models {
                                            for model in models {
                                                let llm_type = LLMType::Ollama(model.name.clone());
                                                ui.selectable_value(
                                                    &mut selected_llm,
                                                    Some(llm_type),
                                                    model.name.clone(),
                                                );
                                            }
                                        }

                                        if selected_llm != local_value.1.clone() {
                                            local_value.1 = selected_llm;
                                            templates_to_add
                                                .push((key.clone(), local_value.clone()));
                                        }
                                    });

                                if ui.checkbox(&mut local_value.2, "").changed() {
                                    templates_to_add.push((key.clone(), local_value.clone()));
                                }

                                ui.horizontal(|ui| {
                                    if ui
                                        .button("Default")
                                        .on_hover_text("Resets to default value if existed.")
                                        .clicked()
                                    {
                                        if let Some(default_value) =
                                            create_prompt_templates().get(key)
                                        {
                                            let local_value = default_value.clone();
                                            templates_to_add
                                                .push((key.clone(), local_value.clone()));
                                        }
                                    }
                                    if ui.button("Remove").clicked() {
                                        templates_to_remove.push(key.clone());
                                    }
                                });

                                if value_edit.changed() {
                                    templates_to_add.push((key.clone(), local_value));
                                }

                                ui.end_row();
                            }

                            ui.end_row();
                            ui.label("Add new template");
                            ui.end_row();
                            let key = self.new_template_key.clone();
                            ui.add(
                                egui::TextEdit::singleline(&mut self.new_template_key)
                                    .desired_width(150.0)
                                    .hint_text("@...")
                                    .text_color_opt(if key.starts_with('@') {
                                        None
                                    } else {
                                        Some(egui::Color32::RED)
                                    }),
                            );
                            ui.text_edit_singleline(&mut self.new_template_value);
                            if ui.button("Add").clicked()
                                && !self.new_template_key.is_empty()
                                && !self.new_template_value.is_empty()
                                && key.starts_with('@')
                            {
                                self.prompt_templates.lock().unwrap().insert(
                                    self.new_template_key.clone(),
                                    (self.new_template_value.clone(), None, false),
                                );
                                self.save_templates();
                                self.new_template_key.clear();
                                self.new_template_value.clear();
                            }
                        });

                    ui.horizontal(|ui| {
                        ui.label("Reset all templates to default:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.reset_templates_confirmation)
                                .hint_text("type 'reset' to confirm"),
                        );
                        if ui.button("Reset all").clicked()
                            && self.reset_templates_confirmation == "reset"
                        {
                            self.reset_templates();
                            self.reset_templates_confirmation.clear();
                        }
                    });
                    // Handle additions and removals
                    let mut templates = self.prompt_templates.lock().unwrap();
                    for key in &templates_to_remove {
                        templates.remove(key);
                    }
                    for (key, value) in &templates_to_add {
                        templates.insert(key.clone(), value.clone());
                    }

                    if !templates_to_remove.is_empty() || !templates_to_add.is_empty() {
                        drop(templates); // Release the lock before saving
                        self.save_templates();
                    }

                    ui.add_space(10.0);
                });
            });
        self.show = show_window; // Update original value
    }

    pub fn save_templates(&self) {
        let templates = self.prompt_templates.lock().unwrap();
        let templates_json =
            serde_json::to_string_pretty(&*templates).expect("Failed to serialize templates");
        let home_dir = dirs::home_dir().expect("Unable to find home directory");
        let config_dir = home_dir.join(".plugovr");
        std::fs::create_dir_all(&config_dir).expect("Failed to create config directory");
        let config_file = config_dir.join("templates.json");
        std::fs::write(config_file, templates_json).expect("Failed to write templates to file");
    }

    pub fn load_templates(&mut self) {
        let home_dir = dirs::home_dir().expect("Unable to find home directory");
        let config_file = home_dir.join(".plugovr").join("templates.json");
        if let Ok(templates_json) = std::fs::read_to_string(config_file) {
            if let Ok(loaded_templates) = serde_json::from_str(&templates_json) {
                *self.prompt_templates.lock().unwrap() = loaded_templates;
            }
        }
    }
    pub fn reset_templates(&mut self) {
        let home_dir = dirs::home_dir().expect("Unable to find home directory");
        let config_file = home_dir.join(".plugovr").join("templates.json");
        match std::fs::remove_file(config_file) {
            Ok(_) => println!("Templates reset successfully"),
            Err(e) => println!("Failed to reset templates: {}", e),
        }
        *self.prompt_templates.lock().unwrap() = create_prompt_templates();
    }
}
pub fn create_prompt_templates() -> HashMap<String, (String, Option<LLMType>, bool)> {
    let mut templates: HashMap<String, (String, Option<LLMType>, bool)> = HashMap::new();
    templates.insert(
        "@correct".to_string(),
        (
            "Correct the text without explanation".to_string(),
            None,
            true,
        ),
    );
    templates.insert(
        "@translate(english)".to_string(),
        (
            "Translate the text to english without explanation".to_string(),
            None,
            true,
        ),
    );
    templates.insert(
        "@translate(german)".to_string(),
        (
            "Translate the text to german without explanation".to_string(),
            None,
            true,
        ),
    );
    templates.insert(
        "@translate(spanish)".to_string(),
        (
            "Translate the text to spanish without explanation".to_string(),
            None,
            true,
        ),
    );
    templates.insert(
        "@summarize".to_string(),
        (
            "Provide a short summary of the text".to_string(),
            None,
            true,
        ),
    );

    templates.insert(
        "@improve".to_string(),
        (
            "Suggest improvements or enhancements for the given text without explanation"
                .to_string(),
            None,
            false,
        ),
    );
    templates.insert(
        "@format".to_string(),
        (
            "Format the text for better readability without explanation".to_string(),
            None,
            true,
        ),
    );

    templates.insert(
        "@simplify".to_string(),
        ("Simplify complex text or concepts".to_string(), None, false),
    );
    templates.insert(
        "@extend".to_string(),
        (
            "continue the text without explanation".to_string(),
            None,
            true,
        ),
    );
    templates.insert(
        "@filename".to_string(),
        ("propose filename for document: structure:date(year_month_day )_topic_company. Output only filename".to_string(), None, false),
    );
    // templates.insert(
    //     "@fillfields".to_string(),
    //     ("Extract caption and coordinates from all textboxes in image 2. For each textbox, find the corresponding content in image 1. Output the information in format of json. [{ \"content_image_1\": \"content\", \"caption_textbox_image_2\": \"caption\", \"coordinates_textbox_image_2\": \"[x1,y1,x2,y2]\" }]".to_string(),
    //     None),
    // );
    templates.insert(
             "@fillform".to_string(),
             ("#computeruse Output the coordinates for each input field / textbox in json format from image 1 (screenshot) and fill with information from images starting from image 2. The original textbox should be empty before we fill it. Json format: [{ \"caption\": \"<caption>\", \"content\": \"<content>\", \"coordinates\": \"[x1, y1, x2, y2]\" }]".to_string(),
        Some(LLMType::Cloud(CloudModel::AnthropicSonnet3_5)), false),
    );
    templates
}
