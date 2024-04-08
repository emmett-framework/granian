use pyo3::prelude::*;
use pyo3::types::PyType;

use std::net::{IpAddr, SocketAddr, TcpListener};
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawSocket, FromRawSocket};

use socket2::{Domain, Protocol, Socket, Type};

#[pyclass(frozen, module = "granian._granian")]
pub struct ListenerHolder {
    socket: TcpListener,
}

#[pymethods]
impl ListenerHolder {
    #[cfg(unix)]
    #[new]
    pub fn new(fd: i32) -> Self {
        let socket = unsafe { TcpListener::from_raw_fd(fd) };
        Self { socket }
    }

    #[cfg(windows)]
    #[new]
    pub fn new(fd: u64) -> Self {
        let socket = unsafe { TcpListener::from_raw_socket(fd) };
        Self { socket }
    }

    #[classmethod]
    pub fn from_address(_cls: &Bound<PyType>, address: &str, port: u16, backlog: i32) -> PyResult<Self> {
        let address: SocketAddr = (address.parse::<IpAddr>()?, port).into();
        let domain = match address {
            SocketAddr::V4(_) => Domain::IPV4,
            SocketAddr::V6(_) => Domain::IPV6,
        };
        let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;

        #[cfg(not(windows))]
        {
            socket.set_reuse_port(true)?;
        }
        socket.set_reuse_address(true)?;
        socket.set_nodelay(true)?;
        socket.bind(&address.into())?;
        socket.listen(backlog)?;
        Ok(Self { socket: socket.into() })
    }

    #[cfg(unix)]
    pub fn __getstate__(&self, py: Python) -> PyObject {
        let fd = self.socket.as_raw_fd();
        (fd.into_py(py),).to_object(py)
    }

    #[cfg(windows)]
    pub fn __getstate__(&self, py: Python) -> PyObject {
        let fd = self.socket.as_raw_socket();
        (fd.into_py(py),).to_object(py)
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

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<ListenerHolder>()?;

    Ok(())
}
