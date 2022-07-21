use pyo3::prelude::*;
use pyo3::types::PyType;

use std::net::{IpAddr, SocketAddr, TcpListener};
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawSocket, FromRawSocket};
use std::time::Duration;

use socket2::{Domain, Protocol, Socket, TcpKeepalive, Type};


#[pyclass(module="granian._granian")]
pub struct SocketHolder {
    socket: Socket
}

#[pymethods]
impl SocketHolder {
    #[cfg(unix)]
    #[new]
    pub fn new(fd: i32) -> PyResult<Self> {
        let socket = unsafe {
            Socket::from_raw_fd(fd)
        };
        Ok(Self { socket: socket })
    }

    #[cfg(windows)]
    #[new]
    pub fn new(fd: u64) -> PyResult<Self> {
        let socket = unsafe {
            Socket::from_raw_socket(fd)
        };
        Ok(Self { socket: socket })
    }

    #[classmethod]
    pub fn from_address(
        _cls: &PyType,
        address: &str,
        port: u16,
        backlog: i32
    ) -> PyResult<Self> {
        let address: SocketAddr = (address.parse::<IpAddr>()?, port).into();
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
        socket.set_reuse_address(true)?;
        socket.set_tcp_keepalive(
            &TcpKeepalive::new().with_time(Duration::from_secs(0))
        )?;
        socket.set_nodelay(true)?;
        socket.bind(&address.into())?;
        socket.listen(backlog)?;
        Ok(Self { socket: socket })
    }

    #[cfg(unix)]
    pub fn __getstate__(&self, py: Python) -> PyObject {
        let fd = self.socket.as_raw_fd();
        (
            fd.into_py(py),
        ).to_object(py)
    }

    #[cfg(windows)]
    pub fn __getstate__(&self, py: Python) -> PyObject {
        let fd = self.socket.as_raw_socket();
        (
            fd.into_py(py),
        ).to_object(py)
    }

    #[cfg(unix)]
    pub fn get_fd(&self, py: Python) -> PyObject {
        self.socket.as_raw_fd().into_py(py).to_object(py)
    }

    #[cfg(windows)]
    pub fn get_fd(&self, py: Python) -> PyObject {
        self.socket.as_raw_socket().into_py(py).to_object(py)
    }
}

impl SocketHolder {
    pub fn get_socket(&self) -> Socket {
        self.socket.try_clone().unwrap()
    }

    pub fn get_listener(&self) -> TcpListener {
        self.socket.try_clone().unwrap().into()
    }
}

#[pyclass(module="granian._granian")]
pub struct ListenerHolder {
    socket: TcpListener
}

#[pymethods]
impl ListenerHolder {
    #[cfg(unix)]
    #[new]
    pub fn new(fd: i32) -> PyResult<Self> {
        let socket = unsafe {
            TcpListener::from_raw_fd(fd)
        };
        Ok(Self { socket: socket })
    }

    #[cfg(windows)]
    #[new]
    pub fn new(fd: u64) -> PyResult<Self> {
        let socket = unsafe {
            TcpListener::from_raw_socket(fd)
        };
        Ok(Self { socket: socket })
    }

    #[classmethod]
    pub fn from_address(
        _cls: &PyType,
        address: &str,
        port: u16,
        backlog: i32
    ) -> PyResult<Self> {
        let address: SocketAddr = (address.parse::<IpAddr>()?, port).into();
        let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
        socket.set_reuse_address(true)?;
        socket.bind(&address.into())?;
        socket.listen(backlog)?;
        let listener: TcpListener = socket.into();
        Ok(Self { socket: listener })
    }

    #[cfg(unix)]
    pub fn __getstate__(&self, py: Python) -> PyObject {
        let fd = self.socket.as_raw_fd();
        (
            fd.into_py(py),
        ).to_object(py)
    }

    #[cfg(windows)]
    pub fn __getstate__(&self, py: Python) -> PyObject {
        let fd = self.socket.as_raw_socket();
        (
            fd.into_py(py),
        ).to_object(py)
    }

    #[cfg(unix)]
    pub fn get_fd(&self, py: Python) -> PyObject {
        self.socket.as_raw_fd().into_py(py).to_object(py)
    }

    #[cfg(windows)]
    pub fn get_fd(&self, py: Python) -> PyObject {
        self.socket.as_raw_socket().into_py(py).to_object(py)
    }
}

impl ListenerHolder {
    pub fn get_clone(&self) -> TcpListener {
        self.socket.try_clone().unwrap()
    }
}


pub(crate) fn init_pymodule(module: &PyModule) -> PyResult<()> {
    module.add_class::<ListenerHolder>()?;
    module.add_class::<SocketHolder>()?;

    Ok(())
}
