use cpython::*;
use hyper::{Body, Request, Response, Server, StatusCode, Method};
use hyper::rt::{self, Future};
use hyper_router::{Route, RouterBuilder, RouterService, Router};
use futures::sync::oneshot::Sender;
use std::thread;
use std::sync::{Mutex, Arc};
use lazy_static::lazy_static;

lazy_static!{
    static ref LOCK: Mutex<()> = Mutex::new(());
}

py_module_initializer!(libpyr, initlibpyr, PyInit_libpyr, |py, m| {
    m.add(py, "__doc__", "Pyr docs.")?;
    m.add(py, "callback", py_fn!(py, callback(fnc: PyObject)))?;
    m.add(py, "start_server", py_fn!(py, start_server(addr: String, routes: PyList)))?;
    m.add(py, "stop_server", py_fn!(py, stop_server(raw_handel: i64)))?;
    m.add_class::<PyRequest>(py)?;
    m.add_class::<PyrRoute>(py)?;
    Ok(())
});

py_class!(pub class PyRequest |py| {
    data _request: i64;

    def __new__(_cls, arg: i64) -> PyResult<PyRequest> {
        PyRequest::create_instance(py, arg)
    }
});

py_class!(pub class PyrRoute |py| {
    data path: String;
    data handler_fn: PyObject;
    data method: PyBytes;

    def __new__(_cls, path: String, handler_fn: PyObject, method: PyBytes) -> PyResult<PyrRoute> {
        PyrRoute::create_instance(py, path, handler_fn, method)
    }
});

fn callback(py: Python, fnc: PyObject) -> PyResult<PyObject> {
    let args = vec![1, 2];
    let tmp: Vec<PyObject> = args.iter().map(|arg| {
        let x:PyLong = arg.into_py_object(py);
        x.into_object()
    }).collect();
    let res = fnc.call(py, PyTuple::new(py, tmp.as_slice()), None).unwrap();
    let num: i64 = res.extract(py).unwrap();
    println!("{}", num);
    Ok(py.None())
}

struct ServerHandel {
    shutdown_channel: Sender<()>,
}

impl ServerHandel {
    fn new(shutdown_channel: Sender<()>) -> ServerHandel {
        ServerHandel {
            shutdown_channel
        }
    }

    fn shutdown(self) -> Result<(), ()> {
        self.shutdown_channel.send(())
    }
}

fn handler(req: Request<Body>, state: Option<Arc<Mutex<PyObject>>>) -> Response<Body> {
    let _lock = LOCK.lock().unwrap();
    let gil = GILGuard::acquire();
    let py = gil.python();
    let state = state.unwrap();
    let handler_fn = state.lock().unwrap();
    let res = match handler_fn.call(py, NoArgs, None) {
        Ok(res) => res,
        Err(e) => {
            e.print(py);
            let error = Body::from("Internal Server Error");
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(error)
                .expect("Failed to construct a response")
        }
    };
    let resp: String = res.extract(py).unwrap();
    Response::new(Body::from(resp))
}

fn parse_routes(py: &Python, routes: PyList) -> Router<Arc<Mutex<PyObject>>> {
    let mut router = RouterBuilder::new();
    for pyr_route in routes.iter(*py) {
        let pyr_route = pyr_route.cast_into::<PyrRoute>(*py).unwrap();
        let path: String = pyr_route.path(*py).clone();
        let handler_fn: PyObject = pyr_route.handler_fn(*py).extract(*py).unwrap();
        let method = Method::from_bytes(pyr_route.method(*py).data(*py)).unwrap();
        let handler_fn = Arc::new(Mutex::new(handler_fn));
        router = router.add(Route::from(method, &path).with_state(handler_fn).using(handler));
    }
    router.build()
}

fn start_server(py: Python, addr: String, routes: PyList) -> PyResult<PyLong> {
    let addr = addr.parse().unwrap();

    let router_service = RouterService::new(parse_routes(&py, routes));
    let (tx, rx) = futures::sync::oneshot::channel::<()>();

    let server = Server::bind(&addr)
        .serve(router_service)
        .with_graceful_shutdown(rx)
        .map_err(|e| eprintln!("server error: {}", e));

    println!("Listening on http://{}", addr);
    thread::spawn(move || {
        rt::run(server);
    });

    let server_handel = ServerHandel::new(tx);

    Ok((Box::into_raw(Box::new(server_handel)) as i64).to_py_object(py))
}

fn stop_server(py: Python, raw_handel: i64) -> PyResult<PyObject> {
    let server_handel: Box<ServerHandel> = unsafe {
        Box::from_raw(raw_handel as *mut _)
    };
    server_handel.shutdown()
        .map_err(|_| PyErr::new::<exc::TypeError, _>(py, "Failed to shutdown server"))?;
    Ok(py.None())
}