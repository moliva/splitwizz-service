use std::env;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::web::Data;
use actix_web::{middleware::Logger, App, HttpServer};
use env_logger::Env;
use futures::lock::Mutex;
use r2d2::Pool;
use redis::Client;

use crate::identity::IdentityService;
use crate::queries::create_connection_pool;

use crate::workers::activity::activity_detector;
use crate::workers::sync::topics_sync;

mod auth;
mod identity;
mod models;
mod queries;
mod routes;
mod utils;
mod workers;

pub type RedisPool = Pool<Client>;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Get the port number to listen on.
    env::set_var("RUST_LOG", "actix_web=debug");
    dotenvy::dotenv().ok();

    // set up database connection pool
    let connspec = env::var("DATABASE_URL").expect("DATABASE_URL");
    println!("Database {connspec}");

    let db_connection = create_connection_pool(&connspec).await.unwrap();

    let connspec = env::var("REDIS_URI").expect("REDIS_URI");
    let redis_connection = redis::Client::open(connspec).expect("redis connected successfully");
    let redis_pool = r2d2::Pool::builder()
        .max_size(15)
        .build(redis_connection)
        .unwrap();

    let port = env::var("PORT")
        .unwrap_or_else(|_| "9000".to_string())
        .parse()
        .expect("PORT must be a number");

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

    // configure logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    println!("Starting server on {host}:{port}");

    actix_web::rt::spawn(activity_detector());
    actix_web::rt::spawn(topics_sync(db_connection.clone()));

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
            .app_data(Data::new(redis_pool.clone()))
            .service(routes::status::status)
            .service(routes::auth::auth)
            .service(routes::auth::login)
            .service(routes::groups::create_group)
            .service(routes::groups::edit_group)
            .service(routes::groups::fetch_groups)
            .service(routes::groups::fetch_detailed_group)
            .service(routes::groups::create_memberships)
            .service(routes::groups::update_membership)
            .service(routes::groups::update_notification)
            .service(routes::groups::update_notifications)
            .service(routes::groups::fetch_notifications)
            .service(routes::groups::fetch_currencies)
            .service(routes::groups::create_expense)
            .service(routes::groups::delete_expense)
            .service(routes::groups::fetch_expenses)
            .service(routes::groups::fetch_balances)
            .service(routes::groups::sync)
    })
    .bind((host, port))
    .unwrap_or_else(|_| panic!("Cannot bind to port {port}"))
    .run()
    .await
}
