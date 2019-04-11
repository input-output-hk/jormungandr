//! helper functions to parse values from the StructOpt command
//! line data

use chain_addr::{Address, AddressReadable, Error as ParseAddressError};
use chain_impl_mockchain::value::Value;

custom_error! {pub ParseValueError
    ParseIntError { source: std::num::ParseIntError } = "Invalid value",
}
pub fn try_parse_value(s: &str) -> Result<Value, ParseValueError> {
    let v = s.parse()?;
    Ok(Value(v))
}

pub fn try_parse_address(s: &str) -> Result<Address, ParseAddressError> {
    let address_readable = AddressReadable::from_string(s)?;
    Ok(address_readable.to_address())
}
