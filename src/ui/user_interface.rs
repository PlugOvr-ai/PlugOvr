use crate::llm::LLMSelector;
use crate::ui::assistance_window::AssistanceWindow;
use crate::ui::main_window::MainWindow;
use crate::ui::screen_dimensions::get_screen_dimensions;
use crate::ui::template_editor::create_prompt_templates;
use crate::ui::template_editor::TemplateEditor;
use crate::ui::template_editor::TemplateMap;
#[cfg(feature = "computeruse_record")]
use crate::usecase_recorder::UseCaseRecorder;
#[cfg(feature = "computeruse_replay")]
use crate::usecase_replay::UseCaseReplay;
#[cfg(feature = "computeruse_editor")]
use crate::usecase_editor::UsecaseEditor;
use crate::version_check;
use crate::ActiveWindow;
use egui_overlay::EguiOverlay;
use std::collections::HashMap;
use tray_icon::{
    menu::AboutMetadata, menu::MenuItem, menu::PredefinedMenuItem, TrayIcon, TrayIconBuilder,
};

#[cfg(feature = "three_d")]
use egui_overlay::egui_render_three_d::ThreeDBackend as DefaultGfxBackend;

use core::f32;
use egui::{FontData, FontDefinitions, FontFamily};
#[cfg(feature = "glow")]
use egui_overlay::egui_render_glow::GlowBackend as DefaultGfxBackend;
#[cfg(feature = "wgpu")]
use egui_overlay::egui_render_wgpu::WgpuBackend as DefaultGfxBackend;

use std::str::FromStr;
use std::sync::{Arc, Mutex};

#[cfg(not(any(feature = "three_d", feature = "wgpu", feature = "glow")))]
compile_error!("you must enable either `three_d`, `wgpu` or `glow` feature to run this example");


pub async fn run(
    text_entry: Arc<Mutex<bool>>,
    text_entryfield_position: Arc<Mutex<(i32, i32)>>,
    mouse_position: Arc<Mutex<(i32, i32)>>,
    ai_context: Arc<Mutex<String>>,

    active_window: Arc<Mutex<ActiveWindow>>,
    shortcut_window: Arc<Mutex<bool>>,
    #[cfg(feature = "computeruse_record")] usecase_recorder: Arc<Mutex<UseCaseRecorder>>,
    #[cfg(feature = "computeruse_replay")] usecase_replay: Arc<Mutex<UseCaseReplay>>,
    #[cfg(feature = "computeruse_editor")] usecase_editor: Arc<Mutex<UsecaseEditor>>,
) {
    // use tracing_subscriber::{fmt, prelude::*, EnvFilter};
    // // if RUST_LOG is not set, we will use the following filters
    // tracing_subscriber::registry()
    //     .with(fmt::layer())
    //     .with(
    //         EnvFilter::try_from_default_env()
    //             .unwrap_or(EnvFilter::new("debug,wgpu=warn,naga=warn")),
    //     )
    //     .init();

    let data = PlugOvr::new(
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
    egui_overlay::start(data);
}

enum MenuUpdate {
    UpdateWelcome(String),
    UpdateLogin(String, bool),
    UpdateUpdater(String, bool),
}
pub struct PlugOvr {
    pub text_entry: Arc<Mutex<bool>>,
    pub text_entryfield_position: Arc<Mutex<(i32, i32)>>,

    pub ai_answer: Arc<Mutex<String>>,
    pub shortcut_window: Arc<Mutex<bool>>,
    pub screen_width: u16,
    pub screen_height: u16,
    pub prompt_templates: TemplateMap,
    pub title: String,
    pub run_tray_icon: bool,
    fonts: FontDefinitions,
    pub llm_selector: Arc<Mutex<LLMSelector>>,

    //pub screenshots: Vec<(ImageBuffer<image::Rgba<u8>, Vec<u8>>, egui::Pos2)>,
    pub template_editor: TemplateEditor,
    pub main_window: MainWindow,
    pub assistance_window: AssistanceWindow,
    pub tray_icon: Option<TrayIcon>,
    pub login_menu_item: MenuItem,
    pub welcome_menu_item: MenuItem,
    pub updater_menu_item: MenuItem,
    menu_update_sender: Arc<Mutex<Option<Sender<MenuUpdate>>>>,
    last_login_state: Option<bool>,
    last_loading_state: Option<bool>,
    #[cfg(feature = "computeruse_record")]
    pub usecase_recorder: Arc<Mutex<UseCaseRecorder>>,
    #[cfg(feature = "computeruse_replay")]
    pub usecase_replay: Arc<Mutex<UseCaseReplay>>,
    #[cfg(feature = "computeruse_editor")]
    pub usecase_editor: Arc<Mutex<UsecaseEditor>>,
}

impl PlugOvr {
    pub async fn new(
        text_entry: Arc<Mutex<bool>>,

        text_entryfield_position: Arc<Mutex<(i32, i32)>>,
        mouse_position: Arc<Mutex<(i32, i32)>>,
        ai_context: Arc<Mutex<String>>,

        active_window: Arc<Mutex<ActiveWindow>>,
        shortcut_window: Arc<Mutex<bool>>,
        #[cfg(feature = "computeruse_record")] usecase_recorder: Arc<Mutex<UseCaseRecorder>>,
        #[cfg(feature = "computeruse_replay")] usecase_replay: Arc<Mutex<UseCaseReplay>>,
        #[cfg(feature = "computeruse_editor")] usecase_editor: Arc<Mutex<UsecaseEditor>>,
    ) -> Self {
        let (screen_width, screen_height) = get_screen_dimensions();
        // Import the user_management module
        #[cfg(feature = "cs")]
        use plugovr_cs::user_management;
        let ai_answer = Arc::new(Mutex::new(String::new()));
        // Add a new field for storing user information
        let user_info = Arc::new(Mutex::new(None));
        let is_loading_user_info = Arc::new(Mutex::new(true));
        let is_loading_user_info_clone = Arc::clone(&is_loading_user_info);
        // Function to fetch user information
        #[cfg(feature = "cs")]
        let fetch_user_info = {
            let user_info: Arc<Mutex<Option<UserInfo>>> = user_info.clone();
            move || match plugovr_cs::user_management::get_user() {
                Ok(info) => {
                    *user_info.lock().unwrap() = Some(info);
                    *is_loading_user_info_clone.lock().unwrap() = false;
                }
                Err(e) => {
                    eprintln!("Failed to fetch user info: {}", e);
                    *is_loading_user_info_clone.lock().unwrap() = false;
                }
            }
        };
        let version_msg = Arc::new(Mutex::new("".to_string()));

        let fetch_version_msg = {
            let version_msg = version_msg.clone();
            move || {
                *version_msg.lock().unwrap() = version_check::update_check().unwrap_or_default();
                println!("Version message: {}", *version_msg.lock().unwrap());
            }
        };

        // Spawn a new thread to fetch user information
        #[cfg(feature = "cs")]
        {
            std::thread::spawn(fetch_user_info);
        }

        std::thread::spawn(fetch_version_msg);
        let prompt_templates = Arc::new(Mutex::new(create_prompt_templates()));

        let mut fonts = FontDefinitions::default();

        // Load the Noto Sans font from a file
        fonts.font_data.insert(
            "NotoSans".to_string(),
            FontData::from_static(include_bytes!("../../assets/NotoSans-Regular.ttf")),
        );

        // Define which fonts to use for each style
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "NotoSans".to_owned());

        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .push("NotoSans".to_owned());

        let llm_selector = Arc::new(Mutex::new(LLMSelector::new(user_info.clone())));

        #[cfg(feature = "computeruse_replay")]
        {
            usecase_replay.lock().unwrap().llm_selector = Some(llm_selector.clone());
        }

        let assistance_window = AssistanceWindow::new(
            active_window.clone(),
            text_entry.clone(),
            text_entryfield_position.clone(),
            ai_context.clone(),
            prompt_templates.clone(),
            mouse_position.clone(),
            ai_answer.clone(),
            screen_width,
            screen_height,
            llm_selector.clone(),
        );
        let mut plug_ovr = Self {
            text_entry,
            text_entryfield_position,
            ai_answer,

            shortcut_window,

            screen_width,
            screen_height,

            prompt_templates: prompt_templates.clone(),
            title: String::from_str("PlugOvr").expect("Failed to create title string"),

            fonts,
            run_tray_icon: true,

            llm_selector: llm_selector.clone(),

            // Add this field to the PlugOvr struct
            template_editor: TemplateEditor::new(prompt_templates.clone()),
            main_window: MainWindow::new(
                user_info.clone(),
                is_loading_user_info.clone(),
                prompt_templates.clone(),
                llm_selector.clone(),
                version_msg.clone(),
                #[cfg(feature = "computeruse_editor")]
                usecase_editor.clone(),
            ),
            assistance_window,
            tray_icon: None,
            login_menu_item: MenuItem::new("Login", true, None),
            welcome_menu_item: MenuItem::new("Welcome", false, None),
            updater_menu_item: MenuItem::new("Updater", false, None),
            menu_update_sender: Arc::new(Mutex::new(None)),
            last_login_state: None,
            last_loading_state: None,
            #[cfg(feature = "computeruse_record")]
            usecase_recorder: usecase_recorder.clone(),
            #[cfg(feature = "computeruse_replay")]
            usecase_replay: usecase_replay.clone(),
            #[cfg(feature = "computeruse_editor")]
            usecase_editor: usecase_editor.clone(),
        };

        plug_ovr.llm_selector.lock().unwrap().load_model().await;

        plug_ovr.template_editor.load_templates();

        plug_ovr
    }
}
fn load_icon_from_memory(icon_data: &[u8]) -> tray_icon::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(icon_data)
            .expect("Failed to load icon from memory")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to create icon")
}
use std::sync::mpsc::{channel, Sender};
#[cfg(target_os = "linux")]
fn run_once_tray_icon_init(
    tray_icon: &mut Option<TrayIcon>,
    login_menu_item: &MenuItem,
    welcome_menu_item: &MenuItem,
    updater_menu_item: &MenuItem,
    menu_map: Arc<Mutex<Option<HashMap<String, String>>>>,
    menu_update_sender: Arc<Mutex<Option<Sender<MenuUpdate>>>>,
) {
    let hashmap_clone = menu_map.clone();

    let (tx, rx) = channel::<MenuUpdate>();
    *menu_update_sender.lock().unwrap() = Some(tx);

    std::thread::spawn(move || {
        gtk::init().unwrap();
        let login_menu_item = MenuItem::new("Login", true, None);
        let welcome_menu_item = MenuItem::new("Welcome", false, None);
        let updater_menu_item = MenuItem::new("Updater", false, None);
        let (tray_menu, icon, map_temp) =
            create_menu(&login_menu_item, &welcome_menu_item, &updater_menu_item);
        *hashmap_clone.lock().unwrap() = Some(map_temp);
        //  gtk::init().unwrap();
        let tray_icon = Some(
            TrayIconBuilder::new()
                .with_menu(Box::new(tray_menu.clone()))
                .with_icon(icon)
                .build()
                .unwrap(),
        );

        // Set up a timeout to check for menu updates
        use glib;
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(update) = rx.try_recv() {
                match update {
                    MenuUpdate::UpdateWelcome(text) => welcome_menu_item.set_text(text),
                    MenuUpdate::UpdateLogin(text, enabled) => {
                        login_menu_item.set_text(text);
                        login_menu_item.set_enabled(enabled);
                    }
                    MenuUpdate::UpdateUpdater(text, enabled) => {
                        updater_menu_item.set_text(text);
                        updater_menu_item.set_enabled(enabled);
                    }
                }
            }
            glib::ControlFlow::Continue
        });

        gtk::main();
    });
}

use tray_icon::menu::Menu;
use tray_icon::Icon;
fn create_menu(
    login_menu_item: &MenuItem,
    welcome_menu_item: &MenuItem,
    updater_menu_item: &MenuItem,
) -> (Menu, Icon, HashMap<String, String>) {
    let tray_menu = Menu::new();
    let icon_data = include_bytes!("../../assets/32x32.png");
    let icon_data_menu = {
        let image = image::load_from_memory(icon_data)
            .expect("Failed to load icon from memory")
            .into_rgba8();
        image.into_raw()
    };
    let icon = load_icon_from_memory(icon_data);

    let template_i = MenuItem::new("Template Editor", true, None);
    let usecase_editor_i = MenuItem::new("Usecase Editor", true, None);
    let llm_selector_i = MenuItem::new("LLM Selector", true, None);
    let quit_i = MenuItem::new("Quit", true, None);
    let about_icon = tray_icon::menu::Icon::from_rgba(icon_data_menu, 32, 32).unwrap();
    #[cfg(feature = "cs")]
    {
        _ = tray_menu.append_items(&[
            welcome_menu_item,
            login_menu_item,
        &PredefinedMenuItem::separator(),
        &llm_selector_i,

        &template_i,
        &usecase_editor_i,
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::about(
            Some("How to"),
            Some(AboutMetadata {
                icon: Some(about_icon.clone()),

                name: Some("How to use PlugOvr\n\n1. Select context\n\n2. Press Ctrl + Alt + I (full view Linux / Windows)\n2. Press Ctrl + I (full view MacOS)\n   or Ctrl + Space (shortcut view)\n\n3. Write instruction or use template\n\n4. Select Replace, Extend or Ignore\n\n5. Accept or Reject AI answer\n\n".to_string()),
                copyright: Some("https://plugovr.ai/howto".to_string()),
                ..Default::default()
            }),
        ),
        updater_menu_item,
        &PredefinedMenuItem::about(
            None,
            Some(AboutMetadata {
                name: Some("PlugOvr".to_string()),
                copyright: Some("Copyright PlugOvr.ai".to_string()),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
                website: Some("https://plugovr.ai".to_string()),
                authors: Some(vec!["Cornelius Wefelscheid".to_string()]),
                icon: Some(about_icon.clone()),
                ..Default::default()
            }),
        ),
        &PredefinedMenuItem::separator(),
            &quit_i,
        ]);
    }

    #[cfg(not(feature = "cs"))]
    {
        _ = tray_menu.append_items(&[
        &llm_selector_i,

        &template_i,
        &usecase_editor_i,
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::about(
            Some("How to"),
            Some(AboutMetadata {
                icon: Some(about_icon.clone()),

                name: Some("How to use PlugOvr\n\n1. Select context\n\n2. Press Ctrl + Alt + I (full view Linux / Windows)\n2. Press Ctrl + I (full view MacOS)\n   or Ctrl + Space (shortcut view)\n\n3. Write instruction or use template\n\n4. Select Replace, Extend or Ignore\n\n5. Accept or Reject AI answer\n\n".to_string()),
                copyright: Some("https://plugovr.ai/howto".to_string()),
                ..Default::default()
            }),
        ),
        updater_menu_item,
        &PredefinedMenuItem::about(
            None,
            Some(AboutMetadata {
                name: Some("PlugOvr".to_string()),
                copyright: Some("Copyright PlugOvr.ai".to_string()),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
                website: Some("https://plugovr.ai".to_string()),
                authors: Some(vec!["Cornelius Wefelscheid".to_string()]),
                icon: Some(about_icon.clone()),
                ..Default::default()
            }),
        ),
        &PredefinedMenuItem::separator(),
            &quit_i,
        ]);
    }

    let mut map = HashMap::new();
    map.insert(
        "LLM Selector".to_string(),
        llm_selector_i.id().0.to_string(),
    );
    map.insert("Template Editor".to_string(), template_i.id().0.to_string());
    map.insert("Login".to_string(), login_menu_item.id().0.to_string());
    map.insert("Quit".to_string(), quit_i.id().0.to_string());
    map.insert("Updater".to_string(), updater_menu_item.id().0.to_string());
    map.insert(
        "Usecase Editor".to_string(),
        usecase_editor_i.id().0.to_string(),
    );
    (tray_menu, icon, map)
}
#[cfg(not(target_os = "linux"))]
fn run_once_tray_icon_init(
    tray_icon: &mut Option<TrayIcon>,
    login_menu_item: &MenuItem,
    welcome_menu_item: &MenuItem,
    updater_menu_item: &MenuItem,
    menu_map: Arc<Mutex<Option<HashMap<String, String>>>>,
) {
    let hashmap_clone = menu_map.clone();
    let (tray_menu, icon, map_temp) =
        create_menu(login_menu_item, welcome_menu_item, updater_menu_item);
    *hashmap_clone.lock().unwrap() = Some(map_temp);

    *tray_icon = Some(
        TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu.clone()))
            .with_icon(icon)
            .build()
            .unwrap(),
    );

    #[cfg(target_os = "macos")]
    {
        unsafe {
            use core_foundation::runloop::{CFRunLoopGetMain, CFRunLoopWakeUp};

            let rl = CFRunLoopGetMain();
            CFRunLoopWakeUp(rl);
        }
    }
}

impl EguiOverlay for PlugOvr {
    fn gui_run(
        &mut self,
        egui_context: &egui::Context,
        _default_gfx_backend: &mut DefaultGfxBackend,
        glfw_backend: &mut egui_overlay::egui_window_glfw_passthrough::GlfwBackend,
    ) {
        if self.run_tray_icon {
            {
                #[cfg(not(target_os = "linux"))]
                run_once_tray_icon_init(
                    &mut self.tray_icon,
                    &self.login_menu_item,
                    &self.welcome_menu_item,
                    &self.updater_menu_item,
                    self.main_window.menu_map.clone(),
                );
                #[cfg(target_os = "linux")]
                run_once_tray_icon_init(
                    &mut self.tray_icon,
                    &self.login_menu_item,
                    &self.welcome_menu_item,
                    &self.updater_menu_item,
                    self.main_window.menu_map.clone(),
                    self.menu_update_sender.clone(),
                );
                self.run_tray_icon = false;
            }

            glfw_backend.set_title(self.title.clone());
            glfw_backend.window.set_pos(0, 0);
        }

        #[cfg(target_os = "windows")]
        {
            self.assistance_window.scale = glfw_backend.scale;
        }

        // Set the window size to cover all screens
        glfw_backend.set_window_size([self.screen_width as f32, self.screen_height as f32]);

        self.main_window.show(egui_context);

        egui_context.set_visuals(egui::Visuals::light());

        let current_user_info = self
            .main_window
            .user_info
            .lock()
            .expect("Failed to lock user_info POISON")
            .clone();
        let is_logged_in = current_user_info.is_some();
        let is_loading = *self
            .main_window
            .is_loading_user_info
            .lock()
            .expect("Failed to lock is_loading_user_info POISON");
        if self.main_window.version_msg.lock().unwrap().to_string()
            != *self.main_window.version_msg_old.lock().unwrap()
        {
            if let Some(sender) = self.menu_update_sender.lock().unwrap().as_ref() {
                _ = sender.send(MenuUpdate::UpdateUpdater(
                    self.main_window.version_msg.lock().unwrap().clone(),
                    true,
                ));
            }

            self.updater_menu_item.set_enabled(true);
            self.updater_menu_item
                .set_text(self.main_window.version_msg.lock().unwrap().clone());
            *self.main_window.version_msg_old.lock().unwrap() =
                self.main_window.version_msg.lock().unwrap().clone();
        }
        // Only update if state has changed
        if self.last_login_state != Some(is_logged_in)
            || self.last_loading_state != Some(is_loading)
        {
            if is_logged_in {
                self.login_menu_item.set_text("Logout");
                self.login_menu_item.set_enabled(true);

                if let Some(sender) = self.menu_update_sender.lock().unwrap().as_ref() {
                    let _ = sender.send(MenuUpdate::UpdateLogin("Logout".to_string(), true));
                }

                if let Some(user_info) = current_user_info.as_ref() {
                    let greeting = if let Some(nickname) = &user_info.nickname {
                        format!("Hello, {}!", nickname)
                    } else if let Some(name) = &user_info.name {
                        format!("Hello, {}!", name)
                    } else {
                        format!("Hello, {}!", user_info.email)
                    };

                    // Only update greeting if it changed

                    self.welcome_menu_item.set_text(greeting.clone());
                    if let Some(sender) = self.menu_update_sender.lock().unwrap().as_ref() {
                        _ = sender.send(MenuUpdate::UpdateWelcome(greeting.clone()));
                    }
                }
            } else if is_loading {
                self.login_menu_item.set_text("user info loading...");
                self.login_menu_item.set_enabled(false);
                if let Some(sender) = self.menu_update_sender.lock().unwrap().as_ref() {
                    let _ = sender.send(MenuUpdate::UpdateLogin(
                        "user info loading...".to_string(),
                        false,
                    ));
                }
            } else {
                self.welcome_menu_item.set_text("Login to use the cloud");
                self.login_menu_item.set_text("Login");
                self.login_menu_item.set_enabled(true);
                if let Some(sender) = self.menu_update_sender.lock().unwrap().as_ref() {
                    _ = sender.send(MenuUpdate::UpdateLogin("Login".to_string(), true));
                    _ = sender.send(MenuUpdate::UpdateWelcome(
                        "Login to use the cloud".to_string(),
                    ));
                }
            }

            // Update state tracking
            self.last_login_state = Some(is_logged_in);
            self.last_loading_state = Some(is_loading);
        }

        if *self
            .shortcut_window
            .lock()
            .expect("Failed to lock shortcut_window POISON")
        {
            self.show_shortcut_window(egui_context, self.assistance_window.scale);
        }
        if *self
            .text_entry
            .lock()
            .expect("Failed to lock text_entry POISON")
            || self.assistance_window.screenshot_mode
        {
            self.assistance_window
                .show(egui_context, self.assistance_window.scale);
            //egui_context.request_repaint_after(std::time::Duration::from_millis(100));
        } else if !self.assistance_window.screenshot_mode
            && !*self
                .shortcut_window
                .lock()
                .expect("Failed to lock shortcut_window POISON")
            && !self.assistance_window.shortcut_clicked
        {
            self.assistance_window.text_entry_changed = true;
        }

        #[cfg(feature = "computeruse_record")]
        {
            if self.usecase_recorder.lock().unwrap().show {
                self.usecase_recorder
                    .lock()
                    .expect("Failed to lock usecase_recorder POISON")
                    .show_window(egui_context);
            }
        }

        #[cfg(feature = "computeruse_replay")]
        {
            if self.usecase_replay.lock().unwrap().show {
                self.usecase_replay
                    .lock()
                    .unwrap()
                    .visualize_next_step(egui_context);
                self.usecase_replay
                    .lock()
                    .unwrap()
                    .visualize_planning(egui_context);
            }
            if self.usecase_replay.lock().unwrap().show_dialog {
                self.usecase_replay
                    .lock()
                    .unwrap()
                    .show_dialog(egui_context);
            }
        }

        // here you decide if you want to be passthrough or not.
        // TODO: add logic based on mouse cursor if passthrough is needed.
        if egui_context.wants_pointer_input() || egui_context.wants_keyboard_input() {
            // we need input, so we need the window to be NOT passthrough
            glfw_backend.set_passthrough(false);
        } else {
            // we don't care about input, so the window can be passthrough now
            glfw_backend.set_passthrough(true)
        }
        egui_context.set_fonts(self.fonts.clone());
        egui_context.request_repaint_after(std::time::Duration::from_millis(100));

        if !self.assistance_window.show {
            *self.ai_answer.lock().unwrap() = String::new();
        }


    }
}
