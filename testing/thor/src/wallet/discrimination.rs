use chain_addr::Discrimination;

pub trait DiscriminationExtension {
    fn into_prefix(self) -> String;
    fn from_testing_bool(testing: bool) -> Self;
    fn from_prefix(prefix: &str) -> Self;
}

impl DiscriminationExtension for Discrimination {
    fn into_prefix(self) -> String {
        match self {
            Discrimination::Test => "ta".to_string(),
            Discrimination::Production => "ca".to_string(),
        }
    }
    fn from_testing_bool(testing: bool) -> Discrimination {
        if testing {
            Discrimination::Test
        } else {
            Discrimination::Production
        }
    }

    fn from_prefix(prefix: &str) -> Self {
        if prefix == "ca" {
            Discrimination::Production
        } else if prefix == "ta" {
            Discrimination::Test
        } else {
            unreachable!("unknown prefix");
        }
    }
}
