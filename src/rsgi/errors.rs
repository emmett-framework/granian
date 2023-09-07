use pyo3::{create_exception, exceptions::PyRuntimeError};

create_exception!(_granian, RSGIProtocolError, PyRuntimeError, "RSGIProtocolError");
create_exception!(_granian, RSGIProtocolClosed, PyRuntimeError, "RSGIProtocolClosed");

macro_rules! error_proto {
    () => {
        Err(super::errors::RSGIProtocolError::new_err("RSGI protocol error").into())
    };
}

macro_rules! error_stream {
    () => {
        Err(super::errors::RSGIProtocolClosed::new_err("RSGI transport is closed").into())
    };
}

pub(crate) use error_proto;
pub(crate) use error_stream;
