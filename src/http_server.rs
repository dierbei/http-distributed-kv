use crate::cache_trait::BCache;
use crate::gossip::{Command, Message};
use anyhow::Result;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex};

/// Starts the HTTP server and binds it to the given address.
///
/// This function sets up the HTTP routes and initializes the server to listen for
/// incoming requests. It also creates a channel for inter-task communication via `Sender` and `Receiver`.
///
/// # Arguments
///
/// * `addr` - The address on which the server will listen for incoming requests.
/// * `bcache` - A thread-safe, asynchronous cache that implements the `BCache` trait.
///
/// # Returns
///
/// * `Result<Receiver<Message>>` - A receiver that can be used to handle messages sent to the gossip system.
///
/// # Errors
///
/// This function will return an error if the server fails to start or bind to the provided address.
///
/// # Example
///
/// ```rust
/// let receiver = start("127.0.0.1:8080".to_string(), bcache).await?;
/// ```
pub async fn start(addr: String, bcache: Arc<Mutex<Box<dyn BCache>>>) -> Result<Receiver<Message>> {
    let (sender, receiver) = mpsc::channel(100);

    let app_state = AppState::new(sender, bcache);

    let app = Router::new()
        .route("/query", get(query))
        .route("/add", post(add))
        .route("/delete", delete(remove))
        .with_state(app_state.clone());

    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    Ok(receiver)
}

/// Holds the application state, which includes a sender for inter-task communication
/// and the shared cache (`bcache`).
///
/// This struct is wrapped in an `Arc<Mutex<>>` to ensure safe concurrent access across tasks.
pub struct AppState {
    pub sender: Sender<Message>,
    pub bcache: Arc<Mutex<Box<dyn BCache>>>,
}

impl AppState {
    /// Creates a new `AppState` instance.
    ///
    /// # Arguments
    ///
    /// * `sender` - A sender for communicating between tasks (e.g., for gossip messages).
    /// * `bcache` - A shared cache instance that implements the `BCache` trait.
    ///
    /// # Returns
    ///
    /// * `Arc<Mutex<AppState>>` - A new wrapped instance of `AppState`.
    pub fn new(sender: Sender<Message>, bcache: Arc<Mutex<Box<dyn BCache>>>) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self { sender, bcache }))
    }
}

/// Represents a standard HTTP response format with a status code, optional data, and a message.
#[derive(Serialize)]
struct Response {
    code: u16,
    data: Option<HashMap<String, String>>,
    message: String,
}

/// Represents a request to add a key-value pair to the cache.
#[derive(Debug, Deserialize, Clone)]
struct AddRequest {
    key: String,
    value: String,
}

/// Represents a request to remove a key-value pair to the cache.
#[derive(Debug, Deserialize, Clone)]
struct RemoveRequest {
    key: String,
}

/// Handles HTTP GET requests to query a value from the cache.
///
/// # Arguments
///
/// * `app_states` - The current application state containing the cache.
/// * `params` - The query parameters containing the key to be looked up.
///
/// # Returns
///
/// * `Json<Response>` - A JSON response containing the key-value pair, or an error message if the key is missing or the query fails.
async fn query(
    State(app_states): State<Arc<Mutex<AppState>>>,
    params: Query<HashMap<String, String>>,
) -> Json<Response> {
    let key = if let Some(k) = params.get("key") {
        k
    } else {
        return Json(Response {
            code: StatusCode::BAD_REQUEST.as_u16(),
            data: None,
            message: "Missing 'key' parameter".to_string(),
        });
    };

    let value = {
        match app_states
            .lock()
            .await
            .bcache
            .lock()
            .await
            .get(key.clone())
            .await
        {
            Ok(v) => v,
            Err(_) => {
                return Json(Response {
                    code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                    data: None,
                    message: "Failed to retrieve value from cache".to_string(),
                });
            }
        }
    };

    let mut data = HashMap::new();
    data.insert(key.clone(), value);

    Json(Response {
        code: StatusCode::OK.as_u16(),
        data: Some(data),
        message: "ok".to_string(),
    })
}

/// Handles HTTP POST requests to add a key-value pair to the cache.
///
/// # Arguments
///
/// * `app_states` - The current application state containing the cache.
/// * `params` - The JSON body containing the key-value pair to be added.
///
/// # Returns
///
/// * `Json<Response>` - A JSON response indicating the success or failure of the operation.
async fn add(
    State(app_states): State<Arc<Mutex<AppState>>>,
    params: Json<AddRequest>,
) -> Json<Response> {
    let key = params.key.clone();
    let value = params.value.clone();
    let app_states = app_states.lock().await;

    app_states
        .bcache
        .lock()
        .await
        .insert(key.clone(), value.clone())
        .await;
    if let Err(e) = app_states
        .sender
        .send(Message {
            cmd: Command::Insert,
            key: key.clone(),
            value: value.clone(),
        })
        .await
    {
        tracing::error!("Failed to send insert message: {:?}", e);
        return Json(Response {
            code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            data: None,
            message: "Failed to process add request".to_string(),
        });
    }

    let mut data = HashMap::new();
    data.insert(params.key.clone(), params.value.clone());

    Json(Response {
        code: StatusCode::OK.as_u16(),
        data: Some(data),
        message: "ok".to_string(),
    })
}

/// Handles HTTP DELETE requests to remove a key from the cache.
///
/// # Arguments
///
/// * `app_states` - The current application state containing the cache.
/// * `params` - The JSON body containing the key to be removed.
///
/// # Returns
///
/// * `Json<Response>` - A JSON response indicating the success or failure of the operation.
async fn remove(
    State(app_states): State<Arc<Mutex<AppState>>>,
    params: Json<RemoveRequest>,
) -> Json<Response> {
    let app_states = app_states.lock().await;
    let key = params.key.clone();

    app_states.bcache.lock().await.remove(key.clone()).await;
    if let Err(e) = app_states
        .sender
        .send(Message {
            cmd: Command::Remove,
            key,
            value: "".to_string(),
        })
        .await
    {
        tracing::error!("Failed to send remove message: {:?}", e);
        return Json(Response {
            code: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            data: None,
            message: "Failed to process remove request".to_string(),
        });
    }

    Json(Response {
        code: StatusCode::OK.as_u16(),
        data: None,
        message: "ok".to_string(),
    })
}
