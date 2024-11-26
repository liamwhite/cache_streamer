use std::env;
use std::env::VarError;

pub struct Configuration {
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub scheme: String,
    pub host: String,
}

impl Configuration {
    // TODO S3
    #[allow(dead_code)]
    pub fn from_env() -> Result<Self, VarError> {
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
