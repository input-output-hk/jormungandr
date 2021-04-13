use crate::crypto::key::Identifier;
use crate::interfaces::config::{Log, Mempool};
use crate::multiaddr as multiaddr_utils;
use crate::time::Duration;
use chain_crypto::Ed25519;
use multiaddr::Multiaddr;
use serde::{de::Visitor, Deserialize, Deserializer, Serialize};
use std::{fmt, net::SocketAddr, path::PathBuf, str::FromStr};
const DEFAULT_PREFERRED_VIEW_MAX: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rest {
    pub listen: SocketAddr,
    /// Enables TLS and disables plain HTTP if provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<Tls>,
    /// Enables CORS if provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cors: Option<Cors>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Tls {
    /// Path to server X.509 certificate chain file, must be PEM-encoded and contain at least 1 item
    pub cert_file: String,
    /// Path to server private key file, must be PKCS8 with single PEM-encoded, unencrypted key
    pub priv_key_file: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Cors {
    /// If none provided, echos request origin
    #[serde(default)]
    pub allowed_origins: Vec<CorsOrigin>,
    /// If none provided, CORS responses won't be cached
    pub max_age_secs: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct CorsOrigin(String);

impl<'de> Deserialize<'de> for CorsOrigin {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CorsOriginVisitor;
        impl<'de> Visitor<'de> for CorsOriginVisitor {
            type Value = CorsOrigin;

            fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                write!(fmt, "an origin in format http[s]://example.com[:3000]",)
            }

            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                use serde::de::Unexpected;

                let uri = http::uri::Uri::from_str(v).map_err(E::custom)?;
                if let Some(s) = uri.scheme_str() {
                    if s != "http" && s != "https" {
                        return Err(E::invalid_value(Unexpected::Str(v), &self));
                    }
                } else {
                    return Err(E::invalid_value(Unexpected::Str(v), &self));
                }
                if let Some(p) = uri.path_and_query() {
                    if p.as_str() != "/" {
                        return Err(E::invalid_value(Unexpected::Str(v), &self));
                    }
                }
                Ok(CorsOrigin(v.trim_end_matches('/').to_owned()))
            }
        }
        deserializer.deserialize_str(CorsOriginVisitor)
    }
}

impl AsRef<str> for CorsOrigin {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2p {
    /// The public address to which other peers may connect to
    pub public_address: Multiaddr,

    /// File with the secret key used to advertise and authenticate the node
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_key_file: Option<PathBuf>,

    /// the rendezvous points for the peer to connect to in order to initiate
    /// the p2p discovery from.
    pub trusted_peers: Vec<TrustedPeer>,

    /// Listen address as a multiaddr.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub listen_address: Option<SocketAddr>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_connections: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_inbound_connections: Option<u32>,

    pub allow_private_addresses: bool,

    pub policy: Option<Policy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub layers: Option<LayersConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TopicsOfInterest {
    pub messages: String,
    pub blocks: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Policy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quarantine_duration: Option<Duration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quarantine_whitelist: Option<Vec<Multiaddr>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Explorer {
    pub enabled: bool,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayersConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_list: Option<PreferredListConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topics_of_interest: Option<TopicsOfInterest>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PreferredViewMax(usize);

impl Default for PreferredViewMax {
    fn default() -> Self {
        Self(DEFAULT_PREFERRED_VIEW_MAX)
    }
}

impl From<PreferredViewMax> for usize {
    fn from(pvm: PreferredViewMax) -> Self {
        pvm.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PreferredListConfig {
    #[serde(default)]
    pub view_max: PreferredViewMax,

    #[serde(default)]
    pub peers: Vec<TrustedPeer>,
}

/// Identifier of a peer node.
pub type NodeId = Identifier<Ed25519>;

/// Configuration item for a trusted peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedPeer {
    /// The peer's routable host address and TCP port as a multiaddr.
    /// The supported address components are `/ip4`, `/ip6`,
    /// `/dns`, `/dns4`, `/dns6`.
    /// The port component must be `/tcp`.
    pub address: Multiaddr,
    /// Node identifier as a bech32-encoded ed25519 public key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<NodeId>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NodeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<PathBuf>,
    pub rest: Rest,
    pub p2p: P2p,
    pub log: Option<Log>,
    pub explorer: Explorer,
    pub mempool: Option<Mempool>,
    pub bootstrap_from_trusted_peers: Option<bool>,
    pub skip_bootstrap: Option<bool>,
}

impl P2p {
    pub fn get_listen_addr(&self) -> Option<SocketAddr> {
        self.listen_address
            .or_else(|| multiaddr_utils::to_tcp_socket_addr(&self.public_address))
    }
}
