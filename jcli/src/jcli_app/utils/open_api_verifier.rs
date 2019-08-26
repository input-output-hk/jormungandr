use jcli_app::utils::rest_api::RestApiRequestBody;
use openapiv3::{OpenAPI, Operation, PathItem, ReferenceOr, RequestBody};
use reqwest::{Method, Request};
use std::env;
use std::fs::File;
use std::io;

pub struct OpenApiVerifier(VerifierMode);

enum VerifierMode {
    AcceptAll,
    Verify(Verifier),
}

struct Verifier {
    openapi: OpenAPI,
    path: String,
}

custom_error! { pub Error
    VerificationFailed { source: VerifierError, path: String }
        = "verification with OpenAPI definition in '{path}' failed",
}

custom_error! { pub VerifierError
    OpenApiFileOpenFailed { source: io::Error } = "could not open OpenApi file",
    OpenApiFileMalformed { source: serde_yaml::Error } = "OpenApi file malformed",
    PathVerificationFailed { source: PathError, url: String } = "failed to validate URL '{url}'",
}

custom_error! { pub PathError
    PathNotFound = "path not found",
    RefError { source: RefError } = "error while reading reference",
    MethodVerificationFailed { source: MethodError, method: String } = "failed to validate method '{method}'",
}

custom_error! { pub MethodError
    MethodNotFound = "method not found",
}

custom_error! { pub RefError
    NotSupported { reference: String } = "references are not supported, found one pointing at '{reference}'",
}

impl OpenApiVerifier {
    pub fn load_from_env() -> Result<Self, Error> {
        match env::var("JCLI_OPEN_API_VERIFY_PATH") {
            Ok(path) => Self::load(path),
            Err(_) => Ok(OpenApiVerifier(VerifierMode::AcceptAll)),
        }
    }

    pub fn load(path: String) -> Result<Self, Error> {
        Verifier::load(path.clone())
            .map(|verifier| OpenApiVerifier(VerifierMode::Verify(verifier)))
            .map_err(|source| Error::VerificationFailed { source, path })
    }

    pub fn verify_request(&self, req: &Request, body: &RestApiRequestBody) -> Result<(), Error> {
        match self.0 {
            VerifierMode::AcceptAll => Ok(()),
            VerifierMode::Verify(ref verifier) => {
                verifier
                    .verify_request(req, body)
                    .map_err(|source| Error::VerificationFailed {
                        source,
                        path: verifier.path(),
                    })
            }
        }
    }
}

impl Verifier {
    pub fn path(&self) -> String {
        self.path.clone()
    }

    pub fn load(path: String) -> Result<Self, VerifierError> {
        let file = File::open(&path)?;
        let openapi = serde_yaml::from_reader(file)?;
        Ok(Verifier { openapi, path })
    }

    pub fn verify_request(
        &self,
        req: &Request,
        body: &RestApiRequestBody,
    ) -> Result<(), VerifierError> {
        verify_path(&self.openapi, req, body).map_err(|source| {
            VerifierError::PathVerificationFailed {
                source,
                url: req.url().path().to_string(),
            }
        })
    }
}

fn verify_path(
    openapi: &OpenAPI,
    req: &Request,
    body: &RestApiRequestBody,
) -> Result<(), PathError> {
    let path = find_path_item(openapi, req.url().path())?;
    verify_method(path, req, body).map_err(|source| PathError::MethodVerificationFailed {
        source,
        method: req.method().to_string(),
    })
}

fn find_path_item<'a>(openapi: &'a OpenAPI, url: &str) -> Result<&'a PathItem, PathError> {
    openapi
        .paths
        .iter()
        .find(|(path, _)| url_matches_path(url, &path))
        .ok_or_else(|| PathError::PathNotFound)
        .and_then(|(_, item)| unpack_reference_or(item).map_err(Into::into))
}

fn url_matches_path(url: &str, path: &str) -> bool {
    let url_segments = url.trim_matches('/').rsplit('/');
    let mut path_segments = path.trim_matches('/').rsplit('/');
    let segments_eq = url_segments
        .zip(path_segments.by_ref())
        .all(url_segment_matches_path_segment);
    let path_exhausted = path_segments.next().is_none();
    segments_eq && path_exhausted
}

fn url_segment_matches_path_segment((url_segment, path_segment): (&str, &str)) -> bool {
    path_segment_is_wildcard(path_segment) || path_segment == url_segment
}

fn path_segment_is_wildcard(path_segment: &str) -> bool {
    path_segment.starts_with('{') && path_segment.ends_with('}')
}

fn verify_method(
    path: &PathItem,
    req: &Request,
    body: &RestApiRequestBody,
) -> Result<(), MethodError> {
    let operation = find_operation(path, req)?;
    Ok(()) //TODO
}

fn find_operation<'a>(path: &'a PathItem, req: &Request) -> Result<&'a Operation, MethodError> {
    let method = req.method();
    match *method {
        Method::GET => &path.get,
        Method::PUT => &path.put,
        Method::POST => &path.post,
        Method::DELETE => &path.delete,
        Method::OPTIONS => &path.options,
        Method::HEAD => &path.head,
        Method::PATCH => &path.patch,
        Method::TRACE => &path.trace,
        _ => &None,
    }
    .as_ref()
    .ok_or_else(|| MethodError::MethodNotFound)
}

fn unpack_reference_or<T>(reference_or: &ReferenceOr<T>) -> Result<&T, RefError> {
    match *reference_or {
        ReferenceOr::Item(ref path_item) => Ok(path_item),
        ReferenceOr::Reference { ref reference } => Err(RefError::NotSupported {
            reference: reference.clone(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_matches_path_tests() {
        assert_url_matches_path("abc", "abc");
        refute_url_matches_path("abc", "def");

        assert_url_matches_path("/abc/", "abc");
        assert_url_matches_path("abc", "/abc/");

        refute_url_matches_path("abc", "abc/def");
        refute_url_matches_path("abc", "def/abc");
        refute_url_matches_path("abc/def", "abc");
        assert_url_matches_path("def/abc", "abc");

        assert_url_matches_path("abc", "{x}");
        refute_url_matches_path("{x}", "abc");
        assert_url_matches_path("abc/def/ghi", "abc/{x}/ghi");
    }

    fn assert_url_matches_path(url: &str, path: &str) {
        let result = url_matches_path(url, path);

        assert!(result, "Url '{}' was not accepted for path '{}'", url, path);
    }

    fn refute_url_matches_path(url: &str, path: &str) {
        let result = url_matches_path(url, path);

        assert!(!result, "Url '{}' was accepted for path '{}'", url, path);
    }
}
