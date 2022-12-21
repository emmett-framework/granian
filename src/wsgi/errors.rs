use pyo3::{create_exception, exceptions::PyRuntimeError};


create_exception!(_granian, WSGIProtocolError, PyRuntimeError, "WSGIProtocolError");

macro_rules! error_proto {
    () => {
        Err(super::errors::WSGIProtocolError::new_err("WSGI protocol error").into())
    };
}

pub(crate) use error_proto;
