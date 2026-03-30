use pyo3::{create_exception, exceptions::PyRuntimeError};

create_exception!(_granian, RSGIProtocolError, PyRuntimeError, "RSGIProtocolError");
create_exception!(_granian, RSGIProtocolClosed, PyRuntimeError, "RSGIProtocolClosed");

macro_rules! pyerr_proto {
    () => {
        crate::rsgi::errors::RSGIProtocolError::new_err("RSGI protocol error")
    };
}

macro_rules! pyerr_stream {
    () => {
        crate::rsgi::errors::RSGIProtocolClosed::new_err("RSGI transport is closed")
    };
}

macro_rules! error_proto {
    () => {
        Err(crate::rsgi::errors::pyerr_proto!().into())
    };
}

macro_rules! error_stream {
    () => {
        Err(crate::rsgi::errors::pyerr_stream!().into())
    };
}

pub(crate) use error_proto;
pub(crate) use error_stream;
pub(crate) use pyerr_proto;
pub(crate) use pyerr_stream;
