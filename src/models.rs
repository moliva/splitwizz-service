use serde::{Deserialize, Serialize};

pub type GroupId = i32;

#[derive(Clone, Debug, PartialEq, PartialOrd, Deserialize, Serialize)]
// #[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Deserialize, Serialize)]
// #[sqlx(type_name = "membership_status", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum MembershipStatus {
    Pending,
    Joined,
    Rejected,
}

impl MembershipStatus {
    pub fn from_str(s: &str) -> Option<Self> {
        use MembershipStatus::*;

        Some(match s {
            "pending" => Pending,
            "joined" => Joined,
            "rejected" => Rejected,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Deserialize, Serialize)]
#[sqlx(type_name = "user_status", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum UserStatus {
    Invited,
    Active,
    Inactive,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct User {
    pub id: String,
    pub email: String,
    pub status: UserStatus,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Notification {
    pub group: Group,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Group {
    pub id: Option<GroupId>,
    pub name: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct MembershipUpdate {
    // pub status: MembershipStatus,
    pub status: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Membership {
    pub user: User,
    pub status: MembershipStatus,
    pub status_updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct DetailedGroup {
    // shared with group
    pub id: GroupId,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    // detailed group specific
    pub creator: User,
    pub members: Vec<Membership>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MembershipInvitation {
    pub emails: Vec<String>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Currency {
    pub id: i32,
    pub acronym: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Expense {
    pub id: i32,

    pub payer: User,
    // pub group: Group,
    pub description: String, // move to its own type
    pub currency: Currency,
    pub amount: f64,
    pub date: chrono::DateTime<chrono::Utc>,
    pub split_strategy: String, // change for a struct with the actual split

    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
