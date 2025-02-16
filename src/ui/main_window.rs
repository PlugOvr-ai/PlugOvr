use crate::llm::LLMSelector;
use crate::ui::template_editor::TemplateEditor;
use crate::ui::template_editor::TemplateMap;

#[cfg(feature = "cs")]
use plugovr_cs::login_window::LoginWindow;

use plugovr_types::UserInfo;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use webbrowser;

#[cfg(feature = "computeruse_editor")]
use crate::usecase_editor::UsecaseEditor;

pub struct MainWindow {
    #[cfg(feature = "cs")]
    login_window: LoginWindow,
    template_editor: TemplateEditor,
    window_pos_initialized: bool,
    pub user_info: Arc<Mutex<Option<UserInfo>>>,
    pub is_loading_user_info: Arc<Mutex<bool>>,
    pub version_msg: Arc<Mutex<String>>,
    pub version_msg_old: Arc<Mutex<String>>,
    pub llm_selector: Arc<Mutex<LLMSelector>>,
    show_template_editor: Arc<Mutex<bool>>,
    show_llm_selector: Arc<Mutex<bool>>,
    show_login_window: Arc<Mutex<bool>>,
    pub menu_map: Arc<Mutex<Option<HashMap<String, String>>>>,
    #[cfg(feature = "computeruse_editor")]
    show_usecase_editor: Arc<Mutex<bool>>,
    #[cfg(feature = "computeruse_editor")]
    usecase_editor: Arc<Mutex<UsecaseEditor>>,
}
impl MainWindow {
    pub fn new(
        user_info: Arc<Mutex<Option<UserInfo>>>,
        is_loading_user_info: Arc<Mutex<bool>>,
        prompt_templates: TemplateMap,
        llm_selector: Arc<Mutex<LLMSelector>>,
        version_msg: Arc<Mutex<String>>,
        #[cfg(feature = "computeruse_editor")]
        usecase_editor: Arc<Mutex<UsecaseEditor>>,
    ) -> Self {
        use tray_icon::menu::MenuEvent;
        let show_login_window = Arc::new(Mutex::new(false));
        let show_template_editor = Arc::new(Mutex::new(false));
        let show_llm_selector = Arc::new(Mutex::new(false));
        #[cfg(feature = "computeruse_editor")]
        let show_usecase_editor = Arc::new(Mutex::new(false));
        #[cfg(feature = "cs")]
        let login_window = LoginWindow::new(user_info.clone(), is_loading_user_info.clone());
        let template_editor = TemplateEditor::new(prompt_templates.clone());
        let menu_map = Arc::new(Mutex::new(Option::<HashMap<String, String>>::None));
        let menu_channel = MenuEvent::receiver();

        {
            let show_login_window = show_login_window.clone();
            let show_template_editor = show_template_editor.clone();
            let show_llm_selector = show_llm_selector.clone();
            let user_info = user_info.clone();
            let menu_map = menu_map.clone();
            #[cfg(feature = "computeruse_editor")]
            let show_usecase_editor = show_usecase_editor.clone();
            std::thread::spawn(move || {
                while let Ok(recv) = menu_channel.recv() {
                    let id = recv.id().0.to_string();
                    let menu_map = menu_map.lock().unwrap().clone();
                    if let Some(menu_map) = menu_map {
                        #[cfg(feature = "cs")]
                        if id == *menu_map.get("Login").unwrap_or(&"".to_string()) {
                            if user_info.lock().unwrap().is_some() {
                                *user_info.lock().unwrap() = None;
                                _ = plugovr_cs::user_management::logout();
                            } else {
                                *show_login_window.lock().unwrap() = true;
                            }
                        }
                        #[cfg(feature = "computeruse_editor")]
                        if id == *menu_map.get("Usecase Editor").unwrap_or(&"".to_string()) {
                            println!("Usecase Editor");
                            *show_usecase_editor.lock().unwrap() = true;
                        }
                        if id == *menu_map.get("Template Editor").unwrap_or(&"".to_string()) {
                            println!("Template Editor");
                            *show_template_editor.lock().unwrap() = true;
                        }
                        if id == *menu_map.get("LLM Selector").unwrap_or(&"".to_string()) {
                            println!("LLM Selector");
                            *show_llm_selector.lock().unwrap() = true;
                        }
                        if id == *menu_map.get("Updater").unwrap_or(&"".to_string()) {
                            let _ = webbrowser::open("https://plugovr.ai/download").is_ok();
                        }
                        if id == *menu_map.get("Quit").unwrap_or(&"".to_string()) {
                            println!("Quit");
                            std::process::exit(0);
                        }
                    }
                }
            });
        }

        Self {
            #[cfg(feature = "cs")]
            login_window,
            template_editor,
            window_pos_initialized: false,
            user_info: user_info.clone(),
            is_loading_user_info,
            version_msg,
            version_msg_old: Arc::new(Mutex::new("".to_string())),
            llm_selector,
            show_template_editor,
            show_llm_selector,
            show_login_window,
            menu_map,
            #[cfg(feature = "computeruse_editor")]
            show_usecase_editor,
            #[cfg(feature = "computeruse_editor")]
            usecase_editor,
        }
    }
    pub fn show(&mut self, egui_context: &egui::Context) {
        #[cfg(feature = "cs")]
        if *self.show_login_window.lock().unwrap() {
            self.login_window.show_login_window = true;
            *self.show_login_window.lock().unwrap() = false;
        }
        #[cfg(feature = "cs")]
        if self.login_window.show_login_window {
            self.login_window.show(egui_context);
        }
        if *self.show_template_editor.lock().unwrap() {
            println!("Show Template Editor");
            self.template_editor.show = true;
            *self.show_template_editor.lock().unwrap() = false;
        }
        if self.template_editor.show {
            self.template_editor.show_template_editor(egui_context);
        }
        #[cfg(feature = "computeruse_editor")]
        if *self.show_usecase_editor.lock().unwrap() {
            self.usecase_editor.lock().unwrap().show(egui_context);
        }
        if *self.show_llm_selector.lock().unwrap() {
            self.llm_selector.lock().unwrap().toggle_window();
            *self.show_llm_selector.lock().unwrap() = false;
        }

        self.llm_selector
            .lock()
            .expect("Failed to lock llm_selector POISON")
            .show_selection_window(egui_context);
    }
}
