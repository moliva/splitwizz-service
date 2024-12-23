use std::env;
use std::time::Duration;

use actix_web::rt::spawn;
use actix_web::web::Query;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Result};
use awc::{self, Client};
use google_jwt_verify::Client as GoogleClient;
use redis::AsyncCommands;
use uuid::Uuid;

use crate::auth::{AuthData, Claims, LoginData, TokenForm, TokenResponse};
use crate::jwt::{generate_access_token, generate_id_token, generate_refresh_token, verify_jwt};
use crate::models::{User, UserStatus};
use crate::queries::{self, upsert_user, DbPool};
use crate::redis::RedisPool;

#[get("/auth")]
async fn auth(
    req: HttpRequest,
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

        let mut user = User {
            id: Uuid::new_v4().to_string(),
            status: UserStatus::Active,
            name,
            email: email.clone(),
            picture,
            created_at: None,
            updated_at: None,
        };

        user.id = upsert_user(&user, &pool)
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

        let access_jwt =
            generate_access_token(&user.id, &user.email, secret_key).expect("jwt generation");
        let (refresh_jwt, expiration) =
            generate_refresh_token(&user.id, &user.email, refresh_secret_key)
                .expect("refresh generation");

        let user_agent = req
            .headers()
            .get("User-Agent")
            .expect("user agent")
            .to_str()
            .expect("to_str");

        let state = auth_data.0.state.unwrap_or("".to_owned());
        let (device_id, redirect) = if state.contains('+') {
            let mut it = state.split('+').map(|s| s.to_owned());

            (it.next().unwrap(), it.next().unwrap())
        } else {
            (Uuid::new_v4().to_string(), state)
        };

        queries::persist_refresh_token(
            &user,
            &refresh_jwt,
            user_agent,
            &device_id,
            expiration,
            &pool,
        )
        .await
        .expect("persist refresh token");

        let id_token = generate_id_token(&user, secret_key).expect("id token");

        HttpResponse::Found()
            .append_header(("set-cookie", cookie_("device_id", device_id)))
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
                    "{}{}?login_success={}",
                    env::var("WEB_URI").expect("web uri not provided"),
                    redirect,
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
async fn login(request: HttpRequest, Query(login_data): Query<LoginData>) -> Result<HttpResponse> {
    let device_id = request.cookie("device_id");
    let redirect = login_data.redirect.unwrap_or("".to_owned());

    let state = if let Some(device_id) = device_id {
        format!("{}+{}", device_id.value(), redirect)
    } else {
        redirect
    };

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
                state,
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

#[post("/refresh")]
async fn refresh(req: HttpRequest, pool: web::Data<DbPool>) -> HttpResponse {
    let refresh_token = req.cookie("refresh_token");
    let device_id = req.cookie("device_id");
    if refresh_token.is_none() || device_id.is_none() {
        return HttpResponse::Unauthorized()
            .append_header(("set-cookie", remove_cookie("access_token")))
            .append_header(("set-cookie", remove_cookie("refresh_token")))
            .finish();
    }

    let refresh_token = refresh_token.expect("refresh_token");
    let refresh_token = refresh_token.value();

    let device_id = device_id.expect("device_id");
    let device_id = device_id.value();

    let refresh_secret_key = env::var("REFRESH_SECRET").expect("refresh secret not provided");
    let refresh_secret_key = refresh_secret_key.as_bytes();

    let secret_key = env::var("JWT_SECRET").expect("jwt secret not provided");
    let secret_key = secret_key.as_bytes();

    // Validate refresh token
    let decoded = verify_jwt::<Claims>(refresh_token, refresh_secret_key);

    match decoded {
        Ok(decoded_token) => {
            let user_id = decoded_token.sub;
            let email = decoded_token.email;

            let result = queries::validate_refresh_token(refresh_token, &user_id, device_id, &pool)
                .await
                .expect("check refresh token");
            if !result {
                return HttpResponse::Unauthorized()
                    .append_header(("set-cookie", remove_cookie("access_token")))
                    .append_header(("set-cookie", remove_cookie("refresh_token")))
                    .finish();
            }

            // Generate new access token
            let access_jwt = generate_access_token(&user_id, &email, secret_key).unwrap();

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

fn cookie_(name: &str, value: String) -> String {
    format!("{}={}; HttpOnly; Secure; SameSite=Strict", name, value,)
}

fn cookie(name: &str, value: String, max_age_seconds: u64) -> String {
    format!(
        "{}={}; HttpOnly; Secure; SameSite=Strict; Max-Age={}",
        name, value, max_age_seconds,
    )
}
