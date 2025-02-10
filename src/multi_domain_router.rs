use crate::acme;
use axum::{
    body::Body,
    extract::Request,
    http::Response,
    response::{Html, IntoResponse},
    Router,
};
use log::trace;
use std::{
    collections::HashMap,
    convert::Infallible,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{Service, ServiceExt};

fn acme_handler(req: Request<Body>) -> impl IntoResponse {
    trace!("Received ACME challenge request: {:?}", req);
    let challenge_token = req
        .uri()
        .path()
        .trim_start_matches("/.well-known/acme-challenge/");
    acme::acme_challenge_response(challenge_token);
    Html(format!("Challenge token: {}", challenge_token))
}

pub struct MultiDomainRouter {
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
    pub fn new() -> Self {
        MultiDomainRouter {
            mapping: HashMap::new(),
        }
    }

    pub fn add_router(&mut self, hostname: &str, router: Router) {
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
        let path = req.uri().path().to_string();
        let hostname = match req.headers().get("host") {
            Some(host) => match host.to_str() {
                Ok(host_str) => match host_str.split(':').next() {
                    Some(hostname) => hostname.to_string(),
                    None => {
                        return Box::pin(async move {
                            Ok(Response::builder()
                                .status(400)
                                .body(Body::from(
                                    "Bad Request: Invalid Host Header - No hostname found",
                                ))
                                .unwrap())
                        });
                    }
                },
                Err(_) => {
                    return Box::pin(async move {
                        Ok(Response::builder()
                    .status(400)
                    .body(Body::from("Bad Request: Invalid Host Header - Host header is not a valid string"))
                    .unwrap())
                    });
                }
            },
            None => {
                return Box::pin(async move {
                    Ok(Response::builder()
                        .status(400)
                        .body(Body::from(
                            "Bad Request: Missing Host Header - Host header is not present",
                        ))
                        .unwrap())
                });
            }
        };
        trace!("{} {} {}", hostname, req.method(), req.uri().path());
        const ACME_CHALLENGE: &str = "/.well-known/acme-challenge/";
        if path.starts_with(ACME_CHALLENGE) {
            return Box::pin(async move { Ok(acme_handler(req).into_response()) });
        }

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
