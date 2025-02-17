use egui::Context;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};
use xcap::Monitor;
pub struct UseCaseRecorder {
    usecase: Option<UseCase>,
    usecase_name: String,
    usecase_instructions: String,
    pub recording: Arc<Mutex<bool>>,
    pub show: bool,
    pub add_image: bool,
    pub add_image_delay: Option<Duration>,
    pub add_image_now: Option<SystemTime>,
    pub screenshot_buffer1: Arc<Mutex<Option<String>>>,
    pub screenshot_buffer2: Arc<Mutex<Option<String>>>,
    pub screenshot_buffer3: Arc<Mutex<Option<String>>>,
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
    Click(Point, String),
    KeyDown(String),
    KeyUp(String),
    Monitor1(String),
    Monitor2(String),
    Monitor3(String),
    Text(String),
}
#[derive(Debug, Serialize, Deserialize)]
pub struct UseCase {
    pub usecase_id: String,
    pub usecase_name: String,
    pub usecase_instructions: String,
    pub usecase_steps: Vec<EventType>,
}
pub fn buffer_screenshots(
    screenshot_buffer1: Arc<Mutex<Option<String>>>,
    screenshot_buffer2: Arc<Mutex<Option<String>>>,
    screenshot_buffer3: Arc<Mutex<Option<String>>>,
    recording: Arc<Mutex<bool>>,
) {
    let monitors = Monitor::all().unwrap();
    while *recording.lock().unwrap() {
        for (i, monitor) in monitors.iter().enumerate() {
            let image: ImageBuffer<Rgba<u8>, Vec<u8>> = monitor.capture_image().unwrap();
            // Resize image to half size
            // #[cfg(target_os = "macos")]
            // let image = image::imageops::resize(
            //     &image,
            //     image.width() / 2,
            //     image.height() / 2,
            //     image::imageops::FilterType::Lanczos3,
            // );
            let base64 = UseCaseRecorder::image_buffer2base64(image);
            if i == 0 {
                screenshot_buffer1.lock().unwrap().replace(base64);
            } else if i == 1 {
                screenshot_buffer2.lock().unwrap().replace(base64);
            } else if i == 2 {
                screenshot_buffer3.lock().unwrap().replace(base64);
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
}

impl UseCaseRecorder {
    pub fn new() -> Self {
        let recording = Arc::new(Mutex::new(false));
        let instance = Self {
            usecase: None,
            usecase_name: String::new(),
            usecase_instructions: String::new(),
            recording: recording.clone(),
            show: false,
            add_image: false,
            add_image_delay: None,
            add_image_now: None,
            screenshot_buffer1: Arc::new(Mutex::new(None)),
            screenshot_buffer2: Arc::new(Mutex::new(None)),
            screenshot_buffer3: Arc::new(Mutex::new(None)),
        };

        instance
    }
    pub fn show_window(&mut self, ctx: &Context) {
        if self.show {
            let mut show = self.show;
            egui::Window::new("Use Case Recorder").open(&mut show).show(ctx, |ui| {
                ui.label("Use Case Recorder");
                ui.label("Filename");
                ui.add(egui::TextEdit::multiline(&mut self.usecase_name));
                ui.label("Instructions");
                ui.add(egui::TextEdit::multiline(&mut self.usecase_instructions));

                if ui.button("Record").clicked() {
                    self.start_recording();
                    self.show = false;
                }
                if ui.button("Stop").clicked() {
                    self.stop_recording();
                }
            });
            if self.show {
                self.show = show;
            }
        }
    }
    pub fn image_buffer2base64(image_buffer: ImageBuffer<Rgba<u8>, Vec<u8>>) -> String {
        use base64::Engine as _;
        let mut buf = vec![];
        #[cfg(target_os = "macos")]
        let image_buffer = image::imageops::resize(
            &image_buffer, 
            image_buffer.width() / 2,
            image_buffer.height() / 2,
            image::imageops::FilterType::Triangle
        );
        
        image_buffer
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .unwrap();
        base64::engine::general_purpose::STANDARD.encode(&buf)
    }

    pub fn add_screenshot(&mut self) {
        println!("Adding screenshot");
        let monitors = Monitor::all().unwrap();

        for (i, monitor) in monitors.iter().enumerate() {
            let image = monitor.capture_image().unwrap();
            let base64 = Self::image_buffer2base64(image);
            if i == 0 {
                self.add_event(EventType::Monitor1(base64));
            } else if i == 1 {
                self.add_event(EventType::Monitor2(base64));
            } else if i == 2 {
                self.add_event(EventType::Monitor3(base64));
            }
        }
    }
    pub fn add_event(&mut self, event: EventType) {
        if let EventType::Monitor1(ref _base64) = event {
            println!("Adding monitor1 image");
        } else if let EventType::Monitor2(ref _base64) = event {
            println!("Adding monitor2 image");
        } else if let EventType::Monitor3(ref _base64) = event {
            println!("Adding monitor3 image");
        } else {
            println!("Adding event: {:?}", event);
        }
        if let EventType::KeyDown(ref key) = event {
            if key == "Escape" {
                self.stop_recording();
            }
            if key == "Enter" {
                //self.add_screenshot();
                //self.add_image = true;
                //let now = SystemTime::now();
                //self.add_image_delay = Some(Duration::from_secs(1));
                //self.add_image_now = Some(now);
            }
        }
        if let EventType::Click(ref _point, ref _op) = event {
            //self.add_image = true;
            //let now = SystemTime::now();
            //self.add_image_delay = Some(Duration::from_secs(1));
            //self.add_image_now = Some(now);
            let screenshot1 = self.screenshot_buffer1.lock().unwrap().clone();
            if let Some(screenshot) = screenshot1 {
                self.add_event(EventType::Monitor1(screenshot));
            }
            let screenshot2 = self.screenshot_buffer2.lock().unwrap().clone();
            if let Some(screenshot) = screenshot2 {
                self.add_event(EventType::Monitor2(screenshot));
            }
            let screenshot3 = self.screenshot_buffer3.lock().unwrap().clone();
            if let Some(screenshot) = screenshot3 {
                self.add_event(EventType::Monitor3(screenshot));
            }
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
        *self.recording.lock().unwrap() = true;
        self.show = false;

        let screenshot_buffer1 = self.screenshot_buffer1.clone();
        let screenshot_buffer2 = self.screenshot_buffer2.clone();
        let screenshot_buffer3 = self.screenshot_buffer3.clone();
        let recording = self.recording.clone();
        thread::spawn(move || {
            buffer_screenshots(
                screenshot_buffer1,
                screenshot_buffer2,
                screenshot_buffer3,
                recording,
            );
        });

        // let screenshot1 = self.screenshot_buffer1.lock().unwrap().clone();
        // if let Some(screenshot) = screenshot1 {
        //     self.add_event(EventType::Monitor1(screenshot));
        // }
        // let screenshot2 = self.screenshot_buffer2.lock().unwrap().clone();
        // if let Some(screenshot) = screenshot2 {
        //     self.add_event(EventType::Monitor2(screenshot));
        // }
        // let screenshot3 = self.screenshot_buffer3.lock().unwrap().clone();
        // if let Some(screenshot) = screenshot3 {
        //     self.add_event(EventType::Monitor3(screenshot));
        // }
        // self.add_image = true;
        // let now = SystemTime::now();
        // self.add_image_delay = Some(Duration::from_secs(0));
        // self.add_image_now = Some(now);
        // //std::thread::sleep(std::time::Duration::from_secs(1));
        //self.add_screenshot();
    }

    fn stop_recording(&mut self) {
        println!("Stopping recording");
        *self.recording.lock().unwrap() = false;

        // Compress keyboard events before saving
        if let Some(usecase) = &mut self.usecase {
            let mut compressed_steps = Vec::new();
            let mut keyboard_events = Vec::new();

            // Process all events
            for event in usecase.usecase_steps.drain(..) {
                match event {
                    EventType::KeyDown(ref key) | EventType::KeyUp(ref key) if key != "Return" => {
                        keyboard_events.push(event);
                    }
                    other_event => {
                        // If we have pending keyboard events, compress them first
                        if !keyboard_events.is_empty() {
                            let text = Self::compress_keyboard_events(&keyboard_events);
                            if !text.is_empty() {
                                compressed_steps.push(EventType::Text(text));
                            }
                            keyboard_events.clear();
                        }
                        compressed_steps.push(other_event);
                    }
                }
            }

            // Handle any remaining keyboard events
            if !keyboard_events.is_empty() {
                let text = Self::compress_keyboard_events(&keyboard_events);
                if !text.is_empty() {
                    compressed_steps.push(EventType::Text(text));
                }
            }

            usecase.usecase_steps = compressed_steps;
        }

        // Save recording to file
        let file_name = format!("{}.json", self.usecase_name);
        let file = File::create(file_name).unwrap();
        serde_json::to_writer_pretty(file, &self.usecase.as_ref().unwrap()).unwrap();
    }

    pub fn compress_keyboard_events(events: &[EventType]) -> String {
        let mut result = String::new();
        let mut shift_pressed = false;
        let mut altgr_pressed = false;

        for event in events {
            match event {
                EventType::KeyDown(key) => match key.as_str() {
                    "ShiftLeft" => shift_pressed = true,
                    "AltGr" => altgr_pressed = true,
                    "Space" => result.push(' '),
                    "Backspace" => {
                        result.pop(); // Remove the last character if any
                    }
                    key => {
                        if altgr_pressed && key == "Q" {
                            result.push('@');
                        } else {
                            let mut c = key.chars().next().unwrap_or_default();
                            if shift_pressed {
                                c = c.to_uppercase().next().unwrap_or(c);
                            } else {
                                c = c.to_lowercase().next().unwrap_or(c);
                            }
                            if shift_pressed {
                                if c == '.' {
                                    c = ':';
                                }
                                if c == ',' {
                                    c = ';';
                                }
                                // if c == '/' {
                                //     c = '?';
                                // }
                                // if c == '-' {
                                //     c = '_';
                                // }
                                // if c == '=' {
                                //     c = '+';
                                // }
                                // if c == '[' {
                                //     c = '{';
                                // }
                                
                                
                            }
                            result.push(c);
                        }
                    }
                },
                EventType::KeyUp(key) => match key.as_str() {
                    "ShiftLeft" => shift_pressed = false,
                    "AltGr" => altgr_pressed = false,
                    _ => {}
                },
                _ => {}
            }
        }

        result
    }
}
