use std::str;

use actix_web::{
    delete, error::ErrorInternalServerError, get, post, put, web, Error, HttpResponse, Result,
};
use serde::{Deserialize, Serialize};

use crate::identity::Identity;
use crate::models;
use crate::queries::DbPool;


// *****************************************************************************************************
// *************** HTTP Utils ***************
// *****************************************************************************************************

fn handle_unknown_error(e: sqlx::Error) -> actix_web::Error {
    eprintln!("{}", e);
    ErrorInternalServerError("internal saraza")
}
