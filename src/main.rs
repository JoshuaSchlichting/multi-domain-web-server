#[allow(non_snake_case)]
mod REGISTER_DOMAINS_HERE;
mod domain_register;
mod multi_domain_router;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// IP address to bind to
    #[arg(short, long, default_value = "0.0.0.0")]
    host: String,

    /// Port number to bind to
    #[arg(short, long, default_value_t = 80)]
    port: u16,
}

async fn run(host_ip: &str, port: u16) {
    let router_service = domain_register::router_with_registered_domains();

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host_ip, port))
        .await
        .unwrap();
    axum::serve(listener, router_service).await.unwrap();
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();

    run(&args.host, args.port).await;
}
