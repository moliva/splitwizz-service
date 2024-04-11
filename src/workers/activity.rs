use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use futures::lock::Mutex;
use futures::StreamExt;
use redis::Commands;

pub async fn activity_detector() {
    println!("INACTIVITY DETECTOR STARTING");
    let connspec = env::var("REDIS_URI").expect("REDIS_URI");
    let redis_client = redis::Client::open(connspec).expect("redis client connected successfully");
    let mut connection = redis_client.get_connection().expect("redis connection");

    let mut pubsub = redis_client.get_async_pubsub().await.expect("redis conn");

    let users_updates = Arc::new(Mutex::new(HashMap::<String, DateTime<Utc>>::new()));

    pubsub
        .subscribe("auth.login")
        .await
        .expect("login subscribe success");

    pubsub
        .subscribe("sync")
        .await
        .expect("sync subscribe success");

    let mut interval = tokio::time::interval(Duration::from_secs(30));

    let mut stream = pubsub.on_message();

    loop {
        tokio::select! {
          _ = interval.tick() => {
            let mut deletion = Vec::default();

            let mut users_updates = users_updates.lock().await;

            let now = Utc::now();
            for (user, time) in users_updates.iter() {
                let diff = now - time;

                if diff.num_seconds() > 60 {
                   // user has not synced for more than 60s, they are logged out
                   deletion.push(user.clone());
                }
            }

            for user in deletion {
                users_updates.remove(&user);
                connection.publish::<&str, &String, ()>("activity.logout", &user).expect("publish logout");
            }
          },
          next = stream.next() => {
             if let Some(msg) = next {
                 let _channel = msg.get_channel_name();
                 let user: String = msg.get_payload().expect("login payload");

                 let now = Utc::now();

                 let result = users_updates.lock().await.insert(user.clone(), now);

                 if result.is_none () {
                   connection.publish::<&str, String, ()>("activity.login", user).expect("publish login");
                 }
             }
          }
        }
    }
}
