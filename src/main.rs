mod models;
mod services;
mod utils;

use axum::{
    Router,
    extract::{Multipart, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Clone)]
struct AppState {
    llm_client: Arc<services::llm::LLMClient>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    // Create the LLM client
    let llm_client = Arc::new(services::llm::LLMClient::new()?);

    // Create the application state
    let app_state = AppState { llm_client };

    // Build our application with a route
    let app = Router::new()
        .route("/", get(index))
        .route("/upload", post(upload_file))
        .route("/health", get(health_check))
        .with_state(app_state)
        // Add CORS layer
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::AllowMethods::any())
                .allow_headers(tower_http::cors::AllowHeaders::any()),
        );

    // Run our application
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index() -> Html<String> {
    let html_content = r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>TXT Chapterizer Service</title>
        <meta charset="utf-8">
        <style>
            body { font-family: Arial, sans-serif; margin: 40px; }
            .info-box { background-color: #f0f8ff; padding: 20px; border-radius: 8px; margin: 20px 0; }
            .endpoint { background-color: #f5f5f5; padding: 10px; margin: 10px 0; border-radius: 4px; font-family: monospace; }
        </style>
    </head>
    <body>
        <h1>TXT Chapterizer Service</h1>
        
        <div class="info-box">
            <h2>Service Information</h2>
            <p>This service helps you process plain text files and split them into chapters.</p>
            <p>It's designed to work with Chinese and other languages that use UTF-8 encoding.</p>
        </div>
        
        <h2>Available Endpoints:</h2>
        <div class="endpoint">GET / - This information page</div>
        <div class="endpoint">GET /health - Health check</div>
        <div class="endpoint">POST /upload - Upload a text file to process</div>
        
        <h2>How to Use:</h2>
        <p>Make a POST request to /upload with a multipart form containing a 'text_file' field</p>
        
        <h2>Supported Languages:</h2>
        <p>This service supports Chinese and other UTF-8 encoded text files.</p>
    </body>
    </html>
    "#.to_string();

    Html(html_content)
}

async fn health_check() -> &'static str {
    "OK"
}

async fn upload_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extract the uploaded text file
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or("unknown");
        if name == "text_file" {
            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;

            // Handle potential BOM (Byte Order Mark) in UTF-8 files
            let text_bytes = data.to_vec();
            let text_content = if text_bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                // Skip the UTF-8 BOM if present
                String::from_utf8(text_bytes[3..].to_vec()).map_err(|_| StatusCode::BAD_REQUEST)?
            } else {
                String::from_utf8(text_bytes).map_err(|_| StatusCode::BAD_REQUEST)?
            };

            // Process the text content into chapters
            let result = services::chapterizer::process_text(&text_content, &state.llm_client)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            return Ok(Json(serde_json::json!({
                "success": true,
                "chapter_count": result.chapters.len(),
                "download_url": format!("/download/{}", result.epub_id)
            })));
        }
    }

    Err(StatusCode::BAD_REQUEST)
}
