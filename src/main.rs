use std::env;
use std::thread::available_parallelism;

use actix_cors::Cors;
use actix_web::rt::spawn;
use actix_web::web::Data;
use actix_web::{middleware::Logger, App, HttpServer};
use env_logger::Env;
use futures::executor::block_on;
use tokio::task::spawn_blocking;

use crate::identity::IdentityService;
use crate::queries::create_connection_pool;

use crate::redis::create_redis_pool;
use crate::workers::activity::activity_detector;
use crate::workers::sync::topics_sync;

mod auth;
mod identity;
mod models;
mod queries;
mod redis;
mod routes;
mod utils;
mod workers;

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
    let redis_pool = create_redis_pool(&connspec).await.expect("redis pool");

    let port = env::var("PORT")
        .unwrap_or_else(|_| "9000".to_string())
        .parse()
        .expect("PORT must be a number");

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

    // configure logging
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    println!("Starting server on {host}:{port}");

    let db2 = db_connection.clone();
    spawn(activity_detector());
    spawn_blocking(move || block_on(topics_sync(db2.clone())));

    let workers_num = available_parallelism().unwrap().get() * 2;

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
    .workers(workers_num)
    .bind((host, port))
    .unwrap_or_else(|_| panic!("Cannot bind to port {port}"))
    .run()
    .await
}
