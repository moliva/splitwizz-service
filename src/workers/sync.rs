use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::web::Data;
use actix_web::{middleware::Logger, App, HttpServer};
use env_logger::Env;
use futures::lock::Mutex;
use redis::{Commands, ToRedisArgs};

pub fn sync_thread() {
    let connspec = env::var("REDIS_URI").expect("REDIS_URI");
    let redis_connection = redis::Client::open(connspec).expect("redis connected successfully");
    let mut con = redis_connection.get_connection().expect("get connection");
    let mut pubsub = con.as_pubsub();

    // let mut users_subjects = HashMap::new();

    pubsub.subscribe("login").expect("login subscribe success");
    pubsub.subscribe("logout").expect("login subscribe success");
    eprintln!("SUBSCRIBED TO LOGIN EVENTS");

    loop {
        let msg = pubsub.get_message().expect("get message login");
        let channel = msg.get_channel_name();

        let payload: String = msg.get_payload().expect("login payload");

        eprintln!("RECEIVED {} {:?}", channel, payload);
    }
}
