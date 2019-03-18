#![doc(html_root_url = "https://marad.github.io/hyper-router/doc/hyper_router")]

//! # Hyper Router
//!
//! This cargo is a small extension to the great Hyper HTTP library. It basically is
//! adds the ability to define routes to request handlers and then query for the handlers
//! by request path.
//!
//! ## Usage
//!
//! To use the library just add:
//!
//! ```text
//! hyper = "^0.12"
//! hyper-router = "^0.5"
//! ```
//!
//! to your dependencies.
//!
//! ```no_run
//! extern crate hyper;
//! extern crate hyper_router;
//!
//! use hyper::header::{CONTENT_LENGTH, CONTENT_TYPE};
//! use hyper::{Request, Response, Body, Method};
//! use hyper::server::Server;
//! use hyper::rt::Future;
//! use hyper_router::{Route, RouterBuilder, RouterService};
//!
//! fn request_handler(_: Request<Body>) -> Response<Body> {
//!     let body = "Hello World";
//!     Response::builder()
//!         .header(CONTENT_LENGTH, body.len() as u64)
//!         .header(CONTENT_TYPE, "text/plain")
//!         .body(Body::from(body))
//!         .expect("Failed to construct the response")
//! }
//!
//! fn router_service() -> Result<RouterService<i8>, std::io::Error> {
//!     let router = RouterBuilder::new()
//!         .add(Route::get("/hello").using(request_handler))
//!         .add(Route::from(Method::PATCH, "/asd").using(request_handler))
//!         .build();
//!
//!     Ok(RouterService::new(router))
//! }
//!
//! fn main() {
//!     let addr = "0.0.0.0:8080".parse().unwrap();
//!     let server = Server::bind(&addr)
//!         .serve(router_service)
//!         .map_err(|e| eprintln!("server error: {}", e));
//!
//!     hyper::rt::run(server)
//! }
//! ```
//!
//! This code will start Hyper server and add use router to find handlers for request.
//! We create the `Route` so that when we visit path `/greet` the `basic_handler` handler
//! will be called.
//!
//! ## Things to note
//!
//! * `Path::new` method accepts regular expressions so you can match every path you please.
//! * If you have request matching multiple paths the one that was first `add`ed will be chosen.
//! * This library is in an early stage of development so there may be breaking changes comming
//! (but I'll try as hard as I can not to break backwards compatibility or break it just a little -
//! I promise I'll try!).
//!
//! # Waiting for your feedback
//!
//! I've created this little tool to help myself learn Rust and to avoid using big frameworks
//! like Iron or rustful. I just want to keep things simple.
//!
//! Obviously I could make some errors or bad design choices so I'm waiting for your feedback!
//! You may create an issue at [project's bug tracker](https://github.com/marad/hyper-router/issues).

extern crate futures;
extern crate hyper;

use futures::future::FutureResult;
use hyper::header::CONTENT_LENGTH;
use hyper::service::{Service, MakeService, NewService};
use hyper::{Body, Request, Response};

use hyper::Method;
use hyper::StatusCode;

mod builder;
pub mod handlers;
mod path;
pub mod route;

pub use self::builder::RouterBuilder;
pub use self::path::Path;
pub use self::route::Route;
pub use self::route::RouteBuilder;

pub type Handler<T> = fn(Request<Body>, Option<T>) -> Response<Body>;
pub type HttpResult<T> = Result<T, StatusCode>;

/// This is the one. The router.
#[derive(Debug, Clone)]
pub struct Router<T: Clone> {
    routes: Vec<Route<T>>,
}

impl<T: Clone> Router<T> {
    /// Finds handler for given Hyper request.
    ///
    /// This method uses default error handlers.
    /// If the request does not match any route than default 404 handler is returned.
    /// If the request match some routes but http method does not match (used GET but routes are
    /// defined for POST) than default method not supported handler is returned.
    pub fn find_handler_with_defaults(&self, request: &Request<Body>) -> Handler<T> {
        let matching_routes = self.find_matching_routes(request.uri().path());
        match matching_routes.len() {
            x if x == 0 => handlers::default_404_handler,
            _ => self
                .find_for_method(&matching_routes, request.method())
                .map(|(handler, _)| handler)
                .unwrap_or(handlers::method_not_supported_handler),
        }
    }

    /// Finds handler for given Hyper request.
    ///
    /// It returns handler if it's found or `StatusCode` for error.
    /// This method may return `NotFound`, `MethodNotAllowed` or `NotImplemented`
    /// status codes.
    pub fn find_handler(&self, request: &Request<Body>) -> HttpResult<(Handler<T>, Option<T>)> {
        let matching_routes = self.find_matching_routes(request.uri().path());
        match matching_routes.len() {
            x if x == 0 => Err(StatusCode::NOT_FOUND),
            _ => self
                .find_for_method(&matching_routes, request.method())
                .map(Ok)
                .unwrap_or(Err(StatusCode::METHOD_NOT_ALLOWED)),
        }
    }

    /// Returns vector of `Route`s that match to given path.
    pub fn find_matching_routes(&self, request_path: &str) -> Vec<&Route<T>> {
        self.routes
            .iter()
            .filter(|route| route.path.matcher.is_match(&request_path))
            .collect()
    }

    fn find_for_method(&self, routes: &[&Route<T>], method: &Method) -> Option<(Handler<T>, Option<T>)> {
        let method = method.clone();
        routes
            .iter()
            .find(|route| route.method == method)
            .map(|route| (route.handler, route.state.clone()))
    }
}

/// The default simple router service.
#[derive(Debug, Clone)]
pub struct RouterService<T: Clone> {
    pub router: Router<T>,
    pub error_handler: fn(StatusCode) -> Response<Body>,
}

impl<T: Clone> RouterService<T> {
    pub fn new(router: Router<T>) -> RouterService<T> {
        RouterService {
            router,
            error_handler: Self::default_error_handler,
        }
    }

    fn default_error_handler(status_code: StatusCode) -> Response<Body> {
        let error = "Routing error: page not found";
        Response::builder()
            .header(CONTENT_LENGTH, error.len() as u64)
            .status(match status_code {
                StatusCode::NOT_FOUND => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            })
            .body(Body::from(error))
            .expect("Failed to construct a response")
    }
}

impl<T: Clone> Service for RouterService<T> {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Future = FutureResult<Response<Body>, hyper::Error>;

    fn call(&mut self, request: Request<Self::ReqBody>) -> Self::Future {
        futures::future::ok(match self.router.find_handler(&request) {
            Ok((handler, state)) => handler(request, state),
            Err(status_code) => (self.error_handler)(status_code),
        })
    }
}

impl<T: Clone> NewService for RouterService<T> {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = hyper::Error;
    type Service = RouterService<T>;
    type Future = FutureResult<Self::Service, Self::InitError>;
    type InitError = hyper::Error;

    fn new_service(&self) -> Self::Future {
        futures::future::ok(self.clone())
    }
}