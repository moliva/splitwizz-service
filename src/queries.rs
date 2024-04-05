use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

use crate::models::{self, DetailedGroup, Expense, GroupId, Notification, SplitStrategy};

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

async fn find_all_users(pool: &DbPool) -> Result<Vec<models::User>, sqlx::Error> {
    sqlx::query_as!(
        models::User,
        r#"SELECT u.id, u.name, u.email, u.picture, u.created_at, u.updated_at, u.status AS "status!: models::UserStatus"
           FROM users u
           ORDER BY u.id"#,
    )
    .fetch_all(pool)
    .await
}

async fn find_all_groups(pool: &DbPool) -> Result<Vec<models::Group>, sqlx::Error> {
    sqlx::query_as!(
        models::Group,
        "SELECT g.*
         FROM groups g
         ORDER BY g.id",
    )
    .fetch_all(pool)
    .await
}

pub async fn find_groups(email: &str, pool: &DbPool) -> Result<Vec<models::Group>, sqlx::Error> {
    sqlx::query_as!(
        models::Group,
        "SELECT g.*
         FROM users u, memberships m, groups g
         WHERE m.user_id = u.id AND u.email = $1 AND g.id = m.group_id
         AND m.status = 'joined'
         ORDER BY g.id",
        email
    )
    .fetch_all(pool)
    .await
}

pub async fn find_memberships(
    group_id: models::GroupId,
    pool: &DbPool,
) -> Result<Vec<models::InternalMembership>, sqlx::Error> {
    sqlx::query_as!(
        models::InternalMembership,
        "SELECT m.user_id, m.group_id, m.created_by_id
         FROM memberships m
         WHERE m.group_id = $1
         AND m.status = 'joined'
         ORDER BY m.user_id",
        group_id
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
        r#"SELECT m.user_id, m.status AS "status!: models::MembershipStatus", m.status_updated_at
         FROM memberships m
         WHERE m.group_id = $1
         ORDER BY m.user_id"#,
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
            status: m.status,
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
        "INSERT INTO memberships (user_id, group_id, status, created_by_id)
        SELECT u.id, $2, 'joined', u.id
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

pub async fn update_notifications(
    update: models::NotificationsUpdate,
    pool: &DbPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE notifications
         SET status = $2
         WHERE id IN (SELECT * FROM UNNEST($1::integer[]))
         "#,
        &update.ids,
        update.status as models::NotificationStatus
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_notification(
    notification_id: i32,
    update: models::NotificationUpdate,
    pool: &DbPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE notifications
         SET status = $2
         WHERE id = $1
         "#,
        notification_id,
        update.status as models::NotificationStatus
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn update_membership(
    email: &str,
    status: models::MembershipStatus,
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
        status as models::MembershipStatus,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn create_membership_invites(
    inviter: &str,
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
        r#"INSERT INTO memberships (user_id, group_id, created_by_id)
           SELECT i, $2, u.id
           FROM UNNEST($1::text[]) as t (i), users u
           WHERE u.email = $3
         "#,
        all_ids.as_slice(),
        group_id,
        inviter
    )
    .execute(pool)
    .await
    .expect("insert all");

    let invite = models::NotificationKind::Invite { group_id };
    let invite = serde_json::to_value(invite).expect("serialized value");

    sqlx::query!(
        r#"INSERT INTO notifications (user_id, data)
           SELECT i, $2
           FROM UNNEST($1::text[]) as t (i)
         "#,
        &all_ids,
        invite,
    )
    .execute(pool)
    .await
    .expect("insert notifications");

    Ok(())
}

pub async fn find_notifications(
    email: &str,
    pool: &DbPool,
) -> Result<Vec<NotificationDto>, sqlx::Error> {
    let notifications = sqlx::query_as!(
        models::Notification,
        r#"SELECT n.id, n.status AS "status!: models::NotificationStatus", n.user_id, n.data, n.status_updated_at, n.created_at
         FROM users u, notifications n
         WHERE n.user_id = u.id AND u.email = $1
         AND n.status != 'archived'
         ORDER BY n.created_at"#,
        email
    )
    .fetch_all(pool)
    .await?;

    // fetch all users
    let users = find_all_users(pool).await?;
    let users = HashMap::<models::UserId, models::User>::from_iter(
        users.into_iter().map(|u| (u.id.clone(), u)),
    );

    // fetch all groups
    let groups = find_all_groups(pool).await?;
    let groups = HashMap::<models::GroupId, models::Group>::from_iter(
        groups.into_iter().map(|g| (g.id.expect("group id"), g)),
    );

    // fetch all payments
    let payment_ids = notifications
        .iter()
        .filter_map(|n| match n.data {
            models::NotificationKind::Payment { expense_id } => Some(expense_id),
            _ => None,
        })
        .collect::<Vec<_>>();

    let payments = sqlx::query_as!(
        models::Expense,
        "SELECT e.*
         FROM expenses e, (SELECT * FROM UNNEST($1::integer[])) as t(i)
         WHERE e.id = t.i",
        &payment_ids,
    )
    .fetch_all(pool)
    .await?;
    let payments = HashMap::<i32, models::Expense>::from_iter(
        payments.into_iter().map(|e| (e.id.expect("id"), e)),
    );

    // fetch all memberships
    let group_ids = notifications
        .iter()
        .filter_map(|n| match n.data {
            models::NotificationKind::Invite { group_id } => Some(group_id),
            _ => None,
        })
        .collect::<Vec<_>>();

    let memberships = sqlx::query_as!(
        models::InternalMembership,
        "SELECT m.user_id, m.group_id, m.created_by_id
         FROM memberships m, (SELECT * FROM UNNEST($2::integer[])) as t(i), users u
         WHERE m.group_id = t.i
         AND u.email = $1 AND u.id = m.user_id",
        email,
        &group_ids,
    )
    .fetch_all(pool)
    .await?;
    let memberships = HashMap::<models::GroupId, models::InternalMembership>::from_iter(
        memberships.into_iter().map(|e| (e.group_id, e)),
    );

    let notifications = notifications
        .into_iter()
        .map(|n| NotificationDto {
            data: match n.data {
                models::NotificationKind::Invite { group_id } => {
                    let m = memberships.get(&group_id).unwrap();
                    NotificationDtoKind::Invite {
                        group: groups.get(&group_id).unwrap().clone(),
                        created_by: users.get(&m.created_by_id).unwrap().clone(),
                    }
                }
                models::NotificationKind::Payment { expense_id } => {
                    let Expense {
                        group_id,
                        currency_id,
                        amount,
                        date,
                        split_strategy,
                        created_by_id,
                        ..
                    } = payments.get(&expense_id).unwrap();

                    match split_strategy {
                        SplitStrategy::Payment { payer, recipient } => {
                            NotificationDtoKind::Payment {
                                group: groups.get(&group_id.unwrap()).unwrap().clone(),
                                currency_id: *currency_id,
                                amount: *amount,
                                date: *date,
                                payer: users.get(payer).unwrap().clone(),
                                recipient: users.get(recipient).unwrap().clone(),
                                created_by: users
                                    .get(&created_by_id.clone().unwrap().clone())
                                    .unwrap()
                                    .clone(),
                            }
                        }
                        _ => panic!("expected payment"),
                    }
                }
            },
            id: n.id,
            user_id: n.user_id,
            status: n.status,
            status_updated_at: n.status_updated_at,
            created_at: n.created_at,
        })
        .collect::<Vec<_>>();

    Ok(notifications)
}

pub async fn find_currencies(pool: &DbPool) -> Result<Vec<models::Currency>, sqlx::Error> {
    sqlx::query_as!(models::Currency, "SELECT * FROM currencies ORDER BY id")
        .fetch_all(pool)
        .await
}

pub async fn delete_expense(
    _email: &str,
    _group_id: GroupId,
    expense_id: i32,
    pool: &DbPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(r#"DELETE FROM expenses WHERE id = $1"#, expense_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn create_expense(
    email: &str,
    group_id: GroupId,
    expense: Expense,
    pool: &DbPool,
) -> Result<(), sqlx::Error> {
    let serialized_value = serde_json::to_value(&expense.split_strategy).expect("serialized value");
    let r = sqlx::query!(
        r#"INSERT INTO expenses (created_by_id, updated_by_id, group_id, description, currency_id, amount, date, split_strategy)
           SELECT                u.id,          u.id,          $2,       $3,          $4,          $5,     $6,   $7
           FROM users u
           WHERE u.email = $1
           LIMIT 1
           RETURNING id
         "#,
        email,
        group_id,
        expense.description,
        expense.currency_id,
        expense.amount,
        expense.date,
        serialized_value,
    )
    .fetch_one(pool)
    .await?;

    if let SplitStrategy::Payment { payer, recipient } = expense.split_strategy {
        let expense_id = r.id;

        let payment = models::NotificationKind::Payment { expense_id };
        let payment = serde_json::to_value(payment).expect("serialized value");

        sqlx::query!(
            r#"INSERT INTO notifications (user_id, data)
           SELECT CASE WHEN u.id = $3 THEN $4 ELSE $3 END, $2
           FROM users u
           WHERE u.email = $1
         "#,
            email,
            payment,
            payer,
            recipient
        )
        .execute(pool)
        .await
        .expect("insert notifications");
    }

    Ok(())
}

// TODO - paging and have `date` as separate to group easily - moliva - 2024/03/21
pub async fn find_expenses(
    email: &str,
    group_id: GroupId,
    pool: &DbPool,
) -> Result<Vec<models::Expense>, sqlx::Error> {
    let expenses = sqlx::query_as!(
        models::Expense,
        "SELECT * FROM expenses WHERE group_id = $1 ORDER BY date DESC",
        group_id
    )
    .fetch_all(pool)
    .await?;

    Ok(expenses)
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind")]
#[serde(rename_all(serialize = "snake_case", deserialize = "snake_case"))]
pub enum NotificationDtoKind {
    Invite {
        group: models::Group,
        created_by: models::User,
    },
    Payment {
        group: models::Group,
        currency_id: models::CurrencyId,
        amount: f64,
        date: chrono::DateTime<chrono::Utc>,
        payer: models::User,
        recipient: models::User,
        created_by: models::User,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all(serialize = "snake_case", deserialize = "snake_case"))]
pub struct NotificationDto {
    pub id: i32,
    pub user_id: models::UserId,
    pub data: NotificationDtoKind,

    pub status: models::NotificationStatus,
    pub status_updated_at: chrono::DateTime<chrono::Utc>,

    pub created_at: chrono::DateTime<chrono::Utc>,
}
