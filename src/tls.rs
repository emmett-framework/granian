use anyhow::{Result, anyhow};
use std::{fs, io, iter::Iterator, sync::Arc};
use tls_listener::{
    TlsListener,
    rustls::{
        TlsAcceptor,
        rustls::{
            SupportedProtocolVersion,
            pki_types::{
                CertificateDer as Certificate, CertificateRevocationListDer as CRL, PrivateKeyDer as PrivateKey,
                pem::PemObject,
            },
            server::ServerConfig,
            version as tls_version,
        },
    },
};

use crate::net::SockAddr;

pub(crate) fn resolve_protocol_versions(min_version: &str) -> Vec<&'static SupportedProtocolVersion> {
    match min_version {
        "tls1.2" => vec![&tls_version::TLS12, &tls_version::TLS13],
        "tls1.3" => vec![&tls_version::TLS13],
        _ => unreachable!(),
    }
}

pub(crate) fn tls_tcp_listener(
    config: Arc<ServerConfig>,
    tcp: std::net::TcpListener,
) -> Result<(TlsListener<tokio::net::TcpListener, TlsAcceptor>, SockAddr)> {
    let tcp_listener = tokio::net::TcpListener::from_std(tcp).unwrap();
    let local_addr = tcp_listener.local_addr()?;
    let listener = TlsListener::new(TlsAcceptor::from(config), tcp_listener);
    Ok((listener, SockAddr::TCP(local_addr)))
}

#[cfg(unix)]
pub(crate) fn tls_uds_listener(
    config: Arc<ServerConfig>,
    uds: std::os::unix::net::UnixListener,
) -> Result<(TlsListener<tokio::net::UnixListener, TlsAcceptor>, SockAddr)> {
    let uds_listener = tokio::net::UnixListener::from_std(uds).unwrap();
    let local_addr = uds_listener.local_addr()?;
    let listener = TlsListener::new(TlsAcceptor::from(config), uds_listener);
    Ok((listener, SockAddr::UDS(local_addr)))
}

pub(crate) fn load_certs(filename: String) -> Vec<Certificate<'static>> {
    Certificate::pem_file_iter(filename)
        .expect("cannot open certificate file")
        .map(|result| result.unwrap())
        .collect()
}

pub(crate) fn load_crls(filenames: impl Iterator<Item = impl AsRef<std::path::Path>>) -> Vec<CRL<'static>> {
    filenames
        .map(|filename| CRL::from_pem_file(filename).expect("cannot read CRL file"))
        .collect()
}

pub(crate) fn load_private_key(filename: String, password: Option<String>) -> PrivateKey<'static> {
    match &password {
        Some(pwd) => {
            let expected_tag = "ENCRYPTED PRIVATE KEY";
            let content = fs::read(filename).expect("cannot load key");
            let sections = pem::parse_many(content).expect("invalid key");
            let mut iter = sections
                .into_iter()
                .filter(|v| v.tag() == expected_tag)
                .map(|v| v.contents().to_vec());
            let key = pkcs8::EncryptedPrivateKeyInfo::try_from(
                iter.next()
                    .map_or_else(|| Err(anyhow!("Invalid key")), Ok)
                    .expect("invalid key")
                    .as_slice(),
            )
            .expect("Invalid key")
            .decrypt(pwd)
            .expect("Cannot decrypt key");
            PrivateKey::Pkcs8(key.to_bytes().to_vec().into())
        }
        None => rustls_pemfile::private_key(&mut io::BufReader::new(
            fs::File::open(filename).expect("cannot load key"),
        ))
        .expect("invalid key")
        .unwrap(),
    }
}
