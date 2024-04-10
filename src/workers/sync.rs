use std::collections::{HashMap, HashSet};
use std::env;

use futures::StreamExt;
use redis::Commands;

use crate::queries::{find_groups, DbPool};

pub async fn topics_sync(pool: DbPool) {
    println!("SYNC DETECTOR STARTING");
    let connspec = env::var("REDIS_URI").expect("REDIS_URI");
    let redis_client = redis::Client::open(connspec).expect("redis client connected successfully");
    let mut connection = redis_client.get_connection().expect("redis connection");

    let mut pubsub = redis_client.get_async_pubsub().await.expect("redis conn");

    pubsub
        .subscribe("activity.login")
        .await
        .expect("login subscribe success");

    pubsub
        .subscribe("activity.logout")
        .await
        .expect("logout subscribe success");

    // TODO - users to topics map and topic to users map - moliva - 2024/04/09
    let mut user_to_topics = HashMap::<String, HashSet<String>>::new();
    let mut topic_to_users = HashMap::<String, HashSet<String>>::new();

    let mut new_topics = Vec::<String>::default();
    let mut topic_user: Option<String> = None;

    loop {
        // subscribe to any new topics
        if let Some(user) = topic_user.take() {
            for topic in new_topics.iter() {
                eprintln!("SUBSCRIBE TO USER TOPIC {}", &topic);
                pubsub.psubscribe(topic).await.expect("psubscribe user");

                if let Some(users) = topic_to_users.get_mut(topic) {
                    users.insert(user.clone());
                } else {
                    let mut users = HashSet::new();
                    users.insert(user.clone());
                    topic_to_users.insert(topic.clone(), users);
                }
            }

            if let Some(topics) = user_to_topics.get_mut(&user) {
                topics.extend(new_topics.into_iter());
            } else {
                let mut topics = HashSet::new();
                topics.extend(new_topics.into_iter());
                user_to_topics.insert(user.clone(), topics);
            }

            new_topics = Vec::default();
        }

        // read stream
        let mut stream = pubsub.on_message();

        while let Some(msg) = stream.next().await {
            let channel = msg.get_channel_name();
            let payload: String = msg.get_payload().expect("payload");

            eprintln!("RECEIVED {} {:?}", channel, payload);

            match channel {
                "activity.login" => {
                    // query, save and subscribe to all topics for the given user
                    let groups = find_groups(&payload, &pool).await.expect("groups");
                    for group in groups {
                        new_topics.push(format!("groups.{}.*", group.id.unwrap()));
                    }

                    new_topics.push(format!("users.{}.*", &payload));

                    topic_user.replace(payload);

                    break; // break read stream loop to subscribe to new topics
                }
                "activity.logout" => {
                    // understand from which topics to unsubscribe and do it
                }
                // TODO - subscribe to all topics and publish them in each user queue - moliva - 2024/04/09
                topic if topic.starts_with("groups.") || topic.starts_with("users.") => {
                    let mut prefix = topic.split('.');
                    let prefix = format!("{}.{}", prefix.next().unwrap(), prefix.next().unwrap());

                    let found = topic_to_users.iter().find(|(t, _)| t.starts_with(&prefix));
                    if let Some((_, users)) = found {
                        for user in users {
                            if user != &payload {
                                // only send to user if not the author of the event
                                connection
                                    .rpush::<String, &str, ()>(format!("events.{}", user), topic)
                                    .expect("publish group topic");
                            }
                        }
                    }
                }
                _ => panic!("unknown topic `{}`", channel),
            }
        }
    }
}
