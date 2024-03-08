use std::os::linux::raw::stat;
use axum::extract::State;
use axum::Form;
use axum::response::Html;
use serde::Deserialize;
use sqlx::{FromRow, Pool, Sqlite};
use crate::update_task;

#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq)]
enum Operation {
    Start,
    Finish,
    Progress,
}


#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq)]
enum Target {
    Qemu,
    RealHardware,
}

#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq)]
pub(crate) enum FinishStatus {
    Success,
    Failed,
    Timeout,
    Skipped,
}

#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq)]
pub struct ReportTestChangeForm {
    task_id: i64,
    test_name: String,
    target: Target,
    operation: Operation,
    output: Option<String>,
    status: Option<FinishStatus>,
}

#[derive(FromRow)]
struct RowID {
    id: i64,
}

pub async fn report_test_change(State(db): State<Pool<Sqlite>>, form: Form<ReportTestChangeForm>) -> Html<String> {
    println!("received form: {form:?}");

    let target_id = match form.target {
        Target::Qemu => { 1 }
        Target::RealHardware => { 2 }
    };
    match form.operation {
        Operation::Start => {
            let query_res = sqlx::query_as::<_, RowID>(
                "UPDATE test_run
            SET status = 2, -- running
                started_at = CURRENT_TIMESTAMP
            WHERE (task_id = $1) AND (test_name = $2) AND (target_id = $3)
        RETURNING id;")
                .bind(form.task_id)
                .bind(&form.test_name)
                .bind(target_id)
                .fetch_one(&db)
                .await;

            let Ok(RowID { id: updated_row_id }) = query_res else {
                // no need to manually call rollback. It is done automatically on Drop
                return Html(format!("Error failed to update a test run in the database to set it as started. Form  was {form} and err {query:?}",
                                    form = html_escape::encode_safe(format!("{form:?}").as_str()),
                                    query = query_res.err()));
            };
            Html(String::from("OK"))
        }
        Operation::Finish => {
            let Some(status) = &form.status else {
                return Html(format!("Error: test_run supposedly finished but can't extract the status code. Form was: {form}",
                                    form = html_escape::encode_safe(format!("{form:?}").as_str())));
            };
            let (status_id, ret_code) = match status {
                FinishStatus::Success => { (3, Some(i64::from(0))) }
                FinishStatus::Failed => { (4, Some(1)) }
                FinishStatus::Timeout => { (5, Some(124)) }
                FinishStatus::Skipped => { (6, None) }
            };

            let query_res = sqlx::query_as::<_, RowID>(
                "UPDATE test_run
            SET status = $4,
                ret_code = $5,
                finished_at = CURRENT_TIMESTAMP
            WHERE (task_id = $1) AND (test_name = $2) AND (target_id = $3)
        RETURNING id;")
                .bind(form.task_id)
                .bind(&form.test_name)
                .bind(target_id)
                .bind(status_id)
                .bind(ret_code)
                .fetch_one(&db)
                .await;

            let Ok(RowID { id: updated_row_id }) = query_res else {
                // no need to manually call rollback. It is done automatically on Drop
                return Html(format!("Error failed to update a test run in the database to set it as completed. Form  was {form} and err {query:?}",
                                    form = html_escape::encode_safe(format!("{form:?}").as_str()),
                                    query = query_res.err()));
            };
            Html(String::from("OK"))
        }
        Operation::Progress => {
            let Some(output) = &form.output else {
                return Html(format!("Can't change progress of a test run if no text is given. Form was: {form}",
                                    form = html_escape::encode_safe(format!("{form:?}").as_str())));
            };

            let query_res = sqlx::query_as::<_, RowID>(
                "UPDATE test_run
                     SET output = output || $4
                     WHERE (task_id = $1) AND (test_name = $2) AND (target_id = $3)
        RETURNING id;"
            )
                .bind(form.task_id)
                .bind(&form.test_name)
                .bind(target_id)
                .bind(output)
                .fetch_one(&db)
                .await;

            let Ok(RowID { id: updated_row_id }) = query_res else {
                // no need to manually call rollback. It is done automatically on Drop
                return Html(format!("Error failed to update the output of a test run in the database. Form  was {form} and err: {query:?}",
                                    form = html_escape::encode_safe(format!("{form:?}").as_str()),
                                    query = query_res.err()));
            };

            Html(String::from("OK"))
        }
    }
}
