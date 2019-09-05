// Bytes in google apis are represented as urlsafe base64 encoded strings.
// This defines a Bytes type that is a simple wrapper around a Vec<u8> used
// internally to handle byte fields in google apis.
use radix64::URL_SAFE as BASE64_CFG;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Bytes(pub Vec<u8>);

impl ::std::convert::From<Vec<u8>> for Bytes {
    fn from(x: Vec<u8>) -> Bytes {
        Bytes(x)
    }
}

impl fmt::Display for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        ::radix64::Display::new(BASE64_CFG, &self.0).fmt(f)
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = BASE64_CFG.encode(&self.0);
        encoded.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Bytes, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        let decoded = BASE64_CFG
            .decode(&encoded)
            .map_err(|_| ::serde::de::Error::custom("invalid base64 input"))?;
        Ok(Bytes(decoded))
    }
}
