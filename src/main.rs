use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use askama::Template;
use axum::{
    extract::Multipart,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use axum_client_ip::InsecureClientIp;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tokio::process::Command;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    if fs::read_dir("./static").is_err() {
        fs::create_dir("./static").unwrap();
    }

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
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn index() -> impl IntoResponse {
    fn into_img(str: String) -> Img {
        let (name, time) = str.split('/').last().unwrap().split_once("@").unwrap();
        let time = time.split_once('.').unwrap().0;

        let rtime = time.parse().unwrap();
        let dt: DateTime<Utc> = DateTime::from_timestamp(rtime, 0).unwrap();

        let mut time = dt.to_rfc2822();
        for _ in 0..6 {
            time.pop();
        }

        Img {
            path: str.clone(),
            name: name.replace('_', " "),
            time,
            rtime,
        }
    }
    struct Img {
        path: String,
        name: String,
        time: String,
        rtime: i64,
    }

    let mut fnames: Vec<_> = fs::read_dir("./static/")
        .unwrap()
        .into_iter()
        .map(|e| e.unwrap().path().to_str().unwrap().to_string())
        .map(into_img)
        .collect();

    fnames.sort_by(|a, b| b.rtime.cmp(&a.rtime));

    #[derive(Template)]
    #[template(path = "index.html")]
    struct IndexTempl {
        images: Vec<Img>,
    }
    IndexTempl { images: fnames }
}

async fn upload(ip: InsecureClientIp, mut multipart: Multipart) -> impl IntoResponse {
    if let Ok(Some(field)) = multipart.next_field().await {
        let ftype = field.file_name().unwrap().split('.').last().unwrap();

        let name = make_fname(ip).await;

        let path = format!("./static/{name}.{ftype}");
        let bytes = field.bytes().await.unwrap();
        if bytes.len() < 1000 {
            return Redirect::to("/");
        }

        fs::write(path, bytes).unwrap();
    }
    Redirect::to("/")
}

async fn reload() -> impl IntoResponse {
    Command::new("git").arg("pull").spawn().unwrap();
}

async fn make_fname(ip: InsecureClientIp) -> String {
    #[allow(non_snake_case)]
    #[derive(Debug, Deserialize)]
    struct IpLoc {
        countryCode: String,
        city: String,
    }

    let ip = ip.0.to_string();

    if ip == "127.0.0.1" {
        return format!(
            "localhost_(DINMOR)@{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
    }

    let json_val = String::from_utf8(
        reqwest::get(format!("http://ip-api.com/json/{ip}"))
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap()
            .into(),
    )
    .unwrap();

    let ip_loc: IpLoc = serde_json::from_str(&json_val).unwrap();

    format!(
        "{}_({})@{}",
        ip_loc.city,
        ip_loc.countryCode,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    )
}
