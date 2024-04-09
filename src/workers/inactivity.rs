use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use futures::lock::Mutex;
use futures::StreamExt;
use redis::Commands;

pub async fn inactivity_detector() {
    eprintln!("INACTIVITY DETECTOR STARTING");
    let connspec = env::var("REDIS_URI").expect("REDIS_URI");
    let redis_client = redis::Client::open(connspec).expect("redis client connected successfully");
    let mut connection = redis_client.get_connection().expect("redis connection");

    let mut pubsub = redis_client.get_async_pubsub().await.expect("redis conn");

    let users_updates = Arc::new(Mutex::new(HashMap::new()));

    pubsub
        .subscribe("login")
        .await
        .expect("login subscribe success");

    pubsub
        .subscribe("sync")
        .await
        .expect("sync subscribe success");

    eprintln!("INACTIVITY::::: SUBSCRIBED TO EVENTS");

    let mut interval = tokio::time::interval(Duration::from_secs(30));

    let mut stream = pubsub.on_message();

    loop {
        tokio::select! {
          _ = interval.tick() => {
            for (user, time) in users_updates.lock().await.iter() {
                eprintln!("TIME {} {:?}", user, time);
                let diff = Utc::now() - time;

                if diff.num_seconds() > 20 {
                   // user has not synced for more than 60s, they are logged out
                   connection.publish::<&str, &String, ()>("logout", user).expect("publish logout");
                   eprintln!("LOGOUT {}", user);
                }
            }
          },
          next = stream.next() => {
             if let Some(msg) = next {
                 let channel = msg.get_channel_name();
                 let user: String = msg.get_payload().expect("login payload");

                 eprintln!("RECEIVED {} {}", channel, user);
                 eprintln!(
                     "LAST TIME {} {:?}",
                     user,
                     users_updates.lock().await.get(&user)
                 );

                 let now = Utc::now();

                 users_updates.lock().await.insert(user, now);
             }
          }
        }
    }
}
