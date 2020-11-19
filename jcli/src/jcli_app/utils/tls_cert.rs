use reqwest::{
    self,
    blocking::{Client, ClientBuilder},
    Certificate,
};
use std::{
    fs::File,
    io::{self, Read},
    path::PathBuf,
};
use structopt::StructOpt;
use thiserror::Error;

#[derive(Debug, StructOpt)]
pub struct TlsCert {
    /// An optional TLS root certificate to be used in a case when the
    /// certificate CA is not present within the webpki certificate bundle.
    #[structopt(long, name = "PATH")]
    tls_cert_path: Option<PathBuf>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not read the provided certificate")]
    Io(#[from] io::Error),
    #[error("expected a valid PEM-encoded certificate")]
    Pem(#[from] reqwest::Error),
}

impl TlsCert {
    pub fn client_builder_with_cert(&self, cb: ClientBuilder) -> Result<ClientBuilder, Error> {
        let path = if let Some(path) = &self.tls_cert_path {
            path
        } else {
            return Ok(cb);
        };
        let mut buf = Vec::new();
        File::open(path)?.read_to_end(&mut buf)?;
        let cert = Certificate::from_pem(&buf)?;
        Ok(cb.use_rustls_tls().add_root_certificate(cert))
    }

    pub fn client_with_cert(&self) -> Result<Client, Error> {
        Ok(self
            .client_builder_with_cert(ClientBuilder::new())?
            .build()
            .expect("Could not build a RequestBuilder"))
    }
}
