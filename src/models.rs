use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub type GroupId = i32;
pub type UserId = String;
pub type CurrencyId = i32;
pub type ExpenseId = i32;

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Deserialize, Serialize)]
#[sqlx(type_name = "notification_status", rename_all = "snake_case")]
#[serde(rename_all(serialize = "snake_case", deserialize = "snake_case"))]
pub enum NotificationStatus {
    New,
    Read,
    Archived,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Deserialize, Serialize)]
#[sqlx(type_name = "membership_status", rename_all = "snake_case")]
#[serde(rename_all(serialize = "snake_case", deserialize = "snake_case"))]
pub enum MembershipStatus {
    Pending,
    Joined,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Deserialize, Serialize)]
#[sqlx(type_name = "user_status", rename_all = "snake_case")]
#[serde(rename_all(serialize = "snake_case", deserialize = "snake_case"))]
pub enum UserStatus {
    Invited,
    Active,
    Inactive,
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug, Clone)]
pub struct User {
    pub id: UserId,

    pub email: String,
    pub status: UserStatus,

    pub name: Option<String>,
    pub picture: Option<String>,

    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(tag = "kind")]
#[serde(rename_all(serialize = "snake_case", deserialize = "snake_case"))]
pub enum NotificationKind {
    Invite { group_id: GroupId },
    Payment { expense_id: ExpenseId },
}

impl From<serde_json::Value> for NotificationKind {
    fn from(value: serde_json::Value) -> Self {
        serde_json::from_value(value).expect("deserialized value")
    }
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Notification {
    pub id: i32,
    pub user_id: UserId,
    pub data: NotificationKind,

    pub status: NotificationStatus,
    pub status_updated_at: chrono::DateTime<chrono::Utc>,

    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all(serialize = "snake_case", deserialize = "snake_case"))]
pub struct BalanceConfig {
    simplified: bool,
}

impl Into<serde_json::Value> for BalanceConfig {
    fn into(self) -> serde_json::Value {
        serde_json::to_value(self).expect("serialized value")
    }
}

impl From<serde_json::Value> for BalanceConfig {
    fn from(value: serde_json::Value) -> Self {
        serde_json::from_value(value).expect("deserialized value")
    }
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Clone)]
pub struct Group {
    pub id: Option<GroupId>,

    pub name: String,
    pub creator_id: Option<UserId>,
    pub default_currency_id: CurrencyId,
    pub balance_config: BalanceConfig,

    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct DetailedGroup {
    // shared with group
    pub id: GroupId,

    pub name: String,
    pub creator_id: UserId,
    pub default_currency_id: CurrencyId,
    pub balance_config: BalanceConfig,

    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    // detailed group specific
    pub creator: User,
    pub members: Vec<Membership>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct MembershipUpdate {
    pub status: MembershipStatus,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct InternalMembership {
    pub user_id: UserId,
    pub group_id: GroupId,

    pub created_by_id: UserId,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct MembershipInvitation {
    pub emails: Vec<String>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct Currency {
    pub id: CurrencyId,
    pub acronym: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Expense {
    pub id: Option<ExpenseId>,
    pub group_id: Option<GroupId>,

    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub deleted: bool,

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
#[serde(rename_all(serialize = "snake_case", deserialize = "snake_case"))]
pub enum SplitStrategy {
    Equally {
        payer: UserId,
        split_between: Vec<UserId>,
    },
    Payment {
        payer: UserId,
        recipient: UserId,
    },
}

impl From<serde_json::Value> for SplitStrategy {
    fn from(value: serde_json::Value) -> Self {
        serde_json::from_value(value).expect("deserialized value")
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NotificationsUpdate {
    pub ids: Vec<i32>,
    pub status: NotificationStatus,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NotificationUpdate {
    pub status: NotificationStatus,
}
