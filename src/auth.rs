use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct LoginData {
    pub redirect: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuthData {
    pub code: Option<String>,
    pub error: Option<String>,
    pub state: Option<String>,
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
