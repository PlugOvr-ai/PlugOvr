use super::user_interface::PlugOvr;
use itertools::Itertools;

impl PlugOvr {
    pub fn show_shortcut_window(&mut self, egui_context: &egui::Context, scale: f32) {
        let mut window = egui::Window::new("PlugOvr Shortcuts")
            .movable(true)
            .drag_to_scroll(true)
            .interactable(true)
            .title_bar(true)
            .collapsible(false);
        let text_entryfield_position = self
            .text_entryfield_position
            .lock()
            .expect("Failed to lock text_entryfield_position POISON");
        let x = text_entryfield_position.0 as f32 / scale;
        let y = text_entryfield_position.1 as f32 / scale;
        window = window.current_pos(egui::pos2(x, y));
        let templates = self
            .prompt_templates
            .lock()
            .expect("Failed to lock prompt_templates POISON");
        window.show(egui_context, |ui| {
            // Calculate the maximum width needed for any button
            let max_width = templates
                .iter()
                .filter(|(_, (_, _, is_shortcut))| *is_shortcut)
                .map(|(key, _)| ui.text_style_height(&egui::TextStyle::Body) * key.len() as f32)
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(100.0);

            for (key, _) in templates
                .iter()
                .filter(|(_, (_, _, is_shortcut))| *is_shortcut)
                .sorted_by(|a, b| a.0.cmp(b.0))
            {
                let button = egui::Button::new(key)
                    .min_size(egui::vec2(max_width, ui.spacing().interact_size.y));

                if ui.add(button).clicked() {
                    self.assistance_window.text = key.clone();
                    *self
                        .text_entry
                        .lock()
                        .expect("Failed to lock text_entry POISON") = true;
                    self.assistance_window.shortcut_clicked = true;
                    self.assistance_window.text_entry_changed = false;
                    self.assistance_window.small_window = true;
                    *self
                        .shortcut_window
                        .lock()
                        .expect("Failed to lock shortcut_window POISON") = false;
                }
            }
        });
    }
}
