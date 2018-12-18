/// Deserialization of blockchain objects.
pub trait Deserialize: Sized {
    /// The type representing deserialization errors.
    type Error;

    /// Deserializes an object from its byte representation.
    fn deserialize(data: &[u8]) -> Result<Self, Self::Error>;
}
