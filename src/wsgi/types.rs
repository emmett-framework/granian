use hyper::{Body, Uri, Request, body::Bytes};
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::{collections::HashMap, net::SocketAddr};


#[pyclass(module="granian._granian")]
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
        request: Request<Body>
    ) -> Self {
        let headers = request.headers();
        let mut pyheaders = HashMap::with_capacity(headers.keys_len());
        for (key, val) in headers.iter() {
            pyheaders.insert(format!("HTTP_{}", key.as_str().replace("-", "_").to_uppercase()), val.to_str().unwrap().into());
        };
        Self {
            scheme: scheme.to_string(),
            method: request.method().to_string(),
            uri: request.uri().clone(),
            server: server.to_string(),
            client: client.to_string(),
            headers: pyheaders,
            body: hyper::body::to_bytes(request).await.unwrap_or(bytes::Bytes::new())
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

    #[getter(body)]
    fn get_body<'p>(&self, py: Python<'p>) -> &'p PyBytes {
        PyBytes::new(py, &self.body.to_vec()[..])
    }
}
