#[cfg(target_os = "linux")]
use x11rb::connection::Connection;
#[cfg(target_os = "macos")]
pub fn get_screen_dimensions() -> (u16, u16) {
    use core_graphics::display::CGDisplay;

    let main_display = CGDisplay::main();
    let width = main_display.pixels_wide() as u16;
    let height = main_display.pixels_high() as u16;
    (width, height)
}
#[cfg(target_os = "linux")]
pub fn get_screen_dimensions() -> (u16, u16) {
    if let Ok((conn, _)) = x11rb::connect(None) {
        let screens = &conn.setup().roots;

        let (total_width, max_height) = screens.iter().fold((0, 0), |(w, h), screen| {
            (w + screen.width_in_pixels, h.max(screen.height_in_pixels))
        });

        (total_width, max_height)
    } else {
        eprintln!("Failed to connect to X11 assuming 1920x1080");
        (1920, 1080)
    }
}

#[cfg(target_os = "windows")]
pub fn get_screen_dimensions() -> (u16, u16) {
    use winapi::um::winuser::{GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN};
    unsafe {
        let width = GetSystemMetrics(SM_CXVIRTUALSCREEN) as u16;
        let height = GetSystemMetrics(SM_CYVIRTUALSCREEN) as u16;
        (width - 1, height - 1)
    }
}
