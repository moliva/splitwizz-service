use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::de::DeserializeOwned;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::auth::{Claims, IdentityToken};
use crate::models::User;

pub fn generate_id_token(
    user: &User,
    secret_key: &[u8],
) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = IdentityToken {
        sub: user.id.to_owned(),
        // other fields
        name: user.name.to_owned(),
        email: user.email.to_owned(),
        picture: user.picture.to_owned(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret_key),
    )
}

pub fn generate_access_token(
    user_id: &str,
    email: &str,
    secret_key: &[u8],
) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3600; // 1 hour expiration

    let claims = Claims {
        sub: user_id.to_owned(),
        exp: expiration as usize,
        // other fields
        // name: user.name.to_owned(),
        email: email.to_owned(),
        // picture: user.picture.to_owned(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret_key),
    )
}

pub fn verify_jwt<T: DeserializeOwned>(
    token: &str,
    secret_key: &[u8],
) -> Result<T, jsonwebtoken::errors::Error> {
    // Create validation rules (enable expiration check)
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true; // Enable expiration check
    validation.leeway = 60; // 1 minute leeway

    let decoded = decode::<T>(token, &DecodingKey::from_secret(secret_key), &validation)?;

    Ok(decoded.claims)
}

pub fn generate_refresh_token(
    user_id: &str,
    email: &str,
    secret_key: &[u8],
) -> Result<(String, u64), jsonwebtoken::errors::Error> {
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 604800; // 7 days

    let claims = Claims {
        sub: user_id.to_owned(),
        exp: expiration as usize,
        email: email.to_owned(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret_key),
    )
    .map(|token| (token, expiration))
}
