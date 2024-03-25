use std::collections::HashMap;

use actix_web::{
    error::ErrorInternalServerError, get, post, put, web, Error, HttpResponse, Result,
};

use crate::identity::Identity;
use crate::models::{self, Balance};
use crate::queries::DbPool;

#[get("/currencies")]
pub async fn fetch_currencies(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let currencies = crate::queries::find_currencies(&pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(&currencies))
}

#[get("/notifications")]
pub async fn fetch_notifications(
    identity: Identity,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.identity().unwrap().email;

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
    let email = identity.identity().unwrap().email;

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
) -> Result<HttpResponse, Error> {
    let email = identity.identity().unwrap().email;

    crate::queries::create_group(&email, &group, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(()))
}

#[get("/groups/{group_id}")]
pub async fn fetch_detailed_group(
    identity: Identity,
    path: web::Path<models::GroupId>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.identity().unwrap().email;
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
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.identity().unwrap().email;
    let group_id = group_id.into_inner();

    let web::Json(models::MembershipUpdate { status }) = membership_invitation;

    crate::queries::update_membership(&email, status, group_id, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(()))
}

#[post("/groups/{group_id}/memberships")]
pub async fn create_memberships(
    identity: Identity,
    group_id: web::Path<i32>,
    membership_invitation: web::Json<models::MembershipInvitation>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.identity().unwrap().email;
    let group_id = group_id.into_inner();

    let web::Json(models::MembershipInvitation { emails }) = membership_invitation;

    crate::queries::create_membership_invites(&email, &emails, group_id, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(()))
}

#[post("/groups/{group_id}/expenses")]
pub async fn create_expense(
    identity: Identity,
    group_id: web::Path<i32>,
    body: web::Json<models::Expense>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.identity().unwrap().email;
    let group_id = group_id.into_inner();

    // TODO - check that current user is joined in group - moliva - 2024/03/21

    let web::Json(expense) = body;

    crate::queries::create_expense(&email, group_id, expense, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(()))
}

#[get("/groups/{group_id}/balances")]
pub async fn fetch_balances(
    identity: Identity,
    group_id: web::Path<i32>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.identity().unwrap().email;
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

    Ok(HttpResponse::Ok().json(&balances.values().collect::<Vec<_>>()))
}

#[get("/groups/{group_id}/expenses")]
pub async fn fetch_expenses(
    identity: Identity,
    group_id: web::Path<i32>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.identity().unwrap().email;
    let group_id = group_id.into_inner();

    // TODO - check that current user is joined in group - moliva - 2024/03/21

    let expenses = crate::queries::find_expenses(&email, group_id, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(&expenses))
}

// *****************************************************************************************************
// *************** HTTP Utils ***************
// *****************************************************************************************************

fn handle_unknown_error(e: sqlx::Error) -> actix_web::Error {
    eprintln!("{}", e);
    ErrorInternalServerError("internal saraza")
}
