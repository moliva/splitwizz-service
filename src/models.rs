use serde::{Deserialize, Serialize};

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub picture: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Group {
    pub id: Option<i32>,
    pub name: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}
