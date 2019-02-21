use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
/// Format of the logger.
pub enum LogFormat {
    Plain,
    Json,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cleared = s.trim().to_lowercase();
        if cleared == "plain" {
            Ok(LogFormat::Plain)
        } else if cleared == "json" {
            Ok(LogFormat::Json)
        } else {
            let mut msg = "unknown format ".to_string();
            msg.push_str(&cleared);
            Err(msg)
        }
    }
}
