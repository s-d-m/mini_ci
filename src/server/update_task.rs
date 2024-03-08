use axum::extract::State;
use axum::Form;
use axum::response::Html;
use serde::Deserialize;
use sqlx::{Error, Pool, Sqlite};
use sqlx::sqlite::SqliteRow;


#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq)]
#[repr(i8)]
enum ReturnStatus {
    Running = 2,
    Success = 3,
    Failed = 4,
    Timeout = 5,
    Skipped = 6,
}

#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq)]
pub struct UpdateTaskForm {
    task_id: i64,
    return_status: ReturnStatus,
    ret_code: Option<i64>,
    output: String,
}

async fn update_build(db: &Pool<Sqlite>, task_id: i64) {
    // since this function is called by update task, we know at least
    // one task belonging to the job has been started.
    // we want to know if there is still a task belonging to the job that hasn't
    // finished

    sqlx::query(
        "UPDATE jobs
SET status = (
    SELECT
        CASE
            WHEN min_val = 2 THEN 2 -- running
            ELSE max_val -- error or success
        END val
    from (
        select min(task_status) as min_val, max(task_status) as max_val
        from (
          select distinct
              CASE
                 WHEN finished_at IS NULL THEN 2  -- running
                 WHEN (status = 3) || (status = 6) then 3 -- success
                 WHEN (status = 4) || (status = 5) then 4 -- error
                 ELSE 1
              END task_status
          from tasks
          where tasks.job_id = (select job_id from tasks where id = $1)
        )
    )
)
WHERE id = (select job_id from tasks where id = $1);"
    )
        .bind(task_id)
        .execute(&*db)
        .await
        .expect("Failed to set the build status");
}

pub(crate) async fn update_task(State(db): State<Pool<Sqlite>>, form: Form<UpdateTaskForm>) -> Html<String> {
    println!("Received requested update: {form:?}");

    let ret_status = form.return_status.clone() as i64;
    let output = form.output.as_str();
    println!("Output is: {output}");

    let mut tx = db
        .begin()
        .await
        .expect("Error when starting a sql transaction");

    sqlx::query(
        "UPDATE tasks
                     SET started_at = CURRENT_TIMESTAMP
                    WHERE (id = $1) AND (started_at IS NULL);"
    )
        .bind(form.task_id)
        .execute(&mut *tx)
        .await
        .expect("setting th start time must work");
    ;

    let res = sqlx::query(
        "UPDATE tasks
                     SET output = output || $2,
                         status = $3,
                         ret_code = $4
                    WHERE id = $1;"
    )
        .bind(form.task_id)
        .bind(output)
        .bind(ret_status)
        .bind(form.ret_code)
        .execute(&mut *tx)
        .await;

    if form.return_status != ReturnStatus::Running {
        sqlx::query(
            "UPDATE tasks
                     SET finished_at = CURRENT_TIMESTAMP
                    WHERE id = $1;"
        )
            .bind(form.task_id)
            .execute(&mut *tx)
            .await
            .expect("Setting finished time must work");
    }


    tx.commit()
        .await
        .expect("error occurred when trying to commit a transaction");

    update_build(&db, form.task_id).await;

    match res {
        Ok(_) => Html(String::from("OK")),
        Err(e) => Html(format!("Error: failed to update table: Err={e:?}"))
    }
}