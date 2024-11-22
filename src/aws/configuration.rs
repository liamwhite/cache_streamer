use std::env;
use std::env::VarError;

pub trait Configuration: Sync + Send {
    fn access_key(&self) -> &str;
    fn secret_key(&self) -> &str;
    fn region(&self) -> &str;
    fn scheme(&self) -> &str;
    fn host(&self) -> &str;
}

pub struct EnvironmentConfiguration {
    access_key: String,
    secret_key: String,
    region: String,
    scheme: String,
    host: String,
}

impl EnvironmentConfiguration {
    pub fn new() -> Result<Self, VarError> {
        let host = format!("{}:{}", env::var("S3_HOST")?, env::var("S3_PORT")?);

        Ok(Self {
            access_key: env::var("AWS_ACCESS_KEY_ID")?,
            secret_key: env::var("AWS_SECRET_ACCESS_KEY")?,
            region: env::var("S3_REGION")?,
            scheme: env::var("S3_SCHEME")?,
            host,
        })
    }
}

impl Configuration for EnvironmentConfiguration {
    fn access_key(&self) -> &str {
        &self.access_key
    }

    fn secret_key(&self) -> &str {
        &self.secret_key
    }

    fn region(&self) -> &str {
        &self.region
    }

    fn host(&self) -> &str {
        &self.host
    }

    fn scheme(&self) -> &str {
        &self.scheme
    }
}
