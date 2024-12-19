// sqlite part -> https://tms-dev-blog.com/rust-sqlx-basics-with-sqlite/
// axum (rust on nails) -> https://rust-on-nails.com/docs/full-stack-web/forms/
// axum part -> https://spacedimp.com/blog/using-rust-axum-postgresql-and-tokio-to-build-a-blog/
// axum tutorials -> https://github.com/tokio-rs/axum/blob/main/ECOSYSTEM.md#tutorials

// SQLX + Actix Streaming Responses -> https://old.reddit.com/r/rust/comments/1ay5e4u/sqlx_actix_streaming_responses/

#![feature(async_closure)]
#![feature(future_join)]

mod common;
mod get_build_details;
mod list_job_queue;
mod post_job;
mod request_task;
mod update_task;
mod add_test_list_to_job;
mod report_test_change;

use axum::{
    response::Html,
    routing::{get, post},
    Router,
};
use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;
use crate::list_job_queue::list_job_queue;

const DB_URL: &str = "sqlite://./ci_db.sqlite";

async fn add_job() -> Html<&'static str> {
    const ADD_JOB_PAGE: &'static str = include_str!("add_job.html");
    Html(ADD_JOB_PAGE)
}

#[tokio::main]
async fn main() {
    if !Sqlite::database_exists(DB_URL).await.unwrap_or(false) {
        println!("Creating database {}", DB_URL);
        match Sqlite::create_database(DB_URL).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    } else {
        println!("Database already exists");
    }

    let db = SqlitePool::connect(DB_URL).await.unwrap();
    let create_schema = include_str!("create_schema.sql");
    let result = sqlx::query(create_schema).execute(&db).await.unwrap();
    println!("Create user table result: {:?}", result);

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // build our application with a single route
    let app = Router::new()
        .route("/", get(list_job_queue::list_job_queue))
        .route("/list_max_id/{max_id}", get(list_job_queue::list_job_queue_with_max_id))
        .route("/list_min_id/{min_id}", get(list_job_queue::list_job_queue_with_min_id))
        .route("/build/{id}", get(get_build_details::get_build_details))
        .route("/add_job", get(add_job))
        .route("/request_task", post(request_task::request_task))
        .route("/update_task", post(update_task::update_task))
        .route("/add_job", post(post_job::post_job))
        .route("/add_test_list_to_job", post(add_test_list_to_job::add_test_list_to_job))
        .route("/report_test_change", post(report_test_change::report_test_change))
        .with_state(db)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
