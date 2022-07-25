use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::{error, fmt};


#[derive(Debug)]
pub(crate) struct RSGIProtocolError;

#[derive(Debug)]
pub(crate) struct ApplicationError;

impl error::Error for RSGIProtocolError {}
impl error::Error for ApplicationError {}

impl fmt::Display for RSGIProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RSGI protocol error")
    }
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RSGI application error")
    }
}

impl From<std::convert::Infallible> for RSGIProtocolError {
    fn from(err: std::convert::Infallible) -> RSGIProtocolError {
        match err {}
    }
}

impl From<std::convert::Infallible> for ApplicationError {
    fn from(err: std::convert::Infallible) -> ApplicationError {
        match err {}
    }
}

impl std::convert::From<PyErr> for RSGIProtocolError {
    fn from(_pyerr: PyErr) -> RSGIProtocolError {
        RSGIProtocolError
    }
}

impl std::convert::From<PyErr> for ApplicationError {
    fn from(_pyerr: PyErr) -> ApplicationError {
        ApplicationError
    }
}

impl std::convert::From<RSGIProtocolError> for PyErr {
    fn from(err: RSGIProtocolError) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}

impl std::convert::From<ApplicationError> for PyErr {
    fn from(err: ApplicationError) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}

macro_rules! error_proto {
    () => {
        Err(RSGIProtocolError.into())
    };
}

pub(crate) use error_proto;
