use axum::extract::State;
use axum::Form;
use axum::response::Html;
use serde::Deserialize;
use sqlx::{FromRow, Pool, QueryBuilder, Sqlite};
use crate::post_job::PostJobForm;

#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq)]
pub struct PostTestListToJobForm {
    task_id: i64,
    tests_to_add: String,
    targets: String,
}

#[derive(FromRow)]
struct RowID {
    id: i64,
}

pub async fn add_test_list_to_job(State(db): State<Pool<Sqlite>>, form: Form<PostTestListToJobForm>) -> Html<String> {
    let tests_to_add = form.tests_to_add
        .split_whitespace()
        .collect::<Vec<_>>();

    let targets = form.targets
        .split_whitespace()
        .collect::<Vec<_>>();

    let mut tx = db
        .begin()
        .await
        .expect("Error when starting a sql transaction");


    for target in &targets {
        let target_id = match *target {
            "qemu" => 1,
            "real_hardware" => 2,
            x => {
                return Html(format!("Error, unknown target. Only 'qemu' and 'real_hardware' are accepted. Got [{x_str}]",
                                    x_str = html_escape::encode_safe(x)));
            }
        };
        for test in &tests_to_add {
            let query_res = sqlx::query_as::<_, RowID>(
                "INSERT INTO test_run(test_name, target_id, task_id, status)
            VALUES($1, $2, $3, 1) -- 1 == pending
            RETURNING id;",
            )
                .bind(test)
                .bind(target_id)
                .bind(form.task_id)
                .fetch_one(&mut *tx)
                .await;

            let Ok(RowID { id: test_run_id }) = query_res else {
                // no need to manually call rollback. It is done automatically on Drop
                return Html(format!(
                    "Error occurred while inserting a test_run into database: {:?}",
                    query_res.err()
                ));
            };
        }
    }

    tx.commit()
        .await
        .expect("error occurred when trying to commit a transaction");


    Html(String::from("OK"))
}
