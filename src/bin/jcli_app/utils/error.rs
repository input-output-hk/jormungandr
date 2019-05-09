/// Structure intended to block `From` generation in custom_error! macro
/// Macro uses `source` field to automatically generate `std::error::Error::source` implementation.
/// When this is the only field, it also generates `From` implementation for this source's type.
/// It causes conflict when two error variants have the same source type.
/// Addition of any field blocks generation of `From`, but doesn't affect `Error::source`.
/// This structure can be added as such field to bypass described issue.
#[derive(Debug)]
pub struct CustomErrorFiller;

impl ToString for CustomErrorFiller {
    fn to_string(&self) -> String {
        "custom error filler".to_string()
    }
}
