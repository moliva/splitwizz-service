use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub type GroupId = i32;
pub type UserId = String;

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
    pub id: UserId,
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
pub struct InternalMembership {
    pub user_id: UserId,
    // pub status: MembershipStatus,
    // pub status_updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Membership {
    pub user: User,
    pub status: MembershipStatus,
    pub status_updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct Balance {
    pub user_id: UserId,
    pub total: HashMap<CurrencyId, f64>,
    pub owes: HashMap<UserId, HashMap<CurrencyId, f64>>,
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

pub type CurrencyId = i32;

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Currency {
    pub id: CurrencyId,
    pub acronym: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Expense {
    pub id: Option<i32>,
    pub group_id: Option<GroupId>,

    pub description: String,
    pub currency_id: i32,
    pub amount: f64,
    pub date: chrono::DateTime<chrono::Utc>,
    pub split_strategy: SplitStrategy,

    pub created_by_id: Option<UserId>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,

    pub updated_by_id: Option<UserId>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(tag = "kind")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum SplitStrategy {
    Equally {
        payer: UserId,
        split_between: Vec<UserId>,
    },
}
