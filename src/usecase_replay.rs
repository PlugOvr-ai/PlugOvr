use crate::usecase_recorder::{EventType, UseCase};

use std::fs::File;
pub struct UsecaseReplay {
    pub index: usize,
    pub UseCaseActions: Option<Vec<UsecaseActions>>,
    pub vec_usecase: Vec<UseCase>,
}

enum ActionTypes {
    Click(String),
    InsertText(String),
    KeyDown(String),
    KeyUp(String),
    GrabScreenshot,
}
struct UsecaseActions {
    pub instruction: String,
    pub action_type: Vec<ActionTypes>,
}
impl UsecaseReplay {
    pub fn new() -> Self {
        Self {
            index: 0,
            UseCaseActions: None,
            vec_usecase: vec![],
        }
    }
    pub fn load_usecase(&mut self, filename: String) {
        let file = File::open(filename).unwrap();
        let usecase: UseCase = serde_json::from_reader(file).unwrap();
        self.vec_usecase.push(usecase);
    }
    pub fn identify_usecase(&mut self, instruction: &String) -> usize {
        //find the usecase that has the most similar instruction
        0
    }
    pub fn create_usecase_actions(&mut self, index: usize, instruction: &String) {
        let mut actions = UsecaseActions {
            instruction: instruction.clone(),
            action_type: vec![],
        };
        for event in self.vec_usecase[index].usecase_steps.iter() {
            match event {
                EventType::Monitor1(_) => {
                    actions.action_type.push(ActionTypes::GrabScreenshot);
                }
                EventType::Click(_, instruction) => {
                    actions
                        .action_type
                        .push(ActionTypes::Click(instruction.clone()));
                }
                EventType::KeyDown(instruction) => {
                    actions
                        .action_type
                        .push(ActionTypes::KeyDown(instruction.clone()));
                }
                EventType::KeyUp(instruction) => {
                    actions
                        .action_type
                        .push(ActionTypes::KeyUp(instruction.clone()));
                }
                EventType::Text(instruction) => {
                    actions
                        .action_type
                        .push(ActionTypes::InsertText(instruction.clone()));
                }
                _ => {}
            }
        }
        self.UseCaseActions = Some(vec![actions]);
    }
    pub fn execute_usecase(&mut self, instruction: String) {
        let index = self.identify_usecase(&instruction);
        self.create_usecase_actions(index, &instruction);
        self.step();
    }
    pub fn step(&mut self) {}
}
