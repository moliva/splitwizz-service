use actix_web::{
    error::ErrorInternalServerError, get, post, put, web, Error, HttpResponse, Result,
};

use crate::identity::Identity;
use crate::models;
use crate::queries::DbPool;

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

// *****************************************************************************************************
// *************** HTTP Utils ***************
// *****************************************************************************************************

fn handle_unknown_error(e: sqlx::Error) -> actix_web::Error {
    eprintln!("{}", e);
    ErrorInternalServerError("internal saraza")
}
