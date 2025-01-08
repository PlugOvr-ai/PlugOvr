use egui::Context;

pub struct UseCaseRecorder {
    usecase_id: String,
    usecase_name: String,
    usecase_instructions: String,
    usecase_steps: Vec<EventType>,
    recording: bool,
    pub show: bool,
}
use egui::Vec2;
use image::{ImageBuffer, Rgba};
enum EventType {
    Click(Vec2),
    Key(String),
    Image(ImageBuffer<Rgba<u8>, Vec<u8>>),
}
impl UseCaseRecorder {
    pub fn new() -> Self {
        Self {
            usecase_id: String::new(),
            usecase_name: String::new(),
            usecase_instructions: String::new(),
            usecase_steps: Vec::new(),
            recording: false,
            show: false,
        }
    }
    pub fn show_window(&mut self, ctx: &Context) {
        if self.show {
            egui::Window::new("Use Case Recorder").show(ctx, |ui| {
                ui.label("Use Case Recorder");
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
    fn add_event(&mut self, event: EventType) {
        self.usecase_steps.push(event);
    }
    fn start_recording(&mut self) {
        self.recording = true;
    }
    fn stop_recording(&mut self) {
        self.recording = false;
    }
}
