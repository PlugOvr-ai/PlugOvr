//#![deny(warnings)]
//#![deny(clippy::unwrap_used)]
//#![deny(clippy::expect_used)]
//#![deny(clippy::panic)]
//#![deny(unused_must_use)]

#![windows_subsystem = "windows"]
use clap::Parser;
#[cfg(feature = "computeruse_remote")]
use rand::{distributions::Alphanumeric, Rng};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

mod llm;
mod ui;
#[cfg(feature = "computeruse_editor")]
mod usecase_editor;
#[cfg(feature = "computeruse_record")]
mod usecase_recorder;

#[cfg(feature = "computeruse_replay")]
mod usecase_replay;
#[cfg(feature = "computeruse_remote")]
mod usecase_webserver;
mod version_check;
mod window_handling;

#[cfg(feature = "computeruse_record")]
use crate::usecase_recorder::EventType;
#[cfg(feature = "computeruse_replay")]
use crate::usecase_replay::UseCaseReplay;
#[cfg(any(target_os = "windows", target_os = "linux"))]
use enigo::{Keyboard, Settings};
#[cfg(not(target_os = "macos"))]
use rdev::listen;
#[cfg(target_os = "macos")]
use rdev::{listen, Event};
use std::error::Error;
use std::time::Duration;
#[cfg(feature = "computeruse_editor")]
use usecase_editor::UsecaseEditor;
#[cfg(feature = "computeruse_record")]
use usecase_recorder::Point;
#[cfg(feature = "computeruse_record")]
use usecase_recorder::UseCaseRecorder;
use window_handling::ActiveWindow;

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

#[cfg(target_os = "macos")]
fn send_cmd_v() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    Command::new("osascript")
        .arg("-e")
        .arg(r#"tell application "System Events" to keystroke "v" using command down"#)
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

// Define command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Disable password protection for the webserver
    #[arg(long)]
    no_password: bool,
}

#[cfg(feature = "computeruse_remote")]
fn generate_random_password(length: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

#[cfg(feature = "computeruse_remote")]
fn load_password() -> Option<String> {
    let password_file = get_config_file("webserver_password.txt");
    match fs::read_to_string(password_file) {
        Ok(password) if !password.trim().is_empty() => Some(password.trim().to_string()),
        _ => None,
    }
}

#[cfg(feature = "computeruse_remote")]
fn save_password(password: &str) {
    let password_file = get_config_file("webserver_password.txt");
    if let Err(e) = fs::write(password_file, password) {
        eprintln!("Failed to save password: {:?}", e);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments
    #[cfg(feature = "computeruse_remote")]
    let args = Args::parse();
    #[cfg(not(feature = "computeruse_remote"))]
    let _args = Args::parse(); // Unused if computeruse_remote is not enabled

    let text_entry = Arc::new(Mutex::new(false));
    let shortcut_window = Arc::new(Mutex::new(false));
    let control_pressed = Arc::new(Mutex::new(false));
    let alt_pressed = Arc::new(Mutex::new(false));
    let text_entryfield_position = Arc::new(Mutex::new((0, 0)));
    let ai_context = Arc::new(Mutex::new(String::new()));
    #[cfg(feature = "computeruse_record")]
    let usecase_recorder = Arc::new(Mutex::new(UseCaseRecorder::new()));
    #[cfg(feature = "computeruse_editor")]
    let usecase_editor = Arc::new(Mutex::new(UsecaseEditor::new()));
    let mouse_position = Arc::new(Mutex::new((0, 0)));
    let hide_ui = Arc::new(Mutex::new(load_bool_config("hide_ui.txt", false)));
    #[cfg(target_os = "linux")]
    let active_window = Arc::new(Mutex::new(ActiveWindow(0)));
    #[cfg(target_os = "windows")]
    let active_window = Arc::new(Mutex::new(ActiveWindow(0)));
    #[cfg(target_os = "macos")]
    let active_window = Arc::new(Mutex::new(ActiveWindow(0)));

    #[cfg(feature = "computeruse_replay")]
    let usecase_replay = Arc::new(Mutex::new(UseCaseReplay::new()));

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
        #[cfg(feature = "computeruse_record")]
        let usecase_recorder = usecase_recorder.clone();
        #[cfg(feature = "computeruse_replay")]
        let usecase_replay = usecase_replay.clone();
        #[cfg(feature = "computeruse_replay")]
        let _ = usecase_replay
            .lock()
            .unwrap()
            .load_usecase("calendar.json".to_string());
        let _ = thread::Builder::new()
            .name("Key Event Thread".to_string())
            .spawn(move || {
                let last_mouse_pos = Arc::new(Mutex::new((0, 0)));
                // Add a delay of 2 seconds
                std::thread::sleep(std::time::Duration::from_secs(2));
                let callback = move |event: Event| {
                    #[cfg(feature = "computeruse_record")]
                    if *usecase_recorder.lock().unwrap().recording.lock().unwrap() {
                        if usecase_recorder.lock().unwrap().add_image {
                            if let Ok(mut recorder) = usecase_recorder.lock() {
                                if recorder.add_image {
                                    //let now = SystemTime::now();
                                    if let Some(add_image_now) = recorder.add_image_now {
                                        match add_image_now.elapsed() {
                                            Ok(elapsed) => {
                                                if elapsed > recorder.add_image_delay.unwrap() {
                                                    // Take screenshot without holding the lock for too long
                                                    recorder.add_image = false;
                                                    recorder.add_image_delay = None;
                                                    recorder.add_image_now = None;
                                                    recorder.add_screenshot();
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("Error getting elapsed time: {:?}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        match event.event_type {
                            rdev::EventType::KeyPress(key) => {
                                let key_str = serde_json::to_string(&key).unwrap();
                                let key_str = key_str.replace("\"", "");
                                let key_str = key_str.replace("Key", "");
                                let key_str = key_str.replace("Dot", ".");
                                let key_str = key_str.replace("Comma", ",");
                                let key_str = key_str.replace("Semicolon", ";");
                                let key_str = key_str.replace("Space", " ");
                                let key_str = key_str.replace("Num", "");
                                usecase_recorder
                                    .lock()
                                    .unwrap()
                                    .add_event(EventType::KeyDown(key_str));
                            }
                            rdev::EventType::KeyRelease(key) => {
                                let key_str = serde_json::to_string(&key).unwrap();
                                let key_str = key_str.replace("\"", "");
                                let key_str = key_str.replace("Key", "");
                                let key_str = key_str.replace("Dot", ".");
                                let key_str = key_str.replace("Comma", ",");
                                let key_str = key_str.replace("Semicolon", ";");
                                let key_str = key_str.replace("Space", " ");
                                let key_str = key_str.replace("Num", "");
                                usecase_recorder
                                    .lock()
                                    .unwrap()
                                    .add_event(EventType::KeyUp(key_str));
                            }
                            rdev::EventType::MouseMove { x, y } => {
                                *last_mouse_pos.lock().unwrap() = (x as i32, y as i32);
                            }
                            rdev::EventType::ButtonPress(button) => {
                                if button == rdev::Button::Left {
                                    let last_mouse_pos = last_mouse_pos.lock().unwrap();
                                    usecase_recorder.lock().unwrap().add_event(EventType::Click(
                                        Point {
                                            x: last_mouse_pos.0 as f32,
                                            y: last_mouse_pos.1 as f32,
                                        },
                                        "".to_string(),
                                        vec![],
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }

                    match event.event_type {
                        rdev::EventType::KeyPress(key) => {
                            #[cfg(feature = "computeruse_replay")]
                            if key == rdev::Key::F2 {
                                usecase_replay.lock().unwrap().step();
                            }
                            #[cfg(feature = "computeruse_replay")]
                            if key == rdev::Key::F4 {
                                usecase_replay.lock().unwrap().vec_instructions =
                                    Arc::new(Mutex::new(vec![]));
                                *usecase_replay
                                    .lock()
                                    .unwrap()
                                    .index_instruction
                                    .lock()
                                    .unwrap() = 0;
                                *usecase_replay.lock().unwrap().index_action.lock().unwrap() = 0;
                                usecase_replay.lock().unwrap().show_dialog = true;
                            }
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
                                #[cfg(any(target_os = "linux", target_os = "windows"))]
                                #[cfg(feature = "computeruse_record")]
                                if key == rdev::Key::KeyR
                                    && *control_pressed.lock().unwrap()
                                    && *alt_pressed.lock().unwrap()
                                {
                                    usecase_recorder.lock().unwrap().show = true;
                                }
                                #[cfg(target_os = "macos")]
                                #[cfg(feature = "computeruse_record")]
                                if key == rdev::Key::KeyR && *control_pressed.lock().unwrap() {
                                    usecase_recorder.lock().unwrap().show = true;
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
                                // if key == rdev::Key::Escape
                                //     && usecase_replay
                                //         .lock()
                                //         .unwrap()
                                //         .vec_instructions
                                //         .lock()
                                //         .unwrap()
                                //         .len()
                                //         > 0
                                // {
                                //     usecase_replay
                                //         .lock()
                                //         .unwrap()
                                //         .vec_instructions
                                //         .lock()
                                //         .unwrap()
                                //         .clear();
                                //     *usecase_replay
                                //         .lock()
                                //         .unwrap()
                                //         .index_instruction
                                //         .lock()
                                //         .unwrap() = 0;
                                //     *usecase_replay.lock().unwrap().index_action.lock().unwrap() =
                                //         0;
                                //     usecase_replay.lock().unwrap().show_dialog = false;
                                // }
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
    #[cfg(feature = "computeruse_remote")]
    {
        let usecase_replay_clone = usecase_replay.clone();

        // Determine password based on command line arguments
        let password = if args.no_password {
            println!("Webserver will run without password protection");
            None
        } else {
            // Try to load existing password
            let password = load_password().unwrap_or_else(|| {
                // Generate a new random password if none exists
                let new_password = generate_random_password(8);
                save_password(&new_password);
                new_password
            });
            Some(password)
        };

        tokio::spawn(async move {
            usecase_webserver::start_server(usecase_replay_clone, password).await;
        });
    }
    {
        let text_entry = text_entry.clone();
        let text_entryfield_position = text_entryfield_position.clone();
        let ai_context = ai_context.clone();

        let active_window = active_window.clone();
        #[cfg(feature = "computeruse_replay")]
        let usecase_replay = usecase_replay.clone();

        ui::user_interface::run(
            text_entry,
            text_entryfield_position,
            mouse_position,
            ai_context,
            active_window,
            shortcut_window,
            #[cfg(feature = "computeruse_record")]
            usecase_recorder,
            #[cfg(feature = "computeruse_replay")]
            usecase_replay,
            #[cfg(feature = "computeruse_editor")]
            usecase_editor,
        )
        .await;
    }

    // Start webserver in a separate thread

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
    let clipboard_backup = match clipboard.get_text() {
        Ok(text) => text,
        Err(e) => {
            eprintln!("Error getting clipboard text: {:?}", e);
            String::new()
        }
    };
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
