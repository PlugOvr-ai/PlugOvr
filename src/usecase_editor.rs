use crate::usecase_recorder::{EventType, Point, UseCase};
use egui_file_dialog::FileDialog;
use image::GenericImageView;
//use rfd;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Default)]
pub struct UsecaseEditor {
    usecase: Option<UseCase>,
    current_step: usize,
    file_dialog: FileDialog,
    picked_file: Option<PathBuf>,
    cached_textures: std::collections::HashMap<String, egui::TextureHandle>,
}

impl UsecaseEditor {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn show_editor(&mut self, ctx: &egui::Context) -> bool {
        let mut show = true;
        egui::Window::new("Usecase Editor")
            .default_size([800.0, 600.0])
            .open(&mut show)
            .show(ctx, |ui| {
                self.show_menu_bar(ui);
                self.show_content(ui);
                // Update the dialog
                self.file_dialog.update(ctx);

                // Check if the user picked a file.
                if let Some(path) = self.file_dialog.take_selected() {
                    self.picked_file = Some(path.to_path_buf());
                    if let Some(path) = &self.picked_file {
                        if let Ok(contents) = fs::read_to_string(path) {
                            if let Ok(usecase) = serde_json::from_str::<UseCase>(&contents) {
                                self.usecase = Some(usecase);
                                self.current_step = 0;
                            }
                        }
                    }
                }
            });
        show
    }

    fn show_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            egui::menu::menu_button(ui, "File", |ui| {
                if ui.button("Open").clicked() {
                    self.file_dialog.select_file();
                }
                if ui.button("Save").clicked() {
                    if let Some(usecase) = &mut self.usecase {
                        if let Some(path) = &self.picked_file {
                            if let Ok(contents) = serde_json::to_string_pretty(usecase) {
                                fs::write(path, contents).unwrap();
                            }
                        }
                    }
                }
            });
        });
    }

    fn show_content(&mut self, ui: &mut egui::Ui) {
        if let Some(usecase) = &mut self.usecase {
            // Make name and instructions editable
            ui.text_edit_singleline(&mut usecase.usecase_name);
            ui.text_edit_multiline(&mut usecase.usecase_instructions);

            ui.add_space(10.0);

            // Navigation controls
            ui.horizontal(|ui| {
                if ui.button("Previous").clicked() && self.current_step > 0 {
                    self.current_step -= 1;
                }
                ui.label(format!(
                    "Step {} of {}",
                    self.current_step + 1,
                    usecase.usecase_steps.len()
                ));
                if ui.button("Next").clicked()
                    && self.current_step < usecase.usecase_steps.len() - 1
                {
                    self.current_step += 1;
                }
            });

            ui.add_space(10.0);

            // Show current step
            let current_step = self.current_step;
            let (prev_steps, current_and_after) = usecase.usecase_steps.split_at_mut(current_step);
            let step = &mut current_and_after[0];
            // let pretty = serde_json::to_string_pretty(&step).unwrap_or_default();
            // let mut text = pretty.clone();
            // if ui.text_edit_multiline(&mut text).changed() {
            //     // Handle text changes if needed
            // }

            // Handle click events and monitor data
            match step {
                EventType::Click(point, desc) => {
                    ui.label(format!("Click coordinates: ({}, {})", point.x, point.y));
                    ui.label("Description:");
                    ui.text_edit_singleline(desc);
                    // Search backwards for the most recent Monitor1 event
                    if let Some(monitor_data) = prev_steps.iter().rev().find_map(|step| {
                        if let EventType::Monitor1(data) = step {
                            Some(data)
                        } else {
                            None
                        }
                    }) {
                        display_step_image(
                            ui,
                            monitor_data,
                            &format!("image_{}", current_step),
                            (point.x as i32, point.y as i32),
                            &mut self.cached_textures,
                        );
                    }
                }
                EventType::Monitor1(data) => {
                    ui.label("Monitor1");
                    display_step_image(
                        ui,
                        data,
                        &format!("image_{}", current_step),
                        (-1, -1),
                        &mut self.cached_textures,
                    );
                }
                EventType::Monitor2(data) => {
                    ui.label("Monitor2");
                    display_step_image(
                        ui,
                        data,
                        &format!("image_{}", current_step),
                        (-1, -1),
                        &mut self.cached_textures,
                    );
                }
                EventType::Monitor3(data) => {
                    ui.label("Monitor3");
                    display_step_image(
                        ui,
                        data,
                        &format!("image_{}", current_step),
                        (-1, -1),
                        &mut self.cached_textures,
                    );
                }
                EventType::Text(text) => {
                    ui.label("Text");
                    ui.text_edit_singleline(text);
                }
                EventType::KeyDown(key) => {
                    ui.label("KeyDown");
                    ui.text_edit_singleline(key);
                }
                EventType::KeyUp(key) => {
                    ui.label("KeyUp");
                    ui.text_edit_singleline(key);
                }
                // Handle other event types as needed
                _ => {}
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Open a usecase file using the File menu");
            });
        }
    }
}

// Helper function to display step images
fn display_step_image(
    ui: &mut egui::Ui,
    monitor_data: &str,
    texture_id: &str,
    coords: (i32, i32),
    cached_textures: &mut std::collections::HashMap<String, egui::TextureHandle>,
) {
    if let Some(texture) = cached_textures.get(texture_id) {
        ui.image(texture);
        return;
    }

    if let Ok(image_data) = base64::decode(monitor_data) {
        if let Ok(image) = image::load_from_memory(&image_data) {
            // Draw a red circle at click coordinates
            let mut image = image.to_rgba8();
            let radius = 10;
            let color = image::Rgba([255, 0, 0, 255]); // Red circle

            // Scale coords to match resized image dimensions
            let scaled_x = coords.0;
            let scaled_y = coords.1;
            if scaled_x != -1 && scaled_y != -1 {
                // Draw circle by iterating over pixels in bounding box
                for y in -radius..=radius {
                    for x in -radius..=radius {
                        // Check if point is within circle using distance formula
                        if x * x + y * y <= radius * radius {
                            let px = scaled_x + x;
                            let py = scaled_y + y;

                            // Only draw if within image bounds
                            if px >= 0
                                && px < image.width() as i32
                                && py >= 0
                                && py < image.height() as i32
                            {
                                image.put_pixel(px as u32, py as u32, color);
                            }
                        }
                    }
                }
            }

            let image = image::DynamicImage::ImageRgba8(image);

            let image = image::imageops::resize(
                &image,
                image.width() / 2,
                image.height() / 2,
                image::imageops::FilterType::CatmullRom,
            );
            let size = image.dimensions();
            let image =
                egui::ColorImage::from_rgba_unmultiplied([size.0 as _, size.1 as _], &image);

            // Create and cache the texture
            let texture = ui
                .ctx()
                .load_texture(texture_id, image, egui::TextureOptions::default());
            cached_textures.insert(texture_id.to_string(), texture.clone());
            ui.image(&texture);
        }
    }
}
