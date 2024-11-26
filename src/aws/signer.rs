use super::configuration::Configuration;
use chrono::{DateTime, Utc};
use hex::ToHex;
use ring::digest;
use ring::hmac;

const SERVICE: &str = "s3";
const AWS4_REQUEST: &str = "aws4_request";
const SIGNED_HEADERS: &str = "host;x-amz-content-sha256;x-amz-date";
const AWS4_HMAC_SHA256: &str = "AWS4-HMAC-SHA256";

fn iso8601_basic(timestamp: &DateTime<Utc>) -> String {
    timestamp.format("%Y%m%dT%H%M%SZ").to_string()
}

fn iso8601_short(timestamp: &DateTime<Utc>) -> String {
    timestamp.format("%Y%m%d").to_string()
}

fn hmac_sign_bytes(key_bytes: &[u8], sign_bytes: &[u8]) -> hmac::Tag {
    let key = hmac::Key::new(hmac::HMAC_SHA256, key_bytes);
    hmac::sign(&key, sign_bytes)
}

fn hex_digest(data: &[u8]) -> String {
    digest::digest(&digest::SHA256, data).encode_hex()
}

pub struct Signature {
    pub authorization: String,
    pub x_amz_date: String,
    pub x_amz_content_sha256: String,
}

pub struct Signer<'c> {
    config: &'c Configuration,
    timestamp_basic: String,
    timestamp_short: String,
}

struct Request<'a> {
    method: &'a str,
    path: &'a str,
    body_digest: String,
}

impl<'c> Signer<'c> {
    pub fn new(config: &'c Configuration) -> Self {
        let timestamp = Utc::now();

        Self {
            config,
            timestamp_basic: iso8601_basic(&timestamp),
            timestamp_short: iso8601_short(&timestamp),
        }
    }

    fn derived_signing_key(&self) -> Vec<u8> {
        let date = hmac_sign_bytes(
            format!("AWS4{}", self.config.secret_key).as_bytes(),
            self.timestamp_short.as_bytes(),
        );

        let region = hmac_sign_bytes(date.as_ref(), self.config.region.as_bytes());
        let service = hmac_sign_bytes(region.as_ref(), SERVICE.as_bytes());
        let type_ = hmac_sign_bytes(service.as_ref(), AWS4_REQUEST.as_bytes());

        type_.as_ref().into()
    }

    fn credential_scope(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.timestamp_short, self.config.region, SERVICE, AWS4_REQUEST
        )
    }

    fn hashed_canonical_request(&self, request: &Request) -> String {
        let canonical = format!(
            concat!(
                "{}\n",                       // method
                "{}\n",                       // path
                "\n",                         //
                "host: {}\n",                 // host
                "x-amz-content-sha256: {}\n", // digest
                "x-amz-date: {}\n",           // date
                "\n",                         //
                "{}\n",                       // signed headers
                "{}"                          // digest
            ),
            request.method,
            request.path,
            self.config.host,
            request.body_digest,
            self.timestamp_basic,
            SIGNED_HEADERS,
            request.body_digest
        );

        hex_digest(canonical.as_bytes())
    }

    fn string_to_sign(&self, request: &Request) -> String {
        format!(
            concat!(
                "{}\n", // AWS4-HMAC-SHA256
                "{}\n", // timestamp
                "{}\n", // credential scope
                "{}"    // canonical request
            ),
            AWS4_HMAC_SHA256,
            self.timestamp_basic,
            self.credential_scope(),
            self.hashed_canonical_request(request)
        )
    }

    fn authorization(&self, request: &Request) -> String {
        let sign_key = self.derived_signing_key();
        let sign_bytes = self.string_to_sign(request);
        let signature: String = hmac_sign_bytes(&sign_key, sign_bytes.as_bytes())
            .as_ref()
            .encode_hex();

        format!(
            concat!(
                "{}",                 // AWS4-HMAC-SHA256
                " Credential={}/{}",  // access key, credential scope
                ", SignedHeaders={}", // signed headers
                ", Signature={}"      // signature value
            ),
            AWS4_HMAC_SHA256,
            self.config.access_key,
            self.credential_scope(),
            SIGNED_HEADERS,
            signature
        )
    }

    pub fn sign_request(self, method: &str, path: &str, body: &[u8]) -> Signature {
        let request = Request {
            method,
            path,
            body_digest: hex_digest(body),
        };

        let authorization = self.authorization(&request);

        Signature {
            authorization,
            x_amz_date: self.timestamp_basic,
            x_amz_content_sha256: request.body_digest,
        }
    }
}
