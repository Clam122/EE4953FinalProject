use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use crate::schema::users;

#[derive(Queryable, Selectable, Serialize, Debug, Clone)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub public_key: String,
    pub hash_verify: String,
    pub visible_to_public: i32,  // SQLite has no bool; 0/1
}

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub email: String,
    pub public_key: String,
    pub hash_verify: String,
    pub visible_to_public: i32,
}

#[derive(Deserialize)]
pub struct CreateUserPayload {
    pub email: String,
    pub public_key: String,
    pub hash_verify: String,
}

#[derive(Deserialize)]
pub struct UpdateUserPayload {
    pub hash_verify: String,
    pub email: Option<String>,
    pub public_key: Option<String>,
    pub new_hash_verify: Option<String>,
    pub visible_to_public: Option<bool>,
}

#[derive(Deserialize)]
pub struct DeleteUserPayload {
    pub hash_verify: String,
}