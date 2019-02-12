use std::fmt;

/// Common error codes for network protocol requests.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Code {
    Canceled,
    Failed,
    NotFound,
    Unimplemented,
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            Code::Canceled => "processing canceled",
            Code::Failed => "processing error",
            Code::NotFound => "not found",
            Code::Unimplemented => "not implemented",
        };
        f.write_str(msg)
    }
}
