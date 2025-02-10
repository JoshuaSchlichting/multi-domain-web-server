use axum::response::Html;
use axum::routing::any;
use axum::{Extension, Router};
use std::sync::{atomic::AtomicUsize, atomic::Ordering, Arc};
use tower_http::services::ServeDir;

pub struct State {
    pub api_requests: AtomicUsize,
}

async fn api_handler(Extension(state): Extension<Arc<State>>) -> Html<String> {
    state.api_requests.fetch_add(1, Ordering::SeqCst);
    Html(format!(
        "api: {}",
        state.api_requests.load(Ordering::SeqCst)
    ))
}

type HostnameRouterCombo = (&'static str, fn() -> Router);

pub const HOSTNAME_ROUTERS: &[HostnameRouterCombo] = &[
    ("api.localhost", || {
        Router::new().route("/", any(api_handler))
    }),
    ("www.localhost", || {
        Router::new().nest_service("/", ServeDir::new("web/dist"))
    }),
];
