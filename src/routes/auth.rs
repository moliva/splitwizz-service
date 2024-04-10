use std::env;
use std::time::Duration;

use actix_web::rt::spawn;
use actix_web::web::Query;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use awc::{self, Client};
use google_jwt_verify::Client as GoogleClient;
use redis::Commands;
use uuid::Uuid;

use crate::auth::{AuthData, TokenForm, TokenResponse};
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

        HttpResponse::Found()
            .append_header((
                "location",
                format!(
                    "{}?login_success={}",
                    env::var("WEB_URI").expect("web uri not provided"),
                    token
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
    let mut redis = redis.get().expect("pooled conn");

    redis
        .publish::<String, String, ()>(topic, payload)
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
