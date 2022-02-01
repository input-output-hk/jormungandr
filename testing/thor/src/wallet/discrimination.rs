use chain_addr::Discrimination;

pub trait DiscriminationExtension {
    fn into_prefix(self) -> String;
}

impl DiscriminationExtension for Discrimination {
    fn into_prefix(self) -> String {
        match self {
            Discrimination::Test => "ta".to_string(),
            Discrimination::Production => "ca".to_string(),
        }
    }
}
