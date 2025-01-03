use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;

use actix_web::delete;
use actix_web::rt::spawn;
use actix_web::{
    error::ErrorInternalServerError, get, post, put, web, Error, HttpResponse, Result,
};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};

use ::auth::identity::Identity;

use crate::models::{self, Balance, SplitStrategy};
use crate::queries::DbPool;
use crate::redis::{publish_topic, RedisPool};

const _15_SECONDS: f64 = 15f64;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Hash)]
#[serde(rename_all(serialize = "snake_case", deserialize = "snake_case"))]
#[serde(tag = "kind")]
enum Event {
    Group { id: models::GroupId, field: String },
    Notification { id: i32 },
}

#[get("/currencies")]
pub async fn fetch_currencies(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let currencies = crate::queries::find_currencies(&pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(&currencies))
}

#[get("/sync")]
pub async fn sync(identity: Identity, redis: web::Data<RedisPool>) -> Result<HttpResponse, Error> {
    let email = identity.claims().email;

    let mut redis = redis.get().await.expect("pooled conn");
    redis
        .publish::<&str, &str, ()>("sync", &email)
        .await
        .expect("publish sync");

    let event = redis
        .brpop::<String, Option<(String, String)>>(format!("events.{}", email), _15_SECONDS)
        .await
        .expect("brpop events");

    if event.is_none() {
        return Ok(HttpResponse::Ok().json(Vec::<Event>::default()));
    }

    let mut events = vec![event.unwrap().1];

    let more = redis
        .rpop::<String, Vec<String>>(format!("events.{}", email), NonZeroUsize::new(99usize))
        .await
        .expect("rpop events");
    events.extend(more);

    let mut set = HashSet::new();
    for event in events {
        if event.starts_with("groups.") {
            let segments = event.split('.');
            let mut segments = segments.skip(1);

            let new = Event::Group {
                id: segments
                    .next()
                    .expect("group id")
                    .parse()
                    .expect("deserialize"),
                field: segments.next().expect("field").to_owned(),
            };
            set.insert(new);
        } else if event.starts_with("users.") {
            let new = Event::Notification { id: 0 }; // TODO - nevermind for now - moliva - 2024/04/10
            set.insert(new);
        }
        // TODO - we might be getting some other stuff from the rpop (the channel name) - moliva - 2024/04/10
    }

    Ok(HttpResponse::Ok().json(&set))
}

#[get("/notifications")]
pub async fn fetch_notifications(
    identity: Identity,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.claims().email;

    let notifications = crate::queries::find_notifications(&email, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(&notifications))
}

#[get("/groups")]
pub async fn fetch_groups(
    identity: Identity,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.claims().email;

    let groups = crate::queries::find_groups(&email, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(&groups))
}

#[post("/groups")]
pub async fn create_group(
    identity: Identity,
    group: web::Json<models::Group>,
    pool: web::Data<DbPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.claims().email;
    let web::Json(group) = group;

    let group_id = crate::queries::create_group(&email, group, &pool)
        .await
        .map_err(handle_unknown_error)?;

    let redis = redis.as_ref().clone();

    spawn(publish_topic(
        redis.clone(),
        format!("groups.{}.config", group_id),
        email.clone(),
    ));

    spawn(publish_topic(
        redis,
        format!("users.{}.groups.{}.joined", email, group_id),
        email,
    ));

    Ok(HttpResponse::Ok().json(()))
}

#[put("/groups/{group_id}")]
pub async fn edit_group(
    identity: Identity,
    path: web::Path<models::GroupId>,
    group: web::Json<models::Group>,
    redis: web::Data<RedisPool>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.claims().email;
    let web::Json(group) = group;
    let group_id = path.into_inner();

    crate::queries::update_group(&email, group_id, group, &pool)
        .await
        .map_err(handle_unknown_error)?;

    let redis = redis.as_ref();
    spawn(publish_topic(
        redis.clone(),
        format!("groups.{}.config", group_id),
        email.clone(),
    ));

    Ok(HttpResponse::Ok().json(()))
}

#[get("/groups/{group_id}")]
pub async fn fetch_detailed_group(
    identity: Identity,
    path: web::Path<models::GroupId>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.claims().email;
    let group_id = path.into_inner();

    let group = crate::queries::find_group(&email, group_id, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(&group))
}

#[put("/notifications")]
pub async fn update_notifications(
    notifications_update: web::Json<models::NotificationsUpdate>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    crate::queries::update_notifications(notifications_update.0, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(()))
}

#[put("/notifications/{notification_id}")]
pub async fn update_notification(
    path: web::Path<i32>,
    notification_update: web::Json<models::NotificationUpdate>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let notification_id = path.into_inner();

    crate::queries::update_notification(notification_id, notification_update.0, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(()))
}

#[put("/groups/{group_id}/memberships")]
pub async fn update_membership(
    identity: Identity,
    group_id: web::Path<models::GroupId>,
    membership_invitation: web::Json<models::MembershipUpdate>,
    redis: web::Data<RedisPool>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.claims().email;
    let group_id = group_id.into_inner();

    let web::Json(models::MembershipUpdate { status }) = membership_invitation;

    crate::queries::update_membership(&email, status, group_id, &pool)
        .await
        .map_err(handle_unknown_error)?;

    let redis = redis.as_ref();
    spawn(publish_topic(
        redis.clone(),
        format!("groups.{}.members.{}", group_id, email),
        email.clone(),
    ));

    spawn(publish_topic(
        redis.clone(),
        format!("users.{}.groups.{}.joined", email, group_id),
        email,
    ));

    Ok(HttpResponse::Ok().json(()))
}

#[post("/groups/{group_id}/memberships")]
pub async fn create_memberships(
    identity: Identity,
    group_id: web::Path<i32>,
    membership_invitation: web::Json<models::MembershipInvitation>,
    pool: web::Data<DbPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.claims().email;
    let group_id = group_id.into_inner();

    let web::Json(models::MembershipInvitation { emails }) = membership_invitation;

    crate::queries::create_membership_invites(&email, &emails, group_id, &pool)
        .await
        .map_err(handle_unknown_error)?;

    let redis = redis.as_ref();
    for invite in emails {
        spawn(publish_topic(
            redis.clone(),
            format!("users.{}.notifications", invite),
            email.clone(),
        ));
    }

    Ok(HttpResponse::Ok().json(()))
}

#[delete("/groups/{group_id}/expenses/{expense_id}")]
pub async fn delete_expense(
    identity: Identity,
    path: web::Path<(models::GroupId, i32)>,
    redis: web::Data<RedisPool>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.claims().email;
    let (group_id, expense_id) = path.into_inner();

    // TODO - check that current user is joined in group - moliva - 2024/03/21

    crate::queries::delete_expense(&email, group_id, expense_id, &pool)
        .await
        .map_err(handle_unknown_error)?;

    let redis = redis.as_ref();
    spawn(publish_topic(
        redis.clone(),
        format!("groups.{}.expenses.{}", group_id, email),
        email,
    ));

    Ok(HttpResponse::Ok().json(()))
}

#[post("/groups/{group_id}/expenses")]
pub async fn create_expense(
    identity: Identity,
    group_id: web::Path<i32>,
    body: web::Json<models::Expense>,
    pool: web::Data<DbPool>,
    redis: web::Data<RedisPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.claims().email;
    let group_id = group_id.into_inner();

    // TODO - check that current user is joined in group - moliva - 2024/03/21

    let web::Json(expense) = body;

    let split_strategy = expense.split_strategy.clone();

    let expense_id = crate::queries::create_expense(&email, group_id, expense, &pool)
        .await
        .map_err(handle_unknown_error)?;

    let redis = redis.as_ref();
    spawn(publish_topic(
        redis.clone(),
        format!("groups.{}.expenses.{}", group_id, expense_id),
        email.clone(),
    ));

    if let SplitStrategy::Payment { payer, recipient } = split_strategy {
        spawn(lookup_and_publish(
            pool.clone(),
            redis.clone(),
            payer,
            recipient,
            email.clone(),
        ));
    }

    Ok(HttpResponse::Ok().json(()))
}

#[get("/groups/{group_id}/balances")]
pub async fn fetch_balances(
    identity: Identity,
    group_id: web::Path<i32>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.claims().email;
    let group_id = group_id.into_inner();

    // TODO - check that current user is joined in group - moliva - 2024/03/21

    let expenses = crate::queries::find_expenses(&email, group_id, &pool)
        .await
        .map_err(handle_unknown_error)?;

    let memberships = crate::queries::find_memberships(group_id, &pool)
        .await
        .map_err(handle_unknown_error)?;

    let mut balances =
        HashMap::<models::UserId, models::Balance>::from_iter(memberships.into_iter().map(|m| {
            (
                m.user_id.clone(),
                Balance {
                    user_id: m.user_id,
                    total: HashMap::default(),
                    owes: HashMap::default(),
                },
            )
        }));

    // TODO - simplified balances - moliva - 2024/03/22

    for expense in expenses {
        match expense.split_strategy {
            models::SplitStrategy::Equally {
                payer,
                split_between,
            } => {
                let roman = expense.amount / split_between.len() as f64;

                for ower in split_between {
                    if ower == payer {
                        // nothing to do here
                        continue;
                    }

                    // add bill to ower in relation to payer
                    balances.entry(ower.clone()).and_modify(|balance| {
                        balance
                            .total
                            .entry(expense.currency_id)
                            .and_modify(|a| *a += roman)
                            .or_insert(roman);

                        balance
                            .owes
                            .entry(payer.clone())
                            .and_modify(|debts| {
                                debts
                                    .entry(expense.currency_id)
                                    .and_modify(|a| *a += roman)
                                    .or_insert(roman);
                            })
                            .or_insert_with(|| {
                                let mut debts = HashMap::default();
                                debts.insert(expense.currency_id, roman);
                                debts
                            });
                    });

                    // decrease bill from payer in realtion to ower
                    balances.entry(payer.clone()).and_modify(|balance| {
                        balance
                            .total
                            .entry(expense.currency_id)
                            .and_modify(|a| *a -= roman)
                            .or_insert(-roman);

                        balance
                            .owes
                            .entry(ower.clone())
                            .and_modify(|debts| {
                                debts
                                    .entry(expense.currency_id)
                                    .and_modify(|a| *a -= roman)
                                    .or_insert(-roman);
                            })
                            .or_insert_with(|| {
                                let mut debts = HashMap::default();
                                debts.insert(expense.currency_id, -roman);
                                debts
                            });
                    });
                }
            }
            models::SplitStrategy::Payment { payer, recipient } => {
                balances.entry(payer.clone()).and_modify(|balance| {
                    balance
                        .total
                        .entry(expense.currency_id)
                        .and_modify(|a| *a -= expense.amount)
                        .or_insert(-expense.amount);

                    balance
                        .owes
                        .entry(recipient.clone())
                        .and_modify(|debts| {
                            debts
                                .entry(expense.currency_id)
                                .and_modify(|a| *a -= expense.amount)
                                .or_insert(-expense.amount);
                        })
                        .or_insert_with(|| {
                            let mut debts = HashMap::default();
                            debts.insert(expense.currency_id, -expense.amount);
                            debts
                        });
                });

                balances.entry(recipient.clone()).and_modify(|balance| {
                    balance
                        .total
                        .entry(expense.currency_id)
                        .and_modify(|a| *a += expense.amount)
                        .or_insert(expense.amount);

                    balance
                        .owes
                        .entry(payer.clone())
                        .and_modify(|debts| {
                            debts
                                .entry(expense.currency_id)
                                .and_modify(|a| *a += expense.amount)
                                .or_insert(expense.amount);
                        })
                        .or_insert_with(|| {
                            let mut debts = HashMap::default();
                            debts.insert(expense.currency_id, expense.amount);
                            debts
                        });
                });
            }
        }
    }

    Ok(HttpResponse::Ok().json(balances.values().collect::<Vec<_>>()))
}

#[get("/groups/{group_id}/expenses")]
pub async fn fetch_expenses(
    identity: Identity,
    group_id: web::Path<i32>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.claims().email;
    let group_id = group_id.into_inner();

    // TODO - check that current user is joined in group - moliva - 2024/03/21

    let expenses = crate::queries::find_expenses(&email, group_id, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(&expenses))
}

// *****************************************************************************************************
// *************** Topic utils ***************
// *****************************************************************************************************

async fn lookup_and_publish(
    pool: web::Data<DbPool>,
    redis: RedisPool,
    payer: String,
    recipient: String,
    email: String,
) {
    let pool: &DbPool = &pool;
    let r = sqlx::query!(
        r#"
           SELECT email
           FROM users
           WHERE email != $1 AND (id = $2 OR id = $3)
           LIMIT 1
         "#,
        email,
        payer,
        recipient
    )
    .fetch_one(pool)
    .await
    .expect("id of other");

    let notif_email = r.email;

    spawn(publish_topic(
        redis,
        format!("users.{}.notifications", notif_email),
        email.clone(),
    ));
}

// *****************************************************************************************************
// *************** HTTP Utils ***************
// *****************************************************************************************************

fn handle_unknown_redis_error(e: redis::RedisError) -> actix_web::Error {
    let error = format!("redis error:\n{}", e);
    eprintln!("{}", &error);
    ErrorInternalServerError(error)
}

fn handle_unknown_error(e: sqlx::Error) -> actix_web::Error {
    let error = format!("db error:\n{}", e);
    eprintln!("{}", &error);
    ErrorInternalServerError(error)
}
