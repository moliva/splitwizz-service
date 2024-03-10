use std::str;

use actix_web::{
    delete, error::ErrorInternalServerError, get, post, put, web, Error, HttpResponse, Result,
};
use serde::{Deserialize, Serialize};

use crate::identity::Identity;
use crate::models;
use crate::queries::DbPool;

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

// *****************************************************************************************************
// *************** HTTP Utils ***************
// *****************************************************************************************************

fn handle_unknown_error(e: sqlx::Error) -> actix_web::Error {
    eprintln!("{}", e);
    ErrorInternalServerError("internal saraza")
}
