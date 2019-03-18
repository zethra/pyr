use crate::Handler;
use crate::Route;

pub struct RouteBuilder<T: Clone> {
    route: Route<T>,
}

impl<T: Clone> RouteBuilder<T> {
    pub fn new(route: Route<T>) -> RouteBuilder<T> {
        RouteBuilder { route }
    }

    pub fn with_state(mut self, state: T) -> RouteBuilder<T> {
        self.route.state = Some(state);
        self
    }

    /// Completes the building process by taking the handler to process the request.
    ///
    /// Returns created route.
    pub fn using(mut self, handler: Handler<T>) -> Route<T> {
        self.route.handler = handler;
        self.route
    }
}
