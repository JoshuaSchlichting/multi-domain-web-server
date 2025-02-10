use crate::multi_domain_router::MultiDomainRouter;
use crate::REGISTER_DOMAINS_HERE::{State, HOSTNAME_ROUTERS};
use axum::{Extension, Router};
use std::sync::{atomic::AtomicUsize, Arc};

pub fn router_with_registered_domains() -> Router {
    let mut master_router = MultiDomainRouter::new();
    for (hostname, router) in HOSTNAME_ROUTERS {
        master_router.add_router(hostname, router());
    }
    let state = Arc::new(State {
        api_requests: AtomicUsize::new(0),
    });
    Router::new()
        .fallback_service(master_router)
        .layer(Extension(state))
}
