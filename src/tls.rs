use anyhow::{Result, anyhow};
use std::{
    fs, io,
    iter::Iterator,
    net::{SocketAddr, TcpListener},
    sync::Arc,
};
use tls_listener::{
    TlsListener,
    rustls::{
        TlsAcceptor,
        rustls::{
            pki_types::{
                CertificateDer as Certificate, CertificateRevocationListDer as CRL, PrivateKeyDer as PrivateKey,
                pem::PemObject,
            },
            server::ServerConfig,
        },
    },
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
