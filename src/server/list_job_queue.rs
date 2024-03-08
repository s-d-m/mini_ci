use std::io::Read;
use axum::extract::{Path, State};
use axum::response::Html;
use serde::Deserialize;
use sqlx::{Error, FromRow, Pool, Sqlite};
use crate::common::{DOCTYPE, get_head_with_title, is_valid_git_hash, JobStatus, URL_OF_GIT_SERVER_FOR_BROWSER_SHOWING_COMMITS};

#[derive(Debug, Deserialize, FromRow, Clone, Eq, Hash, PartialEq)]
pub struct JobProperty {
    id: i64,
    commit_id: String,
    added_at: String,
    status: i64,
}

pub async fn list_job_queue(State(db): State<Pool<Sqlite>>) -> Html<String> {
    list_job_queue_with_max_id(State(db), Path(i64::MAX)).await
}

async fn format_vec_to_html(rows: Vec<JobProperty>) -> Html<String> {
    let smallest_id = rows.last().map(|x| x.id);
    let biggest_id = rows.first().map(|x| x.id);

    let table_in = rows
        .into_iter()
        .map(|r| {
            let id = r.id;
            let commit = r.commit_id;
            let added_at = r.added_at;
            let status = r.status;
            let status = JobStatus::from_i64(status);

            if !is_valid_git_hash(commit.as_str()) { panic!() } // commit must have been validated before entering database

            format!(
                "<tr class=\"{status:?}\" title=\"build_nr_{id}\">
  <td title=\"link_to_details\">
   <a class=\"link_button display_block center_text\" href=\"/build/{id}\">View details of build {id}</a>
  </td>
  <td title=\"commit_value\">
   <a href=\"{URL_OF_GIT_SERVER_FOR_BROWSER_SHOWING_COMMITS}/{commit}\">{commit}</a>
  </td>
  <td title=\"added_at\">
    {added_at} UTC
  </td>
  <td title=\"status\">
    {status:?}
  </td>
</tr>")
        })
        .reduce(|x, y| format!("{x}\n{y}"))
        .unwrap_or(String::from(""));

    let next_button = match smallest_id {
        Some(x) if x > 1 => { format!("<a href=\"/list_max_id/{max}\" class=\"link_button\">Older builds</a>", max = x - 1) }
        _ => String::from("")
    };

    let previous_button = match biggest_id {
        Some(x) => { format!("<a href=\"/list_min_id/{min}\" class=\"link_button\">Newer builds</a>", min = x + 1) }
        _ => String::from("<a href=\"/\" class=\"link_button\">Latest builds</a>")
    };

    let html_head = get_head_with_title("add a build/test request");
    Html(format!(
        "{DOCTYPE}<html lang=\"en-GB\">{html_head}<body>
<h1 class=\"post-title\">Add a build</h1>
<a href=\"/add_job\" class=\"link_button display_inline_block\">Click here to post a new job</a>
<br>
<br>
<h1 class=\"post-title\">Current build list</h1>
  <table>
    <tr>
      <td>job id</td>
      <td>commit id</td>
      <td>job added at</td>
      <td>status</td>
    </tr>
    {table_in}
    </table>
{previous_button}
{next_button}
</body>
</html>"
    ))
}

async fn format_quert_res(query_res: Result<Vec<JobProperty>, Error>) -> Html<String> {
    let Ok(rows) = query_res else {
        return Html(format!(
            "Error occurred while reading the database {:?}",
            query_res.err()
        ));
    };

    format_vec_to_html(rows).await
}

pub async fn list_job_queue_with_min_id(State(db): State<Pool<Sqlite>>, Path(min_id): Path<i64>) -> Html<String> {
    let query_res = sqlx::query_as::<_, JobProperty>(
        "SELECT * FROM (SELECT id, commit_id, added_at, status FROM JOBS
                            WHERE id >= $1
                            ORDER BY id
                            LIMIT 50)
        ORDER BY id DESC;",
    )
        .bind(min_id)
        .fetch_all(&db)
        .await;

    format_quert_res(query_res).await
}


pub async fn list_job_queue_with_max_id(State(db): State<Pool<Sqlite>>,
                                        Path(max_id): Path<i64>, ) -> Html<String> {
    let query_res = sqlx::query_as::<_, JobProperty>(
        "SELECT id, commit_id, added_at, status FROM JOBS
        WHERE id <= $1
        ORDER BY id DESC
        LIMIT 50;",
    )
        .bind(max_id)
        .fetch_all(&db)
        .await;

    format_quert_res(query_res).await
}
