use jcli_app::utils::{rest_api::RestApiRequestBody, CustomErrorFiller};
use mime::Mime;
use openapiv3::{
    OpenAPI, Operation, PathItem, ReferenceOr, RequestBody, Schema, SchemaKind, StringFormat,
    StringType, Type, VariantOrUnknownOrEmpty,
};
use reqwest::{Method, Request};
use serde_json::Value;
use std::env;
use std::fs::File;
use std::io;
use valico::json_schema::{SchemaError as ValicoError, Scope, ValidationState};

const BINARY_BODY_MIME: &'static Mime = &mime::APPLICATION_OCTET_STREAM;
const JSON_BODY_MIME: &'static Mime = &mime::APPLICATION_JSON;

pub struct OpenApiVerifier(VerifierMode);

enum VerifierMode {
    AcceptAll,
    Verify(Verifier),
}

struct Verifier {
    openapi: OpenAPI,
    path: String,
}

type PathWildcardValues<'a> = HashMap<&'a str, &'a str>;

enum SegmentMatch<'a> {
    None,
    Exact,
    Wildcard(&'a str),
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
    NotFound = "path not found",
    RefError { source: RefError } = "error while reading reference",
    MethodVerificationFailed { source: MethodError, method: String }
        = "failed to validate method '{method}'",
}

custom_error! { pub MethodError
    NotFound = "method not found",
    RequestBodyVerificationFailed { source: RequestBodyError } = "failed to validate request body",
}

custom_error! { pub RequestBodyError
    RefError { source: RefError } = "error while reading reference",
    UnexpectedBody = "request should not have body",
    ExpectedBody = "request should have a body",
    MediaTypeVerificationFailed { source: RequestMediaTypeError, mime: &'static Mime }
        = "failed to verify media type '{mime}'"
}

custom_error! { pub RequestMediaTypeError
    NotFound = "media type not found",
    SchemaRefError { source: RefError } = "error while reading schema reference",
    SchemaMissing = "schema is missing",
    SchemaBinaryInvalid = "schema does not match binary blob",
    SchemaJsonInvalid { source: SchemaError } = "failed to validate JSON with schema",
}

custom_error! { pub SchemaError
    SchemaSerializationFailed { source: serde_json::Error, filler: CustomErrorFiller }
        = "schema serialization failed",
    SchemaInvalid { source: ValicoError } = "schema is not valid",
    ValueNotValidJson { source: serde_json::Error, filler: CustomErrorFiller }
        = "value is not a valid JSON",
    ValueValidationFailed { report: ValidationState }
        = @{format_args!("value does not match schema: {:?}", report)},
}

custom_error! { pub RefError
    NotSupported { reference: String }
        = "references are not supported, found one pointing at '{reference}'",
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
        .ok_or_else(|| PathError::NotFound)
        .and_then(|(_, item)| unpack_reference_or(item).map_err(Into::into))
}

fn url_matches_path<'a>(url: &'a str, path: &'a str) -> Option<PathWildcardValues<'a>> {
    let mut url_segments = url.trim_matches('/').rsplit('/');
    let path_segments = path.trim_matches('/').rsplit('/');
    let mut wildcards = HashMap::new();
    for path_segment in path_segments {
        let url_segment = url_segments.next()?;
        match url_segment_matches_path_segment(url_segment, path_segment) {
            SegmentMatch::None => return None,
            SegmentMatch::Exact => continue,
            SegmentMatch::Wildcard(wildcard) => wildcards.insert(wildcard, url_segment),
        };
    }
    Some(wildcards)
}

fn url_segment_matches_path_segment<'a>(
    url_segment: &str,
    path_segment: &'a str,
) -> SegmentMatch<'a> {
    match path_segment_wildcard_name(path_segment) {
        Some(wildcard) => SegmentMatch::Wildcard(wildcard),
        None if path_segment == url_segment => SegmentMatch::Exact,
        None => SegmentMatch::None,
    }
}

fn path_segment_wildcard_name(path_segment: &str) -> Option<&str> {
    match path_segment.starts_with('{') && path_segment.ends_with('}') {
        true => Some(&path_segment[1..path_segment.len() - 1]),
        false => None,
    }
}

fn verify_method(
    path: &PathItem,
    req: &Request,
    body: &RestApiRequestBody,
) -> Result<(), MethodError> {
    let operation = find_operation(path, req)?;
    verify_request_body(&operation.request_body, body)?;
    Ok(())
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
    .ok_or_else(|| MethodError::NotFound)
}

fn verify_request_body(
    body_def_opt: &Option<ReferenceOr<RequestBody>>,
    body: &RestApiRequestBody,
) -> Result<(), RequestBodyError> {
    match unpack_reference_or_opt(body_def_opt)? {
        Some(body_def) => match body {
            RestApiRequestBody::None => verify_none_body(body_def),
            RestApiRequestBody::Binary(_) => verify_binary_body(body_def),
            RestApiRequestBody::Json(ref json) => verify_json_body(body_def, json),
        },
        None => verify_body_is_none(body),
    }
}

fn verify_none_body(body_def: &RequestBody) -> Result<(), RequestBodyError> {
    match body_def.required {
        true => Err(RequestBodyError::ExpectedBody),
        false => Ok(()),
    }
}

fn verify_binary_body(body_def: &RequestBody) -> Result<(), RequestBodyError> {
    verify_binary_media_type(body_def).map_err(|source| {
        RequestBodyError::MediaTypeVerificationFailed {
            source,
            mime: BINARY_BODY_MIME,
        }
    })
}

fn verify_binary_media_type(body_def: &RequestBody) -> Result<(), RequestMediaTypeError> {
    let schema = unpack_request_media_type_schema(body_def, BINARY_BODY_MIME)?;
    let valid_schema_kind = SchemaKind::Type(Type::String(StringType {
        format: VariantOrUnknownOrEmpty::Item(StringFormat::Binary),
        pattern: None,
        enumeration: vec![],
    }));
    if schema.schema_kind != valid_schema_kind {
        return Err(RequestMediaTypeError::SchemaBinaryInvalid);
    }
    Ok(())
}

fn verify_json_body(body_def: &RequestBody, json: &str) -> Result<(), RequestBodyError> {
    verify_json_media_type(body_def, json).map_err(|source| {
        RequestBodyError::MediaTypeVerificationFailed {
            source,
            mime: JSON_BODY_MIME,
        }
    })
}

fn verify_json_media_type(body_def: &RequestBody, json: &str) -> Result<(), RequestMediaTypeError> {
    let schema = unpack_request_media_type_schema(body_def, JSON_BODY_MIME)?;
    verify_schema_json(schema, json)?;
    Ok(())
}

fn unpack_request_media_type_schema<'a>(
    body_def: &'a RequestBody,
    mime: &Mime,
) -> Result<&'a Schema, RequestMediaTypeError> {
    let media_type = body_def
        .content
        .get(mime.as_ref())
        .ok_or_else(|| RequestMediaTypeError::NotFound)?;
    let schema = unpack_reference_or_opt(&media_type.schema)?
        .ok_or_else(|| RequestMediaTypeError::SchemaMissing)?;
    Ok(schema)
}

fn verify_body_is_none(body: &RestApiRequestBody) -> Result<(), RequestBodyError> {
    match body {
        RestApiRequestBody::None => Ok(()),
        _ => Err(RequestBodyError::UnexpectedBody),
    }
}

fn verify_schema_json(schema: &Schema, json: &str) -> Result<(), SchemaError> {
    let value = serde_json::from_str(json).map_err(|source| SchemaError::ValueNotValidJson {
        source,
        filler: CustomErrorFiller,
    })?;
    validate_schema_value(schema, &value)
}

fn validate_schema_value(schema: &Schema, value: &Value) -> Result<(), SchemaError> {
    let schema_value =
        serde_json::to_value(schema).map_err(|source| SchemaError::SchemaSerializationFailed {
            source,
            filler: CustomErrorFiller,
        })?;
    let mut scope = Scope::new();
    let validator = scope.compile_and_return(schema_value, true)?;
    let report = validator.validate(value);
    if !report.is_strictly_valid() {
        Err(SchemaError::ValueValidationFailed { report })?
    }
    Ok(())
}

fn unpack_reference_or_opt<T>(
    reference_or: &Option<ReferenceOr<T>>,
) -> Result<Option<&T>, RefError> {
    reference_or.as_ref().map(unpack_reference_or).transpose()
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
        assert_url_matches_path(hashmap!(), "abc", "abc");
        assert_url_matches_path(None, "abc", "def");

        assert_url_matches_path(hashmap!(), "/abc/", "abc");
        assert_url_matches_path(hashmap!(), "abc", "/abc/");

        assert_url_matches_path(None, "abc", "abc/def");
        assert_url_matches_path(None, "abc", "def/abc");
        assert_url_matches_path(None, "abc/def", "abc");
        assert_url_matches_path(hashmap!(), "def/abc", "abc");

        assert_url_matches_path(hashmap!("x" => "abc"), "abc", "{x}");
        assert_url_matches_path(None, "{x}", "abc");
        assert_url_matches_path(hashmap!("x" => "def"), "abc/def/ghi", "abc/{x}/ghi");
        assert_url_matches_path(
            hashmap!("x" => "abc", "y" => "def", "z" => "ghi"),
            "abc/def/ghi",
            "{x}/{y}/{z}",
        );
    }

    fn assert_url_matches_path<E>(expected: E, url: &str, path: &str)
    where
        E: Into<Option<PathWildcardValues<'static>>>,
    {
        let actual = url_matches_path(url, path);

        assert_eq!(
            expected.into(),
            actual,
            "Invalid result for URL '{}' and path '{}'",
            url,
            path
        );
    }

    #[test]
    fn path_segment_wildcard_name_tests() {
        assert_path_segment_wildcard_name(None, "");
        assert_path_segment_wildcard_name(None, "abc");
        assert_path_segment_wildcard_name(None, "{abc");
        assert_path_segment_wildcard_name(None, "abc}");
        assert_path_segment_wildcard_name("abc", "{abc}");
        assert_path_segment_wildcard_name("{abc}", "{{abc}}");
        assert_path_segment_wildcard_name("", "{}");
    }

    fn assert_path_segment_wildcard_name(expected: impl Into<Option<&'static str>>, input: &str) {
        let actual = path_segment_wildcard_name(input);

        assert_eq!(
            expected.into(),
            actual,
            "Invalid result for input '{}'",
            input
        );
    }
}
