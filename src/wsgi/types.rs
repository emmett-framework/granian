use hyper::{body::Bytes, Body, Method, Request, Uri};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};
use std::{collections::HashMap, net::SocketAddr};

const LINE_SPLIT: u8 = u8::from_be_bytes(*b"\n");


#[pyclass(module = "granian._granian")]
pub(crate) struct WSGIBody {
    inner: Bytes
}

impl WSGIBody {
    pub fn new(body: Bytes) -> Self {
        Self { inner: body }
    }
}

#[pymethods]
impl WSGIBody {
    fn __iter__(pyself: PyRef<'_, Self>) -> PyRef<'_, Self> {
        pyself
    }

    fn __next__<'p>(&mut self, py: Python<'p>) -> Option<&'p PyBytes> {
        match self.inner.iter().position(|&c| c == LINE_SPLIT) {
            Some(next_split) => {
                let bytes = self.inner.split_to(next_split);
                Some(PyBytes::new(py, &bytes[..]))
            },
            _ => None
        }
    }

    #[args(size="None")]
    fn read<'p>(&mut self, py: Python<'p>, size: Option<usize>) -> &'p PyBytes {
        match size {
            None => {
                let bytes = self.inner.split_to(self.inner.len());
                PyBytes::new(py, &bytes[..])
            },
            Some(size) => {
                match size {
                    0 => PyBytes::new(py, b""),
                    size => {
                        let limit = self.inner.len();
                        let rsize = if size > limit { limit } else { size };
                        let bytes = self.inner.split_to(rsize);
                        PyBytes::new(py, &bytes[..])
                    }
                }
            }
        }
    }

    fn readline<'p>(&mut self, py: Python<'p>) -> &'p PyBytes {
        match self.inner.iter().position(|&c| c == LINE_SPLIT) {
            Some(next_split) => {
                let bytes = self.inner.split_to(next_split);
                self.inner = self.inner.slice(1..);
                PyBytes::new(py, &bytes[..])
            },
            _ => PyBytes::new(py, b"")
        }
    }

    #[args(_hint="None")]
    fn readlines<'p>(&mut self, py: Python<'p>, _hint: Option<PyObject>) -> &'p PyList {
        let lines: Vec<&PyBytes> = self.inner
            .split(|&c| c == LINE_SPLIT)
            .map(|item| PyBytes::new(py, &item[..]))
            .collect();
        self.inner.clear();
        PyList::new(py, lines)
    }
}

#[pyclass(module = "granian._granian")]
pub(crate) struct WSGIScope {
    #[pyo3(get)]
    scheme: String,
    #[pyo3(get)]
    method: String,
    uri: Uri,
    #[pyo3(get)]
    server: String,
    #[pyo3(get)]
    client: String,
    #[pyo3(get)]
    headers: HashMap<String, String>,
    body: Bytes
}

impl WSGIScope {
    pub async fn new(
        scheme: &str,
        server: SocketAddr,
        client: SocketAddr,
        request: Request<Body>,
    ) -> Self {
        let headers = request.headers();
        let mut pyheaders = HashMap::with_capacity(headers.keys_len());
        for (key, val) in headers.iter() {
            pyheaders.insert(
                format!("HTTP_{}", key.as_str().replace("-", "_").to_uppercase()),
                val.to_str().unwrap().into(),
            );
        }

        let method = request.method().clone();
        let uri = request.uri().clone();

        let body = match &method {
            &Method::HEAD | &Method::GET | &Method::OPTIONS => { Bytes::new() },
            _ => {
                hyper::body::to_bytes(request)
                    .await
                    .unwrap_or(Bytes::new())
            }
        };

        Self {
            scheme: scheme.to_string(),
            method: method.to_string(),
            uri,
            server: server.to_string(),
            client: client.to_string(),
            headers: pyheaders,
            body
        }
    }
}

#[pymethods]
impl WSGIScope {
    #[getter(path)]
    fn get_path(&self) -> &str {
        self.uri.path()
    }

    #[getter(query_string)]
    fn get_query_string(&self) -> &str {
        self.uri.query().unwrap_or("")
    }

    fn input(pyself: PyRef<'_, Self>) -> PyResult<Py<WSGIBody>> {
        Py::new(pyself.py(), WSGIBody::new(pyself.body.to_owned()))
    }
}
