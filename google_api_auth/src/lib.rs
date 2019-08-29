/// GetAccessToken provides an oauth2 access token. It's used by google api
/// client libraries to retrieve access tokens when making http requests. This
/// library optionally provides a variety of implementations, but users are also
/// free to implement whatever logic they want for retrieving a token.
pub trait GetAccessToken {
    type Error: ::std::error::Error;

    fn access_token(&self) -> Result<&str, Self::Error>;
}