use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use std::{error, fmt};

#[derive(Debug)]
pub(crate) struct UnsupportedASGIMessage;

#[derive(Debug)]
pub(crate) struct ASGIFlowError;

#[derive(Debug)]
pub(crate) struct ASGITransportError;

impl error::Error for UnsupportedASGIMessage {}
impl error::Error for ASGIFlowError {}
impl error::Error for ASGITransportError {}

impl fmt::Display for UnsupportedASGIMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unsupported ASGI message")
    }
}

impl fmt::Display for ASGIFlowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ASGI flow error")
    }
}

impl fmt::Display for ASGITransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ASGI transport closed")
    }
}

impl From<std::convert::Infallible> for ASGIFlowError {
    fn from(err: std::convert::Infallible) -> ASGIFlowError {
        match err {}
    }
}

impl std::convert::From<PyErr> for UnsupportedASGIMessage {
    fn from(_pyerr: PyErr) -> UnsupportedASGIMessage {
        UnsupportedASGIMessage
    }
}

impl std::convert::From<PyErr> for ASGIFlowError {
    fn from(_pyerr: PyErr) -> ASGIFlowError {
        ASGIFlowError
    }
}

impl std::convert::From<UnsupportedASGIMessage> for PyErr {
    fn from(err: UnsupportedASGIMessage) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}

impl std::convert::From<ASGIFlowError> for PyErr {
    fn from(err: ASGIFlowError) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}

impl std::convert::From<ASGITransportError> for PyErr {
    fn from(err: ASGITransportError) -> PyErr {
        PyRuntimeError::new_err(err.to_string())
    }
}

macro_rules! error_flow {
    () => {
        Err(super::errors::ASGIFlowError.into())
    };
}

macro_rules! error_message {
    () => {
        Err(super::errors::UnsupportedASGIMessage.into())
    };
}

pub(crate) use error_flow;
pub(crate) use error_message;
