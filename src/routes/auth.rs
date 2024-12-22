use std::env;
use std::time::Duration;

use actix_web::rt::spawn;
use actix_web::web::Query;
use actix_web::{get, post, web, HttpRequest, HttpResponse, HttpResponseBuilder, Result};
use awc::{self, Client};
use google_jwt_verify::Client as GoogleClient;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AuthData, Claims, TokenForm, TokenResponse};
use crate::jwt::{generate_id_token, generate_jwt, generate_refresh_token, verify_jwt};
use crate::models::{User, UserStatus};
use crate::queries::{upsert_user, DbPool};
use crate::redis::RedisPool;
use crate::utils::gen_random_string;

#[get("/auth")]
async fn auth(
    _req: HttpRequest,
    auth_data: Query<AuthData>,
    pool: web::Data<DbPool>,
    redis: web::Data<RedisPool>,
) -> HttpResponse {
    if let Some(code) = auth_data.code.as_ref() {
        let client = Client::builder().timeout(Duration::from_secs(60)).finish();

        let mut response = client
            .post("https://oauth2.googleapis.com/token")
            .insert_header(("Content-Type", "application/x-www-form-urlencoded"))
            .insert_header(("Accept", "application/json"))
            .timeout(Duration::from_secs(60))
            .send_form(&TokenForm {
                code: code.to_owned(),
                client_id: env::var("CLIENT_ID").expect("client id not provided"),
                client_secret: env::var("CLIENT_SECRET").expect("client secret not provided"),
                redirect_uri: format!(
                    "{}/auth",
                    env::var("SELF_URI").expect("self uri not provided")
                ),
                grant_type: "authorization_code".to_owned(),
            })
            .await
            .unwrap();

        let json = response.json::<TokenResponse>().await.unwrap();

        let TokenResponse { id_token, .. } = json;

        let client_id = env::var("CLIENT_ID").expect("client id env var set");

        let token = id_token.expect("id_token missing");

        let client = GoogleClient::new(&client_id);
        let decoded_token = client
            .verify_id_token_async(&token)
            .await
            .expect("valid token");
        let payload = decoded_token.get_payload();

        let email = payload.get_email();
        let name = Some(payload.get_name());
        let picture = Some(payload.get_picture_url());

        let user = User {
            id: Uuid::new_v4().to_string(),
            status: UserStatus::Active,
            name,
            email: email.clone(),
            picture,
            created_at: None,
            updated_at: None,
        };

        upsert_user(&user, &pool)
            .await
            .map_err(|e| {
                eprintln!("{}", e);
                HttpResponse::InternalServerError().finish()
            })
            .expect("user insert");

        spawn(publish_topic(redis, "auth.login".to_owned(), email));

        let refresh_secret_key = env::var("REFRESH_SECRET").expect("refresh secret not provided");
        let refresh_secret_key = refresh_secret_key.as_bytes();

        let secret_key = env::var("JWT_SECRET").expect("jwt secret not provided");
        let secret_key = secret_key.as_bytes();

        let access_jwt = generate_jwt(&user.id, &user.email, secret_key).expect("jwt generation");
        let refresh_jwt = generate_refresh_token(&user.id, &user.email, refresh_secret_key)
            .expect("refresh generation");
        // TODO(miguel): add refresh token to table - 2024/12/22
        // make sure all other refresh tokens are revoked

        let id_token = generate_id_token(&user, secret_key).expect("id token");

        HttpResponse::Found()
            .append_header((
                "set-cookie",
                cookie("refresh_token", refresh_jwt, 604800), // 7d
            ))
            .append_header((
                "set-cookie",
                cookie("access_token", access_jwt, 604800), // 7d
            ))
            .append_header((
                "location",
                format!(
                    "{}?login_success={}",
                    env::var("WEB_URI").expect("web uri not provided"),
                    id_token
                ),
            ))
            .finish()
    } else {
        HttpResponse::Found()
            .append_header((
                "location",
                format!(
                    "{}?login_error={}",
                    env::var("WEB_URI").expect("web uri not provided"),
                    auth_data.error.as_ref().unwrap()
                ),
            ))
            .finish()
    }
}

async fn publish_topic(redis: web::Data<RedisPool>, topic: String, payload: String) {
    let mut redis = redis.get().await.expect("pooled conn");

    redis
        .publish::<String, String, ()>(topic, payload)
        .await
        .expect("published topic");
}

#[get("/login")]
async fn login() -> Result<HttpResponse> {
    Ok(HttpResponse::Found()
        .append_header((
            "location",
            format!(
                "https://accounts.google.com/o/oauth2/v2/auth?\
            scope=openid profile email&\
            access_type=offline&\
            include_granted_scopes=true&\
            response_type=code&\
            state={}&\
            redirect_uri={}/auth&\
            client_id={}",
                gen_random_string(),
                env::var("SELF_URI").expect("self uri not provided"),
                env::var("CLIENT_ID").expect("client id not provided")
            ),
        ))
        .finish())
}

#[get("/logout")]
async fn logout() -> Result<HttpResponse> {
    Ok(HttpResponse::Found()
        .append_header(("set-cookie", remove_cookie("access_token")))
        .append_header(("set-cookie", remove_cookie("refresh_token")))
        .append_header((
            "location",
            env::var("WEB_URI").expect("web uri not provided"),
        ))
        .finish())
}

#[derive(Debug, Serialize)]
struct RefreshResponse {
    access_token: String,
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[post("/refresh")]
async fn refresh(req: HttpRequest, pool: web::Data<DbPool>) -> HttpResponse {
    let refresh_token = req
        .cookie("refresh_token")
        .expect("refresh token not provided");
    let refresh_token = refresh_token.value();

    let refresh_secret_key = env::var("REFRESH_SECRET").expect("refresh secret not provided");
    let refresh_secret_key = refresh_secret_key.as_bytes();

    let secret_key = env::var("JWT_SECRET").expect("jwt secret not provided");
    let secret_key = secret_key.as_bytes();

    // Validate refresh token
    let decoded = verify_jwt::<Claims>(refresh_token, refresh_secret_key);

    // TODO(miguel): check it's not revoked - 2024/12/22
    // not expired

    match decoded {
        Ok(decoded_token) => {
            let user_id = decoded_token.sub;
            let email = decoded_token.email;

            // Generate new access token
            let access_jwt = generate_jwt(&user_id, &email, secret_key).unwrap();

            HttpResponse::Ok()
                .append_header((
                    "set-cookie",
                    cookie("access_token", access_jwt, 604800), // 7d
                ))
                .json(())
        }
        Err(_) => HttpResponse::Unauthorized()
            .append_header(("set-cookie", remove_cookie("access_token")))
            .append_header(("set-cookie", remove_cookie("refresh_token")))
            .finish(),
    }
}

fn remove_cookie(name: &str) -> String {
    format!("{}=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=/", name)
}

fn cookie(name: &str, value: String, max_age_seconds: u64) -> String {
    format!(
        "{}={}; HttpOnly; Secure; SameSite=Strict; Max-Age={}",
        name, value, max_age_seconds,
    )
}
