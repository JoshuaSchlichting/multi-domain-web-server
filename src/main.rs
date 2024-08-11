use axum::Router;
use log::{debug, info};
use std::net::IpAddr;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    env_logger::init();
    host_site("localhost", 80).await;
}

async fn host_site(mut host: &str, port: u16) {
    if host == "localhost" {
        host = "0.0.0.0";
    } else {
        debug!("Hosting on {}", host);
    }
    let host_ip: IpAddr = host.parse().expect("Invalid IP address");

    // Spawn the game loop in the background
    let app = Router::new().nest_service("/", ServeDir::new("./web/dist"));

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host_ip, port))
        .await
        .unwrap();
    let server_host = if host == "0.0.0.0" {
        "localhost".to_string()
    } else {
        host.to_string()
    };

    info!("Server started at: http://{}:{}", server_host, port);

    axum::serve(listener, app).await.unwrap();
}
