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
                if let Some(path) = self.file_dialog.take_picked() {
                    self.picked_file = Some(path.to_path_buf());
                    if let Some(path) = &self.picked_file {
                        if let Ok(contents) = fs::read_to_string(path) {
                            if let Ok(usecase) = serde_json::from_str::<UseCase>(&contents) {
                                self.usecase = Some(usecase);
                                self.current_step = 0;
                                // Clear existing cached textures
                                self.cached_textures.clear();

                                // Pre-cache textures for all images in the usecase

                                for (step_index, step) in self
                                    .usecase
                                    .as_ref()
                                    .unwrap()
                                    .usecase_steps
                                    .iter()
                                    .enumerate()
                                {
                                    match step {
                                        EventType::Monitor1(data) => {
                                            if let Ok(image_data) = base64::decode(data) {
                                                if let Ok(image) =
                                                    image::load_from_memory(&image_data)
                                                {
                                                    display_step_image(
                                                        ui,
                                                        data,
                                                        &format!("image_{}", step_index),
                                                        (-1, -1),
                                                        &mut self.cached_textures,
                                                        false,
                                                        2.0,
                                                    );
                                                    display_step_image(
                                                        ui,
                                                        data,
                                                        &format!("image_thump_{}", step_index),
                                                        (-1, -1),
                                                        &mut self.cached_textures,
                                                        false,
                                                        8.0,
                                                    );
                                                }
                                            }
                                        }
                                        EventType::Monitor2(data) => {
                                            display_step_image(
                                                ui,
                                                data,
                                                &format!("image_{}", step_index),
                                                (-1, -1),
                                                &mut self.cached_textures,
                                                false,
                                                2.0,
                                            );
                                            display_step_image(
                                                ui,
                                                data,
                                                &format!("image_thump_{}", step_index),
                                                (-1, -1),
                                                &mut self.cached_textures,
                                                false,
                                                8.0,
                                            );
                                        }
                                        _ => {}
                                    }
                                }
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
                    self.file_dialog.pick_file();
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

            // Collect after images before match
            let (step, after_images) = current_and_after.split_first_mut().unwrap();
            let after_images: Vec<_> = after_images
                .iter()
                .enumerate()
                .filter_map(|(i, step)| {
                    if let EventType::Monitor1(data) = step {
                        Some((i + current_step + 1, data))
                    } else {
                        None
                    }
                })
                .collect();

            match step {
                EventType::Click(point, desc, ids) => {
                    ui.label(format!("Click coordinates: ({}, {})", point.x, point.y));
                    ui.label("Description:");
                    let mut desc_clone = desc.clone();
                    if ui.text_edit_singleline(&mut desc_clone).changed() {
                        *desc = desc_clone;
                    }
                    // Search backwards for the most recent Monitor1 event
                    if let Some((monitor_index, monitor_data)) =
                        prev_steps.iter().enumerate().rev().find_map(|(i, step)| {
                            if let EventType::Monitor1(data) = step {
                                Some((i, data))
                            } else {
                                None
                            }
                        })
                    {
                        let _ = display_step_image(
                            ui,
                            monitor_data,
                            &format!("image_{}", monitor_index),
                            (point.x as i32, point.y as i32),
                            &mut self.cached_textures,
                            true,
                            2.0,
                        );
                    }
                    let point = (point.x as i32, point.y as i32);

                    let mut before_images = prev_steps
                        .iter()
                        .enumerate()
                        .filter_map(|(i, step)| {
                            if let EventType::Monitor1(data) = step {
                                Some((i, data))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    let mut ids_tmp = ids.clone();
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let thumbnail_size = egui::Vec2::new(100.0, 60.0);

                            // Show before images
                            for (i, (monitor_index, data)) in before_images.iter().enumerate() {
                                let offset = -(before_images.len() as i32) + i as i32 + 1;
                                ui.vertical(|ui| {
                                    ui.label(format!("T{}", offset));
                                    // Add checkbox
                                    let is_selected = ids_tmp.contains(monitor_index);
                                    let mut checked = is_selected;
                                    if ui.checkbox(&mut checked, "").changed() {
                                        if checked {
                                            if !ids.contains(monitor_index) {
                                                ids.push(*monitor_index);
                                            }
                                        } else {
                                            if let Some(pos) =
                                                ids.iter().position(|x| x == monitor_index)
                                            {
                                                ids.remove(pos);
                                            }
                                        }
                                    }
                                    // Add hover functionality to thumbnail
                                    let response = display_step_image(
                                        ui,
                                        data,
                                        &format!("image_thump_{}", monitor_index),
                                        (point.0, point.1),
                                        &mut self.cached_textures,
                                        true,
                                        8.0,
                                    );

                                    // Show larger image on hover
                                    if response.hovered() {
                                        egui::Window::new(format!(
                                            "hover_preview_{}",
                                            monitor_index
                                        ))
                                        .fixed_pos(ui.input(|i| {
                                            let pos = i.pointer.hover_pos().unwrap_or_default();
                                            pos + egui::vec2(20.0, 20.0) // Offset from cursor
                                        }))
                                        .title_bar(false)
                                        .frame(egui::Frame::none())
                                        .auto_sized()
                                        .show(
                                            ui.ctx(),
                                            |ui| {
                                                display_step_image(
                                                    ui,
                                                    data,
                                                    &format!("image_{}", monitor_index),
                                                    (point.0, point.1),
                                                    &mut self.cached_textures,
                                                    true,
                                                    2.0,
                                                );
                                            },
                                        );
                                    }
                                });
                            }

                            // Show after images
                            for (i, (monitor_index, data)) in after_images.iter().enumerate() {
                                ui.vertical(|ui| {
                                    ui.label(format!("T+{}", i + 1));
                                    // Add checkbox
                                    let is_selected = ids.contains(monitor_index);
                                    let mut checked = is_selected;
                                    if ui.checkbox(&mut checked, "").changed() {
                                        if checked {
                                            if !ids.contains(monitor_index) {
                                                ids.push(*monitor_index);
                                            }
                                        } else {
                                            if let Some(pos) =
                                                ids.iter().position(|x| x == monitor_index)
                                            {
                                                ids.remove(pos);
                                            }
                                        }
                                    }
                                    // Add hover functionality to thumbnail
                                    let response = display_step_image(
                                        ui,
                                        data,
                                        &format!("image_thump_{}", monitor_index),
                                        (point.0, point.1),
                                        &mut self.cached_textures,
                                        true,
                                        8.0,
                                    );

                                    // Show larger image on hover
                                    if response.hovered() {
                                        egui::Window::new(format!(
                                            "hover_preview_{}",
                                            monitor_index
                                        ))
                                        .fixed_pos(ui.input(|i| {
                                            let pos = i.pointer.hover_pos().unwrap_or_default();
                                            pos + egui::vec2(20.0, 20.0) // Offset from cursor
                                        }))
                                        .title_bar(false)
                                        .frame(egui::Frame::none())
                                        .auto_sized()
                                        .show(
                                            ui.ctx(),
                                            |ui| {
                                                display_step_image(
                                                    ui,
                                                    data,
                                                    &format!("image_{}", monitor_index),
                                                    (point.0, point.1),
                                                    &mut self.cached_textures,
                                                    true,
                                                    2.0,
                                                );
                                            },
                                        );
                                    }
                                });
                            }
                        });
                    });
                }
                EventType::Monitor1(data) => {
                    ui.label("Monitor1");
                    display_step_image(
                        ui,
                        data,
                        &format!("image_{}", current_step),
                        (-1, -1),
                        &mut self.cached_textures,
                        true,
                        2.0,
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
                        true,
                        2.0,
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
                        true,
                        2.0,
                    );
                }
                EventType::Text(text) => {
                    ui.label("Text");
                    let mut text_clone = text.clone();
                    if ui.text_edit_singleline(&mut text_clone).changed() {
                        *text = text_clone;
                    }
                }
                EventType::KeyDown(key) => {
                    ui.label("KeyDown");
                    let mut key_clone = key.clone();
                    if ui.text_edit_singleline(&mut key_clone).changed() {
                        *key = key_clone;
                    }
                }
                EventType::KeyUp(key) => {
                    ui.label("KeyUp");
                    let mut key_clone = key.clone();
                    if ui.text_edit_singleline(&mut key_clone).changed() {
                        *key = key_clone;
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
fn display_step_image(
    ui: &mut egui::Ui,
    monitor_data: &str,
    texture_id: &str,
    coords: (i32, i32),
    cached_textures: &mut std::collections::HashMap<String, egui::TextureHandle>,
    show_image: bool,
    scale: f32,
) -> egui::Response {
    if let Some(texture) = cached_textures.get(texture_id) {
        if show_image {
            let before_rect = ui.cursor();
            let response = ui.add(egui::Image::new(texture));
            let after_rect = ui.cursor();
            // Draw circle at cursor position
            if coords.0 != -1 && coords.1 != -1 {
                // Scale click coordinates to match displayed image size
                let circle_x = before_rect.min.x + (coords.0 as f32 / scale);
                let circle_y = before_rect.min.y + (coords.1 as f32 / scale);

                let circle_pos = egui::pos2(circle_x, circle_y);
                let circle_radius = 5.0;

                // Draw red circle
                ui.painter()
                    .circle_filled(circle_pos, circle_radius, egui::Color32::RED);
            }
            return response;
        }
        return ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover());
    }

    if let Ok(image_data) = base64::decode(monitor_data) {
        if let Ok(image) = image::load_from_memory(&image_data) {
            // Draw a red circle at click coordinates
            let mut image = image.to_rgba8();
            // let radius = 10;
            // let color = image::Rgba([255, 0, 0, 255]); // Red circle

            // // Scale coords to match resized image dimensions
            // let scaled_x = coords.0;
            // let scaled_y = coords.1;
            // if scaled_x != -1 && scaled_y != -1 {
            //     // Draw circle by iterating over pixels in bounding box
            //     for y in -radius..=radius {
            //         for x in -radius..=radius {
            //             // Check if point is within circle using distance formula
            //             if x * x + y * y <= radius * radius {
            //                 let px = scaled_x + x;
            //                 let py = scaled_y + y;

            //                 // Only draw if within image bounds
            //                 if px >= 0
            //                     && px < image.width() as i32
            //                     && py >= 0
            //                     && py < image.height() as i32
            //                 {
            //                     image.put_pixel(px as u32, py as u32, color);
            //                 }
            //             }
            //         }
            //     }
            // }

            let image = image::DynamicImage::ImageRgba8(image);

            let image = image::imageops::resize(
                &image,
                image.width() / scale as u32,
                image.height() / scale as u32,
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
            if show_image {
                ui.image(&texture);
            }
        }
    }

    // Return a default response if we didn't show an image
    ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover())
}
