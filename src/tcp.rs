use anyhow::Result;
use pyo3::{prelude::*, IntoPyObjectExt};

use std::net::{IpAddr, SocketAddr, TcpListener};
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawSocket, FromRawSocket};

use socket2::{Domain, Protocol, Socket, Type};

#[pyclass(frozen, module = "granian._granian")]
pub struct ListenerSpec {
    inp: (String, u16, i32),
    address: SocketAddr,
    domain: Domain,
    backlog: i32,
}

impl ListenerSpec {
    pub(crate) fn as_listener(&self) -> Result<TcpListener> {
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
        socket.listen(self.backlog)?;

        Ok(socket.into())
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

    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    fn share(&self, py: Python) -> PyObject {
        py.None()
    }

    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    fn share(&self) -> Result<ListenerHolder> {
        ListenerHolder::from_spec(self)
    }

    pub fn __getstate__(&self, py: Python) -> PyObject {
        self.inp.clone().into_py_any(py).unwrap()
    }
}

#[pyclass(frozen, module = "granian._granian")]
pub struct ListenerHolder {
    socket: TcpListener,
}

impl ListenerHolder {
    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    fn from_spec(spec: &ListenerSpec) -> Result<Self> {
        let socket = spec.as_listener()?;
        Ok(Self { socket })
    }

    pub fn get_clone(&self) -> TcpListener {
        self.socket.try_clone().unwrap()
    }
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

    #[cfg(unix)]
    pub fn __getstate__(&self, py: Python) -> PyObject {
        let fd = self.socket.as_raw_fd();
        (fd,).into_py_any(py).unwrap()
    }

    #[cfg(windows)]
    pub fn __getstate__(&self, py: Python) -> PyObject {
        let fd = self.socket.as_raw_socket();
        (fd,).into_py_any(py).unwrap()
    }

    #[cfg(unix)]
    pub fn get_fd(&self, py: Python) -> PyObject {
        self.socket.as_raw_fd().into_py_any(py).unwrap()
    }

    #[cfg(windows)]
    pub fn get_fd(&self, py: Python) -> PyObject {
        self.socket.as_raw_socket().into_py_any(py).unwrap()
    }
}

pub(crate) fn init_pymodule(module: &Bound<PyModule>) -> PyResult<()> {
    module.add_class::<ListenerSpec>()?;
    module.add_class::<ListenerHolder>()?;

    Ok(())
}
