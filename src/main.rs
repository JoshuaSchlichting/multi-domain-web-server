use axum::http::Response;
use axum::{body::Body, http::Request, response::Html, routing::any, Extension, Router};
use log::trace;
use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Service, ServiceExt};
use tower_http::services::ServeDir;
struct State {
    api_requests: AtomicUsize,
}

#[tokio::main]
async fn main() {
    run("0.0.0.0", 80).await;
}

async fn run(host_ip: &str, port: u16) {
    env_logger::init();

    let api_router = Router::new().route("/", any(api_handler));
    let website_router = Router::new().nest_service("/", ServeDir::new("web/dist"));

    let state = Arc::new(State {
        api_requests: AtomicUsize::new(0),
    });

    let mut router = MultiDomainRouter::new();
    router.add_router("www.localhost", website_router);
    router.add_router("api.localhost", api_router);

    let app = Router::new()
        .fallback_service(router)
        .layer(Extension(state));

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host_ip, port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn api_handler(Extension(state): Extension<Arc<State>>) -> Html<String> {
    state.api_requests.fetch_add(1, Ordering::SeqCst);
    Html(format!(
        "api: {}",
        state.api_requests.load(Ordering::SeqCst)
    ))
}

struct MultiDomainRouter {
    mapping: HashMap<String, Router>,
}

impl Clone for MultiDomainRouter {
    fn clone(&self) -> Self {
        MultiDomainRouter {
            mapping: self.mapping.clone(),
        }
    }
}

impl MultiDomainRouter {
    fn new() -> Self {
        MultiDomainRouter {
            mapping: HashMap::new(),
        }
    }

    fn add_router(&mut self, hostname: &str, router: Router) {
        self.mapping.insert(hostname.to_string(), router);
        println!("Listening on http://{}", hostname);
    }
}

impl Service<Request<Body>> for MultiDomainRouter {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let hostname = req
            .headers()
            .get("host")
            .unwrap()
            .to_str()
            .unwrap()
            .split(':')
            .next()
            .unwrap()
            .to_string();
        trace!("{} {} {}", hostname, req.method(), req.uri().path());
        let router = match self.mapping.get(&hostname) {
            Some(router) => router,
            None => {
                return Box::pin(async move {
                    Ok(Response::builder()
                        .status(404)
                        .body(Body::from("Not Found"))
                        .unwrap())
                });
            }
        };

        let future_response = router.clone().oneshot(req);

        Box::pin(async move {
            match future_response.await {
                Ok(response) => Ok(response),
                Err(_) => Ok(Response::builder()
                    .status(500)
                    .body(Body::from("Internal Server Error"))
                    .unwrap()),
            }
        })
    }
}
