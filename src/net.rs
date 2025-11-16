use anyhow::Result;
use pyo3::{IntoPyObjectExt, prelude::*};

use std::net::{IpAddr, TcpListener};
#[cfg(unix)]
use std::os::unix::{
    io::{AsRawFd, FromRawFd},
    net::UnixListener,
};
#[cfg(windows)]
use std::os::windows::io::{AsRawSocket, FromRawSocket};

use socket2::{Domain, Protocol, Socket, Type};

#[derive(Clone)]
pub(crate) enum SockAddr {
    #[allow(clippy::upper_case_acronyms)]
    TCP(std::net::SocketAddr),
    #[cfg(unix)]
    #[allow(clippy::upper_case_acronyms)]
    UDS(tokio::net::unix::SocketAddr),
}

impl SockAddr {
    pub fn ip(&self) -> String {
        match self {
            Self::TCP(addr) => addr.ip().to_string(),
            #[cfg(unix)]
            Self::UDS(addr) => addr.as_pathname().map_or("", |v| v.to_str().unwrap()).to_string(),
        }
    }

    pub fn port(&self) -> u16 {
        match self {
            Self::TCP(addr) => addr.port(),
            #[cfg(unix)]
            Self::UDS(_) => 0,
        }
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        match self {
            Self::TCP(addr) => addr.to_string(),
            #[cfg(unix)]
            Self::UDS(addr) => addr.as_pathname().map_or("", |v| v.to_str().unwrap()).to_string(),
        }
    }
}

#[pyclass(frozen, module = "granian._granian")]
#[derive(Clone)]
pub struct ListenerSpec {
    inp: (String, u16, i32),
    address: std::net::SocketAddr,
    domain: Domain,
    backlog: i32,
}

#[cfg(unix)]
#[pyclass(frozen, module = "granian._granian")]
#[derive(Clone)]
pub struct UnixListenerSpec {
    inp: (String, i32),
    address: socket2::SockAddr,
    backlog: i32,
    permissions: Option<u32>,
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
        socket.set_tcp_nodelay(true)?;
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
        let address: std::net::SocketAddr = (host.parse::<IpAddr>()?, port).into();
        let domain = match address {
            std::net::SocketAddr::V4(_) => Domain::IPV4,
            std::net::SocketAddr::V6(_) => Domain::IPV6,
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

    pub fn __getstate__(&self, py: Python) -> Py<PyAny> {
        self.inp.clone().into_py_any(py).unwrap()
    }
}

#[cfg(unix)]
impl UnixListenerSpec {
    pub(crate) fn as_socket(&self) -> Result<Socket> {
        use std::{fs, os::unix::fs::PermissionsExt};

        let socket = Socket::new(Domain::UNIX, Type::STREAM, None)?;

        socket.bind(&self.address)?;
        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        socket.listen(self.backlog)?;

        if let Some(mode) = self.permissions {
            let socket_path = self.address.as_pathname().unwrap();
            let mut permissions = fs::metadata(socket_path)?.permissions();
            permissions.set_mode(mode);
            fs::set_permissions(socket_path, permissions)?;
        }

        Ok(socket)
    }
}

#[cfg(unix)]
#[pymethods]
impl UnixListenerSpec {
    #[new]
    fn new(bind: String, backlog: i32, permissions: Option<u32>) -> PyResult<Self> {
        let address = socket2::SockAddr::unix(&bind)?;
        Ok(Self {
            inp: (bind, backlog),
            address,
            backlog,
            permissions,
        })
    }

    fn build(&self) -> Result<SocketHolder> {
        SocketHolder::from_unix_spec(self)
    }

    pub fn __getstate__(&self, py: Python) -> Py<PyAny> {
        self.inp.clone().into_py_any(py).unwrap()
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "freebsd")))]
#[pyclass(frozen, module = "granian._granian")]
pub struct SocketHolder {
    socket: Option<Socket>,
    uds: bool,
}

#[cfg(not(any(windows, target_os = "linux", target_os = "freebsd")))]
impl SocketHolder {
    fn from_spec(spec: &ListenerSpec) -> Result<Self> {
        let socket = spec.as_socket()?;
        Ok(Self {
            socket: Some(socket),
            uds: false,
        })
    }

    fn from_unix_spec(spec: &UnixListenerSpec) -> Result<Self> {
        let socket = spec.as_socket()?;
        Ok(Self {
            socket: Some(socket),
            uds: true,
        })
    }

    #[allow(clippy::unnecessary_wraps)]
    pub fn as_tcp_listener(&self) -> Result<TcpListener> {
        let listener = unsafe { TcpListener::from_raw_fd(self.socket.as_ref().unwrap().as_raw_fd()) };
        Ok(listener)
    }

    #[allow(clippy::unnecessary_wraps)]
    pub fn as_unix_listener(&self) -> Result<UnixListener> {
        let listener = unsafe { UnixListener::from_raw_fd(self.socket.as_ref().unwrap().as_raw_fd()) };
        Ok(listener)
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "freebsd")))]
impl Drop for SocketHolder {
    fn drop(&mut self) {
        std::mem::forget(self.socket.take());
    }
}

#[cfg(not(any(windows, target_os = "linux", target_os = "freebsd")))]
#[pymethods]
impl SocketHolder {
    #[new]
    pub fn new(fd: i32, uds: bool) -> Self {
        let socket = unsafe { Socket::from_raw_fd(fd) };
        Self {
            socket: Some(socket),
            uds,
        }
    }

    pub fn __getstate__(&self, py: Python) -> Py<PyAny> {
        let fd = self.socket.as_ref().unwrap().as_raw_fd();
        (fd, self.uds).into_py_any(py).unwrap()
    }

    pub fn get_fd(&self, py: Python) -> Py<PyAny> {
        self.socket.as_ref().unwrap().as_raw_fd().into_py_any(py).unwrap()
    }

    pub fn is_uds(&self) -> bool {
        self.uds
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
#[pyclass(frozen, module = "granian._granian")]
pub struct SocketHolder {
    socket: Option<Socket>,
    uds: bool,
    backlog: i32,
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
impl SocketHolder {
    fn from_spec(spec: &ListenerSpec) -> Result<Self> {
        let socket = spec.as_socket()?;
        Ok(Self {
            socket: Some(socket),
            uds: false,
            backlog: spec.backlog,
        })
    }

    fn from_unix_spec(spec: &UnixListenerSpec) -> Result<Self> {
        let socket = spec.as_socket()?;
        Ok(Self {
            socket: Some(socket),
            uds: true,
            backlog: spec.backlog,
        })
    }

    pub fn as_tcp_listener(&self) -> Result<TcpListener> {
        let socket = self.socket.as_ref().unwrap();
        socket.listen(self.backlog)?;
        let listener = unsafe { TcpListener::from_raw_fd(socket.as_raw_fd()) };
        Ok(listener)
    }

    pub fn as_unix_listener(&self) -> Result<UnixListener> {
        let socket = self.socket.as_ref().unwrap();
        socket.listen(self.backlog)?;
        let listener = unsafe { UnixListener::from_raw_fd(socket.as_raw_fd()) };
        Ok(listener)
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
impl Drop for SocketHolder {
    fn drop(&mut self) {
        std::mem::forget(self.socket.take());
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
#[pymethods]
impl SocketHolder {
    #[new]
    pub fn new(fd: i32, uds: bool, backlog: i32) -> Self {
        let socket = unsafe { Socket::from_raw_fd(fd) };
        Self {
            socket: Some(socket),
            uds,
            backlog,
        }
    }

    pub fn __getstate__(&self, py: Python) -> Py<PyAny> {
        let fd = self.socket.as_ref().unwrap().as_raw_fd();
        (fd, self.uds, self.backlog).into_py_any(py).unwrap()
    }

    pub fn get_fd(&self, py: Python) -> Py<PyAny> {
        self.socket.as_ref().unwrap().as_raw_fd().into_py_any(py).unwrap()
    }

    pub fn is_uds(&self) -> bool {
        self.uds
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

    pub fn as_tcp_listener(&self) -> Result<TcpListener> {
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

    pub fn __getstate__(&self, py: Python) -> Py<PyAny> {
        let fd = self.socket.as_raw_socket();
        (fd,).into_py_any(py).unwrap()
    }

    pub fn get_fd(&self, py: Python) -> Py<PyAny> {
        self.socket.as_raw_socket().into_py_any(py).unwrap()
    }

    pub fn is_uds(&self) -> bool {
        false
    }
}

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<ListenerSpec>()?;
    module.add_class::<SocketHolder>()?;
    #[cfg(unix)]
    module.add_class::<UnixListenerSpec>()?;

    Ok(())
}
