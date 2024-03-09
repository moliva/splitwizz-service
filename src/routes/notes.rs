use std::str;

use actix_web::{
    delete, error::ErrorInternalServerError, get, post, put, web, Error, HttpResponse, Result,
};
use serde::{Deserialize, Serialize};

use crate::identity::Identity;
use crate::models;
use crate::queries::{find_note, find_notes, ranked_tags, DbPool};

#[get("/tags")]
pub async fn fetch_ranked_tags(
    identity: Identity,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.identity().unwrap().email;

    let tags = ranked_tags(&email, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(&tags))
}

#[get("/notes/{note_id}")]
pub async fn retrieve_note(
    identity: Identity,
    path: web::Path<i32>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.identity().unwrap().email;

    let note_id = path.into_inner();

    let note = find_note(&email, note_id, &pool)
        .await
        .map_err(handle_unknown_error)?;

    let note: NoteDto = note.into();

    Ok(HttpResponse::Ok().json(&note))
}

#[get("/notes")]
pub async fn retrieve_notes(
    identity: Identity,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.identity().unwrap().email;

    let notes = find_notes(&email, &pool)
        .await
        .map_err(handle_unknown_error)?;

    let notes: Vec<NoteDto> = notes.into_iter().map(models::Note::into).collect();

    Ok(HttpResponse::Ok().json(&notes))
}

#[post("/notes")]
pub async fn create_note(
    identity: Identity,
    note: web::Json<NoteDto>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.identity().unwrap().email;

    let note = note.into_inner();
    crate::queries::create_note(&email, &note, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(()))
}

#[put("/notes/{note_id}")]
pub async fn update_note(
    path: web::Path<i32>,
    identity: Identity,
    note: web::Json<NoteDto>,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.identity().unwrap().email;

    let note_id = path.into_inner();
    let note = note.into_inner();

    crate::queries::update_note(&email, &note_id, &note, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(()))
}

#[delete("/notes/{note_id}")]
pub async fn delete_note(
    path: web::Path<i32>,
    identity: Identity,
    pool: web::Data<DbPool>,
) -> Result<HttpResponse, Error> {
    let email = identity.identity().unwrap().email;

    let note_id = path.into_inner();

    crate::queries::delete_note(&email, &note_id, &pool)
        .await
        .map_err(handle_unknown_error)?;

    Ok(HttpResponse::Ok().json(()))
}

// *****************************************************************************************************
// *************** Dto Utils ***************
// *****************************************************************************************************

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineFormat {
    pub line: String,
    pub checkbox: Option<bool>,
    pub check: Option<bool>,
    pub blur: Option<bool>,
    pub link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content(Vec<(LineFormat, Content)>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteDto {
    pub id: Option<i32>,
    pub name: String,
    pub content: Content,
    pub color: String,
    pub tags: Vec<String>,
}

impl From<models::Note> for NoteDto {
    fn from(value: models::Note) -> Self {
        Self {
            id: Some(value.id),
            name: value.name,
            content: serde_json::from_value(value.content).expect("json value to be deserialized"),
            color: value.color,
            tags: value.tags,
        }
    }
}

// *****************************************************************************************************
// *************** HTTP Utils ***************
// *****************************************************************************************************

fn handle_unknown_error(e: sqlx::Error) -> actix_web::Error {
    eprintln!("{}", e);
    ErrorInternalServerError("internal saraza")
}
