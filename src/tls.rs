use std::{
    fs, io,
    iter::Iterator,
    net::{SocketAddr, TcpListener},
    sync::Arc,
};
use tls_listener::{
    rustls::{
        rustls::{
            pki_types::{CertificateDer as Certificate, PrivateKeyDer as PrivateKey},
            server::ServerConfig,
        },
        TlsAcceptor,
    },
    TlsListener,
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

pub(crate) fn load_certs(filename: String) -> io::Result<Vec<Certificate<'static>>> {
    rustls_pemfile::certs(&mut io::BufReader::new(fs::File::open(filename)?)).collect()
}

pub(crate) fn load_private_key(filename: String) -> io::Result<PrivateKey<'static>> {
    rustls_pemfile::private_key(&mut io::BufReader::new(fs::File::open(filename)?)).map(std::option::Option::unwrap)
}
