use crate::jcli_app::utils::{open_api_verifier, CustomErrorFiller, DebugFlag, OpenApiVerifier};
use hex;
use reqwest::{self, header::HeaderValue, Client, Request, RequestBuilder, Response};
use serde::{self, Serialize};
use serde_json::error::Error as SerdeJsonError;
use std::fmt;

pub const DESERIALIZATION_ERROR_MSG: &'static str = "node returned malformed data";

pub struct RestApiSender<'a> {
    builder: RequestBuilder,
    body: RestApiRequestBody,
    debug_flag: &'a DebugFlag,
}

pub struct RestApiResponse {
    body: RestApiResponseBody,
    response: Response,
}

pub enum RestApiRequestBody {
    None,
    Binary(Vec<u8>),
    Json(String),
}

pub enum RestApiResponseBody {
    Text(String),
    Binary(Vec<u8>),
}

custom_error! { pub Error
    RequestFailed { source: reqwest::Error } = @{ reqwest_error_msg(source) },
    VerificationFailed { source: open_api_verifier::Error } = "request didn't pass verification",
    RequestJsonSerializationError { source: SerdeJsonError, filler: CustomErrorFiller }
        = "failed to serialize request JSON",
    ResponseJsonDeserializationError { source: SerdeJsonError, filler: CustomErrorFiller }
        = "response JSON malformed",
}

fn reqwest_error_msg(err: &reqwest::Error) -> &'static str {
    if err.is_timeout() {
        "connection with node timed out"
    } else if err.is_http() {
        "could not connect with node"
    } else if err.is_serialization() {
        DESERIALIZATION_ERROR_MSG
    } else if err.is_redirect() {
        "redirecting error while connecting with node"
    } else if err.is_client_error() {
        "node rejected request because of invalid parameters"
    } else if err.is_server_error() {
        "node internal error"
    } else {
        "communication with node failed in unexpected way"
    }
}

impl<'a> RestApiSender<'a> {
    pub fn new(builder: RequestBuilder, debug_flag: &'a DebugFlag) -> Self {
        Self {
            builder,
            body: RestApiRequestBody::none(),
            debug_flag,
        }
    }

    pub fn with_binary_body(mut self, body: Vec<u8>) -> Self {
        self.body = RestApiRequestBody::from_binary(body);
        self
    }

    pub fn with_json_body(mut self, body: &impl Serialize) -> Result<Self, Error> {
        self.body = RestApiRequestBody::try_from_json(body)?;
        Ok(self)
    }

    pub fn send(self) -> Result<RestApiResponse, Error> {
        let mut request = self.builder.build()?;
        self.body.apply_header(&mut request);
        OpenApiVerifier::load_from_env()?.verify_request(&request, &self.body)?;
        self.debug_flag.write_request(&request, &self.body);
        self.body.apply_body(&mut request);
        let response_raw = Client::new().execute(request)?;
        let response = RestApiResponse::new(response_raw)?;
        self.debug_flag.write_response(&response);
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

    pub fn ok_response(&self) -> Result<&Response, Error> {
        Ok(self.response().error_for_status_ref()?)
    }

    pub fn body(&self) -> &RestApiResponseBody {
        &self.body
    }
}

impl RestApiRequestBody {
    fn none() -> Self {
        RestApiRequestBody::None
    }

    fn from_binary(data: Vec<u8>) -> Self {
        RestApiRequestBody::Binary(data)
    }

    fn try_from_json(data: impl Serialize) -> Result<Self, Error> {
        let json = serde_json::to_string(&data).map_err(|source| {
            Error::RequestJsonSerializationError {
                source,
                filler: CustomErrorFiller,
            }
        })?;
        Ok(RestApiRequestBody::Json(json))
    }

    fn apply_header(&self, request: &mut Request) {
        let content_type = match self {
            RestApiRequestBody::None => return,
            RestApiRequestBody::Binary(_) => &mime::APPLICATION_OCTET_STREAM,
            RestApiRequestBody::Json(_) => &mime::APPLICATION_JSON,
        };
        let header_value = HeaderValue::from_static(content_type.as_ref());
        request
            .headers_mut()
            .insert(reqwest::header::CONTENT_TYPE, header_value);
    }

    fn apply_body(self, request: &mut Request) {
        let body = match self {
            RestApiRequestBody::None => return,
            RestApiRequestBody::Binary(data) => data.into(),
            RestApiRequestBody::Json(data) => data.into(),
        };
        request.body_mut().replace(body);
    }

    pub fn has_body(&self) -> bool {
        match self {
            RestApiRequestBody::None => false,
            _ => true,
        }
    }
}

impl fmt::Display for RestApiRequestBody {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RestApiRequestBody::None => Ok(()),
            RestApiRequestBody::Binary(ref data) => write!(f, "{}", hex::encode(data)),
            RestApiRequestBody::Json(ref data) => write!(f, "{}", data),
        }
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
            false => {
                let data = response.text()?;
                Ok(RestApiResponseBody::Text(data))
            }
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

    pub fn json<'a, T: serde::Deserialize<'a>>(&'a self) -> Result<T, Error> {
        match self {
            RestApiResponseBody::Text(text) => serde_json::from_str(text),
            RestApiResponseBody::Binary(binary) => serde_json::from_slice(binary),
        }
        .map_err(|source| Error::ResponseJsonDeserializationError {
            source,
            filler: CustomErrorFiller,
        })
    }

    pub fn json_value(&self) -> Result<serde_json::Value, Error> {
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
