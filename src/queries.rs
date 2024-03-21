use std::collections::HashSet;

use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

use crate::models::{self, DetailedGroup, Expense, Notification};

pub type DbPool = PgPool;

pub async fn create_connection_pool(connspec: &str) -> Result<DbPool, sqlx::Error> {
    let pool = PgPoolOptions::new().min_connections(5).max_connections(10);

    pool.connect(connspec).await
}

pub async fn upsert_user(user: &models::User, pool: &DbPool) -> Result<bool, sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO users (id, email, name, picture, status)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (email) DO UPDATE
         SET name = $3,
             picture = $4,
             status = $5"#,
        user.id,
        user.email,
        user.name,
        user.picture,
        user.status.clone() as models::UserStatus,
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
        r#"SELECT id, email, status AS "status!: models::UserStatus", name, picture, created_at, updated_at
         FROM users
         WHERE email = $1"#,
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

pub async fn find_group(
    email: &str,
    group_id: models::GroupId,
    pool: &DbPool,
) -> Result<models::DetailedGroup, sqlx::Error> {
    let base_group = sqlx::query!(
        "SELECT g.id, g.name, g.created_at, g.creator_id
         FROM users u, memberships m, groups g
         WHERE g.id = $1
         AND u.email = $2 AND m.user_id = u.id AND m.status = 'joined'",
        group_id,
        email,
    )
    .fetch_one(pool)
    .await?;

    let creator = sqlx::query_as!(
        models::User,
        r#"SELECT id, email, status AS "status!: models::UserStatus", name, picture, created_at, updated_at
           FROM users 
           WHERE id = $1"#,
        base_group.creator_id
    )
    .fetch_one(pool)
    .await?;

    let memberships = sqlx::query!(
        "SELECT m.user_id, m.status, m.status_updated_at
         FROM memberships m
         WHERE m.group_id = $1
         ORDER BY m.user_id",
        group_id
    )
    .fetch_all(pool)
    .await?;

    let membership_details = sqlx::query_as!(
        models::User,
        r#"SELECT u.id, u.email, u.status AS "status!: models::UserStatus", u.name, u.picture, u.created_at, updated_at
           FROM users u
           WHERE u.id IN (SELECT m.user_id FROM memberships m WHERE m.group_id = $1)
           ORDER BY u.id"#,
        group_id
    )
    .fetch_all(pool)
    .await?;

    let members = memberships
        .into_iter()
        .zip(membership_details.into_iter())
        .map(|(m, user)| models::Membership {
            user,
            status: models::MembershipStatus::from_str(&m.status).expect("membership status"),
            status_updated_at: m.status_updated_at,
        })
        .collect::<Vec<_>>();

    Ok(DetailedGroup {
        id: base_group.id,
        name: base_group.name,
        created_at: base_group.created_at,
        creator,
        members,
    })
}

pub async fn create_group(
    email: &str,
    group: &models::Group,
    pool: &DbPool,
) -> Result<(), sqlx::Error> {
    // create group
    let result = sqlx::query!(
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
        result.id,
    )
    .execute(pool)
    .await?;

    // TODO - get the id from the new group - moliva - 2024/03/10
    Ok(())
}

pub async fn update_membership(
    email: &str,
    status: &str,
    group: models::GroupId,
    pool: &DbPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE memberships
         SET status = $3
         WHERE group_id = $2
         AND user_id = (SELECT id FROM users WHERE email = $1 LIMIT 1)
         "#,
        email,
        group,
        status,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn create_membership_invites(
    emails: &Vec<String>,
    group_id: i32,
    pool: &DbPool,
) -> Result<(), sqlx::Error> {
    let existing = sqlx::query!(
        "SELECT u.email, u.id
         FROM users u
         WHERE u.email = ANY($1)",
        emails
    )
    .fetch_all(pool)
    .await
    .expect("existing");

    let (existing_emails, mut all_ids): (HashSet<String>, Vec<String>) =
        existing.into_iter().map(|r| (r.email, r.id)).unzip();

    let (emails_it, ids_it): (Vec<String>, Vec<String>) = emails
        .iter()
        .filter(|&e| !existing_emails.contains(e))
        .cloned()
        .map(|e| (e, Uuid::new_v4().to_string()))
        .unzip();

    sqlx::query!(
        r#"INSERT INTO users (email, status, id)
         SELECT e, $3, i
         FROM UNNEST($1::text[], $2::text[]) as t (e, i)
         "#,
        emails_it.as_slice(),
        ids_it.as_slice(),
        models::UserStatus::Invited as models::UserStatus
    )
    .execute(pool)
    .await
    .expect("new");

    all_ids.extend(ids_it.into_iter());

    sqlx::query!(
        r#"INSERT INTO memberships (user_id, group_id) 
           SELECT i, $2
           FROM UNNEST($1::text[]) as t (i)
         "#,
        all_ids.as_slice(),
        group_id,
    )
    .execute(pool)
    .await
    .expect("insert all");

    Ok(())
}

pub async fn find_notifications(
    email: &str,
    pool: &DbPool,
) -> Result<Vec<models::Notification>, sqlx::Error> {
    let records = sqlx::query!(
        "SELECT g.id, g.name, g.created_at, m.status_updated_at
         FROM users u, memberships m, groups g
         WHERE m.user_id = u.id AND u.email = $1 AND g.id = m.group_id
         AND m.status = 'pending'
         ORDER BY m.status_updated_at",
        email
    )
    .fetch_all(pool)
    .await?;

    let notifications = records
        .into_iter()
        .map(|r| Notification {
            group: models::Group {
                id: Some(r.id),
                name: r.name,
                created_at: Some(r.created_at),
            },
            updated_at: r.status_updated_at,
        })
        .collect();

    Ok(notifications)
}

pub async fn find_currencies(pool: &DbPool) -> Result<Vec<models::Currency>, sqlx::Error> {
    sqlx::query_as!(models::Currency, "SELECT * FROM currencies ORDER BY id")
        .fetch_all(pool)
        .await
}

pub async fn create_expense(
    email: &str,
    group_id: i32,
    expense: Expense,
    pool: &DbPool,
) -> Result<(), sqlx::Error> {
    let serialized_value = serde_json::to_value(&expense.split_strategy).expect("serialized value");
    sqlx::query!(
        r#"INSERT INTO expenses (created_by_id, updated_by_id, group_id, description, currency_id, amount, date, split_strategy)
           SELECT                u.id,          u.id,          $2,       $3,          $4,          $5,     $6,   $7
           FROM users u
           WHERE u.email = $1
           LIMIT 1
         "#,
        email,
        group_id,
        expense.description,
        expense.currency_id,
        expense.amount,
        expense.date,
        serialized_value,
    )
    .execute(pool)
    .await?;

    Ok(())
}
