use chain_core::packer::Codec;
use chain_impl_mockchain::fragment::Fragment;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{DeserializeAs, SerializeAs};

#[derive(Debug)]
pub struct FragmentDef(Fragment);

impl From<Fragment> for FragmentDef {
    fn from(fragment: Fragment) -> Self {
        Self(fragment)
    }
}

impl From<FragmentDef> for Fragment {
    fn from(fragment_def: FragmentDef) -> Self {
        fragment_def.0
    }
}

impl FragmentDef {
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Fragment, D::Error>
    where
        D: Deserializer<'de>,
    {
        <Self as Deserialize>::deserialize(deserializer).map(|fragment_def| fragment_def.0)
    }

    pub fn serialize<S>(fragment: &Fragment, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_fragment(fragment, serializer)
    }
}

impl<'de> Deserialize<'de> for FragmentDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = if deserializer.is_human_readable() {
            let h = String::deserialize(deserializer)?;
            hex::decode(&h).map_err(serde::de::Error::custom)?
        } else {
            Vec::<u8>::deserialize(deserializer)?
        };

        let fragment =
            <Fragment as chain_core::property::DeserializeFromSlice>::deserialize_from_slice(
                &mut Codec::new(bytes.as_ref()),
            )
            .map_err(serde::de::Error::custom)?;

        Ok(FragmentDef(fragment))
    }
}

impl Serialize for FragmentDef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_fragment(&self.0, serializer)
    }
}

impl<'de> DeserializeAs<'de, Fragment> for FragmentDef {
    fn deserialize_as<D>(deserializer: D) -> Result<Fragment, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::deserialize(deserializer)
    }
}

impl SerializeAs<Fragment> for FragmentDef {
    fn serialize_as<S>(source: &Fragment, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_fragment(source, serializer)
    }
}

fn serialize_fragment<S>(fragment: &Fragment, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let bytes = <Fragment as chain_core::property::Serialize>::serialize_as_vec(fragment)
        .map_err(serde::ser::Error::custom)?;

    if serializer.is_human_readable() {
        serializer.serialize_str(&hex::encode(&bytes))
    } else {
        serializer.serialize_bytes(&bytes)
    }
}
