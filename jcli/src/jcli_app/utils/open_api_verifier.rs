use crate::jcli_app::utils::rest_api::RestApiRequestBody;
use mime::Mime;
use openapiv3::{
    OpenAPI, Operation, Parameter, ParameterData, ParameterSchemaOrContent, PathItem, PathStyle,
    Paths, ReferenceOr, RequestBody, Schema, SchemaKind, StringFormat, StringType, Type,
    VariantOrUnknownOrEmpty,
};
use reqwest::{blocking::Request, Method};
use serde_json::Value;
use std::{collections::HashMap, env, fs::File, io};
use thiserror::Error;
use valico::json_schema::{SchemaError as ValicoError, Scope, ValidationState};

const BINARY_BODY_MIME: &Mime = &mime::APPLICATION_OCTET_STREAM;
const JSON_BODY_MIME: &Mime = &mime::APPLICATION_JSON;

pub struct OpenApiVerifier(VerifierMode);

#[allow(clippy::large_enum_variant)]
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

#[derive(Debug, Error)]
pub enum Error {
    #[error("verification with OpenAPI definition in '{path}' failed")]
    VerificationFailed {
        #[source]
        source: VerifierError,
        path: String,
    },
}

#[derive(Debug, Error)]
pub enum VerifierError {
    #[error("could not open OpenApi file")]
    OpenApiFileOpenFailed(#[from] io::Error),
    #[error("OpenApi file malformed")]
    OpenApiFileMalformed(#[from] serde_yaml::Error),
    #[error("failed to validate URL '{url}'")]
    PathVerificationFailed {
        #[source]
        source: PathError,
        url: String,
    },
}

#[derive(Debug, Error)]
pub enum PathError {
    #[error("path not found")]
    NotFound,
    #[error("error while reading reference")]
    RefError(#[from] RefError),
    #[error("failed to validate method '{method}'")]
    MethodVerificationFailed {
        #[source]
        source: MethodError,
        method: String,
    },
}

#[derive(Debug, Error)]
pub enum MethodError {
    #[error("method not found")]
    NotFound,
    #[error("failed to validate request path")]
    RequestPathVerificationFailed(#[from] RequestPathError),
    #[error("failed to validate request body")]
    RequestBodyVerificationFailed(#[from] RequestBodyError),
}

#[derive(Debug, Error)]
pub enum RequestPathError {
    #[error("error while reading reference")]
    RefError(#[from] RefError),
    #[error("parameter '{name}' not found in request URL")]
    ParamNotFound { name: String },
    #[error("parameters {names:?} undocumented")]
    ParamsUndocumented { names: Vec<String> },
    #[error("failed to validate request path value for parameter '{name}'")]
    RequestPathValueVerificationFailed {
        #[source]
        source: RequestPathValueError,
        name: String,
    },
}

#[derive(Debug, Error)]
pub enum RequestPathValueError {
    #[error("error while reading reference")]
    RefError(#[from] RefError),
    #[error("path style '{style:?}' is not supported")]
    PathStyleUnsupported { style: PathStyle },
    #[error("content in path definition not supported")]
    ContentUnsupported,
    #[error("schema does not match path value")]
    SchemaInvalid(#[from] SchemaError),
}

#[derive(Debug, Error)]
pub enum RequestBodyError {
    #[error("error while reading reference")]
    RefError(#[from] RefError),
    #[error("request should not have body")]
    UnexpectedBody,
    #[error("request should have a body")]
    ExpectedBody,
    #[error("failed to verify media type '{mime}'")]
    MediaTypeVerificationFailed {
        #[source]
        source: RequestMediaTypeError,
        mime: &'static Mime,
    },
}

#[derive(Debug, Error)]
pub enum RequestMediaTypeError {
    #[error("media type not found")]
    NotFound,
    #[error("error while reading schema reference")]
    SchemaRefError(#[from] RefError),
    #[error("schema is missing")]
    SchemaMissing,
    #[error("schema does not match binary blob")]
    SchemaBinaryInvalid,
    #[error("failed to validate JSON with schema")]
    SchemaJsonInvalid(#[from] SchemaError),
}

#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("schema serialization failed")]
    SchemaSerializationFailed(#[source] serde_json::Error),
    #[error("schema is not valid")]
    SchemaInvalid(#[from] ValicoError),
    #[error("only schemas with single specific type are supported")]
    SchemaNotSpecific,
    #[error("only schemas for primitive types are supported")]
    SchemaNotPrimitive,
    #[error("value is not a valid primitive")]
    ValueNotValidPrimitive(#[source] serde_json::Error),
    #[error("value is not a valid JSON")]
    ValueNotValidJson(#[source] serde_json::Error),
    #[error("value does not match schema: {report:?}")]
    ValueValidationFailed { report: ValidationState },
}

#[derive(Debug, Error)]
pub enum RefError {
    #[error("references are not supported, found one pointing at '{reference}'")]
    NotSupported { reference: String },
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
    let (wildcards, path_item) = find_path_item(&openapi.paths, req.url().path())?;
    verify_method(wildcards, path_item, req, body).map_err(|source| {
        PathError::MethodVerificationFailed {
            source,
            method: req.method().to_string(),
        }
    })
}

fn find_path_item<'a>(
    paths: &'a Paths,
    url: &'a str,
) -> Result<(PathWildcardValues<'a>, &'a PathItem), PathError> {
    let (wildcards, path_item_ref) = paths
        .iter()
        .find_map(|(path, path_item_ref)| {
            url_matches_path(url, &path).map(|wildcards| (wildcards, path_item_ref))
        })
        .ok_or_else(|| PathError::NotFound)?;
    let path_item = unpack_reference_or(path_item_ref)?;
    Ok((wildcards, path_item))
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
    if path_segment.starts_with('{') && path_segment.ends_with('}') {
        Some(&path_segment[1..path_segment.len() - 1])
    } else {
        None
    }
}

fn verify_method(
    wildcards: PathWildcardValues,
    path_item: &PathItem,
    req: &Request,
    body: &RestApiRequestBody,
) -> Result<(), MethodError> {
    let operation = find_operation(path_item, req)?;
    verify_request_path(wildcards, &operation.parameters)?;
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

fn verify_request_path(
    mut wildcards: PathWildcardValues,
    param_refs: &[ReferenceOr<Parameter>],
) -> Result<(), RequestPathError> {
    for param_ref in param_refs {
        let (param_data, style) = match unpack_reference_or(param_ref)? {
            Parameter::Path {
                parameter_data,
                style,
            } => (parameter_data, style),
            _ => continue,
        };
        let name = &param_data.name;
        let value = wildcards
            .remove(&**name)
            .ok_or_else(|| RequestPathError::ParamNotFound { name: name.clone() })?;
        verify_request_path_value(value, param_data, style).map_err(|source| {
            RequestPathError::RequestPathValueVerificationFailed {
                source,
                name: name.clone(),
            }
        })?;
    }
    let unchecked: Vec<_> = wildcards.keys().map(|key| (*key).to_owned()).collect();
    if !unchecked.is_empty() {
        Err(RequestPathError::ParamsUndocumented { names: unchecked })
    } else {
        Ok(())
    }
}

fn verify_request_path_value(
    value: &str,
    param_data: &ParameterData,
    style: &PathStyle,
) -> Result<(), RequestPathValueError> {
    match style {
        PathStyle::Simple => Ok(()),
        _ => Err(RequestPathValueError::PathStyleUnsupported {
            style: style.clone(),
        }),
    }?;
    let schema = match param_data.format {
        ParameterSchemaOrContent::Schema(ref schema_ref) => unpack_reference_or(schema_ref)?,
        ParameterSchemaOrContent::Content(_) => {
            return Err(RequestPathValueError::ContentUnsupported)
        }
    };
    verify_schema_simple_string(schema, value)?;
    Ok(())
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
    if body_def.required {
        Err(RequestBodyError::ExpectedBody)
    } else {
        Ok(())
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
    let value = serde_json::from_str(json).map_err(SchemaError::ValueNotValidJson)?;
    validate_schema_value(schema, &value)
}

fn verify_schema_simple_string(schema: &Schema, simple: &str) -> Result<(), SchemaError> {
    let schema_type = match schema.schema_kind {
        SchemaKind::Type(ref schema_type) => schema_type,
        _ => return Err(SchemaError::SchemaNotSpecific),
    };
    let value = match schema_type {
        Type::String(_) => Value::String(simple.to_string()),
        Type::Boolean { .. } | Type::Integer(_) | Type::Number(_) => {
            serde_json::from_str(simple).map_err(SchemaError::ValueNotValidPrimitive)?
        }
        Type::Array(_) | Type::Object(_) => return Err(SchemaError::SchemaNotPrimitive),
    };
    validate_schema_value(schema, &value)
}

fn validate_schema_value(schema: &Schema, value: &Value) -> Result<(), SchemaError> {
    let schema_value =
        serde_json::to_value(schema).map_err(SchemaError::SchemaSerializationFailed)?;
    let mut scope = Scope::new();
    let validator = scope.compile_and_return(schema_value, true)?;
    let report = validator.validate(value);
    if !report.is_strictly_valid() {
        Err(SchemaError::ValueValidationFailed { report })
    } else {
        Ok(())
    }
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
