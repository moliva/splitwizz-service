use actix_web::{
    error::ErrorInternalServerError, get, post, put, web, Error, HttpResponse, Result,
};

use crate::identity::Identity;
use crate::models;
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

#[put("/groups/{group_id}/memberships")]
pub async fn update_membership(
    identity: Identity,
    group_id: web::Path<i32>,
    membership_invitation: web::Json<models::MembershipUpdate>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.identity().unwrap().email;
    let group_id = group_id.into_inner();

    let web::Json(models::MembershipUpdate { status }) = membership_invitation;

    crate::queries::update_membership(&email, &status, group_id, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(()))
}

#[post("/groups/{group_id}/memberships")]
pub async fn create_memberships(
    group_id: web::Path<i32>,
    membership_invitation: web::Json<models::MembershipInvitation>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let group_id = group_id.into_inner();

    let web::Json(models::MembershipInvitation { emails }) = membership_invitation;

    crate::queries::create_membership_invites(&emails, group_id, &pool)
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
