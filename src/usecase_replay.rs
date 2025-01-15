use crate::usecase_recorder::{EventType, UseCase};

use egui_overlay::egui_render_three_d::{
    three_d::{self, ColorMaterial, Gm, Mesh},
    ThreeDBackend,
};
use gtk::false_;
use image::{ImageBuffer, Rgba};
use rdev::{simulate, Button};
use regex;
use reqwest::multipart;
use std::fs::File;
use std::io::Cursor;
use std::thread;
use std::time;
use xcap::Monitor;
pub struct UseCaseReplay {
    pub index: usize,
    pub usecase_actions: Option<UseCaseActions>,
    pub recorded_usecases: Vec<UseCase>,
    pub monitor1: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub monitor2: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub monitor3: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub show: bool,
    pub click_position: Option<(f32, f32)>,
    pub model: Option<Gm<Mesh, ColorMaterial>>,
}

#[derive(Clone, Debug)]
enum ActionTypes {
    Click(String),
    ClickPosition(f32, f32),
    InsertText(String),
    KeyDown(String),
    KeyUp(String),
    GrabScreenshot,
}
struct UseCaseActions {
    pub instruction: String,
    pub actions: Vec<ActionTypes>,
}
impl UseCaseReplay {
    pub fn new() -> Self {
        Self {
            index: 0,
            usecase_actions: None,
            recorded_usecases: vec![],
            monitor1: None,
            monitor2: None,
            monitor3: None,
            show: false,
            click_position: None,
            model: None,
        }
    }
    pub fn load_usecase(&mut self, filename: String) {
        let file = File::open(filename).unwrap();
        let usecase: UseCase = serde_json::from_reader(file).unwrap();
        self.recorded_usecases.push(usecase);
    }
    pub fn identify_usecase(&mut self, instruction: &String) -> usize {
        //find the usecase that has the most similar instruction
        0
    }
    pub fn create_usecase_actions(&mut self, index: usize, instruction: &String) {
        let mut actions = UseCaseActions {
            instruction: instruction.clone(),
            actions: vec![],
        };
        for event in self.recorded_usecases[index].usecase_steps.iter() {
            match event {
                EventType::Monitor1(_) => {
                    actions.actions.push(ActionTypes::GrabScreenshot);
                }
                EventType::Click(_, instruction) => {
                    actions
                        .actions
                        .push(ActionTypes::Click(instruction.clone()));
                    actions.actions.push(ActionTypes::ClickPosition(0.0, 0.0));
                }
                EventType::KeyDown(instruction) => {
                    actions
                        .actions
                        .push(ActionTypes::KeyDown(instruction.clone()));
                }
                EventType::KeyUp(instruction) => {
                    actions
                        .actions
                        .push(ActionTypes::KeyUp(instruction.clone()));
                }
                EventType::Text(instruction) => {
                    actions
                        .actions
                        .push(ActionTypes::InsertText(instruction.clone()));
                }
                _ => {}
            }
        }
        self.usecase_actions = Some(actions);
    }
    pub fn execute_usecase(&mut self, instruction: String) {
        let index = self.identify_usecase(&instruction);
        self.create_usecase_actions(index, &instruction);
        self.show = true;
    }
    pub fn grab_screenshot(&mut self) {
        println!("grab_screenshot");
        let monitors = Monitor::all().unwrap();
        for (i, monitor) in monitors.iter().enumerate() {
            let image: ImageBuffer<Rgba<u8>, Vec<u8>> = monitor.capture_image().unwrap();
            if i == 0 {
                self.monitor1 = Some(image);
            } else if i == 1 {
                self.monitor2 = Some(image);
            } else if i == 2 {
                self.monitor3 = Some(image);
            }
        }
    }
    pub fn click(&mut self, instruction: String) {
        println!("click: {}", instruction);
        let client = reqwest::blocking::Client::new();
        // Encode the image directly into the buffer
        let mut buffer = Vec::new();
        self.monitor1
            .as_ref()
            .unwrap()
            .write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let image_part = reqwest::blocking::multipart::Part::bytes(buffer)
            .file_name("image.png")
            .mime_str("image/png")
            .unwrap();

        // Add instruction as a text part
        let instruction_part = reqwest::blocking::multipart::Part::text(instruction);

        let form = reqwest::blocking::multipart::Form::new()
            .part("image", image_part)
            .part("prompt", instruction_part);

        // Send the POST request
        let res = client
            .post("http://192.168.1.106:5001/process-image")
            .multipart(form)
            .send()
            .unwrap();

        // Parse the response text
        let response_text = res.text().unwrap();
        if let Some(coords) = parse_coordinates(&response_text) {
            let (x1, y1, x2, y2) = coords;

            // Get image dimensions
            let width = self.monitor1.as_ref().unwrap().width() as f32;
            let height = self.monitor1.as_ref().unwrap().height() as f32;

            // Calculate center point and scale coordinates
            let center_x = (x1 + x2) / 2.0 * width;
            let center_y = (y1 + y2) / 2.0 * height;

            self.click_position = Some((center_x, center_y));
            self.usecase_actions.as_mut().unwrap().actions[self.index + 1] =
                ActionTypes::ClickPosition(center_x, center_y);

            println!("Click position: {:?}", self.click_position);
        }
    }
    pub fn step(&mut self) {
        if self.index >= self.usecase_actions.as_ref().unwrap().actions.len() {
            return;
        }
        if self.usecase_actions.is_none() {
            return;
        }
        let action = self.usecase_actions.as_ref().unwrap().actions[self.index].clone();
        match action {
            ActionTypes::Click(instruction) => {
                self.click(instruction);
            }
            ActionTypes::ClickPosition(x, y) => {
                println!("click_position: {:?}", (x, y));
                mouse_click(x, y);
            }
            ActionTypes::InsertText(text) => {
                println!("insert_text: {}", text);
                text_input(&text);
            }
            ActionTypes::KeyDown(instruction) => {
                println!("key_down: {}", instruction);
                key_down(&instruction);
            }
            ActionTypes::KeyUp(instruction) => {
                println!("key_up: {}", instruction);
                key_up(&instruction);
            }
            ActionTypes::GrabScreenshot => self.grab_screenshot(),
        }
        self.index += 1;
        if let Some(actions) = &self.usecase_actions {
            if let ActionTypes::KeyUp(key) = &actions.actions[self.index] {
                self.step();
            }
        }
    }

    fn draw_circle(ui: &mut egui::Ui, position: (f32, f32)) {
        if position.1 > 1040.0 {
            ui.painter().arrow(
                egui::pos2(position.0, 1040.0 - 50.0),
                egui::vec2(0.0, 50.0),
                egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 0, 0)),
            );
        } else {
            ui.painter().circle_filled(
                //egui::pos2(position.0, position.1 - 1.0),
                egui::pos2(position.0, position.1),
                10.0,
                egui::Color32::from_rgb(255, 0, 0),
            );
        }
    }
    pub fn vizualize_next_step_3d(
        &mut self,
        egui_context: &egui::Context,
        three_d_backend: &mut ThreeDBackend,
        glfw_backend: &mut egui_overlay::egui_window_glfw_passthrough::GlfwBackend,
    ) {
        self.model
            .get_or_insert_with(|| create_triangle_model(&three_d_backend.context));

        if let Some(model) = &mut self.model {
            // Create a camera
            let camera = three_d::Camera::new_perspective(
                egui_overlay::egui_render_three_d::three_d::Viewport::new_at_origo(
                    glfw_backend.framebuffer_size_physical[0],
                    glfw_backend.framebuffer_size_physical[1],
                ),
                egui_overlay::egui_render_three_d::three_d::vec3(0.0, 0.0, 2.0),
                egui_overlay::egui_render_three_d::three_d::vec3(0.0, 0.0, 0.0),
                egui_overlay::egui_render_three_d::three_d::vec3(0.0, 1.0, 0.0),
                egui_overlay::egui_render_three_d::three_d::degrees(15.0),
                0.1,
                10.0,
            );
            // Update the animation of the triangle
            // model.animate(glfw_backend.glfw.get_time() as _);

            // Get the screen render target to be able to render something on the screen
            egui_overlay::egui_render_three_d::three_d::RenderTarget::<'_>::screen(
                &three_d_backend.context,
                glfw_backend.framebuffer_size_physical[0],
                glfw_backend.framebuffer_size_physical[1],
            )
            // Clear the color and depth of the screen render target. use transparent color.
            .clear(
                egui_overlay::egui_render_three_d::three_d::ClearState::color_and_depth(
                    0.0, 0.0, 0.0, 0.0, 1.0,
                ),
            )
            // Render the triangle with the color material which uses the per vertex colors defined at construction
            .render(&camera, std::iter::once(model), &[]);
        }

        egui::Window::new("Overlay")
            .interactable(false)
            .title_bar(false)
            .default_pos(egui::Pos2::new(1.0, 1.0))
            .min_size(egui::Vec2::new(1920.0 - 2.0, 1080.0 - 2.0))
            //.frame(egui::Frame {
            //     fill: egui::Color32::TRANSPARENT,
            //     ..Default::default()
            // })
            .show(egui_context, |ui| {
                egui::Area::new(egui::Id::new("overlay"))
                    .fixed_pos(egui::pos2(0.0, 0.0))
                    .show(egui_context, |ui| {
                        let action =
                            self.usecase_actions.as_mut().unwrap().actions[self.index].clone();
                        ui.add_sized(
                            egui::Vec2::new(400.0, 30.0),
                            egui::Label::new(egui::RichText::new(format!(
                                "PlugOvr: next action: {:?}",
                                action
                            ))),
                        );
                    });
                if let Some(click_position) = self.click_position {
                    Self::draw_circle(ui, click_position);
                }
            });
    }
    pub fn vizualize_next_step(
        &mut self,
        egui_context: &egui::Context,
        three_d_backend: &mut ThreeDBackend,
        glfw_backend: &mut egui_overlay::egui_window_glfw_passthrough::GlfwBackend,
    ) {
        egui::Window::new("Overlay")
            .interactable(false)
            .title_bar(false)
            .default_pos(egui::Pos2::new(1.0, 1.0))
            .min_size(egui::Vec2::new(1920.0 - 2.0, 1080.0 - 2.0))
            //.frame(egui::Frame {
            //     fill: egui::Color32::TRANSPARENT,
            //     ..Default::default()
            // })
            .show(egui_context, |ui| {
                egui::Area::new(egui::Id::new("overlay"))
                    .fixed_pos(egui::pos2(0.0, 0.0))
                    .show(egui_context, |ui| {
                        let action =
                            self.usecase_actions.as_mut().unwrap().actions[self.index].clone();
                        ui.add_sized(
                            egui::Vec2::new(400.0, 30.0),
                            egui::Label::new(egui::RichText::new(format!(
                                "PlugOvr: next action: {:?}",
                                action
                            ))),
                        );
                    });
                if let ActionTypes::ClickPosition(x, y) =
                    self.usecase_actions.as_mut().unwrap().actions[self.index]
                {
                    Self::draw_circle(ui, (x, y));
                }
            });
    }
}

fn create_triangle_model(three_d_context: &three_d::Context) -> Gm<Mesh, ColorMaterial> {
    use three_d::*;

    // Create a CPU-side mesh consisting of a single colored triangle
    let positions = vec![
        vec3(0.5, -0.5, 0.0),  // bottom right
        vec3(-0.5, -0.5, 0.0), // bottom left
        vec3(0.0, 0.5, 0.0),   // top
    ];
    let colors = vec![
        Srgba::RED,   // bottom right
        Srgba::GREEN, // bottom left
        Srgba::BLUE,  // top
    ];
    let cpu_mesh = CpuMesh {
        positions: Positions::F32(positions),
        colors: Some(colors),
        ..Default::default()
    };

    // Construct a model, with a default color material, thereby transferring the mesh data to the GPU
    let mut model = Gm::new(
        Mesh::new(three_d_context, &cpu_mesh),
        ColorMaterial::default(),
    );

    // Add an animation to the triangle.
    model.set_animation(|time| Mat4::from_angle_y(radians(time * 0.005)));
    model
}
// Add this helper function
fn parse_coordinates(response: &str) -> Option<(f32, f32, f32, f32)> {
    let re = regex::Regex::new(r"<loc_(\d+)>").unwrap();
    let coords: Vec<f32> = re
        .captures_iter(response)
        .map(|cap| cap[1].parse::<f32>().unwrap() / 1000.0)
        .collect();

    if coords.len() == 4 {
        Some((coords[0], coords[1], coords[2], coords[3]))
    } else {
        None
    }
}
#[cfg(not(target_os = "macos"))]
fn mouse_click(x: f32, y: f32) {
    simulate(&rdev::EventType::MouseMove {
        x: x as f64,
        y: y as f64,
    })
    .unwrap();
    thread::sleep(time::Duration::from_millis(40));
    simulate(&rdev::EventType::ButtonPress(rdev::Button::Left)).unwrap();
    thread::sleep(time::Duration::from_millis(40));
    simulate(&rdev::EventType::ButtonRelease(rdev::Button::Left)).unwrap();
}

#[cfg(not(target_os = "macos"))]
fn text_input(text: &str) {
    arboard::Clipboard::new().unwrap().set_text(text).unwrap();
    thread::sleep(time::Duration::from_millis(40));
    simulate(&rdev::EventType::KeyPress(rdev::Key::ControlLeft)).unwrap();
    thread::sleep(time::Duration::from_millis(40));
    simulate(&rdev::EventType::KeyPress(rdev::Key::KeyV)).unwrap();

    thread::sleep(time::Duration::from_millis(40));
    simulate(&rdev::EventType::KeyRelease(rdev::Key::KeyV)).unwrap();
    thread::sleep(time::Duration::from_millis(40));
    simulate(&rdev::EventType::KeyRelease(rdev::Key::ControlLeft)).unwrap();
}
fn from_str(key: &str) -> rdev::Key {
    match key {
        "Alt" => rdev::Key::Alt,
        "AltGr" => rdev::Key::AltGr,
        "Backspace" => rdev::Key::Backspace,
        "CapsLock" => rdev::Key::CapsLock,
        "ControlLeft" => rdev::Key::ControlLeft,
        "ControlRight" => rdev::Key::ControlRight,
        "Delete" => rdev::Key::Delete,
        "DownArrow" => rdev::Key::DownArrow,
        "End" => rdev::Key::End,
        "Escape" => rdev::Key::Escape,
        "F1" => rdev::Key::F1,
        "F10" => rdev::Key::F10,
        "F11" => rdev::Key::F11,
        "F12" => rdev::Key::F12,
        "F2" => rdev::Key::F2,
        "F3" => rdev::Key::F3,
        "F4" => rdev::Key::F4,
        "F5" => rdev::Key::F5,
        "F6" => rdev::Key::F6,
        "F7" => rdev::Key::F7,
        "F8" => rdev::Key::F8,
        "F9" => rdev::Key::F9,
        "Home" => rdev::Key::Home,
        "LeftArrow" => rdev::Key::LeftArrow,
        /// also known as "windows", "super", and "command"
        "MetaLeft" => rdev::Key::MetaLeft,
        /// also known as "windows", "super", and "command"
        "MetaRight" => rdev::Key::MetaRight,
        "PageDown" => rdev::Key::PageDown,
        "PageUp" => rdev::Key::PageUp,
        "Return" => rdev::Key::Return,
        "RightArrow" => rdev::Key::RightArrow,
        "ShiftLeft" => rdev::Key::ShiftLeft,
        "ShiftRight" => rdev::Key::ShiftRight,
        "Space" => rdev::Key::Space,
        "Tab" => rdev::Key::Tab,
        "UpArrow" => rdev::Key::UpArrow,
        "PrintScreen" => rdev::Key::PrintScreen,
        "ScrollLock" => rdev::Key::ScrollLock,
        "Pause" => rdev::Key::Pause,
        "NumLock" => rdev::Key::NumLock,
        "BackQuote" => rdev::Key::BackQuote,
        "Num1" => rdev::Key::Num1,
        "Num2" => rdev::Key::Num2,
        "Num3" => rdev::Key::Num3,
        "Num4" => rdev::Key::Num4,
        "Num5" => rdev::Key::Num5,
        "Num6" => rdev::Key::Num6,
        "Num7" => rdev::Key::Num7,
        "Num8" => rdev::Key::Num8,
        "Num9" => rdev::Key::Num9,
        "Num0" => rdev::Key::Num0,
        "Minus" => rdev::Key::Minus,
        "Equal" => rdev::Key::Equal,
        "KeyQ" => rdev::Key::KeyQ,
        "KeyW" => rdev::Key::KeyW,
        "KeyE" => rdev::Key::KeyE,
        "KeyR" => rdev::Key::KeyR,
        "KeyT" => rdev::Key::KeyT,
        "KeyY" => rdev::Key::KeyY,
        "KeyU" => rdev::Key::KeyU,
        "KeyI" => rdev::Key::KeyI,
        "KeyO" => rdev::Key::KeyO,
        "KeyP" => rdev::Key::KeyP,
        "LeftBracket" => rdev::Key::LeftBracket,
        "RightBracket" => rdev::Key::RightBracket,
        "KeyA" => rdev::Key::KeyA,
        "KeyS" => rdev::Key::KeyS,
        "KeyD" => rdev::Key::KeyD,
        "KeyF" => rdev::Key::KeyF,
        "KeyG" => rdev::Key::KeyG,
        "KeyH" => rdev::Key::KeyH,
        "KeyJ" => rdev::Key::KeyJ,
        "KeyK" => rdev::Key::KeyK,
        "KeyL" => rdev::Key::KeyL,
        "SemiColon" => rdev::Key::SemiColon,
        "Quote" => rdev::Key::Quote,
        "BackSlash" => rdev::Key::BackSlash,
        "IntlBackslash" => rdev::Key::IntlBackslash,
        "KeyZ" => rdev::Key::KeyZ,
        "KeyX" => rdev::Key::KeyX,
        "KeyC" => rdev::Key::KeyC,
        "KeyV" => rdev::Key::KeyV,
        "KeyB" => rdev::Key::KeyB,
        "KeyN" => rdev::Key::KeyN,
        "KeyM" => rdev::Key::KeyM,
        "Comma" => rdev::Key::Comma,
        "Dot" => rdev::Key::Dot,
        "Slash" => rdev::Key::Slash,
        "Insert" => rdev::Key::Insert,
        "KpReturn" => rdev::Key::KpReturn,
        "KpMinus" => rdev::Key::KpMinus,
        "KpPlus" => rdev::Key::KpPlus,
        "KpMultiply" => rdev::Key::KpMultiply,
        "KpDivide" => rdev::Key::KpDivide,
        "Kp0" => rdev::Key::Kp0,
        "Kp1" => rdev::Key::Kp1,
        "Kp2" => rdev::Key::Kp2,
        "Kp3" => rdev::Key::Kp3,
        "Kp4" => rdev::Key::Kp4,
        "Kp5" => rdev::Key::Kp5,
        "Kp6" => rdev::Key::Kp6,
        "Kp7" => rdev::Key::Kp7,
        "Kp8" => rdev::Key::Kp8,
        "Kp9" => rdev::Key::Kp9,
        "KpDelete" => rdev::Key::KpDelete,
        "Function" => rdev::Key::Function,
        "Unknown" => rdev::Key::Unknown(0),
        _ => rdev::Key::Unknown(0),
    }
}
fn key_down(key: &str) {
    simulate(&rdev::EventType::KeyPress(from_str(key))).unwrap();

    thread::sleep(time::Duration::from_millis(40));
}
fn key_up(key: &str) {
    simulate(&rdev::EventType::KeyRelease(from_str(key))).unwrap();
    thread::sleep(time::Duration::from_millis(40));
}
