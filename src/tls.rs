use anyhow::{anyhow, Result};
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
) -> Result<(TlsListener<tokio::net::TcpListener, TlsAcceptor>, SocketAddr)> {
    let tcp_listener = tokio::net::TcpListener::from_std(tcp).unwrap();
    let local_addr = tcp_listener.local_addr()?;
    let listener = TlsListener::new(TlsAcceptor::from(config), tcp_listener);
    Ok((listener, local_addr))
}

pub(crate) fn load_certs(filename: String) -> io::Result<Vec<Certificate<'static>>> {
    rustls_pemfile::certs(&mut io::BufReader::new(fs::File::open(filename)?)).collect()
}

pub(crate) fn load_private_key(filename: String, password: Option<String>) -> Result<PrivateKey<'static>> {
    match &password {
        Some(pwd) => {
            let expected_tag = "ENCRYPTED PRIVATE KEY";
            let content = fs::read(filename)?;
            let sections = pem::parse_many(content)?;
            let mut iter = sections
                .into_iter()
                .filter(|v| v.tag() == expected_tag)
                .map(|v| v.contents().to_vec());
            let key = pkcs8::EncryptedPrivateKeyInfo::try_from(
                iter.next().map_or_else(|| Err(anyhow!("Invalid key")), Ok)?.as_slice(),
            )
            .map_err(|_| anyhow!("Invalid key"))?
            .decrypt(pwd)
            .map_err(|_| anyhow!("Cannot decrypt key"))?;
            Ok(PrivateKey::Pkcs8(key.to_bytes().to_vec().into()))
        }
        None => rustls_pemfile::private_key(&mut io::BufReader::new(fs::File::open(filename)?))
            .map_err(std::convert::Into::into)
            .map(std::option::Option::unwrap),
    }
}
