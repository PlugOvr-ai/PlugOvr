use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

use crate::usecase_replay::{ActionTypes, UseCaseActions, UseCaseReplay};

#[derive(Clone)]
struct WebServerState {
    usecase_replay: Arc<Mutex<UseCaseReplay>>,
    clients: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
}

#[derive(Serialize, Deserialize)]
struct WebCommand {
    command: String,
    instruction: Option<String>,
}

pub async fn start_server(usecase_replay: Arc<Mutex<UseCaseReplay>>) {
    let state = WebServerState {
        usecase_replay,
        clients: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/ws", get(ws_handler))
        .route("/command", post(command_handler))
        .nest_service("/assets", ServeDir::new("assets"))
        .with_state(state);

    println!("Starting webserver on http://localhost:3000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index_handler() -> impl IntoResponse {
    Html(include_str!("../assets/index.html"))
}

async fn command_handler(
    State(state): State<WebServerState>,
    Json(command): Json<WebCommand>,
) -> impl IntoResponse {
    match command.command.as_str() {
        "next" => {
            state.usecase_replay.lock().unwrap().step();
            "Next action triggered".to_string()
        }
        "new_instruction" => {
            if let Some(instruction) = command.instruction {
                state
                    .usecase_replay
                    .lock()
                    .unwrap()
                    .vec_instructions
                    .lock()
                    .unwrap()
                    .clear();
                state
                    .usecase_replay
                    .lock()
                    .unwrap()
                    .execute_usecase(instruction);
                state.usecase_replay.lock().unwrap().show = true;
                *state
                    .usecase_replay
                    .lock()
                    .unwrap()
                    .index_action
                    .lock()
                    .unwrap() = 0;
                *state
                    .usecase_replay
                    .lock()
                    .unwrap()
                    .index_instruction
                    .lock()
                    .unwrap() = 0;
                "New instruction executed".to_string()
            } else {
                "No instruction provided".to_string()
            }
        }
        _ => "Unknown command".to_string(),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<WebServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: WebServerState) {
    let (mut sender, mut receiver) = socket.split();

    let client_id = uuid::Uuid::new_v4().to_string();
    let (tx, mut rx) = broadcast::channel(100);
    state
        .clients
        .lock()
        .unwrap()
        .insert(client_id.clone(), tx.clone());

    // Spawn task to send screenshots and overlay data
    let state_clone = state.clone();
    let tx_clone = tx.clone();

    // Spawn a task to forward broadcast messages to the WebSocket
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Spawn task to capture and send updates
    let update_task = tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            let mut usecase_replay = state_clone.usecase_replay.lock().unwrap();
            usecase_replay.grab_screenshot();
            if let Some(screenshot) = &usecase_replay.monitor1 {
                let mut buffer = Vec::new();
                screenshot
                    .write_to(
                        &mut std::io::Cursor::new(&mut buffer),
                        image::ImageFormat::Png,
                    )
                    .unwrap();
                let base64_image = STANDARD.encode(&buffer);

                let index_instruction = *usecase_replay.index_instruction.lock().unwrap();
                let index_action = *usecase_replay.index_action.lock().unwrap();

                let current_action = if let Some(actions) = usecase_replay
                    .vec_instructions
                    .lock()
                    .unwrap()
                    .get(index_instruction)
                {
                    if index_action < actions.actions.len() {
                        Some(actions.actions[index_action].clone())
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Get the full action plan and mark executed actions
                let action_plan = if let Some(actions) = usecase_replay
                    .vec_instructions
                    .lock()
                    .unwrap()
                    .get(index_instruction)
                {
                    let mut plan_with_status: Vec<serde_json::Value> = actions
                        .actions
                        .iter()
                        .enumerate()
                        .map(|(i, action)| {
                            serde_json::json!({
                                "action": action,
                                "executed": i < index_action,
                                "current": i == index_action
                            })
                        })
                        .collect();
                    Some(plan_with_status)
                } else {
                    None
                };

                let update = serde_json::json!({
                    "type": "update",
                    "screenshot": base64_image,
                    "current_action": current_action,
                    "action_plan": action_plan,
                    "computing": *usecase_replay.computing_action.lock().unwrap(),
                    "computing_plan": *usecase_replay.computing_plan.lock().unwrap(),
                    "show": usecase_replay.show,
                });
                //println!("Sending update: {}", update);

                if let Err(e) = tx_clone.send(update.to_string()) {
                    println!("Error sending update: {}", e);
                    break;
                }
            }
        }
    });

    // Handle incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            println!("Received message: {}", text);
        }
    }

    // Clean up when client disconnects
    state.clients.lock().unwrap().remove(&client_id);
    update_task.abort();
    send_task.abort();
}
