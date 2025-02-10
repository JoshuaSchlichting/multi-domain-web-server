#[allow(non_snake_case)]
mod REGISTER_DOMAINS_HERE;
mod domain_register;
mod multi_domain_router;

#[tokio::main]
async fn main() {
    env_logger::init();
    run("0.0.0.0", 80).await;
}

async fn run(host_ip: &str, port: u16) {
    let router_service = domain_register::router_with_registered_domains();

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host_ip, port))
        .await
        .unwrap();
    axum::serve(listener, router_service).await.unwrap();
}
