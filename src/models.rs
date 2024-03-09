use serde::Serialize;

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub picture: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Note {
    pub user_id: String,
    pub id: i32,
    pub name: String,
    pub tags: Vec<String>,
    pub content: serde_json::Value,
    pub color: String,
}
