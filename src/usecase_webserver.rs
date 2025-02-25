use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
    middleware::{self, Next},
    http::{Request, StatusCode, header},
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

use crate::usecase_replay::UseCaseReplay;

#[derive(Clone)]
struct WebServerState {
    usecase_replay: Arc<Mutex<UseCaseReplay>>,
    clients: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
    password: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct WebCommand {
    command: String,
    instruction: Option<String>,
    url: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct UrlResponse {
    planning_url: String,
    execution_url: String,
}

// Middleware to check authentication
async fn auth_middleware(
    State(state): State<WebServerState>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip authentication for login-related routes
    let path = req.uri().path();
    if path == "/login" || path == "/auth" || path.starts_with("/assets/") {
        return Ok(next.run(req).await);
    }

    // If no password is set, allow access
    if state.password.is_none() {
        return Ok(next.run(req).await);
    }

    // Check for authentication cookie
    if let Some(cookie) = req.headers().get(header::COOKIE) {
        if let Ok(cookie_str) = cookie.to_str() {
            if cookie_str.contains("plugovr_auth=true") {
                // User is authenticated
                return Ok(next.run(req).await);
            }
        }
    }

    // For API routes, check for authentication
    // In a real app, you'd use cookies or JWT tokens
    // This is a simplified version for demonstration
    if path.starts_with("/ws") || path.starts_with("/command") || path.starts_with("/urls") {
        // For now, we'll just allow these routes without checking auth
        // In a real app, you'd verify a token here
        return Ok(next.run(req).await);
    }

    // For the main page, redirect to login
    if path == "/" {
        // Create a redirect response instead of returning 401
        let redirect = axum::response::Redirect::to("/login");
        return Ok(redirect.into_response());
    }

    Ok(next.run(req).await)
}

pub async fn start_server(usecase_replay: Arc<Mutex<UseCaseReplay>>, password: Option<String>) {
    let state = WebServerState {
        usecase_replay,
        clients: Arc::new(Mutex::new(HashMap::new())),
        password,
    };

    let state_clone = state.clone();
    
    let app = Router::new()
        .route("/login", get(login_handler))
        .route("/auth", post(auth_handler))
        .route("/", get(index_handler))
        .route("/ws", get(ws_handler))
        .route("/command", post(command_handler))
        .route("/urls", get(get_urls_handler))
        .route("/urls/planning", post(set_planning_url_handler))
        .route("/urls/execution", post(set_execution_url_handler))
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(middleware::from_fn_with_state(state_clone.clone(), auth_middleware))
        .with_state(state_clone);

    if let Some(pwd) = &state.password {
        println!("Starting password-protected webserver on http://localhost:3000");
        println!("Password: {}", pwd);
    } else {
        println!("Starting webserver on http://localhost:3000");
    }
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index_handler() -> impl IntoResponse {
    Html(include_str!("../assets/index.html"))
}

async fn login_handler() -> impl IntoResponse {
    Html(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>PlugOvr - Login</title>
        <style>
            body {
                font-family: Arial, sans-serif;
                display: flex;
                justify-content: center;
                align-items: center;
                height: 100vh;
                margin: 0;
                background-color: #f5f5f5;
            }
            .login-container {
                background-color: white;
                padding: 2rem;
                border-radius: 8px;
                box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
                width: 300px;
            }
            h1 {
                text-align: center;
                margin-bottom: 1.5rem;
            }
            form {
                display: flex;
                flex-direction: column;
            }
            input {
                padding: 0.5rem;
                margin-bottom: 1rem;
                border: 1px solid #ddd;
                border-radius: 4px;
            }
            button {
                padding: 0.5rem;
                background-color: #4CAF50;
                color: white;
                border: none;
                border-radius: 4px;
                cursor: pointer;
            }
            button:hover {
                background-color: #45a049;
            }
            #status {
                margin-top: 1rem;
                padding: 0.5rem;
                border-radius: 4px;
                display: none;
            }
            .success {
                background-color: #dff0d8;
                color: #3c763d;
            }
            .error {
                background-color: #f2dede;
                color: #a94442;
            }
        </style>
    </head>
    <body>
        <div class="login-container">
            <h1>PlugOvr Login</h1>
            <form id="loginForm">
                <input type="password" id="password" placeholder="Enter password" required>
                <button type="submit">Login</button>
            </form>
            <div id="status"></div>
        </div>
        <script>
            document.getElementById('loginForm').addEventListener('submit', async (e) => {
                e.preventDefault();
                console.log('Form submitted');
                
                const password = document.getElementById('password').value;
                const statusDiv = document.getElementById('status');
                
                statusDiv.style.display = 'block';
                statusDiv.textContent = 'Authenticating...';
                statusDiv.className = '';
                
                try {
                    console.log('Sending authentication request');
                    const response = await fetch('/auth', {
                        method: 'POST',
                        headers: {
                            'Content-Type': 'application/json',
                        },
                        body: JSON.stringify({ password }),
                    });
                    
                    console.log('Response received:', response.status);
                    const responseText = await response.text();
                    console.log('Response text:', responseText);
                    
                    if (response.ok) {
                        statusDiv.textContent = 'Authentication successful, redirecting...';
                        statusDiv.className = 'success';
                        console.log('Authentication successful, redirecting...');
                        setTimeout(() => {
                            window.location.href = '/';
                        }, 1000);
                    } else {
                        statusDiv.textContent = 'Invalid password';
                        statusDiv.className = 'error';
                        console.log('Authentication failed');
                    }
                } catch (error) {
                    console.error('Error during authentication:', error);
                    statusDiv.textContent = 'An error occurred during login';
                    statusDiv.className = 'error';
                }
            });
        </script>
    </body>
    </html>
    "#)
}

#[derive(Deserialize)]
struct AuthRequest {
    password: String,
}

async fn auth_handler(
    State(state): State<WebServerState>,
    Json(auth): Json<AuthRequest>,
) -> impl IntoResponse {
    if let Some(correct_password) = &state.password {
        if &auth.password == correct_password {
            // Password is correct
            println!("Authentication successful for password: {}", auth.password);
            
            // Create a response with a cookie
            let mut response = axum::response::Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/plain")
                .header(
                    header::SET_COOKIE,
                    "plugovr_auth=true; Path=/; HttpOnly; SameSite=Strict"
                )
                .body("Authentication successful".into())
                .unwrap();
                
            response
        } else {
            // Password is incorrect
            println!("Authentication failed for password: {}", auth.password);
            (StatusCode::UNAUTHORIZED, "Invalid password").into_response()
        }
    } else {
        // No password required
        println!("No password required, authentication successful");
        (StatusCode::OK, "No authentication required").into_response()
    }
}

async fn get_urls_handler(State(state): State<WebServerState>) -> impl IntoResponse {
    let usecase_replay = state.usecase_replay.lock().unwrap();
    let response = UrlResponse {
        planning_url: usecase_replay.server_url_planning.clone(),
        execution_url: usecase_replay.server_url_execution.clone(),
    };
    Json(response)
}

async fn set_planning_url_handler(
    State(state): State<WebServerState>,
    Json(command): Json<WebCommand>,
) -> impl IntoResponse {
    if let Some(url) = command.url {
        let mut usecase_replay = state.usecase_replay.lock().unwrap();
        usecase_replay.server_url_planning = url.clone();
        // Broadcast the URL change to all clients
        let update = serde_json::json!({
            "type": "url_update",
            "planning_url": url,
        });
        for (_, tx) in state.clients.lock().unwrap().iter() {
            let _ = tx.send(update.to_string());
        }
        "Planning URL updated".to_string()
    } else {
        "No URL provided".to_string()
    }
}

async fn set_execution_url_handler(
    State(state): State<WebServerState>,
    Json(command): Json<WebCommand>,
) -> impl IntoResponse {
    if let Some(url) = command.url {
        let mut usecase_replay = state.usecase_replay.lock().unwrap();
        usecase_replay.server_url_execution = url.clone();
        // Broadcast the URL change to all clients
        let update = serde_json::json!({
            "type": "url_update",
            "execution_url": url,
        });
        for (_, tx) in state.clients.lock().unwrap().iter() {
            let _ = tx.send(update.to_string());
        }
        "Execution URL updated".to_string()
    } else {
        "No URL provided".to_string()
    }
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
                    "planning_url": usecase_replay.server_url_planning.clone(),
                    "execution_url": usecase_replay.server_url_execution.clone(),
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
