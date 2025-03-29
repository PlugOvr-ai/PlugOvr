// Add these imports at the top of the file
use kalosm::language::*;
#[cfg(feature = "cs")]
use plugovr_cs::cloud_llm::call_aws_lambda;
use plugovr_types::{Screenshots, UserInfo};
use std::error::Error;
use std::io::Write;
use std::sync::{Arc, Mutex};

#[cfg(feature = "cs")]
use plugovr_cs::user_management::get_user;

use egui::{Context, Window};

use image_24::{ImageBuffer, Rgba};
use ollama_rs::{
    Ollama,
    generation::chat::{ChatMessage, MessageRole, request::ChatMessageRequest},
    generation::images::Image,
    generation::options::GenerationOptions,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::File;
use std::io::Read;
use strum::EnumIter;

async fn call_ollama(
    ollama: Arc<Mutex<Option<Ollama>>>,
    model: String,
    input: String,
    _context: String,
    _instruction: String,
    ai_answer: Arc<Mutex<String>>,
    screenshots: &Screenshots,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    use base64::{Engine as _, engine::general_purpose};

    let screenshot_base64 = screenshots
        .iter()
        .map(|img| {
            let mut buf = vec![];
            img.0
                .write_to(
                    &mut std::io::Cursor::new(&mut buf),
                    image_24::ImageOutputFormat::Png,
                )
                .unwrap();
            general_purpose::STANDARD.encode(&buf)
        })
        .collect::<Vec<String>>();
    let images = screenshot_base64
        .iter()
        .map(|base64| Image::from_base64(base64))
        .collect::<Vec<Image>>();

    let options = GenerationOptions::default()
        .temperature(0.2)
        .repeat_penalty(1.1)
        .top_k(40)
        .top_p(0.5)
        .num_predict(500);

    // let mut stream = ollama
    //     .as_ref()
    //     .lock()
    //     .unwrap()
    //     .as_ref()
    //     .unwrap()
    //     .generate_stream(
    //         GenerationRequest::new(model, input)
    //             .options(options)
    //             .images(images),
    //     )
    //     .await
    //     .unwrap();
    let message = ChatMessage::new(MessageRole::User, input.to_string());
    let messages = vec![message.clone().with_images(images)];
    let ollama_instance = {
        let guard = ollama.lock().unwrap();
        guard.as_ref().unwrap().clone()
    };

    let stream = ollama_instance
        .send_chat_messages_stream(ChatMessageRequest::new(model, messages).options(options))
        .await;
    let mut response = String::new();
    match stream {
        Err(e) => {
            *ai_answer.lock().unwrap() = format!("Error: {}", e);
            response = format!("Error: {}", e);
            Ok(response)
        }
        Ok(mut stream) => {
            while let Some(Ok(res)) = stream.next().await {
                let assistant_message = res.message;
                response += assistant_message.content.as_str();
                *ai_answer.lock().unwrap() = response.clone();
            }
            Ok(response)
        }
    }
}
async fn call_local_llm(
    _input: String,
    context: String,
    instruction: String,
    ai_answer: Arc<Mutex<String>>,
    model: Arc<Mutex<Option<Llama>>>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let model = model.try_lock();
    if model.is_err() {
        eprintln!("Model is not locked");
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Model is not locked",
        )));
    }

    let prompt = format!("Context: {} Instruction: {}", context, instruction);

    let model_instance = {
        let mut guard = model.unwrap();
        guard.as_mut().unwrap().clone()
    };
    let mut stream = model_instance(&prompt);

    let mut response = String::new();

    while let Some(token) = stream.next().await {
        response.push_str(&token);

        // Update ai_answer with the current response
        *ai_answer.lock().unwrap() = response.clone();
    }
    Ok(response)
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum LLMType {
    Cloud(CloudModel),
    Local(LocalModel),
    Ollama(String),
}
use strum::IntoEnumIterator;

#[derive(Clone, Copy, PartialEq, EnumIter, Serialize, Deserialize)]
pub enum CloudModel {
    AnthropicHaiku,
    AnthropicSonnet3_5,
}
impl CloudModel {
    pub fn description(&self) -> String {
        match self {
            CloudModel::AnthropicHaiku => "Anthropic Haiku".to_string(),
            CloudModel::AnthropicSonnet3_5 => "Anthropic Sonnet 3.5".to_string(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, EnumIter, Serialize, Deserialize)]
pub enum LocalModel {
    Llama32S1bChat,
    Llama32S3bChat,
}
impl LocalModel {
    pub fn description(&self) -> String {
        match self {
            LocalModel::Llama32S1bChat => "Llama 3.2 1B Chat".to_string(),
            LocalModel::Llama32S3bChat => "Llama 3.2 3B Chat".to_string(),
        }
    }
}
impl LLMType {
    pub fn description(&self) -> String {
        match self {
            LLMType::Cloud(cloud_model) => format!("{} - Cloud", cloud_model.description()),
            LLMType::Local(local_model) => format!("{} - Local", local_model.description()),
            LLMType::Ollama(model) => format!("Ollama - {}", model),
        }
    }
}
impl LocalModel {
    pub fn source(&self) -> LlamaSource {
        match self {
            LocalModel::Llama32S1bChat => LlamaSource::llama_3_2_1b_chat(),
            LocalModel::Llama32S3bChat => LlamaSource::llama_3_2_3b_chat(),
        }
    }
}

pub struct LLMSelector {
    llm_type: LLMType,
    model: Arc<Mutex<Option<Llama>>>,
    show_window: bool,
    download_progress: Arc<Mutex<f32>>,
    download_error: Arc<Mutex<Option<String>>>,
    pub user_info: Arc<Mutex<Option<UserInfo>>>,
    ollama: Arc<Mutex<Option<Ollama>>>,
    pub ollama_models: Arc<Mutex<Option<Vec<ollama_rs::models::LocalModel>>>>,
}

impl LLMSelector {
    pub fn new(user_info: Arc<Mutex<Option<UserInfo>>>) -> Self {
        let llm_type = load_llm_type().unwrap_or(LLMType::Cloud(CloudModel::AnthropicHaiku));
        let ollama = Ollama::default();
        let ollama_models = Arc::new(Mutex::new(None));
        {
            let ollama_models = ollama_models.clone();
            tokio::task::spawn(async move {
                if let Ok(models) = ollama.list_local_models().await {
                    *ollama_models.lock().unwrap() = Some(models);
                }
            });
        }
        let ollama = Ollama::default();
        LLMSelector {
            llm_type,
            model: Arc::new(Mutex::new(None)),
            show_window: false,
            download_progress: Arc::new(Mutex::new(0.0)),
            download_error: Arc::new(Mutex::new(None)),
            user_info,
            ollama: Arc::new(Mutex::new(Some(ollama))),
            ollama_models,
        }
    }

    pub async fn load_model(&self) {
        let mut model = self.model.lock().unwrap();
        *model = match &self.llm_type {
            LLMType::Local(LocalModel::Llama32S1bChat) => Some(
                Llama::builder()
                    .with_source(LlamaSource::llama_3_2_1b_chat())
                    .build()
                    .await
                    .unwrap(),
            ),
            LLMType::Local(LocalModel::Llama32S3bChat) => Some(
                Llama::builder()
                    .with_source(LlamaSource::llama_3_2_3b_chat())
                    .build()
                    .await
                    .unwrap(),
            ),
            LLMType::Cloud(CloudModel::AnthropicHaiku) => None,
            LLMType::Cloud(CloudModel::AnthropicSonnet3_5) => None,
            LLMType::Ollama(_model) => None,
        };
    }

    pub fn process_input(
        &self,
        prompt: String,
        context: String,
        screenshots: Vec<(ImageBuffer<Rgba<u8>, Vec<u8>>, egui::Pos2)>,
        instruction: String,
        ai_answer: Arc<Mutex<String>>,
        max_tokens_reached: Arc<Mutex<bool>>,
        spinner: Arc<Mutex<bool>>,
        llm_from_template: Option<LLMType>,
    ) -> Result<tokio::task::JoinHandle<()>, Box<dyn Error + Send + Sync>> {
        let mut llm_type = self.llm_type.clone();
        if let Some(llm_from_template) = llm_from_template {
            llm_type = llm_from_template;
        }

        let model = self.model.clone();
        let spinner_clone = spinner.clone();
        let user_info = self.user_info.clone();
        if (llm_type == LLMType::Cloud(CloudModel::AnthropicHaiku)
            || llm_type == LLMType::Cloud(CloudModel::AnthropicSonnet3_5))
            && user_info.lock().unwrap().is_none()
        {
            *ai_answer.lock().unwrap() =
                "Please login to use cloud LLM or switch to local LLM".to_string();
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Please login to use cloud LLM or switch to local LLM",
            )));
        }
        let ollama = self.ollama.clone();

        let handle = tokio::task::spawn_blocking(move || {
            *spinner_clone.lock().unwrap() = true;
            let llm_type_clone = llm_type.clone();
            let result: Result<(String, bool), Box<dyn Error + Send + Sync>> = match llm_type {
                LLMType::Cloud(CloudModel::AnthropicHaiku)
                | LLMType::Cloud(CloudModel::AnthropicSonnet3_5) => {
                    let user_info = user_info.lock().unwrap().as_ref().unwrap().clone();
                    let model = llm_type.clone().to_string();
                    #[cfg(feature = "cs")]
                    {
                        Ok(call_aws_lambda(
                            user_info,
                            prompt.clone(),
                            model,
                            &screenshots,
                        ))
                    }
                    #[cfg(not(feature = "cs"))]
                    {
                        Ok((
                            "Download PlugOvr from https://plugovr.ai to use cloud LLM".to_string(),
                            false,
                        ))
                    }
                }
                LLMType::Local(_) => {
                    use tokio::runtime::Runtime;
                    let rt = Runtime::new().unwrap();
                    rt.block_on(async {
                        let result = call_local_llm(
                            prompt.clone(),
                            context,
                            instruction,
                            ai_answer.clone(),
                            model,
                        )
                        .await;
                        Ok((result?, false))
                    })
                }
                LLMType::Ollama(model) => {
                    use tokio::runtime::Runtime;
                    let rt = Runtime::new().unwrap();
                    let model_clone = model.clone();
                    rt.block_on(async {
                        let result = call_ollama(
                            ollama,
                            model_clone,
                            prompt.clone(),
                            context,
                            instruction,
                            ai_answer.clone(),
                            &screenshots,
                        )
                        .await;
                        Ok((result?, false))
                    })
                }
            };

            let mut result = result.unwrap();
            #[cfg(feature = "cs")]
            if result.0.contains("Access token expired") {
                match get_user() {
                    Ok(user_info_tmp) => {
                        *user_info.lock().unwrap() = Some(user_info_tmp);

                        if llm_type_clone == LLMType::Cloud(CloudModel::AnthropicHaiku)
                            || llm_type_clone == LLMType::Cloud(CloudModel::AnthropicSonnet3_5)
                        {
                            let user_info = user_info.lock().unwrap().as_ref().unwrap().clone();
                            let model = llm_type_clone.clone().to_string();
                            result = call_aws_lambda(user_info, prompt, model, &screenshots);
                        }
                    }
                    Err(_e) => {
                        *user_info.lock().unwrap() = None;
                    }
                }
            }
            *ai_answer.lock().unwrap() = result.0;
            *max_tokens_reached.lock().unwrap() = result.1;
            *spinner.lock().unwrap() = false;
        });
        Ok(handle)
    }

    pub fn show_selection_window(&mut self, ctx: &Context) {
        Window::new("LLM Selection")
            .open(&mut self.show_window)
            .collapsible(false)
            .max_width(400.0)
            .show(ctx, |ui| {
                ui.heading("Cloud Models");
                let cloud_models = CloudModel::iter().collect::<Vec<_>>();
                for cloud_model in cloud_models {
                    if ui
                        .radio_value(
                            &mut self.llm_type,
                            LLMType::Cloud(cloud_model),
                            LLMType::Cloud(cloud_model).description(),
                        )
                        .changed()
                    {
                        save_llm_type(LLMType::Cloud(cloud_model))
                            .unwrap_or_else(|e| eprintln!("Failed to save LLM type: {}", e));
                    }
                }

                ui.heading("Local Models");
                for local_model in LocalModel::iter() {
                    let requires_download = Llama::builder()
                        .with_source(local_model.source())
                        .requires_download();

                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !requires_download,
                                egui::RadioButton::new(
                                    self.llm_type == LLMType::Local(local_model),
                                    LLMType::Local(local_model).description(),
                                ),
                            )
                            .clicked()
                        {
                            self.llm_type = LLMType::Local(local_model);
                            save_llm_type(LLMType::Local(local_model))
                                .unwrap_or_else(|e| eprintln!("Failed to save LLM type: {}", e));

                            let model = self.model.clone();
                            let llama_source = local_model.source();

                            tokio::spawn(async move {
                                let llama = Llama::builder()
                                    .with_source(llama_source)
                                    .build()
                                    .await
                                    .unwrap();
                                if let Ok(mut model_guard) = model.lock() {
                                    *model_guard = Some(llama);
                                } else {
                                    eprintln!("Failed to acquire lock on model");
                                }
                            });
                        }
                        if !requires_download {
                            ui.label("Downloaded");
                        } else if ui.button("Download").clicked() {
                            let llama_source = local_model.source();
                            let download_progress = self.download_progress.clone();
                            let download_error = self.download_error.clone();
                            tokio::spawn(async move {
                                let download_progress = download_progress.clone();
                                let _llama = match Llama::builder()
                                    .with_source(llama_source)
                                    .build_with_loading_handler(move |x| match x.clone() {
                                        ModelLoadingProgress::Downloading { .. } => {
                                            *download_progress.lock().unwrap() = x.progress()
                                        }
                                        ModelLoadingProgress::Loading { progress } => {
                                            *download_progress.lock().unwrap() = progress;
                                        }
                                    })
                                    .await
                                {
                                    Ok(llama) => llama,
                                    Err(e) => {
                                        eprintln!("Failed to download/load model: {}", e);
                                        *download_error.lock().unwrap() = Some(e.to_string());
                                        return;
                                    }
                                };
                            });
                        }
                    });
                }
                if let Some(ollama_models) = self.ollama_models.lock().unwrap().as_ref() {
                    ui.heading("Ollama Models");
                    let ollama_models = ollama_models.clone();
                    if ollama_models.is_empty() {
                        ui.label(
                            "No ollama models found, pull some models with e.g. ollama pull llama3.2:1b"
                        );
                    } else {
                        for ollama_model in ollama_models {
                            if ui
                            .radio_value(
                                &mut self.llm_type,
                                LLMType::Ollama(ollama_model.name.clone()),
                                LLMType::Ollama(ollama_model.name.clone()).description(),
                            )
                            .changed()
                            {
                                save_llm_type(LLMType::Ollama(ollama_model.name.clone()))
                                    .unwrap_or_else(|e| eprintln!("Failed to save LLM type: {}", e));
                            }
                        }
                    }
                } else {
                    ui.heading("Ollama not installed");
                }

                if *self.download_progress.lock().unwrap() > 0.0 {
                    ui.label("Download progress");
                    ui.add(
                        egui::ProgressBar::new(*self.download_progress.lock().unwrap())
                            .show_percentage(),
                    );
                }
                if let Some(error) = self.download_error.lock().unwrap().as_ref() {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
            });
    }

    pub fn toggle_window(&mut self) {
        self.show_window = !self.show_window;
    }

    pub fn get_llm_type(&self) -> LLMType {
        self.llm_type.clone()
    }
}
// Add this new function to save the LLMType
fn save_llm_type(llm_type: LLMType) -> std::io::Result<()> {
    let mut path = dirs::home_dir().expect("Unable to get home directory");
    path.push(".plugovr");
    std::fs::create_dir_all(&path)?;
    path.push("llm_type.json");

    let serialized = serde_json::to_string(&llm_type)?;
    let mut file = File::create(path)?;
    file.write_all(serialized.as_bytes())?;
    Ok(())
}

// Add this new function to load the LLMType
fn load_llm_type() -> std::io::Result<LLMType> {
    let mut path = dirs::home_dir().expect("Unable to get home directory");
    path.push(".plugovr");
    path.push("llm_type.json");

    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let llm_type: LLMType = serde_json::from_str(&contents)?;
    Ok(llm_type)
}

impl fmt::Display for LLMType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LLMType::Cloud(CloudModel::AnthropicHaiku) => write!(f, "AnthropicHaiku"),
            LLMType::Cloud(CloudModel::AnthropicSonnet3_5) => write!(f, "AnthropicSonnet3_5"),
            LLMType::Local(LocalModel::Llama32S1bChat) => write!(f, "Llama32S1bChat"),
            LLMType::Local(LocalModel::Llama32S3bChat) => write!(f, "Llama32S3bChat"),
            LLMType::Ollama(model) => write!(f, "{}", model),
        }
    }
}
