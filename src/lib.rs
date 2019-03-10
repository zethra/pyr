#[macro_use]
extern crate cpython;

use cpython::*;
use hyper::{Body, Request, Response, Server};
use hyper::service::service_fn_ok;
use hyper::rt::{self, Future};
use futures::sync::oneshot::Sender;
use std::thread;
use std::sync::{Mutex, Arc};

py_module_initializer!(libpyr, initlibpyr, PyInit_libpyr, |py, m| {
    m.add(py, "__doc__", "Pyr docs.")?;
    m.add(py, "callback", py_fn!(py, callback(fnc: PyObject)))?;
    m.add(py, "start_server", py_fn!(py, start_server(handler: PyObject)))?;
    m.add(py, "stop_server", py_fn!(py, stop_server(raw_handel: i64)))?;
    Ok(())
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

fn start_server(py: Python, handler_fn: PyObject) -> PyResult<PyLong> {
    let addr = ([127, 0, 0, 1], 3000).into();

    let (tx, rx) = futures::sync::oneshot::channel::<()>();
    let handler_mutex = Arc::new(Mutex::new(handler_fn));

    let server = Server::bind(&addr).serve(move || {
        let handler_mutex = handler_mutex.clone();
        service_fn_ok(move |req: Request<Body>| {
            let gil = GILGuard::acquire();
            let py = gil.python();
            let handler_fn = handler_mutex.lock().unwrap();
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
    server_handel.shutdown().map_err(|_| PyErr::new::<exc::TypeError, _>(py, "Failed to shutdown server"))?;
    Ok(py.None())
}