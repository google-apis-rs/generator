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

impl<T> ::std::fmt::Debug for YupAuthenticator<T> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "{}", "YupAuthenticator{..}")
    }
}

impl<T> crate::GetAccessToken for YupAuthenticator<T>
where
    T: ::yup_oauth2::GetToken,
{
    fn access_token(&self) -> Result<String, Box<dyn ::std::error::Error>> {
        let mut auth = self
            .auth
            .lock()
            .expect("thread panicked while holding lock");
        let fut = auth.token(&self.scopes);
        let mut runtime = ::tokio::runtime::Runtime::new().expect("unable to start tokio runtime");
        Ok(runtime.block_on(fut)?.access_token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GetAccessToken;
    use hyper;
    use std::path::PathBuf;
    use yup_oauth2 as oauth2;

    #[test]
    fn it_works() {
        let client = hyper::Client::new();
        let inf = oauth2::InstalledFlow::new(
            client.clone(),
            yup_oauth2::DefaultFlowDelegate,
            oauth2::ApplicationSecret::default(),
            yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect(8081),
        );

        let auth = oauth2::Authenticator::new_disk(
            client,
            inf,
            oauth2::DefaultAuthenticatorDelegate,
            PathBuf::from("./").join("token.json").to_string_lossy(),
        )
        .expect("create a new statically known client");
        let auth = from_authenticator(auth, vec!["foo", "bar"]);

        fn this_should_work<T: GetAccessToken>(_x: T) {};
        this_should_work(auth);
    }
}
