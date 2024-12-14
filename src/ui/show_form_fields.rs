use crate::ui::answer_analyser::FormFields;

//#[cfg(any(target_os = "linux", target_os = "windows"))]
//use enigo::Direction;
//#[cfg(any(target_os = "linux", target_os = "windows"))]
//use enigo::Mouse;
//#[cfg(target_os = "macos")]
//use enigo::{Enigo, Key, Keyboard, Settings, };
//#[cfg(any(target_os = "linux", target_os = "windows"))]
use rdev::{simulate, Button, EventType};
use std::collections::HashSet;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::{thread, time};
#[cfg(target_os = "macos")]
fn send_cmd_v() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events" to keystroke "v" using command down"#)
        .output()?;

    Ok(())
}
#[cfg(target_os = "macos")]
fn send_fill_command(
    x: i32,
    y: i32,
    _mouse_position_x: i32,
    _mouse_position_y: i32,
    field_value: &str,
) -> Result<(), Box<dyn Error>> {
    // Set clipboard content
    arboard::Clipboard::new()?.set_text(field_value)?;

    thread::spawn(move || {
        simulate(&EventType::MouseMove {
            x: x as f64,
            y: y as f64,
        })
        .unwrap();
        let delay = time::Duration::from_millis(40);
        thread::sleep(time::Duration::from_millis(40));
        simulate(&EventType::ButtonPress(Button::Left)).unwrap();
        thread::sleep(delay);
        simulate(&EventType::ButtonRelease(Button::Left)).unwrap();

        thread::sleep(time::Duration::from_millis(40));
        simulate(&EventType::ButtonPress(Button::Left)).unwrap();
        thread::sleep(delay);
        simulate(&EventType::ButtonRelease(Button::Left)).unwrap();

        thread::sleep(time::Duration::from_millis(40));
        simulate(&EventType::ButtonPress(Button::Left)).unwrap();
        thread::sleep(delay);
        simulate(&EventType::ButtonRelease(Button::Left)).unwrap();

        // Send Command+V to paste the clipboard content (simulates the Cmd+V keyboard press)
        send_cmd_v().unwrap();
        thread::sleep(time::Duration::from_millis(40));
        arboard::Clipboard::new().unwrap().set_text("").unwrap();
    });
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn send_fill_command(
    x: i32,
    y: i32,
    mouse_position_x: i32,
    mouse_position_y: i32,
    field_value: &str,
) -> Result<(), Box<dyn Error>> {
    //fill clipboard with field_value
    arboard::Clipboard::new()?.set_text(field_value)?;
    //send mouse click to (x,y)

    // if let Err(e) = activate_window(active_window) {
    //    println!("Failed to activate window '{}': {:?}", active_window.0, e);
    //} else {
    //    println!("Activated window '{}'", active_window.0);
    //}
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    thread::spawn(move || {
        //println!("send_fill_command");
        use rdev::Key;
        let delay = time::Duration::from_millis(40);
        simulate(&EventType::MouseMove {
            x: x as f64,
            y: y as f64,
        })
        .unwrap();

        thread::sleep(time::Duration::from_millis(40));
        simulate(&EventType::ButtonPress(Button::Left)).unwrap();
        thread::sleep(delay);
        simulate(&EventType::ButtonRelease(Button::Left)).unwrap();

        thread::sleep(time::Duration::from_millis(40));
        simulate(&EventType::ButtonPress(Button::Left)).unwrap();
        thread::sleep(delay);
        simulate(&EventType::ButtonRelease(Button::Left)).unwrap();

        thread::sleep(time::Duration::from_millis(40));
        simulate(&EventType::ButtonPress(Button::Left)).unwrap();
        thread::sleep(delay);
        simulate(&EventType::ButtonRelease(Button::Left)).unwrap();

        thread::sleep(time::Duration::from_millis(40));
        simulate(&EventType::KeyPress(Key::ControlLeft)).unwrap();
        thread::sleep(delay);
        simulate(&EventType::KeyPress(Key::KeyV)).unwrap();

        thread::sleep(delay);
        simulate(&EventType::KeyRelease(Key::KeyV)).unwrap();
        thread::sleep(delay);
        simulate(&EventType::KeyRelease(Key::ControlLeft)).unwrap();
        simulate(&EventType::MouseMove {
            x: mouse_position_x as f64,
            y: mouse_position_y as f64,
        })
        .unwrap();
        thread::sleep(time::Duration::from_millis(40));
        arboard::Clipboard::new().unwrap().set_text("").unwrap();
    });
    // thread::spawn(move || {
    //     let delay = time::Duration::from_millis(400);
    //     thread::sleep(delay);
    //     let mut enigo = Enigo::new(&Settings::default()).unwrap();
    //     enigo
    //         .move_mouse(x as i32 - 10, y as i32, enigo::Coordinate::Abs)
    //         .unwrap();
    //     enigo
    //         .button(enigo::Button::Left, enigo::Direction::Click)
    //         .unwrap();
    //     enigo
    //         .button(enigo::Button::Left, enigo::Direction::Click)
    //         .unwrap();
    //     enigo.key(Key::Control, Direction::Press).unwrap();
    //     enigo.key(Key::Unicode('v'), Direction::Press).unwrap();
    //     enigo.key(Key::Unicode('v'), Direction::Release).unwrap();
    //     enigo.key(Key::Control, Direction::Release).unwrap();
    // });

    Ok(())
}

pub struct FormFieldsOverlay {
    pub form_fields: Option<Vec<FormFields>>,
    pub hidden_fields: Arc<Mutex<HashSet<usize>>>,
    pub mouse_position: Arc<Mutex<(i32, i32)>>,
}
impl FormFieldsOverlay {
    pub fn new(mouse_position: Arc<Mutex<(i32, i32)>>) -> Self {
        Self {
            form_fields: None,
            hidden_fields: Arc::new(Mutex::new(HashSet::new())),
            mouse_position,
        }
    }
    pub fn show(&self, egui_context: &egui::Context, screenshot_pos: Option<&egui::Pos2>) {
        egui::Window::new("Overlay")
            .interactable(false)
            .title_bar(false)
            .default_pos(egui::Pos2::new(0.0, 0.0))
            .auto_sized()
            .frame(egui::Frame {
                fill: egui::Color32::TRANSPARENT,
                ..Default::default()
            })
            .show(egui_context, |_ui| {
                if let Some(form_fields) = &self.form_fields {
                    for (i, field) in form_fields.iter().enumerate() {
                        // Skip if field is marked as hidden
                        if self
                            .hidden_fields
                            .lock()
                            .expect("Failed to lock hidden_fields POISON")
                            .contains(&i)
                        {
                            continue;
                        }
                        //draw rect to check how precise the coordinates are
                        /*ui.painter().rect(
                            egui::Rect::from_min_max(
                                egui::pos2(
                                    field.field_coords.x1 as f32
                                        + screenshot_pos.unwrap_or(&egui::Pos2::ZERO).x,
                                    field.field_coords.y1 as f32
                                        + screenshot_pos.unwrap_or(&egui::Pos2::ZERO).y,
                                ),
                                egui::pos2(
                                    field.field_coords.x2 as f32
                                        + screenshot_pos.unwrap_or(&egui::Pos2::ZERO).x,
                                    field.field_coords.y2 as f32
                                        + screenshot_pos.unwrap_or(&egui::Pos2::ZERO).y,
                                ),
                            ),
                            0.0,
                            egui::Color32::from_rgba_premultiplied(150, 150, 150, 100),
                            egui::Stroke::new(1.0, egui::Color32::BLACK),
                        );*/
                        egui::Area::new(egui::Id::new(i.to_string()))
                            .fixed_pos(egui::pos2(
                                field.field_coords.x1 as f32
                                    + screenshot_pos.unwrap_or(&egui::Pos2::ZERO).x
                                    + 20.0,
                                (field.field_coords.y1 as f32 + field.field_coords.y2 as f32
                                    - 15.0)
                                    / 2.0
                                    + screenshot_pos.unwrap_or(&egui::Pos2::ZERO).y,
                            ))
                            .interactable(true)
                            .default_width(1000.0)
                            .show(egui_context, |ui| {
                                ui.add(egui::Label::new(egui::RichText::new(
                                    field.field_value.as_str(),
                                )));
                                if field.field_value.as_str() != ""
                                    && ui.button("fill").interact(egui::Sense::click()).clicked()
                                {
                                    println!("fill: {}", field.field_value.as_str());
                                    let mouse_position_x = self
                                        .mouse_position
                                        .lock()
                                        .expect("Failed to lock mouse_position POISON")
                                        .0;
                                    let mouse_position_y = self
                                        .mouse_position
                                        .lock()
                                        .expect("Failed to lock mouse_position POISON")
                                        .1;
                                    let _ = send_fill_command(
                                        field.field_coords.x1
                                            + screenshot_pos.unwrap_or(&egui::Pos2::ZERO).x as i32
                                            + 10,
                                        (field.field_coords.y1
                                            + screenshot_pos.unwrap_or(&egui::Pos2::ZERO).y as i32
                                            + field.field_coords.y2
                                            + screenshot_pos.unwrap_or(&egui::Pos2::ZERO).y as i32)
                                            / 2,
                                        mouse_position_x,
                                        mouse_position_y,
                                        field.field_value.as_str(),
                                    );
                                    // Instead of removing, add to hidden fields
                                    self.hidden_fields.lock().unwrap().insert(i);
                                }
                            });
                    }
                }
            });
    }
}
