use openapiv3::{OpenAPI, PathItem, ReferenceOr};
use reqwest::Request;
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
    VerificationFailed { source: ImplError, path: String }
        = "verification with OpenAPI definition in '{path}' failed",
}

custom_error! { pub ImplError
    OpenApiFileOpenFailed { source: io::Error } = "could not open OpenApi file",
    OpenApiFileMalformed { source: serde_yaml::Error } = "OpenApi file malformed",
    OpenApiFilePathRef { path: String } = "path '{path}' in OpenApi file is a reference",
    PathNotFound { url: String } = "no path found for URL '{url}'",
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

    pub fn verify_request(&self, req: &Request) -> Result<(), Error> {
        match self.0 {
            VerifierMode::AcceptAll => Ok(()),
            VerifierMode::Verify(ref verifier) => {
                verifier
                    .verify_request(req)
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

    pub fn load(path: String) -> Result<Self, ImplError> {
        let file = File::open(&path)?;
        let openapi = serde_yaml::from_reader(file)?;
        Ok(Verifier { openapi, path })
    }

    pub fn verify_request(&self, req: &Request) -> Result<(), ImplError> {
        let url = req.url().path();
        let _item = self.find_path_item(url)?;
        Ok(())
    }

    fn find_path_item(&self, url: &str) -> Result<&PathItem, ImplError> {
        let (path, item) = self
            .openapi
            .paths
            .iter()
            .find(|(path, _)| url_matches_path(url, &path))
            .ok_or_else(|| ImplError::PathNotFound {
                url: url.to_string(),
            })?;
        match item {
            ReferenceOr::Reference { .. } => {
                Err(ImplError::OpenApiFilePathRef { path: path.clone() })
            }
            ReferenceOr::Item(ref path_item) => Ok(path_item),
        }
    }
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
