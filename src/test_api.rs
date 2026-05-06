use axum::{
    response::Html,
    routing::{get, post},
    Json, // NEW: For JSON responses
    Router,
};
use serde::Deserialize;
use serde::Serialize; // NEW: For converting to JSON // NEW: For deserializing JSON

#[derive(Deserialize)]
struct GreetingRequest {
    name: String,
}

// NEW: A struct to return as JSON
#[derive(Serialize)]
struct Message {
    text: String,
    from: String,
}
#[derive(Serialize)]
struct GreetingResponse {
    message: String,
}
// Route 1: Returns HTML
async fn hello_world() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

// Route 2: Returns JSON
async fn hello_json() -> Json<Message> {
    let msg = Message {
        text: "Hello from JSON!".to_string(),
        from: "Rust API".to_string(),
    };
    Json(msg)
}

// Route 3: Returns plain text
async fn hello_text() -> &'static str {
    "Hello, plain text!"
}

async fn greet_person(Json(payload): Json<GreetingRequest>) -> Json<GreetingResponse> {
    let response = GreetingResponse {
        message: format!("Hello, {}!", payload.name),
    };
    Json(response)
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(hello_world))
        .route("/json", get(hello_json)) // NEW
        .route("/text", get(hello_text)) // NEW
        .route("/greet", post(greet_person));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001")
        .await
        .unwrap();

    println!("🚀 Server running on http://127.0.0.1:3001");
    println!("   Try these URLs:");
    println!("   - http://127.0.0.1:3001/");
    println!("   - http://127.0.0.1:3001/json");
    println!("   - http://127.0.0.1:3001/text");

    axum::serve(listener, app).await.unwrap();
}
