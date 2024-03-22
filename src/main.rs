use std::{env, str};

use actix_cors::Cors;
use actix_web::web::Data;
use actix_web::{middleware::Logger, App, HttpServer};
use env_logger::Env;

use crate::identity::IdentityService;
use crate::queries::create_connection_pool;

mod auth;
mod identity;
mod models;
mod queries;
mod routes;
mod utils;

// TODO - use this to discover all the services instead of hardcoding them
const _GOOGLE_DISCOVERY_ENDPOINT: &str =
    "https://accounts.google.com/.well-known/openid-configuration";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Get the port number to listen on.
    env::set_var("RUST_LOG", "actix_web=debug");
    dotenvy::dotenv().ok();

    // set up database connection pool
    let connspec = env::var("DATABASE_URL").expect("DATABASE_URL");
    println!("Database {connspec}");

    let db_connection = create_connection_pool(&connspec).await.unwrap();

    let port = env::var("PORT")
        .unwrap_or_else(|_| "9000".to_string())
        .parse()
        .expect("PORT must be a number");

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

    // configure logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    println!("Starting server on {host}:{port}");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(
                Cors::default()
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                    .supports_credentials()
                    .allow_any_header()
                    .allow_any_origin()
                    .max_age(3600),
            )
            .wrap(IdentityService)
            .app_data(Data::new(db_connection.clone()))
            .service(routes::status::status)
            .service(routes::auth::auth)
            .service(routes::auth::login)
            .service(routes::groups::create_group)
            .service(routes::groups::fetch_groups)
            .service(routes::groups::fetch_detailed_group)
            .service(routes::groups::create_memberships)
            .service(routes::groups::update_membership)
            .service(routes::groups::fetch_notifications)
            .service(routes::groups::fetch_currencies)
            .service(routes::groups::create_expense)
            .service(routes::groups::fetch_expenses)
            .service(routes::groups::fetch_balances)
    })
    .bind((host, port))
    .unwrap_or_else(|_| panic!("Cannot bind to port {port}"))
    .run()
    .await
}
