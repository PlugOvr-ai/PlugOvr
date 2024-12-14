use crate::ui::assistance_window::AiResponseAction;
use arboard::Clipboard;
use std::error::Error;
use std::sync::{Arc, Mutex};

#[cfg(target_os = "linux")]
use x11rb::connection::Connection;
#[cfg(target_os = "linux")]
use x11rb::protocol::xproto::ConnectionExt;
#[cfg(target_os = "linux")]
pub struct ActiveWindow(pub u32);
#[cfg(target_os = "macos")]
pub struct ActiveWindow(pub u64);

#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;

#[cfg(target_os = "windows")]
pub struct ActiveWindow(pub usize);

#[cfg(target_os = "linux")]
pub fn activate_window(window_title: &ActiveWindow) -> Result<(), Box<dyn Error>> {
    use std::time::Duration;
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{ConnectionExt, EventMask, Window};
    use x11rb::protocol::Event;

    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;
    let window_id = Window::from(window_title.0);

    // Prepare atoms
    let net_active_window = conn
        .intern_atom(false, b"_NET_ACTIVE_WINDOW")?
        .reply()?
        .atom;
    let net_wm_state = conn.intern_atom(false, b"_NET_WM_STATE")?.reply()?.atom;
    let net_wm_state_focused = conn
        .intern_atom(false, b"_NET_WM_STATE_FOCUSED")?
        .reply()?
        .atom;

    // Send _NET_ACTIVE_WINDOW message
    conn.send_event(
        false,
        root,
        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
        x11rb::protocol::xproto::ClientMessageEvent::new(
            32,
            window_id,
            net_active_window,
            [2, 0, 0, 0, 0],
        ),
    )?;

    // Send _NET_WM_STATE message to set focus
    conn.send_event(
        false,
        window_id,
        EventMask::PROPERTY_CHANGE,
        x11rb::protocol::xproto::ClientMessageEvent::new(
            32,
            window_id,
            net_wm_state,
            [1, net_wm_state_focused, 0, 0, 0],
        ),
    )?;

    conn.flush()?;

    // Wait for the window to be activated
    let start_time = std::time::Instant::now();
    let timeout = Duration::from_millis(10);
    while start_time.elapsed() < timeout {
        if let Some(event) = conn.poll_for_event()? {
            if let Event::PropertyNotify(e) = event {
                if e.window == window_id && e.atom == net_wm_state {
                    break;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    Ok(())
}

#[cfg(target_os = "macos")]
use cocoa::appkit::NSApplicationActivateIgnoringOtherApps;
#[cfg(target_os = "macos")]

pub fn activate_window(id: &ActiveWindow) -> Result<(), Box<dyn Error>> {
    //println!("activate_window: {:?}", id.0);
    use cocoa::base::id;
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        let shared_app: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let running_apps: id = msg_send![shared_app, runningApplications];
        let count: usize = msg_send![running_apps, count];

        for i in 0..count {
            let app: id = msg_send![running_apps, objectAtIndex: i];
            let pid: u64 = msg_send![app, processIdentifier];

            if pid == id.0 {
                let _: () =
                    msg_send![app, activateWithOptions:NSApplicationActivateIgnoringOtherApps];
                return Ok(());
            }
        }
    }

    Err("Window not found".into())
}
#[cfg(target_os = "windows")]
pub fn activate_window(hwnd_value: &ActiveWindow) -> Result<(), Box<dyn Error>> {
    force_foreground_window(hwnd_value.0 as HWND);

    Ok(())
}

#[cfg(target_os = "windows")]
use winapi::shared::minwindef::DWORD;

#[cfg(target_os = "windows")]
use winapi::um::processthreadsapi::GetCurrentThreadId;
#[cfg(target_os = "windows")]
use winapi::um::winuser::{
    AttachThreadInput, BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, ShowWindow,
};
#[cfg(target_os = "windows")]
pub fn force_foreground_window(hwnd: HWND) {
    unsafe {
        let foreground_window = GetForegroundWindow();
        let mut foreground_thread_id = 0;
        let window_thread_process_id =
            GetWindowThreadProcessId(foreground_window, &mut foreground_thread_id);
        let current_thread_id = GetCurrentThreadId();
        const SW_SHOW: DWORD = 5;

        AttachThreadInput(window_thread_process_id, current_thread_id, 1);
        BringWindowToTop(hwnd);
        ShowWindow(hwnd, SW_SHOW as i32);
        AttachThreadInput(window_thread_process_id, current_thread_id, 0);
    }
}
#[cfg(target_os = "linux")]
pub fn find_window_by_title(title: &str) -> Option<u32> {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let windows = conn.query_tree(root).unwrap().reply().unwrap().children;
    for window in windows {
        if let Ok(name) = conn
            .get_property(
                false,
                window,
                x11rb::protocol::xproto::AtomEnum::WM_NAME,
                x11rb::protocol::xproto::AtomEnum::STRING,
                0,
                1024,
            )
            .unwrap()
            .reply()
        {
            if let Ok(window_title) = String::from_utf8(name.value) {
                if window_title == title {
                    return Some(window);
                }
            }
        }
    }
    None
}
#[cfg(target_os = "macos")]
pub fn find_window_by_title(title: &str) -> Option<u64> {
    use cocoa::base::id;
    use objc::{msg_send, sel, sel_impl};

    unsafe {
        let shared_app: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        let running_apps: id = msg_send![shared_app, runningApplications];
        let count: usize = msg_send![running_apps, count];

        for i in 0..count {
            let app: id = msg_send![running_apps, objectAtIndex: i];
            let app_name: id = msg_send![app, localizedName];
            let name = cocoa::foundation::NSString::UTF8String(app_name);
            let name_str = std::ffi::CStr::from_ptr(name).to_string_lossy();

            if name_str == title {
                let pid: u64 = msg_send![app, processIdentifier];
                return Some(pid);
            }
        }
        None
    }
}
#[cfg(target_os = "windows")]
use std::ptr::null;
#[cfg(target_os = "windows")]
pub fn activate_window_title(window_title: &String) -> Result<(), Box<dyn Error>> {
    use winapi::um::winuser::FindWindowA;
    unsafe {
        let hwnd = FindWindowA(null(), window_title.as_ptr() as *const i8);
        //println!("hwnd: {:?}", hwnd);
        if !hwnd.is_null() {
            //SetForegroundWindow(hwnd);
            force_foreground_window(hwnd);
            //println!("Activated window '{}'", window_title);
            Ok(())
        } else {
            Err(format!("Failed to activate window '{}'", window_title).into())
        }
    }
}

#[cfg(target_os = "linux")]
pub fn get_active_window() -> Option<ActiveWindow> {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let atom_name = "_NET_ACTIVE_WINDOW";
    let atom = conn
        .intern_atom(false, atom_name.as_bytes())
        .unwrap()
        .reply()
        .unwrap()
        .atom;

    let active_window = conn
        .get_property(
            false,
            root,
            atom,
            x11rb::protocol::xproto::AtomEnum::WINDOW,
            0,
            1,
        )
        .unwrap()
        .reply()
        .unwrap();
    // Extract the window title from the active window

    active_window
        .value32()
        .and_then(|mut v| v.next())
        .map(ActiveWindow)
}

/*#[cfg(target_os = "linux")]
fn get_active_window2() -> Option<ActiveWindow> {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = &conn.setup().roots[screen_num];
    let root = screen.root;

    let atom_name = "_NET_ACTIVE_WINDOW";
    let atom = conn
        .intern_atom(false, atom_name.as_bytes())
        .unwrap()
        .reply()
        .unwrap()
        .atom;

    let active_window = conn
        .get_property(
            false,
            root,
            atom,
            x11rb::protocol::xproto::AtomEnum::WINDOW,
            0,
            1,
        )
        .unwrap()
        .reply()
        .unwrap();
    // Extract the window title from the active window
    let active_window_title =
        if let Some(window_id) = active_window.value32().and_then(|mut v| v.next()) {
            let window = Window::from(window_id);
            let wm_name = conn
                .get_property(
                    false,
                    window,
                    conn.intern_atom(false, b"_NET_WM_NAME")
                        .unwrap()
                        .reply()
                        .unwrap()
                        .atom,
                    conn.intern_atom(false, b"UTF8_STRING")
                        .unwrap()
                        .reply()
                        .unwrap()
                        .atom,
                    0,
                    u32::max_value(),
                )
                .unwrap()
                .reply();

            match wm_name {
                Ok(property) => String::from_utf8(property.value).ok(),
                Err(_) => None,
            }
        } else {
            None
        };

    // Return the title if found, otherwise return None
    return Some(ActiveWindow(active_window_title.unwrap()));
}

#[cfg(target_os = "windows")]
fn get_active_window1() -> Option<String> {
    use std::ptr;
    use winapi::um::winuser::{GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd != ptr::null_mut() {
            let mut buffer = [0u16; 1024];
            let mut text_length = GetWindowTextLengthW(hwnd);
            if text_length > 0 {
                text_length += 1; // Include null terminator
                GetWindowTextW(
                    hwnd,
                    &mut buffer as *mut u16 as *mut u16,
                    text_length as i32,
                );
                Some(String::from_utf16_lossy(&buffer).to_string())
            } else {
                None
            }
        } else {
            None
        }
    }
}*/

#[cfg(target_os = "macos")]
pub fn get_active_window() -> Option<ActiveWindow> {
    use active_win_pos_rs::get_active_window;
    let active_window = get_active_window().ok();
    if let Some(active_window) = active_window {
        Some(ActiveWindow(active_window.process_id))
    } else {
        None
    }
}
#[cfg(target_os = "windows")]
pub fn get_active_window() -> Option<ActiveWindow> {
    use winapi::um::winuser::{GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW};

    unsafe {
        let hwnd = GetForegroundWindow();
        if !hwnd.is_null() {
            let mut buffer = [0u16; 1024];
            let mut text_length = GetWindowTextLengthW(hwnd);
            if text_length > 0 {
                text_length += 1; // Include null terminator
                GetWindowTextW(hwnd, &mut buffer as *mut u16, text_length as i32);
                Some(ActiveWindow(hwnd as usize))
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[cfg(target_os = "windows")]
pub fn find_window_by_title(title: &str) -> Option<usize> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::winuser::FindWindowW;

    // Convert &str to wide string (UTF-16)
    let wide: Vec<u16> = OsStr::new(title)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), wide.as_ptr());
        if hwnd.is_null() {
            None
        } else {
            Some(hwnd as usize)
        }
    }
}

pub fn send_results(
    active_window: Arc<Mutex<ActiveWindow>>,
    ai_context: Arc<Mutex<String>>,
    ai_answer: Arc<Mutex<String>>,
    ai_resonde_action: AiResponseAction,
) -> Result<(), Box<dyn Error>> {
    //println!("trigger action to take over answer and move focus back");
    // let window_title = active_window.lock().unwrap().to_string();
    //println!("activate {:}", window_title);
    if let Err(e) = activate_window(&active_window.lock().unwrap()) {
        eprintln!("Failed to activate window: {:?}", e);
    }
    // Get the AI answer
    let ai_answer = ai_answer.lock().unwrap().clone();
    let ai_context = ai_context.lock().unwrap().clone();

    // Copy AI answer to clipboard
    //  use clipboard::{ClipboardContext, ClipboardProvider};
    //   let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    let mut clipboard = Clipboard::new().unwrap();

    match ai_resonde_action {
        AiResponseAction::Replace => {
            let _ = clipboard.set_text(ai_answer.to_owned());
        }
        AiResponseAction::Extend => {
            let _ = clipboard.set_text(ai_context.to_owned() + " " + &ai_answer.to_owned());
        }
        AiResponseAction::Ignore => {
            return Ok(());
        }
    }

    // Send Ctrl+V to paste
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        use enigo::{Direction, Enigo, Key, Keyboard, Settings};
        if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
            enigo.key(Key::Control, Direction::Press)?;
            enigo.key(Key::Unicode('v'), Direction::Press)?;
            enigo.key(Key::Unicode('v'), Direction::Release)?;
            enigo.key(Key::Control, Direction::Release)?;
        }
    }

    #[cfg(target_os = "macos")]
    send_cmd_v()?;

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
