use std::{
    fs, io,
    iter::Iterator,
    net::{SocketAddr, TcpListener},
    sync::Arc,
};
use tls_listener::TlsListener;
use tokio_rustls::{
    rustls::{Certificate, PrivateKey, ServerConfig},
    TlsAcceptor,
};

pub(crate) fn tls_listener(
    config: Arc<ServerConfig>,
    tcp: TcpListener,
) -> anyhow::Result<(TlsListener<tokio::net::TcpListener, TlsAcceptor>, SocketAddr)> {
    let tcp_listener = tokio::net::TcpListener::from_std(tcp).unwrap();
    let local_addr = tcp_listener.local_addr()?;
    let listener = TlsListener::new(TlsAcceptor::from(config), tcp_listener);
    Ok((listener, local_addr))
}

fn tls_error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

pub(crate) fn load_certs(filename: &str) -> io::Result<Vec<Certificate>> {
    let certfile = fs::File::open(filename).map_err(|e| tls_error(format!("failed to open {filename}: {e}")))?;
    let mut reader = io::BufReader::new(certfile);

    let certs = rustls_pemfile::certs(&mut reader).map_err(|_| tls_error("failed to load certificate".into()))?;
    Ok(certs.into_iter().map(Certificate).collect())
}

pub(crate) fn load_private_key(filename: &str) -> io::Result<PrivateKey> {
    let keyfile = fs::File::open(filename).map_err(|e| tls_error(format!("failed to open {filename}: {e}")))?;
    let mut reader = io::BufReader::new(keyfile);

    let keys = rustls_pemfile::read_all(&mut reader).map_err(|_| tls_error("failed to load private key".into()))?;
    if keys.len() != 1 {
        return Err(tls_error("expected a single private key".into()));
    }

    let key = match &keys[0] {
        rustls_pemfile::Item::RSAKey(key) => PrivateKey(key.clone()),
        rustls_pemfile::Item::PKCS8Key(key) => PrivateKey(key.clone()),
        rustls_pemfile::Item::ECKey(key) => PrivateKey(key.clone()),
        _ => {
            return Err(tls_error("failed to load private key".into()));
        }
    };

    Ok(key)
}
