// Bytes in google apis are represented as urlsafe base64 encoded strings.
// This defines a Bytes type that is a simple wrapper around a Vec<u8> used
// internally to handle byte fields in google apis.
#[allow(dead_code)]
mod bytes {
    use radix64::URL_SAFE as BASE64_CFG;

    #[derive(Debug,Clone,Eq,PartialEq,Ord,PartialOrd,Hash)]
    pub(crate) Bytes(Vec<u8>);

    impl ::std::convert::From<Vec<u8>> for Bytes {
        fn from(x: Vec<u8>) -> Bytes {
            Bytes(x)
        }
    }

    impl ::std::fmt::Display for Bytes {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> ::std::fmt::Result<()> {
            ::radix64::Display::New(&self.0).fmt(f)
        }
    }

    impl ::serde::Serialize for Bytes {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let encoded = BASE64_CFG.encode(&self.0);
            encoded.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Bytes {
        fn deserialize<D>(deserializer: D) -> Result<Bytes, D::Error>
        where
            D: Deserializer<'de>,
        {
            let encoded = String::deserialize(deserializer)?;
            let decoded = BASE64_CFG.decode(&encoded).map_err(|_| ::serde::de::Error::custom("invalid base64 input"))?;
            Ok(Bytes(decoded))
        }
    }
}