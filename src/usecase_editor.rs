use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use rfd;
use image::GenericImageView;
use crate::usecase_recorder::{EventType, UseCase, Point};

pub struct UsecaseEditor {
    usecase: Option<UseCase>,
    current_step: usize,
    file_path: Option<PathBuf>,
    show_file_dialog: bool,
}

impl Default for UsecaseEditor {
    fn default() -> Self {
        Self {
            usecase: None,
            current_step: 0,
            file_path: None,
            show_file_dialog: false,
        }
    }
}

impl UsecaseEditor {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        egui::Window::new("Usecase Editor")
            .default_size([800.0, 600.0])
            .show(ctx, |ui| {
                self.show_menu_bar(ui);
                self.show_content(ui);
            });
    }

    fn show_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            egui::menu::menu_button(ui, "File", |ui| {
                if ui.button("Open").clicked() {
                    self.show_file_dialog = true;
                }
                if ui.button("Save").clicked() {
                    if let Some(usecase) = &self.usecase {
                        if let Some(path) = &self.file_path {
                            if let Ok(json) = serde_json::to_string_pretty(usecase) {
                                let _ = fs::write(path, json);
                            }
                        }
                    }
                }
            });
        });

        if self.show_file_dialog {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .pick_file()
            {
                self.file_path = Some(path.clone());
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(usecase) = serde_json::from_str(&content) {
                        self.usecase = Some(usecase);
                        self.current_step = 0;
                    }
                }
                self.show_file_dialog = false;
            }
        }
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
                ui.label(format!("Step {} of {}", 
                    self.current_step + 1, 
                    usecase.usecase_steps.len()));
                if ui.button("Next").clicked() && self.current_step < usecase.usecase_steps.len() - 1 {
                    self.current_step += 1;
                }
            });

            ui.add_space(10.0);

            // Show current step
            let current_step = self.current_step;
            let step = &usecase.usecase_steps[current_step];
            let pretty = serde_json::to_string_pretty(&step).unwrap_or_default();
            let mut text = pretty.clone();
            if ui.text_edit_multiline(&mut text).changed() {
                // Handle text changes if needed
            }

            // Handle click events and monitor data
            match step {
                EventType::Click(point, _) => {
                    if let Some(prev_step) = usecase.usecase_steps.get(current_step.saturating_sub(1)) {
                        if let EventType::Monitor1(monitor_data) = prev_step {
                            ui.label(format!("Click coordinates: ({}, {})", point.x, point.y));
                            display_step_image(ui, monitor_data, "prev_step_image", (point.x as i32, point.y as i32));
                        }
                    }
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
fn display_step_image( ui: &mut egui::Ui, monitor_data: &str, texture_id: &str, coords: (i32, i32)) {
    if let Ok(image_data) = base64::decode(monitor_data) {
        if let Ok(image) = image::load_from_memory(&image_data) {
            // Draw a red circle at click coordinates
            let mut image = image.to_rgba8();
            let radius = 10;
            let color = image::Rgba([255, 0, 0, 255]); // Red circle
            
            // Scale coords to match resized image dimensions
            let scaled_x = coords.0 / 2;
            let scaled_y = coords.1 / 2;

            // Draw circle by iterating over pixels in bounding box
            for y in -radius..=radius {
                for x in -radius..=radius {
                    // Check if point is within circle using distance formula
                    if x*x + y*y <= radius*radius {
                        let px = scaled_x + x;
                        let py = scaled_y + y;
                        
                        // Only draw if within image bounds
                        if px >= 0 && px < image.width() as i32 && 
                           py >= 0 && py < image.height() as i32 {
                            image.put_pixel(px as u32, py as u32, color);
                        }
                    }
                }
            }
            let image = image::DynamicImage::ImageRgba8(image);
            let size = image.dimensions();
            let image = image.resize(size.0 / 2, size.1 / 2, image::imageops::FilterType::CatmullRom);
            let size = image.dimensions();
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [size.0 as _, size.1 as _],
                &image.to_rgba8(),
            );
            
            let texture = ui.ctx().load_texture(
                texture_id,
                image,
                egui::TextureOptions::default(),
            );
            ui.image(&texture);
        }
    }
}