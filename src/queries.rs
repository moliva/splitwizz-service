use sqlx::{postgres::PgPoolOptions, PgPool};

use crate::{models, routes::notes::NoteDto};

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

pub async fn ranked_tags(email: &str, pool: &DbPool) -> Result<Vec<String>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT UNNEST(n.tags)
         FROM notes n, users u
         WHERE n.user_id = u.id AND u.email = $1
         GROUP BY UNNEST(n.tags) ORDER BY COUNT(1) DESC",
        email,
    )
    .fetch_all(pool)
    .await?;

    Ok(result.into_iter().map(|r| r.unnest.unwrap()).collect())
}

pub async fn find_note(
    email: &str,
    note_id: i32,
    pool: &DbPool,
) -> Result<models::Note, sqlx::Error> {
    sqlx::query_as!(
        models::Note,
        "SELECT n.*
         FROM users u, notes n
         WHERE n.user_id = u.id AND u.email = $1
         AND n.id = $2",
        email,
        note_id,
    )
    .fetch_one(pool)
    .await
}

pub async fn find_notes(email: &str, pool: &DbPool) -> Result<Vec<models::Note>, sqlx::Error> {
    sqlx::query_as!(
        models::Note,
        "SELECT n.*
         FROM users u, notes n
         WHERE n.user_id = u.id AND u.email = $1
         ORDER BY n.id",
        email
    )
    .fetch_all(pool)
    .await
}

pub async fn create_note(email: &str, note: &NoteDto, pool: &DbPool) -> Result<(), sqlx::Error> {
    let content = serde_json::to_value(&note.content).expect("content to be serializable");

    sqlx::query!(
        "INSERT INTO notes (name, color, tags, content, user_id) 
        SELECT $1, $2, $3, $4, u.id
        FROM users u
        WHERE u.email = $5 LIMIT 1",
        note.name,
        note.color,
        &note.tags,
        content,
        email
    )
    .execute(pool)
    .await?;

    // TODO - get the id from the new note - moliva - 2023/10/09
    Ok(())
}

pub async fn update_note(
    email: &str,
    note_id: &i32,
    note: &NoteDto,
    pool: &DbPool,
) -> Result<(), sqlx::Error> {
    let content = serde_json::to_value(&note.content).expect("content to be serializable");

    sqlx::query!(
        "UPDATE notes 
         SET name = $1, color=$2, tags = $3, content = $4
         WHERE id = $5
         AND user_id = (SELECT id FROM users WHERE email = $6 LIMIT 1)",
        note.name,
        note.color,
        &note.tags,
        content,
        note_id,
        email,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_note(email: &str, note_id: &i32, pool: &DbPool) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM notes
         WHERE id = $1
         AND user_id = (SELECT id FROM users WHERE email = $2 LIMIT 1)",
        note_id,
        email
    )
    .execute(pool)
    .await?;

    Ok(())
}
