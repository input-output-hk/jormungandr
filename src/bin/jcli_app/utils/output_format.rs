use gtmpl::Value as GtmplValue;
use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use std::fmt::{self, Display, Formatter};
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct OutputFormat {
    /// Format of output data. Possible values: json, yaml.
    /// Any other value is treated as a custom format using values from output data structure.
    /// Syntax is Go text template: https://golang.org/pkg/text/template/.
    #[structopt(long = "output-format", default_value = "yaml", parse(from_str))]
    format: FormatVariant,
}

enum FormatVariant {
    Yaml,
    Json,
    Custom(String),
}

impl<'a> From<&'a str> for FormatVariant {
    fn from(format: &'a str) -> Self {
        match format.trim().to_ascii_lowercase().as_str() {
            "yaml" => FormatVariant::Yaml,
            "json" => FormatVariant::Json,
            _ => FormatVariant::Custom(format.to_string()),
        }
    }
}

custom_error! { pub Error
    YamlFormattingFailed { source: serde_yaml::Error } = "failed to format output as YAML",
    JsonFormattingFailed { source: serde_json::Error } = "failed to format output as JSON",
    CustomFormattingFailed { source: GtmplError } = "failed to format output as custom format",
}

#[derive(Debug)]
pub struct GtmplError(String);

impl Display for GtmplError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl std::error::Error for GtmplError {}

impl OutputFormat {
    pub fn format_json(&self, data: JsonValue) -> Result<String, Error> {
        Ok(match self.format {
            FormatVariant::Yaml => serde_yaml::to_string(&data)?,
            FormatVariant::Json => serde_json::to_string_pretty(&data)?,
            FormatVariant::Custom(ref format) => {
                let gtmpl_value = json_value_to_gtmpl(data);
                gtmpl::template(format.as_str(), gtmpl_value).map_err(GtmplError)?
            }
        })
    }
}

fn json_value_to_gtmpl(value: JsonValue) -> GtmplValue {
    match value {
        JsonValue::Null => GtmplValue::Nil,
        JsonValue::Bool(boolean) => boolean.into(),
        JsonValue::Number(number) => json_number_to_gtmpl(number),
        JsonValue::String(string) => string.into(),
        JsonValue::Array(array) => json_array_to_gtmpl(array),
        JsonValue::Object(object) => json_object_to_gtmpl(object),
    }
}

fn json_number_to_gtmpl(number: JsonNumber) -> GtmplValue {
    None.or_else(|| number.as_u64().map(Into::into))
        .or_else(|| number.as_i64().map(Into::into))
        .or_else(|| number.as_f64().map(Into::into))
        .unwrap()
}

fn json_array_to_gtmpl(array: Vec<JsonValue>) -> GtmplValue {
    let values = array.into_iter().map(json_value_to_gtmpl).collect();
    GtmplValue::Array(values)
}

fn json_object_to_gtmpl(object: JsonMap<String, JsonValue>) -> GtmplValue {
    let values = object
        .into_iter()
        .map(|(key, value)| (key, json_value_to_gtmpl(value)))
        .collect();
    GtmplValue::Object(values)
}
