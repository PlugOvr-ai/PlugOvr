//#![deny(warnings)]
//#![deny(clippy::unwrap_used)]
//#![deny(clippy::expect_used)]
//#![deny(clippy::panic)]
//#![deny(unused_must_use)]

#![windows_subsystem = "windows"]
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

mod llm;
mod ui;
mod version_check;
mod window_handling;

use window_handling::ActiveWindow;

use std::error::Error;

#[cfg(not(target_os = "macos"))]
use rdev::listen;
#[cfg(target_os = "macos")]
use rdev::{listen, Event};

#[cfg(any(target_os = "windows", target_os = "linux"))]
use enigo::{Keyboard, Settings};

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn send_cmd_c() -> Result<(), Box<dyn std::error::Error>> {
    let mut enigo = enigo::Enigo::new(&Settings::default())?;
    enigo.key(enigo::Key::Alt, enigo::Direction::Release)?;
    enigo.key(enigo::Key::Control, enigo::Direction::Press)?;

    #[cfg(target_os = "windows")]
    enigo.key(enigo::Key::C, enigo::Direction::Click)?;
    #[cfg(target_os = "linux")]
    enigo.key(enigo::Key::Unicode('c'), enigo::Direction::Click)?;

    enigo.key(enigo::Key::Control, enigo::Direction::Release)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn send_cmd_c() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events" to keystroke "c" using command down"#)
        .output()?;

    Ok(())
}

fn get_config_file(filename: &str) -> PathBuf {
    let config_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".plugovr");

    // Create config directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(&config_dir) {
        eprintln!("Failed to create config directory: {:?}", e);
    }

    config_dir.join(filename)
}

fn load_bool_config(filename: &str, default: bool) -> bool {
    let config_file = get_config_file(filename);
    fs::read_to_string(config_file)
        .map(|s| s.trim() == "true")
        .unwrap_or(default)
}

fn save_bool_config(filename: &str, value: bool) {
    let config_file = get_config_file(filename);
    if let Err(e) = fs::write(config_file, value.to_string()) {
        eprintln!("Failed to save config {}: {:?}", filename, e);
    }
}

// New helper function to handle window activation
fn activate_plugovr_window() -> Result<(), Box<dyn Error>> {
    #[cfg(target_os = "linux")]
    let window_title = "PlugOvr";
    #[cfg(target_os = "macos")]
    let window_title = "PlugOvr";
    #[cfg(target_os = "windows")]
    let window_title = "PlugOvr\0";

    #[cfg(target_os = "windows")]
    {
        window_handling::activate_window_title(&window_title.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        let window_id = window_handling::find_window_by_title(window_title);
        window_handling::activate_window(&ActiveWindow(window_id.unwrap_or(0)))?;
    }

    #[cfg(target_os = "macos")]
    {
        let window_id = window_handling::find_window_by_title(window_title);
        window_handling::activate_window(&ActiveWindow(window_id.unwrap()))?;
    }

    Ok(())
}

// New helper function to handle text selection
fn handle_text_selection(
    mouse_position: &Arc<Mutex<(i32, i32)>>,
    text_entryfield_position: &Arc<Mutex<(i32, i32)>>,
    ai_context: &Arc<Mutex<String>>,
) {
    let pos = mouse_position.lock().unwrap();
    *text_entryfield_position.lock().unwrap() = (pos.0, pos.1);

    match get_selected_text() {
        Ok(selected_text) => {
            *ai_context.lock().unwrap() = selected_text;
        }
        Err(e) => {
            eprintln!("Error getting selected text: {:?}", e);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let text_entry = Arc::new(Mutex::new(false));
    let shortcut_window = Arc::new(Mutex::new(false));
    let control_pressed = Arc::new(Mutex::new(false));
    let alt_pressed = Arc::new(Mutex::new(false));
    let text_entryfield_position = Arc::new(Mutex::new((0, 0)));
    let ai_context = Arc::new(Mutex::new(String::new()));

    let mouse_position = Arc::new(Mutex::new((0, 0)));
    let hide_ui = Arc::new(Mutex::new(load_bool_config("hide_ui.txt", false)));
    #[cfg(target_os = "linux")]
    let active_window = Arc::new(Mutex::new(ActiveWindow(0)));
    #[cfg(target_os = "windows")]
    let active_window = Arc::new(Mutex::new(ActiveWindow(0)));
    #[cfg(target_os = "macos")]
    let active_window = Arc::new(Mutex::new(ActiveWindow(0)));

    std::env::set_var("RUST_LOG", "error");

    // tracing_subscriber::fmt::init();
    #[cfg(target_os = "macos")]
    use rdev::set_is_main_thread;
    use rdev::Event;
    #[cfg(target_os = "macos")]
    set_is_main_thread(false);

    {
        let shortcut_window = shortcut_window.clone();
        let text_entry = text_entry.clone();
        let text_entryfield_position = text_entryfield_position.clone();
        let mouse_position = mouse_position.clone();
        let ai_context = ai_context.clone();
        let active_window = active_window.clone();
        let alt_pressed = alt_pressed.clone();

        let _ = thread::Builder::new()
            .name("Key Event Thread".to_string())
            .spawn(move || {
                // Add a delay of 2 seconds
                std::thread::sleep(std::time::Duration::from_secs(2));
                let callback = move |event: Event| {
                    match event.event_type {
                        rdev::EventType::KeyPress(key) => {
                            if key == rdev::Key::ControlLeft {
                                *control_pressed.lock().unwrap() = true;
                            }
                            if key == rdev::Key::Alt {
                                *alt_pressed.lock().unwrap() = true;
                            }
                            if (key == rdev::Key::Space && *control_pressed.lock().unwrap())
                                || (key == rdev::Key::KeyI && *control_pressed.lock().unwrap())
                            {
                                std::thread::sleep(std::time::Duration::from_millis(400));

                                handle_text_selection(
                                    &mouse_position,
                                    &text_entryfield_position,
                                    &ai_context,
                                );

                                if let Some(_active_window) = window_handling::get_active_window() {
                                    *active_window.lock().unwrap() = ActiveWindow(_active_window.0);

                                    if let Err(e) = activate_plugovr_window() {
                                        eprintln!("Failed to activate PlugOvr window: {:?}", e);
                                    }
                                }
                                if key == rdev::Key::Space {
                                    *shortcut_window.lock().unwrap() = true;
                                } else {
                                    *text_entry.lock().unwrap() = true;
                                }
                            }
                            {
                                #[cfg(any(target_os = "linux", target_os = "windows"))]
                                if key == rdev::Key::KeyP
                                    && *control_pressed.lock().unwrap()
                                    && *alt_pressed.lock().unwrap()
                                {
                                    let mut hide_ui_guard = hide_ui.lock().unwrap();
                                    *hide_ui_guard = !*hide_ui_guard;
                                    save_bool_config("hide_ui.txt", *hide_ui_guard);
                                }
                                #[cfg(target_os = "macos")]
                                if key == rdev::Key::KeyP && *control_pressed.lock().unwrap() {
                                    let mut hide_ui_guard = hide_ui.lock().unwrap();
                                    *hide_ui_guard = !*hide_ui_guard;
                                    save_bool_config("hide_ui.txt", *hide_ui_guard);
                                }

                                if key == rdev::Key::Escape
                                    && (*text_entry.lock().unwrap()
                                        || *shortcut_window.lock().unwrap())
                                {
                                    *text_entry.lock().unwrap() = false;
                                    *shortcut_window.lock().unwrap() = false;
                                    // Add a 200ms delay
                                    std::thread::sleep(std::time::Duration::from_millis(400));

                                    if let Err(e) = window_handling::activate_window(
                                        &active_window.lock().unwrap(),
                                    ) {
                                        eprintln!("Failed to activate window: {:?}", e);
                                    }
                                }
                            }
                        }
                        rdev::EventType::KeyRelease(key) => {
                            if key == rdev::Key::ControlLeft {
                                *control_pressed.lock().unwrap() = false;
                            }
                            if key == rdev::Key::Alt {
                                *alt_pressed.lock().unwrap() = false;
                            }
                        }
                        rdev::EventType::MouseMove { x, y } => {
                            *mouse_position.lock().unwrap() = (x as i32, y as i32);
                        }
                        _ => {}
                    }
                };

                #[cfg(target_os = "macos")]
                {
                    listen_with_retry(callback);
                }

                #[cfg(not(target_os = "macos"))]
                {
                    if let Err(error) = listen(callback) {
                        eprintln!("Error: {:?}", error)
                    }
                }
            });
    }

    {
        let text_entry = text_entry.clone();
        let text_entryfield_position = text_entryfield_position.clone();
        let ai_context = ai_context.clone();

        let active_window = active_window.clone();

        ui::user_interface::run(
            text_entry,
            text_entryfield_position,
            mouse_position,
            ai_context,
            active_window,
            shortcut_window,
        )
        .await;
    }

    Ok(())
}

// #[cfg(target_os = "linux")]
// fn get_selected_text() -> Result<String, Box<dyn std::error::Error>> {
//     use x11_clipboard::Clipboard;
//     let clipboard = Clipboard::new()?;

//     // Directly load the selection without clearing first
//     let selection = match clipboard.load(
//         clipboard.getter.atoms.primary,
//         clipboard.getter.atoms.utf8_string,
//         clipboard.getter.atoms.property,
//         std::time::Duration::from_secs(3),
//     ) {
//         Ok(data) if !data.is_empty() => data,
//         _ => return Ok(String::new()), // Return empty string if no selection or error
//     };

//     // Convert the selection to a string
//     let selected_text = String::from_utf8(selection)?;

//     Ok(selected_text)
// }
//#[cfg(any(target_os = "windows", target_os = "macos"))]
fn get_selected_text() -> Result<String, Box<dyn std::error::Error>> {
    use arboard::Clipboard;
    let mut clipboard = Clipboard::new()?;
    //backup clipboard
    let clipboard_backup = clipboard.get_text()?;
    //clear clipboard
    clipboard.set_text("")?;
    // Send Cmd+C using AppleScript
    send_cmd_c()?;

    // Wait a bit for the clipboard to be updated
    std::thread::sleep(std::time::Duration::from_millis(100));

    let selected_text = clipboard.get_text()?;

    //restore clipboard
    clipboard.set_text(clipboard_backup)?;
    Ok(selected_text)
}

#[cfg(target_os = "macos")]
fn listen_with_retry<F>(callback: F)
where
    F: Fn(Event) + Send + Clone + 'static,
{
    let max_retries = 3;
    let mut retry_count = 0;

    while retry_count < max_retries {
        match listen(callback.clone()) {
            Ok(_) => break,
            Err(error) => {
                eprintln!(
                    "Error in event listener (attempt {}/{}): {:?}",
                    retry_count + 1,
                    max_retries,
                    error
                );
                std::thread::sleep(std::time::Duration::from_secs(1));
                retry_count += 1;
            }
        }
    }
}
