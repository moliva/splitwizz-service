use actix_web::rt::spawn;
use actix_web::web::Query;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Result};
use google_jwt_verify::IdPayload;
use uuid::Uuid;

use ::auth::auth::{AuthData, LoginData};

use crate::models::{User, UserStatus};
use crate::queries::{self, DbPool};
use crate::redis::{publish_topic, RedisPool};

#[get("/auth")]
async fn auth(
    req: HttpRequest,
    Query(auth_data): Query<AuthData>,
    pool: web::Data<DbPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse> {
    let pool_ = pool.clone();
    let handle_user = |payload: IdPayload| async move {
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

        user.id = queries::upsert_user(&user, &pool_)
            .await
            .map_err(|e| {
                eprintln!("{}", e);
                HttpResponse::InternalServerError().finish()
            })
            .expect("user insert");

        let redis = redis.get_ref();
        spawn(publish_topic(redis.clone(), "auth.login".to_owned(), email));

        user
    };

    let handle_refresh_token = |user: User,
                                device_id: String,
                                refresh_token: String,
                                user_agent: String,
                                expiration: u64| async move {
        queries::persist_refresh_token(
            &user,
            &refresh_token,
            &user_agent,
            &device_id,
            expiration,
            &pool,
        )
        .await
        .expect("persist refresh token");
    };

    ::auth::handlers::auth(req, auth_data, handle_user, handle_refresh_token).await
}

#[get("/login")]
async fn login(request: HttpRequest, Query(login_data): Query<LoginData>) -> Result<HttpResponse> {
    ::auth::handlers::login(request, login_data).await
}

#[get("/logout")]
async fn logout() -> HttpResponse {
    ::auth::handlers::logout().await
}

#[post("/refresh")]
async fn refresh(req: HttpRequest, pool: web::Data<DbPool>) -> HttpResponse {
    let validate_refresh_token = |user_id: String, device_id: String, refresh_token: String| async move {
        queries::validate_refresh_token(&refresh_token, &user_id, &device_id, &pool)
            .await
            .expect("validate refresh token")
    };

    ::auth::handlers::refresh(req, validate_refresh_token).await
}
