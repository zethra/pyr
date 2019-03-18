use cpython::*;
use hyper::{Body, Request, Response, Server, StatusCode};
use hyper::service::service_fn_ok;
use hyper::rt::{self, Future};
use futures::sync::oneshot::Sender;
use std::thread;
use std::sync::{Mutex, Arc};
use std::collections::HashMap;

py_module_initializer!(libpyr, initlibpyr, PyInit_libpyr, |py, m| {
    m.add(py, "__doc__", "Pyr docs.")?;
    m.add(py, "callback", py_fn!(py, callback(fnc: PyObject)))?;
    m.add(py, "start_server", py_fn!(py, start_server(routes: PyList)))?;
    m.add(py, "stop_server", py_fn!(py, stop_server(raw_handel: i64)))?;
    m.add_class::<PyRequest>(py)?;
    m.add_class::<Route>(py)?;
    Ok(())
});

py_class!(pub class PyRequest |py| {
    data _request: i64;

    def __new__(_cls, arg: i64) -> PyResult<PyRequest> {
        PyRequest::create_instance(py, arg)
    }
});

py_class!(pub class Route |py| {
    data path: String;
    data handler_fn: PyObject;
    data method: String;

    def __new__(_cls, path: String, handler_fn: PyObject, method: String) -> PyResult<Route> {
        Route::create_instance(py, path, handler_fn, method)
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

fn parse_routes(py: &Python, routes: PyList) -> HashMap<String, PyObject> {
    let mut ret = HashMap::new();
    for route in routes.iter(*py) {
        let route = route.cast_into::<Route>(*py).unwrap();
        let path: String = route.path(*py).clone();
        let handler_fn: PyObject = route.handler_fn(*py).extract(*py).unwrap();
        ret.insert(path, handler_fn);
    }
    ret
}

fn start_server(py: Python, routes: PyList) -> PyResult<PyLong> {
    let addr = ([127, 0, 0, 1], 3000).into();

    let routes_mutex =
        Arc::new(Mutex::new(parse_routes(&py, routes)));
    let (tx, rx) = futures::sync::oneshot::channel::<()>();

    let server = Server::bind(&addr).serve(move || {
        let routes_mutex = routes_mutex.clone();
        service_fn_ok(move |req: Request<Body>| {
            let routes = routes_mutex.lock().unwrap();
            let handler_fn = match routes.get(req.uri().path()) {
                Some(value) => value,
                None => return Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from("404"))
                    .unwrap()
            };
            let gil = GILGuard::acquire();
            let py = gil.python();
            let res = handler_fn.call(py, NoArgs, None).unwrap();
            let resp: String = res.extract(py).unwrap();
            Response::new(Body::from(resp))
        })
    }).with_graceful_shutdown(rx)
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