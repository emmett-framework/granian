use anyhow::Result;
use pyo3::{IntoPyObjectExt, prelude::*};

use std::net::{IpAddr, SocketAddr, TcpListener};
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawSocket, FromRawSocket};

use socket2::{Domain, Protocol, Socket, Type};

#[pyclass(frozen, module = "granian._granian")]
#[derive(Clone)]
pub struct ListenerSpec {
    inp: (String, u16, i32),
    address: SocketAddr,
    domain: Domain,
    backlog: i32,
}

impl ListenerSpec {
    pub(crate) fn as_socket(&self) -> Result<Socket> {
        let socket = Socket::new(self.domain, Type::STREAM, Some(Protocol::TCP))?;

        #[cfg(not(windows))]
        {
            socket.set_reuse_port(true)?;
        }
        #[cfg(target_os = "freebsd")]
        {
            socket.set_reuse_port_lb(true)?;
        }
        socket.set_reuse_address(true)?;
        socket.set_nodelay(true)?;
        socket.bind(&self.address.into())?;

        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        socket.listen(self.backlog)?;

        Ok(socket)
    }
}

#[pymethods]
impl ListenerSpec {
    #[new]
    fn new(host: String, port: u16, backlog: i32) -> PyResult<Self> {
        let address: SocketAddr = (host.parse::<IpAddr>()?, port).into();
        let domain = match address {
            SocketAddr::V4(_) => Domain::IPV4,
            SocketAddr::V6(_) => Domain::IPV6,
        };
        Ok(Self {
            inp: (host, port, backlog),
            address,
            domain,
            backlog,
        })
    }

    fn build(&self) -> Result<SocketHolder> {
        SocketHolder::from_spec(self)
    }

    pub fn __getstate__(&self, py: Python) -> PyObject {
        self.inp.clone().into_py_any(py).unwrap()
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "freebsd")))]
#[pyclass(frozen, module = "granian._granian")]
pub struct SocketHolder {
    socket: Socket,
}

#[cfg(not(any(windows, target_os = "linux", target_os = "freebsd")))]
impl SocketHolder {
    fn from_spec(spec: &ListenerSpec) -> Result<Self> {
        let socket = spec.as_socket()?;
        Ok(Self { socket })
    }

    #[allow(clippy::unnecessary_wraps)]
    pub fn as_listener(&self) -> Result<TcpListener> {
        let listener = unsafe { TcpListener::from_raw_fd(self.socket.as_raw_fd()) };
        Ok(listener)
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "freebsd")))]
#[pymethods]
impl SocketHolder {
    #[new]
    pub fn new(fd: i32) -> Self {
        let socket = unsafe { Socket::from_raw_fd(fd) };
        Self { socket }
    }

    pub fn __getstate__(&self, py: Python) -> PyObject {
        let fd = self.socket.as_raw_fd();
        (fd,).into_py_any(py).unwrap()
    }

    pub fn get_fd(&self, py: Python) -> PyObject {
        self.socket.as_raw_fd().into_py_any(py).unwrap()
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
#[pyclass(frozen, module = "granian._granian")]
pub struct SocketHolder {
    socket: Socket,
    backlog: i32,
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
impl SocketHolder {
    fn from_spec(spec: &ListenerSpec) -> Result<Self> {
        let socket = spec.as_socket()?;
        Ok(Self {
            socket,
            backlog: spec.backlog,
        })
    }

    pub fn as_listener(&self) -> Result<TcpListener> {
        self.socket.listen(self.backlog)?;
        let listener = unsafe { TcpListener::from_raw_fd(self.socket.as_raw_fd()) };
        Ok(listener)
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
#[pymethods]
impl SocketHolder {
    #[new]
    pub fn new(fd: i32, backlog: i32) -> Self {
        let socket = unsafe { Socket::from_raw_fd(fd) };
        Self { socket, backlog }
    }

    pub fn __getstate__(&self, py: Python) -> PyObject {
        let fd = self.socket.as_raw_fd();
        (fd, self.backlog).into_py_any(py).unwrap()
    }

    pub fn get_fd(&self, py: Python) -> PyObject {
        self.socket.as_raw_fd().into_py_any(py).unwrap()
    }
}

#[cfg(windows)]
#[pyclass(frozen, module = "granian._granian")]
pub struct SocketHolder {
    socket: TcpListener,
}

#[cfg(windows)]
impl SocketHolder {
    fn from_spec(spec: &ListenerSpec) -> Result<Self> {
        let socket = spec.as_socket()?;
        Ok(Self { socket: socket.into() })
    }

    pub fn as_listener(&self) -> Result<TcpListener> {
        Ok(self.socket.try_clone()?)
    }
}

#[cfg(windows)]
#[pymethods]
impl SocketHolder {
    #[new]
    pub fn new(fd: u64) -> Self {
        let socket = unsafe { TcpListener::from_raw_socket(fd) };
        Self { socket }
    }

    pub fn __getstate__(&self, py: Python) -> PyObject {
        let fd = self.socket.as_raw_socket();
        (fd,).into_py_any(py).unwrap()
    }

    pub fn get_fd(&self, py: Python) -> PyObject {
        self.socket.as_raw_socket().into_py_any(py).unwrap()
    }
}

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<ListenerSpec>()?;
    module.add_class::<SocketHolder>()?;

    Ok(())
}
