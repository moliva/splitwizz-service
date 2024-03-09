use actix_web::{get, Error, HttpResponse, Result};

#[get("/status")]
pub async fn status() -> Result<HttpResponse, Error> {
    let status = "everything working!";
    Ok(HttpResponse::Ok().json(&status))
}
