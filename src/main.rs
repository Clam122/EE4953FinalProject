mod db;
mod models;
mod schema;
mod routes;

use axum::{
    routing::get,
    Router,
};
use routes::users::*;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let conn = db::establish_connection();
    let state: AppState = Arc::new(Mutex::new(conn));

    let app = Router::new()
        .route("/index",                    get(index))
        .route("/mirror",                   get(send_mirror))
        .route("/get_mirror/{url}",         get(get_mirror))
        .route("/users",                    get(get_users).post(create_user))
        .route("/users/{id}",               get(get_user_by_id).delete(delete_user).put(update_user))
        .route("/getuserbyemail/{email}",   get(get_user_by_email))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5000").await.unwrap();
    println!("Listening on http://localhost:5000");
    axum::serve(listener, app).await.unwrap();
}