pub mod models;
pub mod services;
mod utils;

use axum::{
    Router,
    extract::{Multipart, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
};
use http;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
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
        .route("/download/:id", get(download_file))
        .nest_service("/static", ServeDir::new("static"))
        .fallback_service(ServeDir::new("static"))
        .with_state(app_state)
        // Add request logging layer
        .layer(
            TraceLayer::new_for_http()
                .on_request(|request: &http::Request<axum::body::Body>, _layer: &tracing::Span| {
                    println!("Request: {} {}", request.method(), request.uri());
                })
                .on_response(|response: &http::Response<axum::body::Body>, latency: std::time::Duration, _span: &tracing::Span| {
                    println!("Response: {} in {}ms", response.status(), latency.as_millis());
                })
        )
        // Add CORS layer
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::AllowMethods::any())
                .allow_headers(tower_http::cors::AllowHeaders::any()),
        );

    // Print server location and available routes
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    let addr = listener.local_addr()?;
    println!();
    println!("🚀 Server running at: http://{}", addr);
    println!();
    println!("📋 Available Routes:");
    println!("   GET  /               - Home page");
    println!("   POST /upload         - Upload text file for chapterization");
    println!("   GET  /health         - Health check endpoint");
    println!("   GET  /download/:id   - Download generated EPUB file");
    println!("   GET  /static/*        - Static files");
    println!();

    axum::serve(listener, app).await?;

    Ok(())
}

async fn index() -> Html<String> {
    Html(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>DUANZH - Text Chapterizer API</title>
        <meta charset="utf-8">
        <style>
            body {
                font-family: Arial, sans-serif;
                max-width: 800px;
                margin: 0 auto;
                padding: 20px;
                line-height: 1.6;
            }
            h1 {
                color: #333;
                border-bottom: 2px solid #3498db;
                padding-bottom: 10px;
            }
            .endpoint {
                background-color: #f8f9fa;
                border: 1px solid #e9ecef;
                border-radius: 4px;
                padding: 10px;
                margin: 10px 0;
            }
            .method {
                display: inline-block;
                padding: 2px 6px;
                background-color: #3498db;
                color: white;
                border-radius: 3px;
                font-weight: bold;
                margin-right: 10px;
            }
            .upload-form {
                margin-top: 20px;
                padding: 20px;
                background-color: #f9f9f9;
                border-radius: 5px;
            }
        </style>
    </head>
    <body>
        <h1>DUANZH - Text Chapterizer API</h1>
        <p>Welcome to the Text Chapterizer API service. This service converts plain text files into structured EPUB books with chapters.</p>
        
        <h2>Available Endpoints</h2>
        <div class="endpoint">
            <span class="method">GET</span>
            <strong>/</strong> - This home page
        </div>
        <div class="endpoint">
            <span class="method">GET</span>
            <strong>/health</strong> - Health check endpoint
        </div>
        <div class="endpoint">
            <span class="method">POST</span>
            <strong>/upload</strong> - Upload text file for chapterization
        </div>
        <div class="endpoint">
            <span class="method">GET</span>
            <strong>/download/:id</strong> - Download generated EPUB file
        </div>
        <div class="endpoint">
            <span class="method">GET</span>
            <strong>/static/*</strong> - Static files
        </div>
        
        <div class="upload-form">
            <h2>Try It Out</h2>
            <p>You can directly upload a text file using the form below:</p>
            <form action="/upload" method="post" enctype="multipart/form-data">
                <div>
                    <label for="text_file">Choose a text file to chapterize:</label><br>
                    <input type="file" id="text_file" name="text_file" accept=".txt" required><br><br>
                    <input type="submit" value="Upload and Process">
                </div>
            </form>
        </div>
        
        <p><a href="/static/index.html">Go to Static UI →</a></p>
    </body>
    </html>
    "#.to_string())
}

async fn health_check() -> &'static str {
    "OK"
}

async fn upload_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extract the uploaded text file
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
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
                .map_err(|e| {
                    eprintln!("Error processing text: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            return Ok(Json(serde_json::json!({
                "success": true,
                "chapter_count": result.chapters.len(),
                "download_url": format!("/download/{}", result.epub_id)
            })));
        }
    }

    Err(StatusCode::BAD_REQUEST)
}

use axum::extract::Path;

async fn download_file(Path(id): Path<String>) -> Result<axum::response::Response, StatusCode> {
    use std::fs;
    use std::path::Path as StdPath;

    // Construct the file path
    let file_path = format!("./output/{}.epub", id);

    // Check if the file exists
    if !StdPath::new(&file_path).exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Read the file content
    let file_content = fs::read(&file_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create a response with the file content
    Ok(axum::response::Response::builder()
        .header("Content-Type", "application/epub+zip")
        .header(
            "Content-Disposition",
            format!("attachment; filename=\"{}.epub\"", id),
        )
        .body(axum::body::Body::from(file_content))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
}
