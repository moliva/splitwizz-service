use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AuthData {
    pub code: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct TokenForm {
    pub code: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub grant_type: String,
}

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub id_token: Option<String>,
    pub access_token: Option<String>,
    pub expires_in: Option<i64>,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub refresh_token: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct IdentityToken {
    pub iss: String,
    pub azp: String,
    pub aud: String,
    pub sub: String,
    pub at_hash: String,
    pub name: String,
    pub email: String,
    pub picture: String,
    pub given_name: String,
    pub family_name: String,
    pub locale: String,
    pub iat: i32,
    pub exp: i32,
}
