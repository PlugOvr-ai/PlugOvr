use crate::usecase_recorder::{EventType, UseCase};

use crate::llm::CloudModel;
use crate::llm::LLMSelector;
use crate::llm::LLMType;
use egui_overlay::egui_render_three_d::{
    three_d::{self, ColorMaterial, Gm, Mesh},
    ThreeDBackend,
};
use futures;
#[cfg(target_os = "linux")]
use gtk::false_;
use image::{ImageBuffer, Rgba};
use json_fixer::JsonFixer;
#[cfg(feature = "cs")]
use plugovr_cs::cloud_llm::call_aws_lambda;
use rdev::{simulate, Button};
use regex;
use repair_json;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;
use xcap::Monitor;
//use anyhow;
//use mistralrs::{IsqType, TextMessageRole, VisionLoaderType, VisionMessages, DeviceLayerMapMetadata,Device,
//    VisionModelBuilder, DeviceMapSetting, DeviceMapMetadata, get_auto_device_map_params,ModelSelected, ChatTemplate};

//const MODEL_ID: &str = "Qwen/Qwen2-VL-2B-Instruct";
//const MODEL_ID: &str = "bytedance-research/UI-TARS-2B-SFT";
//const MODEL_ID: &str = "Qwen/Qwen2.5-VL-3B-Instruct";

pub struct UseCaseReplay {
    pub index_instruction: usize,
    pub index_action: usize,
    pub vec_instructions: Arc<Mutex<Vec<UseCaseActions>>>,
    pub recorded_usecases: Vec<UseCase>,
    pub monitor1: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub monitor2: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub monitor3: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    pub show: bool,
    pub show_dialog: bool,
    pub click_position: Option<(f32, f32)>,
    pub model: Option<Gm<Mesh, ColorMaterial>>,
    pub llm_selector: Option<Arc<Mutex<LLMSelector>>>,
    instruction_dialog: String,
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
        //   //      let model_selected = ModelSelected::VisionPlain::default();

        //         let device = Device::new_metal(0).unwrap();
        //         let model = VisionModelBuilder::new(MODEL_ID, VisionLoaderType::Qwen2VL)
        //         //.with_isq(IsqType::Q4K)
        //         .with_logging()
        //         .with_token_source(mistralrs::TokenSource::None)
        //         .from_max_edge(1024)
        //         .with_dtype(mistralrs::ModelDType::Auto)
        //         //.with_chat_template("qwen2vl_chat_template.json")

        //         //.with_device_mapping(DeviceMapSetting::Map(DeviceMapMetadata::from_num_device_layers(vec![DeviceLayerMapMetadata{ordinal: 1, layers: 28}])))

        //         .with_max_num_seqs(4096)
        //         .build()
        //         .await;

        //         let prompt = r#"You are a GUI agent. You are given a task and your action history, with screenshots. You need to perform the next action to complete the task.

        // ## Output Format
        // ```\nThought: ...
        // Action: ...\n```

        // ## Action Space

        // click(start_box='<|box_start|>(x1,y1)<|box_end|>')
        // left_double(start_box='<|box_start|>(x1,y1)<|box_end|>')
        // right_single(start_box='<|box_start|>(x1,y1)<|box_end|>')
        // drag(start_box='<|box_start|>(x1,y1)<|box_end|>', end_box='<|box_start|>(x3,y3)<|box_end|>')
        // hotkey(key='')
        // type(content='') #If you want to submit your input, use \"\
        // \" at the end of `content`.
        // scroll(start_box='<|box_start|>(x1,y1)<|box_end|>', direction='down or up or right or left')
        // wait() #Sleep for 5s and take a screenshot to check for any changes.
        // finished()
        // call_user() # Submit the task and call the user when the task is unsolvable, or when you need the user's help.

        // ## Note
        // - Use English in `Thought` part.
        // - Summarize your next action (with its target element) in one sentence in `Thought` part.

        // ## User Instruction
        // "#;
        //         let instruction = "click on Chrome";
        //         let prompt = format!("{}", instruction);
        //         match model {
        //             Ok(model) => {
        //                 println!("model loaded");
        //                 //let bytes = include_bytes!("../debug_screenshot.png");
        //                 //let image = image::load_from_memory(&bytes).unwrap();
        //                 let image = load_image_from_file("debug_screenshot.png").unwrap();
        //                 let resized = image::imageops::resize(
        //                     &image,
        //                     1024,
        //                     1024,
        //                     image::imageops::FilterType::Lanczos3
        //                  );

        //                 println!("image loaded");
        //                 let messages = VisionMessages::new().add_image_message(
        //                     TextMessageRole::User,
        //                     prompt,
        //                     image::DynamicImage::ImageRgba8(resized),
        //                     &model,
        //                 );
        //                 if let Ok(messages) = messages {
        //                     if let Ok(response) = model.send_chat_request(messages).await {
        //                         println!("{}", response.choices[0].message.content.as_ref().unwrap());
        //                         dbg!(
        //                             response.usage.avg_prompt_tok_per_sec,
        //                             response.usage.avg_compl_tok_per_sec
        //                         );
        //                     }
        //                 }
        //             }
        //             Err(e) => {
        //                 println!("model not loaded: {}", e);
        //             }
        //         }

        Self {
            index_instruction: 0,
            index_action: 0,
            vec_instructions: Arc::new(Mutex::new(vec![])),
            recorded_usecases: vec![],
            monitor1: None,
            monitor2: None,
            monitor3: None,
            show: false,
            show_dialog: false,
            click_position: None,
            model: None,
            llm_selector: None,
            instruction_dialog: "".to_string(),
        }
    }
    pub fn show_dialog(&mut self, egui_context: &egui::Context) {
        if !self.show_dialog {
            return;
        }

        egui::Window::new("UseCaseReplay").show(egui_context, |ui| {
            ui.add(egui::Label::new("Agent Instructions"));
            ui.add(egui::TextEdit::multiline(&mut self.instruction_dialog));

            if ui.button("Run").clicked() {
                self.index_instruction = 0;
                self.index_action = 0;
                let instruction = self.instruction_dialog.clone();
                //self.instruction_dialog = "".to_string();

                self.execute_usecase(instruction);
                self.show_dialog = false;
                self.show = true;
            }
        });
    }
    pub fn load_usecase(&mut self, filename: String) {
        let file = File::open(filename).unwrap();
        let usecase: UseCase = serde_json::from_reader(file).unwrap();
        self.recorded_usecases.push(usecase);
    }
    pub fn identify_usecase(&mut self, instruction: &String) -> usize {
        //find the usecase that has the most similar instruction
        0
    }
    // pub fn create_usecase_actions(&mut self, index: usize, instruction: &String) {
    //     let mut actions = UseCaseActions {
    //         instruction: instruction.clone(),
    //         actions: vec![],
    //     };
    //     for event in self.recorded_usecases[index].usecase_steps.iter() {
    //         match event {
    //             EventType::Monitor1(_) => {
    //                 actions.actions.push(ActionTypes::GrabScreenshot);
    //             }
    //             EventType::Click(_, instruction) => {
    //                 actions
    //                     .actions
    //                     .push(ActionTypes::Click(instruction.clone()));
    //                 actions.actions.push(ActionTypes::ClickPosition(0.0, 0.0));
    //             }
    //             EventType::KeyDown(instruction) => {
    //                 actions
    //                     .actions
    //                     .push(ActionTypes::KeyDown(instruction.clone()));
    //             }
    //             EventType::KeyUp(instruction) => {
    //                 actions
    //                     .actions
    //                     .push(ActionTypes::KeyUp(instruction.clone()));
    //             }
    //             EventType::Text(instruction) => {
    //                 actions
    //                     .actions
    //                     .push(ActionTypes::InsertText(instruction.clone()));
    //             }
    //             _ => {}
    //         }
    //     }
    //     self.usecase_actions.lock().unwrap().replace(actions);
    // }
    // pub fn update_usecase_actions(&mut self) {
    //     let usecase_actions_json =
    //         serde_json::to_string(&self.usecase_actions.lock().unwrap().as_ref().unwrap()).unwrap();
    //     println!("usecase_actions_json: {}", usecase_actions_json);
    //     let prompt = format!(
    //         "update the json based on the instruction, output only the json: {}",
    //         usecase_actions_json
    //     );
    //     let ai_answer = Arc::new(Mutex::new(String::new()));
    //     if let Some(llm_selector) = self.llm_selector.clone() {
    //         let model = LLMType::Cloud(CloudModel::AnthropicHaiku).to_string();
    //         let user_info = llm_selector
    //             .lock()
    //             .unwrap()
    //             .user_info
    //             .lock()
    //             .unwrap()
    //             .clone();
    //         //let screenshots = vec![];
    //         #[cfg(feature = "cs")]
    //         {
    //             let (result, max_tokens_reached) =
    //                 call_aws_lambda(user_info.unwrap(), prompt, model, &vec![]);
    //             let result = if let Some(json_start) = result.find('{') {
    //                 if let Some(json_end) = result.rfind('}') {
    //                     result[json_start..=json_end].to_string()
    //                 } else {
    //                     result
    //                 }
    //             } else {
    //                 result
    //             };
    //             println!("result: {:?}", result);
    //             if !max_tokens_reached {
    //                 let usecase_actions: UseCaseActions =
    //                     serde_json::from_str(&result.to_string()).unwrap();
    //                 self.usecase_actions
    //                     .lock()
    //                     .unwrap()
    //                     .replace(usecase_actions);
    //                 println!("usecase_actions: {:?}", self.usecase_actions);
    //             }
    //         }
    //     }
    // }
    pub fn generate_usecase_actions(&mut self, instruction: &String) {
        let instruction = instruction.clone();
        self.grab_screenshot();
        let monitor1 = self.monitor1.clone();

        let vec_instructions = self.vec_instructions.clone();
        self.index_instruction = 0;
        self.index_action = 0;
        std::thread::spawn(move || {
            let mut retries = 0;
            const MAX_RETRIES: u32 = 3;

            while retries < MAX_RETRIES {
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

                let instruction_part =
                    reqwest::blocking::multipart::Part::text(instruction.clone());
                let form = reqwest::blocking::multipart::Form::new()
                    .part("image", image_part)
                    .part("prompt", instruction_part);

                // Send the POST request with error handling
                let res = match client
                    .post("http://192.168.1.106:5001/get_execution_plan")
                    .multipart(form)
                    .send()
                {
                    Ok(response) => response,
                    Err(e) => {
                        println!("Failed to send request: {}", e);
                        retries += 1;
                        if retries < MAX_RETRIES {
                            println!("Retrying... (attempt {} of {})", retries + 1, MAX_RETRIES);
                            thread::sleep(time::Duration::from_secs(1));
                            continue;
                        }
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
                let json_start = response_text.find("```json").unwrap_or(0);
                let json_end = response_text.rfind("```").unwrap_or(0);
                let json_str = if json_start == 0 && json_end == 0 {
                    response_text.to_string()
                } else {
                    response_text[json_start + 7..json_end].to_string()
                };
                println!("json_str: {}", json_str);
                vec_instructions.lock().unwrap().clear();
                let json_str = repair_json::repair(json_str.clone()).unwrap_or(json_str);
                let json_str = JsonFixer::fix(&json_str.clone()).unwrap_or(json_str);
                // Parse JSON
                let mut parsed_json: Value =
                    serde_json::from_str(&json_str).expect("Failed to parse JSON");

                // Output fixed JSON as a formatted string
                let fixed_json =
                    serde_json::to_string_pretty(&parsed_json).expect("Failed to format JSON");

                println!("json_str fixed: {}", fixed_json);
                match serde_json::from_str::<StepFormat>(&fixed_json) {
                    Ok(StepFormat::SingleStep {
                        instruction,
                        actions,
                    }) => {
                        vec_instructions.lock().unwrap().push(UseCaseActions {
                            instruction,
                            actions: actions.into_iter().map(|a| a.into()).collect(),
                        });
                        break; // Success - exit the retry loop
                    }
                    Ok(StepFormat::MultiStep(steps)) => {
                        vec_instructions.lock().unwrap().clear();
                        let mut current_instruction = String::new();
                        for step in steps {
                            if let Some(instruction) = step.instruction {
                                current_instruction = instruction;
                            }
                            if let Some(actions) = step.actions {
                                vec_instructions.lock().unwrap().push(UseCaseActions {
                                    instruction: current_instruction.clone(),
                                    actions: actions.into_iter().map(|a| a.into()).collect(),
                                });
                            }
                        }
                        break; // Success - exit the retry loop
                    }
                    Err(e) => {
                        println!("Failed to parse JSON response: {}", e);
                        retries += 1;
                        if retries < MAX_RETRIES {
                            println!("Retrying... (attempt {} of {})", retries + 1, MAX_RETRIES);
                            thread::sleep(time::Duration::from_secs(1));
                            continue;
                        }
                    }
                }
            }

            if retries >= MAX_RETRIES {
                println!(
                    "Failed to generate usecase actions after {} attempts",
                    MAX_RETRIES
                );
            }
        });
    }
    pub fn execute_usecase(&mut self, instruction: String) {
        //let index = self.identify_usecase(&instruction);
        //self.create_usecase_actions(index, &instruction);
        self.generate_usecase_actions(&instruction);
        // self.update_usecase_actions();
        //self.show = true;
    }
    pub fn grab_screenshot(&mut self) {
        println!("grab_screenshot");
        let monitors = Monitor::all().unwrap();
        for (i, monitor) in monitors.iter().enumerate() {
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
        println!("click: {}", instruction);
        let client = reqwest::blocking::Client::new();

        // Encode the image directly into the buffer
        let mut buffer = Vec::new();
        match self.monitor1.as_ref() {
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
            .post("http://192.168.1.106:5001/get_location")
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

        if let Some(coords) = parse_coordinates(&response_text) {
            let (x1, y1, x2, y2) = coords;

            // Get image dimensions
            let width = self.monitor1.as_ref().unwrap().width() as f32;
            let height = self.monitor1.as_ref().unwrap().height() as f32;

            // Calculate center point and scale coordinates
            let center_x = (x1 + x2) / 2.0;
            let center_y = (y1 + y2) / 2.0;

            self.click_position = Some((center_x, center_y));

            if let Some(usecase_actions) = self
                .vec_instructions
                .lock()
                .unwrap()
                .get_mut(self.index_instruction)
            {
                if self.index_action + 1 < usecase_actions.actions.len() {
                    if let ActionTypes::ClickPosition(x, y) =
                        usecase_actions.actions[self.index_action + 1]
                    {
                        usecase_actions.actions[self.index_action + 1] =
                            ActionTypes::ClickPosition(center_x, center_y);
                    } else {
                        usecase_actions.actions.insert(
                            self.index_action + 1,
                            ActionTypes::ClickPosition(center_x, center_y),
                        );
                    }
                }
            }

            println!("Click position: {:?}", self.click_position);
        } else {
            println!("Failed to parse coordinates from response");
        }
    }
    pub fn step(&mut self) {
        if self.index_instruction >= self.vec_instructions.lock().unwrap().len() {
            self.index_instruction = 0;
            self.index_action = 0;
            return;
        }
        if self.index_action
            >= self
                .vec_instructions
                .lock()
                .unwrap()
                .get(self.index_instruction)
                .unwrap()
                .actions
                .len()
        {
            self.index_instruction += 1;
            self.index_action = 0;
            return;
        }
        let action = self
            .vec_instructions
            .lock()
            .unwrap()
            .get(self.index_instruction)
            .unwrap()
            .actions[self.index_action]
            .clone();
        match action {
            ActionTypes::Click(instruction) => {
                self.grab_screenshot();
                self.click(instruction);
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

        self.index_action += 1;
        if self.index_action
            >= self
                .vec_instructions
                .lock()
                .unwrap()
                .get(self.index_instruction)
                .unwrap()
                .actions
                .len()
        {
            self.index_instruction += 1;
            self.index_action = 0;
        }

        let mut trigger_step = false;
        if let Some(actions) = self
            .vec_instructions
            .lock()
            .unwrap()
            .get(self.index_instruction)
        {
            if self.index_action >= actions.actions.len() {
                self.show = false;
            } else if let ActionTypes::KeyUp(key) = &actions.actions[self.index_action] {
                trigger_step = true;
            }
        }
        if trigger_step {
            self.step();
        }
    }

    fn draw_circle(ui: &mut egui::Ui, position: (f32, f32)) {
        #[cfg(target_os = "macos")]
        let position = (position.0, position.1 - 40.0); //adjust for menubar in macos
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        if position.1 > 1040.0 {
            ui.painter().arrow(
                egui::pos2(position.0, 1040.0 - 50.0),
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
    pub fn vizualize_next_step_3d(
        &mut self,
        egui_context: &egui::Context,
        three_d_backend: &mut ThreeDBackend,
        glfw_backend: &mut egui_overlay::egui_window_glfw_passthrough::GlfwBackend,
    ) {
        self.model
            .get_or_insert_with(|| create_triangle_model(&three_d_backend.context));

        if let Some(model) = &mut self.model {
            // Create a camera
            let camera = three_d::Camera::new_perspective(
                egui_overlay::egui_render_three_d::three_d::Viewport::new_at_origo(
                    glfw_backend.framebuffer_size_physical[0],
                    glfw_backend.framebuffer_size_physical[1],
                ),
                egui_overlay::egui_render_three_d::three_d::vec3(0.0, 0.0, 2.0),
                egui_overlay::egui_render_three_d::three_d::vec3(0.0, 0.0, 0.0),
                egui_overlay::egui_render_three_d::three_d::vec3(0.0, 1.0, 0.0),
                egui_overlay::egui_render_three_d::three_d::degrees(15.0),
                0.1,
                10.0,
            );
            // Update the animation of the triangle
            // model.animate(glfw_backend.glfw.get_time() as _);

            // Get the screen render target to be able to render something on the screen
            egui_overlay::egui_render_three_d::three_d::RenderTarget::<'_>::screen(
                &three_d_backend.context,
                glfw_backend.framebuffer_size_physical[0],
                glfw_backend.framebuffer_size_physical[1],
            )
            // Clear the color and depth of the screen render target. use transparent color.
            .clear(
                egui_overlay::egui_render_three_d::three_d::ClearState::color_and_depth(
                    0.0, 0.0, 0.0, 0.0, 1.0,
                ),
            )
            // Render the triangle with the color material which uses the per vertex colors defined at construction
            .render(&camera, std::iter::once(model), &[]);
        }

        egui::Window::new("Overlay")
            .interactable(false)
            .title_bar(false)
            .default_pos(egui::Pos2::new(1.0, 1.0))
            .min_size(egui::Vec2::new(1920.0 - 2.0, 1080.0 - 2.0))
            .show(egui_context, |ui| {
                egui::Area::new(egui::Id::new("overlay"))
                    .fixed_pos(egui::pos2(0.0, 0.0))
                    .show(egui_context, |ui| {
                        let action = self
                            .vec_instructions
                            .lock()
                            .unwrap()
                            .get(self.index_instruction)
                            .unwrap()
                            .actions
                            .get(self.index_action)
                            .unwrap()
                            .clone();
                        ui.add_sized(
                            egui::Vec2::new(400.0, 30.0),
                            egui::Label::new(egui::RichText::new(format!(
                                "PlugOvr: next action: {:?}",
                                action
                            ))),
                        );
                    });
                if let Some(click_position) = self.click_position {
                    Self::draw_circle(ui, click_position);
                }
            });
    }
    pub fn vizualize_next_step(
        &mut self,
        egui_context: &egui::Context,
        three_d_backend: &mut ThreeDBackend,
        glfw_backend: &mut egui_overlay::egui_window_glfw_passthrough::GlfwBackend,
    ) {
        if self.vec_instructions.lock().unwrap().is_empty() {
            return;
        }
        if self.index_instruction >= self.vec_instructions.lock().unwrap().len() {
            return;
        }
        if self.index_action
            >= self
                .vec_instructions
                .lock()
                .unwrap()
                .get(self.index_instruction)
                .unwrap()
                .actions
                .len()
        {
            return;
        }
        egui::Window::new("Overlay")
            .interactable(false)
            .title_bar(false)
            .default_pos(egui::Pos2::new(10.0, 1.0))
            .min_size(egui::Vec2::new(1920.0 - 2.0, 1080.0 - 2.0))
            .show(egui_context, |ui| {
                egui::Area::new(egui::Id::new("overlay"))
                    .fixed_pos(egui::pos2(0.0, 0.0))
                    .show(egui_context, |ui| {
                        let action = self
                            .vec_instructions
                            .lock()
                            .unwrap()
                            .get(self.index_instruction)
                            .unwrap()
                            .actions
                            .get(self.index_action)
                            .unwrap()
                            .clone();
                        ui.add_sized(
                            egui::Vec2::new(400.0, 30.0),
                            egui::Label::new(
                                egui::RichText::new(format!("PlugOvr: next action: {:?}", action))
                                    .background_color(egui::Color32::from_rgb(255, 255, 255)),
                            ),
                        );
                    });
                if let ActionTypes::ClickPosition(x, y) = self
                    .vec_instructions
                    .lock()
                    .unwrap()
                    .get(self.index_instruction)
                    .unwrap()
                    .actions
                    .get(self.index_action)
                    .unwrap()
                {
                    Self::draw_circle(ui, (*x, *y));
                }
            });
    }
}

fn create_triangle_model(three_d_context: &three_d::Context) -> Gm<Mesh, ColorMaterial> {
    use three_d::*;

    // Create a CPU-side mesh consisting of a single colored triangle
    let positions = vec![
        vec3(0.5, -0.5, 0.0),  // bottom right
        vec3(-0.5, -0.5, 0.0), // bottom left
        vec3(0.0, 0.5, 0.0),   // top
    ];
    let colors = vec![
        Srgba::RED,   // bottom right
        Srgba::GREEN, // bottom left
        Srgba::BLUE,  // top
    ];
    let cpu_mesh = CpuMesh {
        positions: Positions::F32(positions),
        colors: Some(colors),
        ..Default::default()
    };

    // Construct a model, with a default color material, thereby transferring the mesh data to the GPU
    let mut model = Gm::new(
        Mesh::new(three_d_context, &cpu_mesh),
        ColorMaterial::default(),
    );

    // Add an animation to the triangle.
    model.set_animation(|time| Mat4::from_angle_y(radians(time * 0.005)));
    model
}
// Add this helper function
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
    let re = regex::Regex::new(r"\[\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\]").unwrap();
    if let Some(caps) = re.captures(response) {
        let coords: Vec<f32> = (1..=4).map(|i| caps[i].parse::<f32>().unwrap()).collect();
        Some((coords[0], coords[1], coords[2], coords[3]))
    } else {
        None
    }
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

fn fix_unescaped_quotes(json_str: &str) -> String {
    let mut result = String::with_capacity(json_str.len());
    let mut in_string = false;
    let mut prev_char: Option<char> = None;

    for c in json_str.chars() {
        match c {
            '"' => {
                if let Some(prev) = prev_char {
                    // If previous char is not a backslash and we're in a string,
                    // this quote needs escaping
                    if in_string && prev != '\\' && prev != '{' && prev != ':' {
                        result.push('\\');
                    }
                    // Toggle in_string state if this quote is a string delimiter
                    if prev == ':' || prev == '{' || !in_string {
                        in_string = !in_string;
                    }
                }
                result.push(c);
            }
            _ => result.push(c),
        }
        prev_char = Some(c);
    }
    result
}

// fn load_image_from_file(path: &str) -> anyhow::Result<image::DynamicImage> {
//     Ok(image::open(path)?)
// }
