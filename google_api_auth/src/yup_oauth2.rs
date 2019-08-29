use ::std::sync::Mutex;

pub fn from_authenticator<T, I, S>(auth: T, scopes: I) -> impl crate::GetAccessToken + Send + Sync
where
    T: ::yup_oauth2::GetToken + Send,
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    YupAuthenticator {
        auth: Mutex::new(auth),
        scopes: scopes.into_iter().map(Into::into).collect(),
    }
}

struct YupAuthenticator<T> {
    auth: Mutex<T>,
    scopes: Vec<String>,
}

impl<T> crate::GetAccessToken for YupAuthenticator<T>
where
    T: ::yup_oauth2::GetToken,
{
    type Error = ::yup_oauth2::RequestError;

    fn access_token(&self) -> Result<String, Self::Error> {
        let mut auth = self
            .auth
            .lock()
            .expect("thread panicked while holding lock");
        let fut = auth.token(&self.scopes);
        let mut runtime = ::tokio::runtime::Runtime::new().expect("unable to start tokio runtime");
        Ok(runtime.block_on(fut)?.access_token)
    }
}
