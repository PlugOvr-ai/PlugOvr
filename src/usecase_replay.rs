use crate::llm::LLMSelector;
use crate::usecase_recorder::EventType;
use crate::usecase_recorder::UseCase;

use egui_overlay::egui_render_three_d::{
    three_d::{ColorMaterial, Gm, Mesh},
    ThreeDBackend,
};

#[cfg(target_os = "linux")]
use gtk::false_;
use image::{ImageBuffer, Rgba};
use json_fixer::JsonFixer;
use openai_dive::v1::api::Client;

use openai_dive::v1::resources::chat::{
    ChatCompletionParametersBuilder, ChatMessage, ChatMessageContent, ChatMessageContentPart,
    ChatMessageImageContentPart, ImageUrlType,
};
#[cfg(feature = "cs")]
use plugovr_cs::cloud_llm::call_aws_lambda;
use rdev::simulate;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;

use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;
use xcap::Monitor;

pub struct UseCaseReplay {
    pub index_instruction: Arc<Mutex<usize>>,
    pub index_action: Arc<Mutex<usize>>,
    pub vec_instructions: Arc<Mutex<Vec<UseCaseActions>>>,
    pub monitor1: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub monitor2: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub monitor3: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub show: bool,
    pub show_dialog: bool,
    pub llm_selector: Option<Arc<Mutex<LLMSelector>>>,
    instruction_dialog: String,
    pub computing_action: Arc<Mutex<bool>>,
    pub computing_plan: Arc<Mutex<bool>>,
    pub server_url_planning: String,
    pub server_url_execution: String,
    pub image_width: u32,
    pub image_height: u32,
    pub recorded_usecases: Vec<UseCaseActions>,
    pub monitor: Option<Vec<Monitor>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ActionTypes {
    Click(String),
    ClickPosition(f32, f32),
    InsertText(String),
    KeyDown(String),
    KeyUp(String),
    KeyPress(String),
    GrabScreenshot,
    Replan,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct UseCaseActions {
    pub instruction: String,
    pub actions: Vec<ActionTypes>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StepFormat {
    SingleStep {
        instruction: String,
        #[serde(rename = "actions")]
        actions: Vec<Action>,
    },
    MultiStep(Vec<Step>),
}

#[derive(Deserialize)]
struct Step {
    instruction: Option<String>,
    actions: Option<Vec<Action>>,
}

#[derive(Deserialize)]
struct Action {
    #[serde(rename = "type")]
    action_type: String,
    value: String,
}

impl From<Action> for ActionTypes {
    fn from(action: Action) -> Self {
        match action.action_type.as_str() {
            "Click" => ActionTypes::Click(action.value),
            "InsertText" => ActionTypes::InsertText(action.value),
            "KeyPress" => ActionTypes::KeyPress(action.value),
            _ => ActionTypes::Click(action.value),
        }
    }
}

impl UseCaseReplay {
    pub fn new() -> Self {
        let server_url_planning =
            load_server_url_planning().unwrap_or("http://127.0.0.1:8000/v1".to_string());
        let server_url_execution =
            load_server_url_execution().unwrap_or("http://127.0.0.1:8000/v1".to_string());
        Self {
            index_instruction: Arc::new(Mutex::new(0)),
            index_action: Arc::new(Mutex::new(0)),
            vec_instructions: Arc::new(Mutex::new(vec![])),
            monitor1: None,
            monitor2: None,
            monitor3: None,
            show: false,
            show_dialog: false,
            llm_selector: None,
            instruction_dialog: "".to_string(),
            computing_action: Arc::new(Mutex::new(false)),
            computing_plan: Arc::new(Mutex::new(false)),
            server_url_planning,
            server_url_execution,
            image_width: 0,
            image_height: 0,
            recorded_usecases: vec![],
            monitor: None,
        }
    }
    pub fn show_dialog(&mut self, egui_context: &egui::Context) {
        if !self.show_dialog {
            return;
        }
        let mut instruction_dialog = self.instruction_dialog.clone();
        let index_instruction = self.index_instruction.clone();
        let index_action = self.index_action.clone();
        let mut show_dialog = self.show_dialog;
        let window = egui::Window::new("UseCaseReplay")
            .movable(true)
            .drag_to_scroll(true)
            .interactable(true)
            .title_bar(true)
            .open(&mut show_dialog)
            .collapsible(false);
        window.show(egui_context, |ui| {
            ui.add(egui::Label::new("Agent Instructions"));
            ui.add(egui::TextEdit::multiline(&mut instruction_dialog));

            if ui.button("Run").clicked() {
                *index_instruction.lock().unwrap() = 0;
                *index_action.lock().unwrap() = 0;

                self.execute_usecase(instruction_dialog.clone());
                self.show_dialog = false;
                self.show = true;
            }
            ui.add(egui::Label::new("Server URL Planning"));
            if ui
                .add(egui::TextEdit::singleline(&mut self.server_url_planning))
                .changed()
            {
                let _ = save_server_url_planning(&self.server_url_planning);
            }
            ui.add(egui::Label::new("Server URL Execution"));
            if ui
                .add(egui::TextEdit::singleline(&mut self.server_url_execution))
                .changed()
            {
                let _ = save_server_url_execution(&self.server_url_execution);
            }
        });
        if self.show_dialog {
            self.show_dialog = show_dialog;

            self.instruction_dialog = instruction_dialog;
        }
    }

    pub fn load_usecase(&mut self, filename: String) {
        let file = File::open(filename).unwrap();
        let usecase: UseCase = serde_json::from_reader(file).unwrap();
        //println!("usecase: {:?}", usecase);

        let mut usecase_actions = UseCaseActions {
            instruction: usecase.usecase_instructions,
            actions: vec![],
        };

        for step in usecase.usecase_steps.iter() {
            match step {
                EventType::Click(_, instruction) => {
                    usecase_actions
                        .actions
                        .push(ActionTypes::Click(instruction.clone()));
                }
                EventType::KeyDown(instruction) => {
                    usecase_actions
                        .actions
                        //.push(ActionTypes::KeyDown(instruction.clone()));
                        .push(ActionTypes::KeyPress(instruction.clone()));
                }
                // EventType::KeyUp(instruction) => {
                //     usecase_actions
                //         .actions
                //         .push(ActionTypes::KeyUp(instruction.clone()));
                // }
                EventType::Text(instruction) => {
                    usecase_actions
                        .actions
                        .push(ActionTypes::InsertText(instruction.clone()));
                }
                _ => {}
            }
        }

        self.recorded_usecases.push(usecase_actions);
    }

    pub fn generate_usecase_actions(&mut self, instruction: &str) {
        let instruction = instruction.to_string();
        self.grab_screenshot();
        let monitor1: Option<ImageBuffer<Rgba<u8>, Vec<u8>>> = self.monitor1.clone();

        let vec_instructions = self.vec_instructions.clone();
        *self.index_instruction.lock().unwrap() = 0;
        *self.index_action.lock().unwrap() = 0;
        *self.computing_plan.lock().unwrap() = true;
        let computing_plan = self.computing_plan.clone();
        let server_url_planning = self.server_url_planning.clone();
        let server_url_execution = self.server_url_execution.clone();
        // Relies on OPENAI_KEY and optionally OPENAI_BASE_URL.

        let examplejson = r#"{
            "instruction": "Write an email to Cornelius",
            "actions": [
              {
                "type": "Click",
                "value": "Click on the 'Google Chrome' icon."
              },
              {
                "type": "Click",
                "value": "Click on the search bar."
              },
              {
                "type": "InsertText",
                "value": "www.gmail.com"
              },
              {
                "type": "KeyPress",
                "value": "Return"
              },
              {
                "type": "Click",
                "value": "Click on 'Schreiben'."
              },
              {
                "type": "Click",
                "value": "Click on 'An'."
              },
              {
                "type": "InsertText",
                "value": "info@plugovr.ai"
              },
              {
                "type": "KeyPress",
                "value": "Return"
              },
              {
                "type": "Click",
                "value": "Click on 'Betreff'."
              },
              {
                "type": "InsertText",
                "value": "Hi"
              },
              {
                "type": "Click",
                "value": "Click on main message field."
              },
              {
                "type": "KeyPress",
                "value": "Home"
              },
              {
                "type": "KeyPress",
                "value": "PageUp"
              },
              {
                "type": "InsertText",
                "value": "Hi Cornelius"
              },
              {
                "type": "Click",
                "value": "Click on 'Senden'."
              }
            ]
          }"#;

        let example_calendar_json = r#"{
            "instruction": "Add a new event to the calendar 'Plugovr Meeting'",
            "actions": [
              {
                "type": "Click",
                "value": "Click on the 'Google Chrome' icon."
              },
              {
                "type": "Click",
                "value": "Click on the search bar."
              },
              {
                "type": "InsertText",
                "value": "calendar.google.com"
              },
              {
                "type": "KeyPress",
                "value": "Return"
              },
              {
                "type": "Click",
                "value": "Click on 'Eintragen'."
              },
              {
                "type": "Click",
                "value": "Click on 'Termin'."
              },
              {
                "type": "Click",
                "value": "Click on 'Titel hinzufügen'."
              },              
              {
                "type": "InsertText",
                "value": "Plugovr Meeting"
              },
              {
                "type": "Click",
                "value": "Click on 'Termin'."
              },
              {
                "type": "InsertText",
                "value": "2025-03-01"
              },
              {
                "type": "Click",
                "value": "Click on Starttime."
              },
              {
                "type": "InsertText",
                "value": "10:00"
              },
              {
                "type": "Click",
                "value": "Click on Endtime."
              },
              {
                "type": "InsertText",
                "value": "11:00"
              },              
              {
                "type": "Click",
                "value": "Click on 'Speichern'."
              }
            ]
          }"#;
        let add_examples = self
            .recorded_usecases
            .iter()
            .map(|usecase| {
                format!(
                    "Here is an example of the JSON format: {}",
                    serde_json::to_string_pretty(&usecase).unwrap()
                )
            })
            .collect::<Vec<String>>()
            .join("\n");
        // println!("{}", examplejson);
        println!("{}", add_examples);
        let system_prompt = format!(
            "You are an expert in controlling a computer, you can click on the screen, write text, and press keys. {} {} think about the steps to complete the task, jump to the beginning of large text boxes with Home and PageUp, output the actions in JSON format.",
            add_examples,examplejson
        );

        // Convert monitor1 to base64 string
        let base64_image = match monitor1 {
            Some(ref image) => {
                use base64::Engine as _;
                let mut buffer = Vec::new();
                image
                    .write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
                    .expect("Failed to encode image");
                base64::engine::general_purpose::STANDARD.encode(&buffer)
            }
            None => {
                println!("No monitor screenshot available");
                return;
            }
        };

        // Create a new thread to handle the blocking operation
        std::thread::spawn(move || {
            // Create a new Tokio runtime for this thread
            let rt = tokio::runtime::Runtime::new().unwrap();

            // Execute the async code within the Tokio runtime
            rt.block_on(async {
                let mut client = Client::new("".to_string());
                client.set_base_url(&server_url_planning);
                if let Ok(parameters) = ChatCompletionParametersBuilder::default()
                    .model("Qwen/Qwen2.5-VL-7B-Instruct".to_string())
                    .messages(vec![
                        ChatMessage::System {
                            content: ChatMessageContent::Text(system_prompt.to_string()),
                            name: None,
                        },
                        ChatMessage::User {
                            content: ChatMessageContent::Text(instruction.to_string()),
                            name: None,
                        },
                        ChatMessage::User {
                            content: ChatMessageContent::ContentPart(vec![
                                ChatMessageContentPart::Image(ChatMessageImageContentPart {
                                    r#type: "image_url".to_string(),
                                    image_url: ImageUrlType {
                                        url: format!("data:image/png;base64,{}", base64_image),
                                        detail: None,
                                    },
                                }),
                            ]),
                            name: None,
                        },
                    ])
                    .max_completion_tokens(1024u32)
                    .temperature(0.0)
                    .build()
                {
                    if let Ok(result) = client.chat().create(parameters).await {
                        let msg = result.choices[0].message.clone();
                        match msg {
                            ChatMessage::Assistant { content, .. } => {
                                let response_text = content.unwrap().to_string();
                                println!("response_text: {}", response_text);
                                // Save response text to file
                                // let mut file = File::create("response.txt").unwrap();
                                // file.write_all(response_text.as_bytes()).unwrap();
                                let json_start = response_text.find("```json").unwrap_or(0);
                                let json_end = response_text.rfind("```").unwrap_or(0);
                                let json_str = if json_start == 0 && json_end == 0 {
                                    response_text.to_string()
                                } else {
                                    response_text[json_start + 7..json_end].to_string()
                                };

                                vec_instructions.lock().unwrap().clear();

                                // Clean the JSON string before parsing
                                let cleaned_json = json_str
                                    .replace('\n', " ")
                                    .replace('\r', "")
                                    .replace('\t', " ")
                                    .replace(char::from(0), "")
                                    .trim()
                                    .to_string();

                                let json_str = repair_json::repair(cleaned_json.clone())
                                    .unwrap_or(cleaned_json);
                                let json_str =
                                    JsonFixer::fix(&json_str.clone()).unwrap_or(json_str);

                                // Parse JSON
                                match serde_json::from_str::<Value>(&json_str) {
                                    Ok(parsed_json) => {
                                        // Output fixed JSON as a formatted string
                                        let fixed_json = serde_json::to_string_pretty(&parsed_json)
                                            .expect("Failed to format JSON");

                                        match serde_json::from_str::<StepFormat>(&fixed_json) {
                                            Ok(StepFormat::SingleStep {
                                                instruction,
                                                actions,
                                            }) => {
                                                vec_instructions.lock().unwrap().push(
                                                    UseCaseActions {
                                                        instruction,
                                                        actions: actions
                                                            .into_iter()
                                                            .map(|a| a.into())
                                                            .collect(),
                                                    },
                                                );
                                            }
                                            Ok(StepFormat::MultiStep(steps)) => {
                                                vec_instructions.lock().unwrap().clear();
                                                let mut current_instruction = String::new();
                                                for step in steps {
                                                    if let Some(instruction) = step.instruction {
                                                        current_instruction = instruction;
                                                    }
                                                    if let Some(actions) = step.actions {
                                                        vec_instructions.lock().unwrap().push(
                                                            UseCaseActions {
                                                                instruction: current_instruction
                                                                    .clone(),
                                                                actions: actions
                                                                    .into_iter()
                                                                    .map(|a| a.into())
                                                                    .collect(),
                                                            },
                                                        );
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                println!("Failed to parse JSON: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("Failed to parse JSON: {}", e);
                                    }
                                }
                            }
                            _ => println!("Unexpected message type"),
                        }
                    } else {
                        println!("Failed to create chat completion");
                    }
                } else {
                    println!("Failed to build parameters");
                }
                *computing_plan.lock().unwrap() = false;
            });
        });
    }

    pub fn execute_usecase(&mut self, instruction: String) {
        //let index = self.identify_usecase(&instruction);
        //self.create_usecase_actions(index, &instruction);

        self.generate_usecase_actions(&instruction);

        // self.generate_usecase_actions(&instruction);
        // self.update_usecase_actions();
        //self.show = true;
    }
    pub fn grab_screenshot(&mut self) {
        println!("grab_screenshot");
        if self.monitor.is_none() {
            let monitors = Monitor::all().unwrap();
            self.monitor = Some(monitors);
        }
        for (i, monitor) in self.monitor.as_ref().unwrap().iter().enumerate() {
            let image: ImageBuffer<Rgba<u8>, Vec<u8>> = monitor.capture_image().unwrap();
            if i == 0 {
                //self.monitor1 = Some(image);

                // Resize image to half size
                #[cfg(target_os = "macos")]
                {
                    let resized = image::imageops::resize(
                        &image,
                        image.width() / 2,
                        image.height() / 2,
                        image::imageops::FilterType::Lanczos3,
                    );
                    self.image_width = image.width();
                    self.image_height = image.height();
                    self.monitor1 = Some(resized);
                    // Save resized image to disk for debugging
                    let debug_path = format!("debug_screenshot.png");
                    if let Err(e) = self.monitor1.as_ref().unwrap().save(&debug_path) {
                        println!("Failed to save debug screenshot: {}", e);
                    } else {
                        println!("Saved debug screenshot to {}", debug_path);
                    }
                    println!(
                        "monitor1 width: {}",
                        self.monitor1.as_ref().unwrap().width()
                    );
                    println!(
                        "monitor1 height: {}",
                        self.monitor1.as_ref().unwrap().height()
                    );
                }
                #[cfg(any(target_os = "linux", target_os = "windows"))]
                {
                    self.image_width = image.width();
                    self.image_height = image.height();
                    self.monitor1 = Some(image);
                }
            } else if i == 1 {
                self.monitor2 = Some(image);
            } else if i == 2 {
                self.monitor3 = Some(image);
            }
        }
    }
    pub fn click(&mut self, instruction: String) {
        println!("click_openai: {}", instruction);
        *self.computing_action.lock().unwrap() = true;
        let monitor1 = self.monitor1.clone();
        let vec_instructions = self.vec_instructions.clone();
        let index_instruction = self.index_instruction.clone();
        let index_action = self.index_action.clone();
        let computing_action = self.computing_action.clone();
        let server_url_execution = self.server_url_execution.clone();

        // Create a new thread to handle the blocking operation
        std::thread::spawn(move || {
            // Create a new Tokio runtime for this thread
            let rt = tokio::runtime::Runtime::new().unwrap();

            // Execute the async code within the Tokio runtime
            rt.block_on(async {
                let mut client = Client::new("".to_string());
                client.set_base_url(&server_url_execution);
                use base64::Engine as _;
                // Convert monitor1 to base64 string
                let base64_image = match monitor1 {
                    Some(ref image) => {
                        let mut buffer = Vec::new();
                        image
                            .write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
                            .expect("Failed to encode image");
                        base64::engine::general_purpose::STANDARD.encode(&buffer)
                    }
                    None => {
                        println!("No monitor screenshot available");
                        return;
                    }
                };

                //let system_prompt = "You are an expert in analyzing screenshots. Given an instruction and a screenshot, output the coordinates [x1, y1, x2, y2] of where to click. The coordinates should be in pixels and represent a bounding box around the target element.";
                let system_prompt = "You are a helpful assistant";
                if let Ok(parameters) = ChatCompletionParametersBuilder::default()
                    .model("Qwen/Qwen2.5-VL-7B-Instruct".to_string())
                    .messages(vec![
                        ChatMessage::System {
                            content: ChatMessageContent::Text(system_prompt.to_string()),
                            name: None,
                        },
                        ChatMessage::User {
                            content: ChatMessageContent::Text(
                                instruction.to_string()
                                    + " output its bbox coordinates using JSON format.",
                            ),
                            name: None,
                        },
                        ChatMessage::User {
                            content: ChatMessageContent::ContentPart(vec![
                                ChatMessageContentPart::Image(ChatMessageImageContentPart {
                                    r#type: "image_url".to_string(),
                                    image_url: ImageUrlType {
                                        url: format!("data:image/png;base64,{}", base64_image),
                                        detail: None,
                                    },
                                }),
                            ]),
                            name: None,
                        },
                    ])
                    .max_completion_tokens(1024u32)
                    .temperature(0.0)
                    .build()
                {
                    if let Ok(result) = client.chat().create(parameters).await {
                        let msg = result.choices[0].message.clone();
                        match msg {
                            ChatMessage::Assistant { content, .. } => {
                                let response_text = content.unwrap().to_string();
                                println!("response_text: {}", response_text);

                                if let Some(coords) = parse_coordinates(&response_text) {
                                    let (x1, y1, x2, y2) = coords;
                                    let center_x = (x1 + x2) / 2.0;
                                    let center_y = (y1 + y2) / 2.0;

                                    let index_instruction = *index_instruction.lock().unwrap();
                                    let index_action = *index_action.lock().unwrap();
                                    if let Some(usecase_actions) =
                                        vec_instructions.lock().unwrap().get_mut(index_instruction)
                                    {
                                        if index_action + 1 < usecase_actions.actions.len() {
                                            if let ActionTypes::ClickPosition(_x, _y) =
                                                usecase_actions.actions[index_action + 1]
                                            {
                                                usecase_actions.actions[index_action + 1] =
                                                    ActionTypes::ClickPosition(center_x, center_y);
                                            } else {
                                                usecase_actions.actions.insert(
                                                    index_action + 1,
                                                    ActionTypes::ClickPosition(center_x, center_y),
                                                );
                                            }
                                        } else {
                                            usecase_actions.actions.push(
                                                ActionTypes::ClickPosition(center_x, center_y),
                                            );
                                        }
                                    }
                                }
                            }
                            _ => println!("Unexpected message type"),
                        }
                    } else {
                        println!("Failed to create chat completion");
                    }
                } else {
                    println!("Failed to build parameters");
                }
                *computing_action.lock().unwrap() = false;
                *index_action.lock().unwrap() += 1;
            });
        });
    }
    pub fn click_florence2(&mut self, instruction: String) {
        println!("click_florence2: {}", instruction);
        *self.computing_action.lock().unwrap() = true;
        let monitor1 = self.monitor1.clone();
        let vec_instructions = self.vec_instructions.clone();
        let index_instruction = self.index_instruction.clone();
        let index_action = self.index_action.clone();
        let computing_action = self.computing_action.clone();
        let server_url_execution = self.server_url_execution.clone();
        std::thread::spawn(move || {
            let client = reqwest::blocking::Client::new();
            // Encode the image directly into the buffer
            let mut buffer = Vec::new();
            match monitor1.as_ref() {
                Some(monitor) => {
                    if let Err(e) =
                        monitor.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
                    {
                        println!("Failed to encode image: {}", e);
                        return;
                    }
                }
                None => {
                    println!("No monitor screenshot available");
                    return;
                }
            }
            let image_part = match reqwest::blocking::multipart::Part::bytes(buffer)
                .file_name("image.png")
                .mime_str("image/png")
            {
                Ok(part) => part,
                Err(e) => {
                    println!("Failed to create multipart form: {}", e);
                    return;
                }
            };

            let instruction_part = reqwest::blocking::multipart::Part::text(instruction);
            let form = reqwest::blocking::multipart::Form::new()
                .part("image", image_part)
                .part("prompt", instruction_part);

            // Send the POST request with error handling
            let res = match client
                .post(format!("{}/get_location", server_url_execution))
                .multipart(form)
                .send()
            {
                Ok(response) => response,
                Err(e) => {
                    println!("Failed to send request: {}", e);
                    return;
                }
            };
            // Parse the response text
            let response_text = match res.text() {
                Ok(text) => text,
                Err(e) => {
                    println!("Failed to read response: {}", e);
                    return;
                }
            };
            println!("response_text: {}", response_text);
            if let Some(coords) = parse_coordinates(&response_text) {
                println!("coords: {:?}", coords);
                let (x1, y1, x2, y2) = coords;
                let center_x = ((x1 + x2) / 2.0).floor();
                let center_y = ((y1 + y2) / 2.0).floor();
                let index_instruction = *index_instruction.lock().unwrap();
                let index_action = *index_action.lock().unwrap();
                if let Some(usecase_actions) =
                    vec_instructions.lock().unwrap().get_mut(index_instruction)
                {
                    if index_action + 1 < usecase_actions.actions.len() {
                        if let ActionTypes::ClickPosition(_x, _y) =
                            usecase_actions.actions[index_action + 1]
                        {
                            usecase_actions.actions[index_action + 1] =
                                ActionTypes::ClickPosition(center_x, center_y);
                        } else {
                            usecase_actions.actions.insert(
                                index_action + 1,
                                ActionTypes::ClickPosition(center_x, center_y),
                            );
                        }
                    } else {
                        usecase_actions
                            .actions
                            .push(ActionTypes::ClickPosition(center_x, center_y));
                    }
                }
            }
            *computing_action.lock().unwrap() = false;
            *index_action.lock().unwrap() += 1;
        });
    }

    pub fn step(&mut self) {
        let index_instruction = *self.index_instruction.lock().unwrap();
        let index_action = *self.index_action.lock().unwrap();
        if index_instruction >= self.vec_instructions.lock().unwrap().len() {
            *self.index_instruction.lock().unwrap() = 0;
            *self.index_action.lock().unwrap() = 0;
            return;
        }
        if *self.computing_action.lock().unwrap() {
            return;
        }
        if *self.computing_plan.lock().unwrap() {
            return;
        }
        if index_action
            >= self
                .vec_instructions
                .lock()
                .unwrap()
                .get(index_instruction)
                .unwrap()
                .actions
                .len()
        {
            *self.index_instruction.lock().unwrap() += 1;
            *self.index_action.lock().unwrap() = 0;
            return;
        }
        let action = self
            .vec_instructions
            .lock()
            .unwrap()
            .get(index_instruction)
            .unwrap()
            .actions[index_action]
            .clone();
        match action.clone() {
            ActionTypes::Click(instruction) => {
                self.grab_screenshot();
                if self.server_url_execution.contains(":5001") {
                    self.click_florence2(instruction);
                } else {
                    self.click(instruction);
                }
            }
            ActionTypes::ClickPosition(x, y) => {
                println!("click_position: {:?}", (x, y));
                mouse_click(x, y);
            }
            ActionTypes::InsertText(text) => {
                println!("insert_text: {}", text);
                text_input(&text);
            }
            ActionTypes::KeyDown(instruction) => {
                println!("key_down: {}", instruction);
                key_down(&instruction);
            }
            ActionTypes::KeyUp(instruction) => {
                println!("key_up: {}", instruction);
                key_up(&instruction);
            }
            ActionTypes::KeyPress(instruction) => {
                println!("key_press: {}", instruction);
                key_down(&instruction);
                thread::sleep(time::Duration::from_millis(100));
                key_up(&instruction);
            }
            ActionTypes::GrabScreenshot => {
                self.grab_screenshot();
            }
            ActionTypes::Replan => {
                let instruction = self.instruction_dialog.clone();
                self.generate_usecase_actions(&instruction);
            }
        }

        if !*self.computing_action.lock().unwrap() && !matches!(action, ActionTypes::Click(_)) {
            *self.index_action.lock().unwrap() += 1;
        }
        let index_action = *self.index_action.lock().unwrap();
        let index_instruction = *self.index_instruction.lock().unwrap();
        if index_action
            >= self
                .vec_instructions
                .lock()
                .unwrap()
                .get(index_instruction)
                .unwrap()
                .actions
                .len()
            && !*self.computing_action.lock().unwrap()
        {
            *self.index_instruction.lock().unwrap() += 1;
            *self.index_action.lock().unwrap() = 0;
        }
        let index_action = *self.index_action.lock().unwrap();
        let index_instruction = *self.index_instruction.lock().unwrap();
        let mut trigger_step = false;
        if let Some(actions) = self.vec_instructions.lock().unwrap().get(index_instruction) {
            if index_action >= actions.actions.len() {
                self.show = false;
            } else if let ActionTypes::KeyUp(_key) = &actions.actions[index_action] {
                trigger_step = true;
            }
        }
        if trigger_step {
            self.step();
        }
    }

    fn draw_circle(ui: &mut egui::Ui, position: (f32, f32), image_height: f32) {
        #[cfg(target_os = "macos")]
        let position = (position.0, position.1 - 40.0); //adjust for menubar in macos
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        if position.1 > image_height - 50.0 {
            ui.painter().arrow(
                egui::pos2(position.0, image_height - 100.0),
                egui::vec2(0.0, 50.0),
                egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 0, 0)),
            );
        } else {
            ui.painter().circle_filled(
                //egui::pos2(position.0, position.1 - 1.0),
                egui::pos2(position.0, position.1),
                10.0,
                egui::Color32::from_rgb(255, 0, 0),
            );
        }

        #[cfg(target_os = "macos")]
        if position.1 > 905.0 {
            ui.painter().arrow(
                egui::pos2(position.0, 850.0 - 50.0),
                egui::vec2(0.0, 50.0),
                egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 0, 0)),
            );
        } else {
            ui.painter().circle_filled(
                //egui::pos2(position.0, position.1 - 1.0),
                egui::pos2(position.0, position.1),
                10.0,
                egui::Color32::from_rgb(255, 0, 0),
            );
        }
    }
    pub fn visualize_next_step(&mut self, egui_context: &egui::Context) {
        if self.vec_instructions.lock().unwrap().is_empty() {
            return;
        }
        let index_instruction = *self.index_instruction.lock().unwrap();
        if index_instruction >= self.vec_instructions.lock().unwrap().len() {
            return;
        }
        let index_action = *self.index_action.lock().unwrap();
        let index_instruction = *self.index_instruction.lock().unwrap();
        if index_action
            >= self
                .vec_instructions
                .lock()
                .unwrap()
                .get(index_instruction)
                .unwrap()
                .actions
                .len()
        {
            return;
        }
        let computing_action = self.computing_action.lock().unwrap();
        let computing_text = if *computing_action {
            "Computing..."
        } else {
            ""
        };
        egui::Window::new("Overlay")
            .interactable(false)
            .title_bar(false)
            .default_pos(egui::Pos2::new(10.0, 1.0))
            .min_size(egui::Vec2::new(
                self.image_width as f32 - 2.0,
                self.image_height as f32 - 2.0,
            ))
            .show(egui_context, |ui| {
                egui::Area::new(egui::Id::new("overlay"))
                    .fixed_pos(egui::pos2(0.0, 0.0))
                    .show(egui_context, |ui| {
                        let action = self
                            .vec_instructions
                            .lock()
                            .unwrap()
                            .get(index_instruction)
                            .unwrap()
                            .actions
                            .get(index_action)
                            .unwrap()
                            .clone();

                        ui.add_sized(
                            egui::Vec2::new(600.0, 30.0),
                            egui::Label::new(
                                egui::RichText::new(format!(
                                    "PlugOvr: next action: {:?} {}",
                                    action, computing_text
                                ))
                                .background_color(egui::Color32::from_rgb(255, 255, 255)),
                            ),
                        );
                    });
                if let ActionTypes::ClickPosition(x, y) = self
                    .vec_instructions
                    .lock()
                    .unwrap()
                    .get(index_instruction)
                    .unwrap()
                    .actions
                    .get(index_action)
                    .unwrap()
                {
                    Self::draw_circle(ui, (*x, *y), self.image_height as f32);
                }
            });
    }

    pub fn visualize_planning(&mut self, egui_context: &egui::Context) {
        if !*self.computing_plan.lock().unwrap() {
            return;
        }
        egui::Window::new("Overlay")
            .interactable(false)
            .title_bar(false)
            .default_pos(egui::Pos2::new(10.0, 1.0))
            .min_size(egui::Vec2::new(
                self.image_width as f32 - 2.0,
                self.image_height as f32 - 2.0,
            ))
            .show(egui_context, |_ui| {
                egui::Area::new(egui::Id::new("overlay"))
                    .fixed_pos(egui::pos2(0.0, 0.0))
                    .show(egui_context, |ui| {
                        ui.add_sized(
                            egui::Vec2::new(400.0, 30.0),
                            egui::Label::new(
                                egui::RichText::new("PlugOvr: planning...")
                                    .background_color(egui::Color32::from_rgb(255, 255, 255)),
                            ),
                        );
                    });
            });
    }
}

fn parse_coordinates_florence2(response: &str) -> Option<(f32, f32, f32, f32)> {
    let re = regex::Regex::new(r"<loc_(\d+)>").unwrap();
    let coords: Vec<f32> = re
        .captures_iter(response)
        .map(|cap| cap[1].parse::<f32>().unwrap() / 1000.0)
        .collect();

    if coords.len() == 4 {
        Some((coords[0], coords[1], coords[2], coords[3]))
    } else {
        None
    }
}

fn parse_coordinates(response: &str) -> Option<(f32, f32, f32, f32)> {
    // Try the original format [x1, y1, x2, y2]
    let re = regex::Regex::new(r"\[\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\]").unwrap();
    if let Some(caps) = re.captures(response) {
        let coords: Vec<f32> = (1..=4).map(|i| caps[i].parse::<f32>().unwrap()).collect();
        return Some((coords[0], coords[1], coords[2], coords[3]));
    }

    // Try the simplified format [x, y]
    let re_simple = regex::Regex::new(r"\[\s*(\d+)\s*,\s*(\d+)\s*\]").unwrap();
    if let Some(caps) = re_simple.captures(response) {
        let x = caps[1].parse::<f32>().unwrap();
        let y = caps[2].parse::<f32>().unwrap();
        // Create a small bounding box around the point
        return Some((x - 5.0, y - 5.0, x + 5.0, y + 5.0));
    }

    None
}
//#[cfg(not(target_os = "macos"))]
fn mouse_click(x: f32, y: f32) {
    simulate(&rdev::EventType::MouseMove {
        x: x as f64,
        y: y as f64,
    })
    .unwrap();
    thread::sleep(time::Duration::from_millis(40));
    simulate(&rdev::EventType::ButtonPress(rdev::Button::Left)).unwrap();
    thread::sleep(time::Duration::from_millis(40));
    simulate(&rdev::EventType::ButtonRelease(rdev::Button::Left)).unwrap();
}

#[cfg(not(target_os = "macos"))]
fn text_input(text: &str) {
    arboard::Clipboard::new().unwrap().set_text(text).unwrap();
    thread::sleep(time::Duration::from_millis(40));
    simulate(&rdev::EventType::KeyPress(rdev::Key::ControlLeft)).unwrap();
    thread::sleep(time::Duration::from_millis(40));
    simulate(&rdev::EventType::KeyPress(rdev::Key::KeyV)).unwrap();

    thread::sleep(time::Duration::from_millis(40));
    simulate(&rdev::EventType::KeyRelease(rdev::Key::KeyV)).unwrap();
    thread::sleep(time::Duration::from_millis(40));
    simulate(&rdev::EventType::KeyRelease(rdev::Key::ControlLeft)).unwrap();
}
#[cfg(target_os = "macos")]
fn text_input(text: &str) {
    use crate::send_cmd_v;

    arboard::Clipboard::new().unwrap().set_text(text).unwrap();
    thread::sleep(time::Duration::from_millis(100));
    let _ = send_cmd_v();
}

fn from_str(key: &str) -> rdev::Key {
    match key {
        "Alt" => rdev::Key::Alt,
        "AltGr" => rdev::Key::AltGr,
        "Backspace" => rdev::Key::Backspace,
        "CapsLock" => rdev::Key::CapsLock,
        "ControlLeft" => rdev::Key::ControlLeft,
        "ControlRight" => rdev::Key::ControlRight,
        "Delete" => rdev::Key::Delete,
        "DownArrow" => rdev::Key::DownArrow,
        "End" => rdev::Key::End,
        "Escape" => rdev::Key::Escape,
        "F1" => rdev::Key::F1,
        "F10" => rdev::Key::F10,
        "F11" => rdev::Key::F11,
        "F12" => rdev::Key::F12,
        "F2" => rdev::Key::F2,
        "F3" => rdev::Key::F3,
        "F4" => rdev::Key::F4,
        "F5" => rdev::Key::F5,
        "F6" => rdev::Key::F6,
        "F7" => rdev::Key::F7,
        "F8" => rdev::Key::F8,
        "F9" => rdev::Key::F9,
        "Home" => rdev::Key::Home,
        "LeftArrow" => rdev::Key::LeftArrow,
        /// also known as "windows", "super", and "command"
        "MetaLeft" => rdev::Key::MetaLeft,
        /// also known as "windows", "super", and "command"
        "MetaRight" => rdev::Key::MetaRight,
        "PageDown" => rdev::Key::PageDown,
        "PageUp" => rdev::Key::PageUp,
        "Return" => rdev::Key::Return,
        "RightArrow" => rdev::Key::RightArrow,
        "ShiftLeft" => rdev::Key::ShiftLeft,
        "ShiftRight" => rdev::Key::ShiftRight,
        "Space" => rdev::Key::Space,
        "Tab" => rdev::Key::Tab,
        "UpArrow" => rdev::Key::UpArrow,
        "PrintScreen" => rdev::Key::PrintScreen,
        "ScrollLock" => rdev::Key::ScrollLock,
        "Pause" => rdev::Key::Pause,
        "NumLock" => rdev::Key::NumLock,
        "BackQuote" => rdev::Key::BackQuote,
        "Num1" => rdev::Key::Num1,
        "Num2" => rdev::Key::Num2,
        "Num3" => rdev::Key::Num3,
        "Num4" => rdev::Key::Num4,
        "Num5" => rdev::Key::Num5,
        "Num6" => rdev::Key::Num6,
        "Num7" => rdev::Key::Num7,
        "Num8" => rdev::Key::Num8,
        "Num9" => rdev::Key::Num9,
        "Num0" => rdev::Key::Num0,
        "Minus" => rdev::Key::Minus,
        "Equal" => rdev::Key::Equal,
        "KeyQ" => rdev::Key::KeyQ,
        "KeyW" => rdev::Key::KeyW,
        "KeyE" => rdev::Key::KeyE,
        "KeyR" => rdev::Key::KeyR,
        "KeyT" => rdev::Key::KeyT,
        "KeyY" => rdev::Key::KeyY,
        "KeyU" => rdev::Key::KeyU,
        "KeyI" => rdev::Key::KeyI,
        "KeyO" => rdev::Key::KeyO,
        "KeyP" => rdev::Key::KeyP,
        "LeftBracket" => rdev::Key::LeftBracket,
        "RightBracket" => rdev::Key::RightBracket,
        "KeyA" => rdev::Key::KeyA,
        "KeyS" => rdev::Key::KeyS,
        "KeyD" => rdev::Key::KeyD,
        "KeyF" => rdev::Key::KeyF,
        "KeyG" => rdev::Key::KeyG,
        "KeyH" => rdev::Key::KeyH,
        "KeyJ" => rdev::Key::KeyJ,
        "KeyK" => rdev::Key::KeyK,
        "KeyL" => rdev::Key::KeyL,
        "SemiColon" => rdev::Key::SemiColon,
        "Quote" => rdev::Key::Quote,
        "BackSlash" => rdev::Key::BackSlash,
        "IntlBackslash" => rdev::Key::IntlBackslash,
        "KeyZ" => rdev::Key::KeyZ,
        "KeyX" => rdev::Key::KeyX,
        "KeyC" => rdev::Key::KeyC,
        "KeyV" => rdev::Key::KeyV,
        "KeyB" => rdev::Key::KeyB,
        "KeyN" => rdev::Key::KeyN,
        "KeyM" => rdev::Key::KeyM,
        "Comma" => rdev::Key::Comma,
        "Dot" => rdev::Key::Dot,
        "Slash" => rdev::Key::Slash,
        "Insert" => rdev::Key::Insert,
        "KpReturn" => rdev::Key::KpReturn,
        "KpMinus" => rdev::Key::KpMinus,
        "KpPlus" => rdev::Key::KpPlus,
        "KpMultiply" => rdev::Key::KpMultiply,
        "KpDivide" => rdev::Key::KpDivide,
        "Kp0" => rdev::Key::Kp0,
        "Kp1" => rdev::Key::Kp1,
        "Kp2" => rdev::Key::Kp2,
        "Kp3" => rdev::Key::Kp3,
        "Kp4" => rdev::Key::Kp4,
        "Kp5" => rdev::Key::Kp5,
        "Kp6" => rdev::Key::Kp6,
        "Kp7" => rdev::Key::Kp7,
        "Kp8" => rdev::Key::Kp8,
        "Kp9" => rdev::Key::Kp9,
        "KpDelete" => rdev::Key::KpDelete,
        "Function" => rdev::Key::Function,
        "Unknown" => rdev::Key::Unknown(0),
        _ => rdev::Key::Unknown(0),
    }
}

fn key_down(key: &str) {
    simulate(&rdev::EventType::KeyPress(from_str(key))).unwrap();

    thread::sleep(time::Duration::from_millis(40));
}
fn key_up(key: &str) {
    simulate(&rdev::EventType::KeyRelease(from_str(key))).unwrap();
    thread::sleep(time::Duration::from_millis(40));
}

// fn load_image_from_file(path: &str) -> anyhow::Result<image::DynamicImage> {
//     Ok(image::open(path)?)
// }
fn save_server_url_planning(server_url: &str) -> std::io::Result<()> {
    let mut path = dirs::home_dir().expect("Unable to get home directory");
    path.push(".plugovr");
    std::fs::create_dir_all(&path)?;
    path.push("server_url_planning.json");

    let serialized = serde_json::to_string(&server_url)?;
    let mut file = File::create(path)?;
    file.write_all(serialized.as_bytes())?;
    Ok(())
}

fn save_server_url_execution(server_url: &str) -> std::io::Result<()> {
    let mut path = dirs::home_dir().expect("Unable to get home directory");
    path.push(".plugovr");
    std::fs::create_dir_all(&path)?;
    path.push("server_url_execution.json");

    let serialized = serde_json::to_string(&server_url)?;
    let mut file = File::create(path)?;
    file.write_all(serialized.as_bytes())?;
    Ok(())
}

fn load_server_url_planning() -> std::io::Result<String> {
    let mut path = dirs::home_dir().expect("Unable to get home directory");
    path.push(".plugovr");
    path.push("server_url_planning.json");
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let server_url: String = serde_json::from_str(&contents)?;
    Ok(server_url)
}

fn load_server_url_execution() -> std::io::Result<String> {
    let mut path = dirs::home_dir().expect("Unable to get home directory");
    path.push(".plugovr");
    path.push("server_url_execution.json");
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let server_url: String = serde_json::from_str(&contents)?;
    Ok(server_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_example_json() {
        let json = r#"{
            "instruction": "Write an email to Cornelius",
            "actions": [
              {
                "type": "Click",
                "value": "Click on the 'Google Chrome' icon."
              },
              {
                "type": "Click",
                "value": "Click on the search bar."
              },
              {
                "type": "InsertText",
                "value": "www.gmail.com"
              },
              {
                "type": "KeyPress",
                "value": "Return"
              },
              {
                "type": "Click",
                "value": "Click on 'Schreiben'."
              },
              {
                "type": "Click",
                "value": "Click on 'An'."
              },
              {
                "type": "InsertText",
                "value": "info@plugovr.ai"
              },
              {
                "type": "KeyPress",
                "value": "Return"
              },
              {
                "type": "Click",
                "value": "Click on 'Betreff'."
              },
              {
                "type": "InsertText",
                "value": "Hi"
              },
              {
                "type": "Click",
                "value": "Click on main message field."
              },
              {
                "type": "KeyPress",
                "value": "Home"
              },
              {
                "type": "KeyPress",
                "value": "PageUp"
              },
              {
                "type": "InsertText",
                "value": "Hi Cornelius"
              },
              {
                "type": "Click",
                "value": "Click on 'Senden'."
              }
            ]
          }"#;

        // Parse the JSON
        let parsed = serde_json::from_str::<StepFormat>(json);
        assert!(parsed.is_ok(), "Failed to parse JSON");

        // Verify the parsed content
        match parsed.unwrap() {
            StepFormat::SingleStep {
                instruction,
                actions,
            } => {
                assert_eq!(instruction, "Write an email to Cornelius");
                assert_eq!(actions.len(), 15);

                // Verify first action
                assert_eq!(actions[0].action_type, "Click");
                assert_eq!(actions[0].value, "Click on the 'Google Chrome' icon.");

                // Verify a KeyPress action
                assert_eq!(actions[3].action_type, "KeyPress");
                assert_eq!(actions[3].value, "Return");

                // Verify an InsertText action
                assert_eq!(actions[2].action_type, "InsertText");
                assert_eq!(actions[2].value, "www.gmail.com");

                // Verify last action
                assert_eq!(actions[14].action_type, "Click");
                assert_eq!(actions[14].value, "Click on 'Senden'.");
            }
            StepFormat::MultiStep(_) => {
                panic!("Expected SingleStep format, got MultiStep");
            }
        }
    }
}
