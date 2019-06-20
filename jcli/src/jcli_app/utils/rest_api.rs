use hex;
use jcli_app::utils::DebugFlag;
use reqwest::{Error, RequestBuilder, Response};
use std::{fmt, io::Write};

pub struct RestApiSender<'a> {
    builder: RequestBuilder,
    request_body_debug: Option<String>,
    debug_flag: &'a DebugFlag,
}

pub struct RestApiResponse {
    body: RestApiResponseBody,
    response: Response,
}

pub enum RestApiResponseBody {
    Text(String),
    Binary(Vec<u8>),
}

impl<'a> RestApiSender<'a> {
    pub fn new(builder: RequestBuilder, debug_flag: &'a DebugFlag) -> Self {
        Self {
            builder,
            request_body_debug: None,
            debug_flag,
        }
    }

    pub fn with_binary_body(mut self, body: Vec<u8>) -> Self {
        if self.debug_flag.debug_writer().is_some() {
            self.request_body_debug = Some(hex::encode(&body));
        }
        self.builder = self
            .builder
            .header(
                reqwest::header::CONTENT_TYPE,
                mime::APPLICATION_OCTET_STREAM.as_ref(),
            )
            .body(body);
        self
    }

    pub fn send(self) -> Result<RestApiResponse, Error> {
        let request = self.builder.build()?;
        if let Some(mut writer) = self.debug_flag.debug_writer() {
            writeln!(writer, "{:#?}", request).unwrap();
            if let Some(body) = self.request_body_debug {
                writeln!(writer, "Request body:\n{}", body).unwrap();
            }
        }
        let response = reqwest::Client::new()
            .execute(request)
            .and_then(RestApiResponse::new)?;
        if let Some(mut writer) = self.debug_flag.debug_writer() {
            writeln!(writer, "{:#?}", response.response()).unwrap();
            if !response.body().is_empty() {
                writeln!(writer, "Response body:\n{}", response.body()).unwrap();
            }
        }
        Ok(response)
    }
}

impl RestApiResponse {
    pub fn new(mut response: Response) -> Result<Self, Error> {
        Ok(RestApiResponse {
            body: RestApiResponseBody::new(&mut response)?,
            response,
        })
    }

    pub fn response(&self) -> &Response {
        &self.response
    }

    pub fn body(&self) -> &RestApiResponseBody {
        &self.body
    }
}

impl RestApiResponseBody {
    fn new(response: &mut Response) -> Result<Self, Error> {
        match is_body_binary(response) {
            true => {
                let mut data = Vec::with_capacity(response.content_length().unwrap_or(0) as usize);
                response.copy_to(&mut data)?;
                Ok(RestApiResponseBody::Binary(data))
            }
            false => response.text().map(RestApiResponseBody::Text),
        }
    }

    pub fn text<'a>(&'a self) -> impl AsRef<str> + 'a {
        match self {
            RestApiResponseBody::Text(text) => text.into(),
            RestApiResponseBody::Binary(binary) => String::from_utf8_lossy(binary),
        }
    }

    pub fn binary(&self) -> &[u8] {
        match self {
            RestApiResponseBody::Text(text) => text.as_bytes(),
            RestApiResponseBody::Binary(binary) => binary,
        }
    }

    pub fn json<'a, T: serde::Deserialize<'a>>(&'a self) -> Result<T, serde_json::Error> {
        match self {
            RestApiResponseBody::Text(text) => serde_json::from_str(text),
            RestApiResponseBody::Binary(binary) => serde_json::from_slice(binary),
        }
    }

    pub fn json_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        self.json()
    }

    pub fn is_empty(&self) -> bool {
        match self {
            RestApiResponseBody::Text(text) => text.is_empty(),
            RestApiResponseBody::Binary(binary) => binary.is_empty(),
        }
    }
}

impl fmt::Display for RestApiResponseBody {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            RestApiResponseBody::Text(text) => text.fmt(f),
            RestApiResponseBody::Binary(binary) => hex::encode(binary).fmt(f),
        }
    }
}

fn is_body_binary(response: &Response) -> bool {
    response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|header| header.to_str().ok())
        .and_then(|header_str| header_str.parse::<mime::Mime>().ok())
        == Some(mime::APPLICATION_OCTET_STREAM)
}
