use std::collections::{HashMap, HashSet};
use std::env;

use redis::{Commands, PubSub};

use crate::queries::{find_groups, DbPool};

pub async fn topics_sync(pool: DbPool) {
    println!("SYNC DETECTOR STARTING");
    let connspec = env::var("REDIS_URI").expect("REDIS_URI");
    let redis_client = redis::Client::open(connspec).expect("redis client connected successfully");
    let mut connection = redis_client.get_connection().expect("redis connection");

    let mut connection2 = redis_client.get_connection().expect("redis conn");
    let mut pubsub = connection2.as_pubsub();

    pubsub
        .subscribe("activity.login")
        .expect("login subscribe success");

    pubsub
        .subscribe("activity.logout")
        .expect("logout subscribe success");

    let mut user_to_topics = HashMap::<String, HashSet<String>>::new();
    let mut topic_to_users = HashMap::<String, HashSet<String>>::new();

    // read stream
    while let Ok(msg) = pubsub.get_message() {
        let channel = msg.get_channel_name();
        let payload: String = msg.get_payload().expect("payload");

        match channel {
            "activity.login" => {
                // query, save and subscribe to all topics for the given user
                let groups = find_groups(&payload, &pool).await.expect("groups");
                let mut new_topics = Vec::default();
                for group in groups {
                    new_topics.push(format!("groups.{}.*", group.id.unwrap()));
                }
                new_topics.push(format!("users.{}.*", &payload));

                add_topics(
                    &mut pubsub,
                    &mut topic_to_users,
                    &mut user_to_topics,
                    &payload,
                    new_topics,
                );
            }
            "activity.logout" => {
                // understand from which topics to unsubscribe and do it
            }
            topic if topic.starts_with("groups.") || topic.starts_with("users.") => {
                if topic.ends_with(".joined") {
                    // update topics for current user

                    // reading this topic format!("users.{}.groups.{}.joined", email, group_id)
                    let mut tuqui = topic.split('.');
                    let group_id = {
                        for token in &mut tuqui {
                            if token == "groups" {
                                break;
                            }
                        }
                        tuqui.next().unwrap()
                    };

                    let mut new_topics = Vec::default();
                    new_topics.push(format!("groups.{}.*", group_id));

                    add_topics(
                        &mut pubsub,
                        &mut topic_to_users,
                        &mut user_to_topics,
                        &payload,
                        new_topics,
                    );
                }

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

fn add_topics(
    pubsub: &mut PubSub,
    topic_to_users: &mut HashMap<String, HashSet<String>>,
    user_to_topics: &mut HashMap<String, HashSet<String>>,
    user: &String,
    new_topics: Vec<String>,
) {
    for topic in new_topics.iter() {
        pubsub.psubscribe(topic).expect("psubscribe user");

        if let Some(users) = topic_to_users.get_mut(topic) {
            users.insert(user.clone());
        } else {
            let mut users = HashSet::new();
            users.insert(user.clone());
            topic_to_users.insert(topic.clone(), users);
        }
    }

    if let Some(topics) = user_to_topics.get_mut(user) {
        topics.extend(new_topics);
    } else {
        let mut topics = HashSet::new();
        topics.extend(new_topics);
        user_to_topics.insert(user.clone(), topics);
    }
}
