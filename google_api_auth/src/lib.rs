/// GetAccessToken provides an oauth2 access token. It's used by google api
/// client libraries to retrieve access tokens when making http requests. This
/// library optionally provides a variety of implementations, but users are also
/// free to implement whatever logic they want for retrieving a token.
#[async_trait::async_trait]
pub trait GetAccessToken: ::std::fmt::Debug + Send + Sync {
    async fn access_token(&self) -> Result<String, Box<dyn ::std::error::Error + Send + Sync>>;
}

impl<T> From<T> for Box<dyn GetAccessToken>
where
    T: GetAccessToken + 'static,
{
    fn from(x: T) -> Self {
        Box::new(x)
    }
}

#[cfg(feature = "with-yup-oauth2")]
pub mod yup_oauth2;
