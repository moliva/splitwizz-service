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

pub async fn find_groups(email: &str, pool: &DbPool) -> Result<Vec<models::Group>, sqlx::Error> {
    sqlx::query_as!(
        models::Group,
        "SELECT g.id, g.name, g.created_at
         FROM users u, memberships m, groups g
         WHERE m.user_id = u.id AND u.email = $1 AND g.id = m.group_id
         AND m.status = 'joined'
         ORDER BY g.id",
        email
    )
    .fetch_all(pool)
    .await
}

pub async fn create_group(
    email: &str,
    group: &models::Group,
    pool: &DbPool,
) -> Result<(), sqlx::Error> {
    // create group
    let r = sqlx::query!(
        "INSERT INTO groups (name, creator_id) 
        SELECT $1, u.id
        FROM users u
        WHERE u.email = $2 LIMIT 1
        RETURNING id",
        group.name,
        email
    )
    .fetch_one(pool)
    .await?;

    // join group
    sqlx::query!(
        "INSERT INTO memberships (user_id, group_id, status)
        SELECT u.id, $2, 'joined'
        FROM users u
        WHERE u.email = $1 LIMIT 1",
        email,
        r.id,
    )
    .execute(pool)
    .await?;

    // TODO - get the id from the new group - moliva - 2024/03/10
    Ok(())
}
