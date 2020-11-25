use bytes::Bytes;
use reqwest::{
    blocking::{Client, RequestBuilder},
    Url,
};
use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

#[derive(StructOpt)]
pub struct RestArgs {
    /// node API address. Must always have `http://` or `https://` prefix.
    /// E.g. `-h http://127.0.0.1`, `--host https://node.com:8443/cardano/api`
    #[structopt(short, long, env = "JORMUNGANDR_RESTAPI_URL")]
    host: Url,
    /// print additional debug information to stderr.
    /// The output format is intentionally undocumented and unstable
    #[structopt(long)]
    debug: bool,
    /// An optional TLS root certificate to be used in a case when the
    /// certificate CA is not present within the webpki certificate bundle.
    #[structopt(long, name = "PATH", env = "JORMUNGANDR_TLS_CERT_PATH")]
    tls_cert_path: Option<PathBuf>,
}

pub struct RestResponse(reqwest::blocking::Response);

#[derive(Debug, Error)]
pub enum Error {
    #[error("Host address '{addr}' isn't valid address base")]
    HostAddrNotBase { addr: Url },
    #[error("could not read the provided certificate")]
    CertIo(#[source] std::io::Error),
    #[error("expected a valid PEM-encoded certificate")]
    Pem(#[source] reqwest::Error),
    #[error("failed to build an HTTP client")]
    Client(#[source] reqwest::Error),
    #[error("invalid request")]
    Request(#[source] reqwest::Error),
    #[error("could not deserialize the response as JSON")]
    Json(#[source] reqwest::Error),
    #[error("could not get the response bytes")]
    Bytes(#[source] reqwest::Error),
    #[error("could not get the response text")]
    Text(#[source] reqwest::Error),
    #[error("connection with the node timed out")]
    Timeout,
    #[error("node rejected request because of invalid parameters")]
    InvalidParams(#[source] reqwest::Error),
    #[error("node internal error")]
    InternalError(#[source] reqwest::Error),
    #[error("redirecting error while connecting with node")]
    Redirecton(#[source] reqwest::Error),
    #[error("communication with node failed in unexpected way")]
    UnexpectedError(#[source] reqwest::Error),
}

impl RestArgs {
    pub fn request_with_args<F>(
        self,
        address_segments: &[&str],
        f: F,
    ) -> Result<RestResponse, Error>
    where
        F: FnOnce(&Client, Url) -> RequestBuilder,
    {
        use reqwest::{blocking::ClientBuilder, Certificate};
        use std::{fs::File, io::Read};

        let Self {
            tls_cert_path,
            mut host,
            debug,
        } = self;

        let client_builder = ClientBuilder::new();

        // load certificate
        let client_builder = if let Some(path) = tls_cert_path {
            let mut buf = Vec::new();
            File::open(path)
                .map_err(Error::CertIo)?
                .read_to_end(&mut buf)
                .map_err(Error::CertIo)?;
            let cert = Certificate::from_pem(&buf).map_err(Error::Pem)?;
            client_builder.use_rustls_tls().add_root_certificate(cert)
        } else {
            client_builder
        };

        let client = client_builder.build().map_err(Error::Client)?;

        // we need a base host address
        host.path_segments_mut()
            .map(|mut host_segments| {
                host_segments.extend(address_segments);
            })
            .map_err(|_| Error::HostAddrNotBase { addr: host.clone() })?;

        let request = f(&client, host).build().map_err(Error::Request)?;

        if debug {
            eprintln!("Request: {:?}", request);
        }

        let response = client
            .execute(request)
            .and_then(|response| response.error_for_status())
            .map_err(|e| {
                if e.is_timeout() {
                    Error::Timeout
                } else if let Some(status) = e.status() {
                    if status.is_client_error() {
                        Error::InvalidParams(e)
                    } else if status.is_server_error() {
                        Error::InternalError(e)
                    } else if status.is_redirection() {
                        Error::Redirecton(e)
                    } else {
                        Error::UnexpectedError(e)
                    }
                } else {
                    Error::UnexpectedError(e)
                }
            })?;

        if debug {
            eprintln!("Response: {:?}", response);
        }

        Ok(RestResponse(response))
    }
}

impl RestResponse {
    pub fn json<T>(self) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        self.0.json().map_err(Error::Json)
    }

    pub fn bytes(self) -> Result<Bytes, Error> {
        self.0.bytes().map_err(Error::Bytes)
    }

    pub fn text(self) -> Result<String, Error> {
        self.0.text().map_err(Error::Text)
    }
}
