use std::fs;

use askama::Template;
use axum::{
    extract::Multipart,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use tokio::process::Command;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let serve_dir = ServeDir::new("static");

    let app = Router::new()
        .nest_service("/static", serve_dir)
        .route("/", get(index))
        .route("/upload", post(upload))
        .route("/reload", post(reload));

    let addr = {
        #[cfg(debug_assertions)]
        {
            "0.0.0.0:3000"
        }
        #[cfg(not(debug_assertions))]
        "0.0.0.0:80"
    };

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn index() -> impl IntoResponse {
    let images = fs::read_dir("./static/")
        .unwrap()
        .into_iter()
        .map(|e| e.unwrap().path().to_str().unwrap().to_string())
        .collect();

    #[derive(Template)]
    #[template(path = "index.html")]
    struct IndexTempl {
        images: Vec<String>,
    }
    IndexTempl { images }
}

async fn upload(mut multipart: Multipart) -> impl IntoResponse {
    while let Ok(Some(field)) = multipart.next_field().await {
        let ftype = field.file_name().unwrap().split('.').last().unwrap();
        let name = uuid::Uuid::new_v4().to_string();

        let path = format!("./static/{name}.{ftype}");
        let bytes = field.bytes().await.unwrap();
        if bytes.len() < 1000 {
            continue;
        }

        fs::write(path, bytes).unwrap();
    }
    Redirect::to("/")
}

async fn reload() -> impl IntoResponse {
    Command::new("git").arg("pull").spawn().unwrap();
}
