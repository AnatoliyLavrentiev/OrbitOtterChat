use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::errors::AppError;

#[derive(Clone)]
pub struct JwtConfig {
    pub issuer: String,
    pub audience: String,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessClaims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: i64,
    pub iat: i64,
}

impl JwtConfig {
    pub fn new_from_env() -> Self {
        let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET missing");

        Self {
            issuer: "rtc_backend".to_string(),
            audience: "rtc_client".to_string(),
            access_ttl: Duration::minutes(15),
            refresh_ttl: Duration::days(30),
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    pub fn make_access_claims(&self, user_id: Uuid) -> AccessClaims {
        let now = OffsetDateTime::now_utc().unix_timestamp();
        let exp = (OffsetDateTime::now_utc() + self.access_ttl).unix_timestamp();

        AccessClaims {
            sub: user_id.to_string(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            iat: now,
            exp,
        }
    }

    pub fn sign_access_token(&self, user_id: Uuid) -> Result<String, AppError> {
        let claims = self.make_access_claims(user_id);

        let mut header = Header::new(Algorithm::HS256);
        header.typ = Some("JWT".to_string());

        encode(&header, &claims, &self.encoding)
            .map_err(|e| AppError::Internal(format!("jwt encode: {e}")))
    }

    pub fn verify_access_token(&self, token: &str) -> Result<Uuid, AppError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[self.audience.as_str()]);

        let data = decode::<AccessClaims>(token, &self.decoding, &validation)
            .map_err(|_| AppError::Unauthorized)?;

        Uuid::parse_str(&data.claims.sub).map_err(|_| AppError::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::JwtConfig;
    use time::Duration;
    use uuid::Uuid;

    use jsonwebtoken::{DecodingKey, EncodingKey};

    fn test_config() -> JwtConfig {
        let secret = b"test-secret-key";
        JwtConfig {
            issuer: "rtc_backend_test".to_string(),
            audience: "rtc_client_test".to_string(),
            access_ttl: Duration::minutes(15),
            refresh_ttl: Duration::days(30),
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }

    #[test]
    fn sign_and_verify_access_token_roundtrip() {
        let cfg = test_config();
        let user_id = Uuid::new_v4();

        let token = cfg
            .sign_access_token(user_id)
            .expect("token should be signed");
        let verified = cfg
            .verify_access_token(&token)
            .expect("token should be valid");

        assert_eq!(verified, user_id);
    }

    #[test]
    fn verify_rejects_token_signed_with_another_secret() {
        let cfg_ok = test_config();

        let cfg_other = JwtConfig {
            issuer: cfg_ok.issuer.clone(),
            audience: cfg_ok.audience.clone(),
            access_ttl: cfg_ok.access_ttl,
            refresh_ttl: cfg_ok.refresh_ttl,
            encoding: EncodingKey::from_secret(b"other-secret"),
            decoding: DecodingKey::from_secret(b"other-secret"),
        };

        let token = cfg_other
            .sign_access_token(Uuid::new_v4())
            .expect("token should be signed");

        assert!(cfg_ok.verify_access_token(&token).is_err());
    }
}
