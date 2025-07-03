use std::fs;

use axum::{
    body::Body,
    extract::Multipart,
    http::{header, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::error;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // Clean up previous temp directory if it exists
    if !fs::exists("/tmp/adept").unwrap() {
        // Run adept_activate command
        let adept_status = std::process::Command::new("adept_activate")
            .arg("-aO")
            .arg("/tmp/adept")
            .status()
            .expect("Failed to execute adept_activate");

        if !adept_status.success() {
            panic!("adept_activate command failed with status: {}", adept_status);
        }

    }

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/dl", post(dl));

    // run our app with hyper, listening globally on port 3300
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3300")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

// basic handler that responds with a static string
async fn root() -> Response<Body> {
    // multiline string
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(
            r#"<!DOCTYPE html><html><body>
                <!-- form to submit a file -->
                <h1>Upload een URLLink.acsm bestand</h1>
                <form action="/dl" method="post" enctype="multipart/form-data">
                    <input type="file" name="file_upload">
                    <button type="submit">Submit</button>
                </form>
            </body></html>"#
                .into(),
        )
        .unwrap()
}

// endpoint that handles multipart file updload
// this handler will be called when a POST request is made to `/dl`
async fn dl(
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let field = multipart
        .next_field()
        .await
        .map_err(|e| {
            error!("Failed to get next field from multipart: {}", e);
            (StatusCode::BAD_REQUEST, "Failed to read multipart upload".to_string())
        })?
        .ok_or_else(|| {
            (StatusCode::BAD_REQUEST, "No file uploaded".to_string())
        })?;

    let content = field.bytes().await.map_err(|e| {
        error!("Failed to read bytes from multipart field: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file content".to_string())
    })?;

    let acsm_file_path = "/tmp/URLLink.acsm";
    tokio::fs::write(acsm_file_path, content)
        .await
        .map_err(|e| {
            error!("Failed to write to {}: {}", acsm_file_path, e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save uploaded file".to_string())
        })?;

    let epub_path = "/tmp/book.epub";
    let output = tokio::process::Command::new("acsmdownloader")
        .arg("-D")
        .arg("/tmp/adept")
        .arg("-o")
        .arg(epub_path)
        .arg(acsm_file_path)
        .output()
        .await
        .map_err(|e| {
            error!("Failed to execute acsmdownloader: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to run epub downloader".to_string())
        })?;


    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stdout);
        error!("acsmdownloader failed: {}", stderr);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to download book: {}", stderr),
        ));
    }

    std::process::Command::new("adept_remove").arg("-o").arg("/tmp/book.epub").arg("-D").arg("/tmp/adept").arg("/tmp/book.epub").spawn().unwrap().wait().unwrap();
    let epub_content = tokio::fs::read(epub_path).await.map_err(|e| {
        error!("Failed to read epub file {}: {}", epub_path, e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to read the generated book file".to_string(),
        )
    })?;

    // return a response with the file name and content
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/epub+zip")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"book.epub\"",
        )
        .body(Body::from(epub_content))
        .map_err(|e| {
            error!("Failed to build response: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })
}
