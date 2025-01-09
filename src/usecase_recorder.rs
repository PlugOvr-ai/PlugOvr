use egui::Context;
use std::time::{Duration, SystemTime};
use xcap::Monitor;
pub struct UseCaseRecorder {
    usecase: Option<UseCase>,
    usecase_name: String,
    usecase_instructions: String,
    pub recording: bool,
    pub show: bool,
    pub add_image: bool,
    pub add_image_delay: Option<Duration>,
    pub add_image_now: Option<SystemTime>,
}

use image::{ImageBuffer, Rgba};
use serde::{Deserialize, Serialize};
use std::fs::File;
#[derive(Debug, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
#[derive(Debug, Serialize, Deserialize)]
pub enum EventType {
    Click(Point),
    Key(String),
    Image(String),
}
#[derive(Debug, Serialize, Deserialize)]
struct UseCase {
    usecase_id: String,
    usecase_name: String,
    usecase_instructions: String,
    usecase_steps: Vec<EventType>,
}
impl UseCaseRecorder {
    pub fn new() -> Self {
        Self {
            usecase: None,
            usecase_name: String::new(),
            usecase_instructions: String::new(),
            recording: false,
            show: false,
            add_image: false,
            add_image_delay: None,
            add_image_now: None,
        }
    }
    pub fn show_window(&mut self, ctx: &Context) {
        if self.show {
            egui::Window::new("Use Case Recorder").show(ctx, |ui| {
                ui.label("Use Case Recorder");
                ui.add(egui::TextEdit::multiline(&mut self.usecase_name));
                ui.add(egui::TextEdit::multiline(&mut self.usecase_instructions));

                if ui.button("Record").clicked() {
                    self.start_recording();
                }
                if ui.button("Stop").clicked() {
                    self.stop_recording();
                }
            });
        }
    }
    pub fn image_buffer2base64(image_buffer: ImageBuffer<Rgba<u8>, Vec<u8>>) -> String {
        use base64::{engine::general_purpose, Engine as _};
        let mut buf = vec![];
        image_buffer
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        base64::engine::general_purpose::STANDARD.encode(&buf)
    }
    pub fn add_screenshot(&mut self) {
        println!("Adding screenshot");
        let monitors = Monitor::all().unwrap();

        for monitor in monitors {
            let image = monitor.capture_image().unwrap();
            let base64 = Self::image_buffer2base64(image);
            self.add_event(EventType::Image(base64));
        }
    }
    pub fn add_event(&mut self, event: EventType) {
        if let EventType::Image(ref base64) = event {
            println!("Adding image");
        } else {
            println!("Adding event: {:?}", event);
        }
        if let EventType::Key(ref key) = event {
            if key == "Escape" {
                self.stop_recording();
            }
            if key == "Enter" {
                //self.add_screenshot();
                self.add_image = true;
                let now = SystemTime::now();
                self.add_image_delay = Some(Duration::from_secs(1));
                self.add_image_now = Some(now);
            }
        }
        if let EventType::Click(ref point) = event {
            self.add_image = true;
            let now = SystemTime::now();
            self.add_image_delay = Some(Duration::from_secs(1));
            self.add_image_now = Some(now);
        }

        self.usecase.as_mut().unwrap().usecase_steps.push(event);
    }
    fn start_recording(&mut self) {
        self.usecase = Some(UseCase {
            usecase_id: uuid::Uuid::new_v4().to_string(),
            usecase_name: self.usecase_name.clone(),
            usecase_instructions: self.usecase_instructions.clone(),
            usecase_steps: Vec::new(),
        });
        println!("Starting recording");
        self.recording = true;
        self.show = false;
        self.add_image = true;
        let now = SystemTime::now();
        self.add_image_delay = Some(Duration::from_secs(0));
        self.add_image_now = Some(now);
        //std::thread::sleep(std::time::Duration::from_secs(1));
        //self.add_screenshot();
    }
    fn stop_recording(&mut self) {
        println!("Stopping recording");
        self.recording = false;
        //save recording to file
        let file_name = format!("{}.json", self.usecase_name);
        let file = File::create(file_name).unwrap();
        serde_json::to_writer_pretty(file, &self.usecase.as_ref().unwrap()).unwrap();
    }
}
