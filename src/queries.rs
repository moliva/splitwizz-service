use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::models;

pub type DbPool = PgPool;

pub async fn create_connection_pool(connspec: &str) -> Result<DbPool, sqlx::Error> {
    let pool = PgPoolOptions::new().min_connections(5).max_connections(10);

    pool.connect(connspec).await
}

pub async fn upsert_user(user: &models::User, pool: &DbPool) -> Result<bool, sqlx::Error> {
    sqlx::query!(
        "INSERT INTO users (id, name, email, picture) 
         VALUES ($1, $2, $3, $4)
         ON CONFLICT DO NOTHING",
        user.id,
        user.name,
        user.email,
        user.picture
    )
    .execute(pool)
    .await
    .map(|r| r.rows_affected() > 0u64)
}

pub async fn find_user_by_email(
    email: &str,
    pool: &DbPool,
) -> Result<Option<models::User>, sqlx::Error> {
    sqlx::query_as!(
        models::User,
        "SELECT *
         FROM users
         WHERE email = $1",
        email
    )
    .fetch_optional(pool)
    .await
}
