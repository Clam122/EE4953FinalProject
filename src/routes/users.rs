use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use diesel::prelude::*;
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tracing::info;
use crate::models::{CreateUserPayload, DeleteUserPayload, NewUser, UpdateUserPayload, User};
use crate::schema::users::dsl::*;

pub type AppState = Arc<Mutex<SqliteConnection>>;

// GET /index
pub async fn index() -> &'static str {
    info!("GET /index");
    "This server is a proof-of-concept version of a distributed keyserver using RSA encryption\n\
     If you are a server trying to mirror the data on this server, GET on /mirror for all the data this server exposes publicly!"
}

// GET /mirror
pub async fn send_mirror(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    info!("GET /mirror — serving public users to mirror requester");
    let conn = &mut *state.lock().unwrap();
    let all_users: Vec<User> = users.select(User::as_select()).load(conn).unwrap_or_default();

    let public: Vec<Value> = all_users
        .iter()
        .filter(|u| u.visible_to_public != 0)
        .map(|u| json!({ "id": u.id, "email": u.email, "public_key": u.public_key }))
        .collect();

    info!("GET /mirror — sending {} public users", public.len());
    (StatusCode::OK, Json(json!(public)))
}

// GET /get_mirror/:url
pub async fn get_mirror(
    Path(url): Path<String>,
    State(state): State<AppState>,
) -> (StatusCode, Json<Value>) {
    info!("GET /get_mirror/{} — attempting to pull mirror", url);
    let client = Client::new();
    let remote: Vec<Value> = match client.get(format!("http://{}/mirror", url)).send().await {
        Ok(r) => match r.json().await {
            Ok(j) => j,
            Err(e) => {
                info!("GET /get_mirror/{} — failed to parse response: {}", url, e);
                return (StatusCode::BAD_GATEWAY, Json(json!({ "error": e.to_string() })));
            }
        },
        Err(e) => {
            info!("GET /get_mirror/{} — failed to reach remote: {}", url, e);
            return (StatusCode::BAD_GATEWAY, Json(json!({ "error": e.to_string() })));
        }
    };

    let conn = &mut *state.lock().unwrap();
    let (mut added, mut skipped) = (0, 0);

    for u in &remote {
        let r_email = u["email"].as_str().unwrap_or_default();
        let r_key = u["public_key"].as_str().unwrap_or_default();

        let email_exists = users.filter(email.eq(r_email)).first::<User>(conn).is_ok();
        let key_exists = users.filter(public_key.eq(r_key)).first::<User>(conn).is_ok();

        if email_exists || key_exists {
            info!("Mirror skipping duplicate: email={}", r_email);
            skipped += 1;
            continue;
        }

        let new = NewUser {
            email: r_email.to_string(),
            public_key: r_key.to_string(),
            hash_verify: "mirrored".to_string(),
            visible_to_public: 1,
        };
        diesel::insert_into(users).values(&new).execute(conn).ok();
        info!("Mirror added user: email={}", r_email);
        added += 1;
    }

    info!("GET /get_mirror/{} — complete: added={} skipped={}", url, added, skipped);
    (StatusCode::OK, Json(json!({ "message": "Mirror complete", "added": added, "skipped": skipped })))
}

// GET /users
pub async fn get_users(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    info!("GET /users");
    let conn = &mut *state.lock().unwrap();
    let all: Vec<User> = users.select(User::as_select()).load(conn).unwrap_or_default();

    let public: Vec<Value> = all
        .iter()
        .filter(|u| u.visible_to_public != 0)
        .map(|u| json!({ "id": u.id, "email": u.email, "public_key": u.public_key }))
        .collect();

    info!("GET /users — returned {} public users", public.len());
    (StatusCode::OK, Json(json!(public)))
}

// GET /users/:id
pub async fn get_user_by_id(
    Path(user_id): Path<i32>,
    State(state): State<AppState>,
) -> (StatusCode, Json<Value>) {
    info!("GET /users/{}", user_id);
    let conn = &mut *state.lock().unwrap();
    match users.filter(id.eq(user_id)).first::<User>(conn) {
        Ok(u) => {
            info!("GET /users/{} — found: email={}", user_id, u.email);
            (StatusCode::OK, Json(json!({
                "id": u.id, "email": u.email,
                "public_key": u.public_key, "visible_to_public": u.visible_to_public != 0
            })))
        }
        Err(_) => {
            info!("GET /users/{} — not found", user_id);
            (StatusCode::NOT_FOUND, Json(json!({ "error": "User not found" })))
        }
    }
}

// GET /getuserbyemail/:email
pub async fn get_user_by_email(
    Path(user_email): Path<String>,
    State(state): State<AppState>,
) -> (StatusCode, Json<Value>) {
    info!("GET /getuserbyemail/{}", user_email);
    let conn = &mut *state.lock().unwrap();
    match users.filter(email.eq(&user_email)).first::<User>(conn) {
        Ok(u) if u.visible_to_public != 0 => {
            info!("GET /getuserbyemail/{} — found: id={}", user_email, u.id);
            (StatusCode::OK, Json(json!({
                "id": u.id, "email": u.email,
                "public_key": u.public_key, "visible_to_public": true
            })))
        }
        _ => {
            info!("GET /getuserbyemail/{} — not found or not public", user_email);
            (StatusCode::NOT_FOUND, Json(json!({ "error": "User not found" })))
        }
    }
}

// POST /users
pub async fn create_user(
    State(state): State<AppState>,
    Json(body): Json<CreateUserPayload>,
) -> (StatusCode, Json<Value>) {
    info!("POST /users — attempting to create user: email={}", body.email);
    let conn = &mut *state.lock().unwrap();

    if users.filter(email.eq(&body.email)).first::<User>(conn).is_ok() {
        info!("POST /users — rejected: email already registered: {}", body.email);
        return (StatusCode::CONFLICT, Json(json!({ "error": "Email already registered" })));
    }
    if users.filter(public_key.eq(&body.public_key)).first::<User>(conn).is_ok() {
        info!("POST /users — rejected: public key already registered for email={}", body.email);
        return (StatusCode::CONFLICT, Json(json!({ "error": "Public key already registered" })));
    }

    let hashed = match hash(&body.hash_verify, DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => {
            info!("POST /users — hashing failed for email={}", body.email);
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": "Hashing failed" })));
        }
    };

    let new = NewUser {
        email: body.email.clone(),
        public_key: body.public_key.clone(),
        hash_verify: hashed,
        visible_to_public: 0,
    };

    diesel::insert_into(users).values(&new).execute(conn).unwrap();
    let created: User = users.filter(email.eq(&body.email)).first(conn).unwrap();
    info!("POST /users — user created: id={} email={}", created.id, created.email);
    (StatusCode::CREATED, Json(json!({
        "message": "User created successfully",
        "id": created.id,
        "email": created.email
    })))
}

// DELETE /users/:id
pub async fn delete_user(
    Path(user_id): Path<i32>,
    State(state): State<AppState>,
    Json(body): Json<DeleteUserPayload>,
) -> (StatusCode, Json<Value>) {
    info!("DELETE /users/{}", user_id);
    let conn = &mut *state.lock().unwrap();

    let user: User = match users.filter(id.eq(user_id)).first(conn) {
        Ok(u) => u,
        Err(_) => {
            info!("DELETE /users/{} — not found", user_id);
            return (StatusCode::NOT_FOUND, Json(json!({ "error": "User not found" })));
        }
    };

    if !verify(&body.hash_verify, &user.hash_verify).unwrap_or(false) {
        info!("DELETE /users/{} — rejected: invalid hash", user_id);
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Invalid hash" })));
    }

    diesel::delete(users.filter(id.eq(user_id))).execute(conn).unwrap();
    info!("DELETE /users/{} — user deleted: email={}", user_id, user.email);
    (StatusCode::OK, Json(json!({ "message": "User deleted successfully" })))
}

// PUT /users/:id
pub async fn update_user(
    Path(user_id): Path<i32>,
    State(state): State<AppState>,
    Json(body): Json<UpdateUserPayload>,
) -> (StatusCode, Json<Value>) {
    info!("PUT /users/{}", user_id);
    let conn = &mut *state.lock().unwrap();

    let user: User = match users.filter(id.eq(user_id)).first(conn) {
        Ok(u) => u,
        Err(_) => {
            info!("PUT /users/{} — not found", user_id);
            return (StatusCode::NOT_FOUND, Json(json!({ "error": "User not found" })));
        }
    };

    if !verify(&body.hash_verify, &user.hash_verify).unwrap_or(false) {
        info!("PUT /users/{} — rejected: invalid hash", user_id);
        return (StatusCode::UNAUTHORIZED, Json(json!({ "error": "Invalid hash" })));
    }

    let mut updated_fields: Vec<&str> = vec![];

    if let Some(ref new_email) = body.email {
        let conflict = users.filter(email.eq(new_email)).filter(id.ne(user_id)).first::<User>(conn).is_ok();
        if conflict {
            info!("PUT /users/{} — rejected: email already in use: {}", user_id, new_email);
            return (StatusCode::CONFLICT, Json(json!({ "error": "Email already in use" })));
        }
        diesel::update(users.filter(id.eq(user_id))).set(email.eq(new_email)).execute(conn).unwrap();
        updated_fields.push("email");
    }

    if let Some(ref new_key) = body.public_key {
        let conflict = users.filter(public_key.eq(new_key)).filter(id.ne(user_id)).first::<User>(conn).is_ok();
        if conflict {
            info!("PUT /users/{} — rejected: public key already in use", user_id);
            return (StatusCode::CONFLICT, Json(json!({ "error": "Public key already in use" })));
        }
        diesel::update(users.filter(id.eq(user_id))).set(public_key.eq(new_key)).execute(conn).unwrap();
        updated_fields.push("public_key");
    }

    if let Some(ref new_hash) = body.new_hash_verify {
        let hashed = hash(new_hash, DEFAULT_COST).unwrap();
        diesel::update(users.filter(id.eq(user_id))).set(hash_verify.eq(hashed)).execute(conn).unwrap();
        updated_fields.push("hash_verify");
    }

    if let Some(vis) = body.visible_to_public {
        diesel::update(users.filter(id.eq(user_id))).set(visible_to_public.eq(vis as i32)).execute(conn).unwrap();
        updated_fields.push("visible_to_public");
    }

    if updated_fields.is_empty() {
        info!("PUT /users/{} — rejected: no valid fields provided", user_id);
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "No valid fields provided to update" })));
    }

    let u: User = users.filter(id.eq(user_id)).first(conn).unwrap();
    info!("PUT /users/{} — updated fields: [{}]", user_id, updated_fields.join(", "));
    (StatusCode::OK, Json(json!({
        "message": "User updated successfully",
        "id": u.id, "email": u.email, "public_key": u.public_key
    })))
}