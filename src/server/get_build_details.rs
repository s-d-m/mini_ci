use std::borrow::Cow;
use std::collections::HashMap;
use crate::common::{Compiler, DOCTYPE, get_head_with_title, is_valid_git_hash, JobStatus, TaskProperties, TaskType};
use axum::extract::{Path, State};
use axum::response::Html;
use serde::Deserialize;
use sqlx::{FromRow, Pool, Sqlite};
use std::fmt::{Debug, Formatter};
use tracing_subscriber::fmt::format;

#[derive(Debug, Deserialize, FromRow, Clone, Eq, Hash, PartialEq)]
pub struct JobProperties {
    commit_id: String,
    added_at: String,
    status: i64,
}

#[derive(FromRow)]
struct TestSetup {
    id: i64,
    compiler_id: i64,
    required_tests: i64,
    mentioned_tests: Option<String>,
    run_tests_on_qemu: i64,
    run_tests_on_real_hardware: i64,
}

fn compiler_str_from_id(compiler_id: i64) -> &'static str {
    let compiler_str = match compiler_id {
        1 => "gcc provided by HardwareVendor",
        2 => "gcc from distro",
        _ => "Unknown compiler",
    };

    compiler_str
}

impl Debug for TestSetup {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let TestSetup {
            id,
            compiler_id,
            required_tests,
            mentioned_tests,
            run_tests_on_qemu,
            run_tests_on_real_hardware,
        } = self;
        let compiler_str = compiler_str_from_id(*compiler_id);
        let tests_to_run = match required_tests {
            1 => String::from("All tests"),
            2 => String::from("No tests, only compile"),
            // 3 left out on purpose as it corresponds to "not even compile"
            4 => {
                match mentioned_tests {
                    None => { String::from("errr ... it should be all tests expects some explicitly designated, but no test was designated. Was this simply all tests???") }
                    Some(tests_names) => {
                        let tests_names = Self::join_test_names(tests_names);
                        format!("All tests except {tests_names}")
                    }
                }
            }
            5 => {
                match mentioned_tests {
                    None => { String::from("errr ... it should be only some explicitly designated tests, but no test was designated. Was this simply all tests???") }
                    Some(tests_names) => {
                        let tests_names = Self::join_test_names(tests_names);
                        format!("{tests_names}")
                    }
                }
            }
            _ => String::from("Error: no idea what tests should have been run"),
        };

        let run_tests_on_qemu = (*run_tests_on_qemu != 0);
        let run_tests_on_real_hardware = (*run_tests_on_real_hardware != 0);
        let targets = match (run_tests_on_qemu, run_tests_on_real_hardware) {
            (true, true) => { "both qemu and real hardware" }
            (false, true) => { "real hardware only" }
            (true, false) => { "qemu only" }
            (false, false) => { "None." }
        };

        let tests_to_run = encode_html_with_escape_codepoint(tests_to_run.as_str());
        write!(
            f,
            "TestSetup id: {id}<br>
Compiler: {compiler_str}<br>
Tests to run: {tests_to_run}<br>
Targets to run tests on: {targets}")
    }
}

impl TestSetup {
    fn join_test_names(tests_names: &String) -> String {
        let tests_names = tests_names
            .lines()
            .map(|x| x.split_whitespace())
            .flatten()
            .map(|x| String::from(x))
            .reduce(|a, b| format!("{a}, {b}"))
            .unwrap();
        tests_names
    }
}

#[derive(FromRow, Debug)]
struct TestRunQuery {
    test_name: String,
    started_at: Option<String>,
    finished_at: Option<String>,
    status: i64,
    ret_code: Option<i64>,
    output: String,
    target_id: i64,
}

fn get_target_str_from_id(target_id: i64) -> &'static str {
    match target_id {
        1 => "Qemu",
        2 => "Real Hardware",
        _ => "Unknown platform"
    }
}

fn get_tr(test_run: &TestRunQuery) -> String {
    let TestRunQuery { test_name, started_at, finished_at, status, ret_code, output, target_id } = test_run;
    let class_v = JobStatus::from_i64(*status);
    let class_v = format!("{class_v:?}");

    let started_at = match started_at {
        None => { String::from("started at: ---") }
        Some(time) => { format!("started at: {time} utc") }
    };

    let finished_at = match finished_at {
        None => { String::from("finished at: ---") }
        Some(time) => { format!("finished at: {time} utc") }
    };

    let ret_code = match ret_code {
        None => { String::from("") }
        Some(x) => { format!("ret code: {x}") }
    };

    let target = get_target_str_from_id(*target_id).replace(" ", "_");
    let output = encode_html_with_escape_codepoint(output.as_str());

    format!(
        "<td class=\"{class_v}\" title=\"{target}\">
<details>
<summary>{class_v}</summary>
<pre>
{started_at}
{finished_at}
{ret_code}
output:

{output}
</pre>
</details>
</td>")
}

// Note that the produced HTML might not always be valid as per HTML 4.01. For example
// if the output retrieves from the database contains valid UTF-8 code points which fall
// outside the range of what HTML 4.01 accepts. In particular, escape (U+001B) is not
// valid in HTML 4.01 and easily produced by commands which outputs terminal control sequence,
// for example to display text with colours in the terminal

async fn format_task(task: &TaskProperties, db: Pool<Sqlite>) -> String {
    let TaskProperties {
        id,
        status,
        ret_code,
        task_type,
        output,
    } = task;

    let task_type = TaskType::from_i64(*task_type);
    let status_str = format!("{x:?}", x = JobStatus::from_i64(*status));

    let ret_code_str = if let Some(code) = *ret_code {
        format!("ret code: {code}<br>")
    } else {
        String::from("")
    };

    let task_id = id;
    let h1_title = format!("<h1 class=\"post-title\">task: {task_type:?}</h1>");

    let output = match output {
        None => { None }
        Some(s) => { Some(encode_html_with_escape_codepoint(s.as_str())) }
    };

    let TaskType::Tests = task_type else {
        let task_output_str = if let Some(output) = output {
            format!("<blockquote><details><summary>output: </summary><pre title=\"output\">{output}</pre></details></blockquote><br>")
        } else {
            String::from("")
        };
        let task_detail = format!(
            "task_id: {task_id}<br>
status: {status_str}<br>
{ret_code_str}
{task_output_str}");

        return format!("{h1_title}<br><div class=\"{status_str}\" title=\"{task_type:?}\">{task_detail}</div>");
    };

    let test_setup =
        sqlx::query_as::<_, TestSetup>(
            "SELECT id, compiler_id, required_tests, mentioned_tests, run_tests_on_qemu, run_tests_on_real_hardware
        FROM test_setup
        WHERE task_id = $1;"
        )
            .bind(task_id)
            .fetch_one(&db)
            .await;

    let Ok(test_setup) = test_setup else {
        let task_detail = format!(
            "task_id: {task_id}<br>
status: {status_str}<br>
{ret_code_str}");
        return format!(
            "{h1_title}<br><div title=\"{task_type:?}\">{task_detail}<br>Failed to extract the test setup for task with id {task_id}<br></div>"
        );
    };

    let task_output_str = if let Some(output) = output {
        format!("<blockquote><details><summary>compile output: </summary><pre title=\"compile_output\">{output}</pre></details></blockquote>")
    } else {
        String::from("")
    };

    let task_detail = format!(
        "task_id: {task_id}<br>
status: {status_str}<br>
{ret_code_str}
{test_setup:?}
{task_output_str}");

    let test_runs = sqlx::query_as::<_, TestRunQuery>(
        "SELECT test_name, started_at, finished_at, status, ret_code, output, target_id
        FROM test_run
        WHERE task_id = $1
        ORDER BY test_name, target_id;"
    )
        .bind(task_id)
        .fetch_all(&db)
        .await;

    let Ok(test_runs) = test_runs else {
        return format!(
            "{h1_title}<br><div title=\"{task_type:?}\">{task_detail}<br>Failed to extract the tests executed for task with id {task_id}<br></div>"
        );
    };

    let mut test_name_to_results = HashMap::<&str, Vec<&TestRunQuery>>::new();
    for test_run in &test_runs {
        match test_name_to_results.get_mut(test_run.test_name.as_str()) {
            None => { test_name_to_results.insert(test_run.test_name.as_str(), vec![test_run]); }
            Some(v) => { v.push(test_run) }
        }
    }

    let mut grouped_result_by_name = test_name_to_results
        .iter()
        .collect::<Vec<_>>();
    grouped_result_by_name.sort_by(|a, b| (*a).0.cmp((*b).0));

    let grouped_result_by_name = grouped_result_by_name;
    let table_content = grouped_result_by_name
        .iter()
        .map(|(name, values)| {
            let td_cell_for_name = format!("<td>{x}</td>", x = encode_html_with_escape_codepoint(name));
            let values_str = values
                .iter()
                .map(|x| {
                    get_tr(*x)
                })
                .reduce(|a, b| format!("{a}{b}"))
                .unwrap();
            format!("<tr title=\"{x}\">{td_cell_for_name}{values_str}</tr>", x = encode_html_with_escape_codepoint(name))
        })
        .reduce(|a, b| format!("{a}\n{b}"));

    let table_header = if grouped_result_by_name.len() > 0 {
        let first_result = grouped_result_by_name[0].1;
        let end_of_header = first_result
            .iter()
            .map(|x| {
                let target_name = get_target_str_from_id(x.target_id);
                format!("<th>{target_name}</th>")
            })
            .reduce(|a, b| format!("{a}{b}"))
            .unwrap();
        Some(format!("<tr><th>test name</th>{end_of_header}</tr>"))
    } else {
        None
    };

    let compiler_str = compiler_str_from_id(test_setup.compiler_id)
        .replace(" ", "_");

    let table = match (table_header, table_content) {
        (None, _) | (_, None) => { String::from("") }
        (Some(header), Some(content)) => { format!("<table>{header}{content}</table>") }
    };
    format!("{h1_title}<br><div title=\"{task_type:?}_{compiler_str}\" class=\"{status_str}\">{task_detail}{table}</div>")
}

// This is used to avoid html issues of the type "invalid UTF-8 codepoint"
// For example, the codepoint corresponding to escape U+001b can easily
// appear in the output of a command, when said command prints to console
// with control characters (e.g. when printing text with colours). The set
// of UTF-8 codepoints valid in HTML is a subset of the valid utf-8. This
// here will ensure to encode the utf8 codepoints outside of the printable
// range to not trigger such errors
fn encode_html_with_escape_codepoint(text: &str) -> String {
    html_escape::encode_safe(text)
        .escape_default()
        .to_string()
        .replace("\\n", "\n")
        .replace("\\r", "\r")
        .replace("\\t", "\t")
}

pub async fn get_build_details(
    State(db): State<Pool<Sqlite>>,
    Path(build_id): Path<i64>,
) -> Html<String> {
    let query_res = sqlx::query_as::<_, JobProperties>(
        "SELECT commit_id, added_at, status FROM JOBS
        WHERE id = $1;",
    )
        .bind(build_id)
        .fetch_one(&db)
        .await;

    let Ok(JobProperties {
               status,
               commit_id,
               added_at,
           }) = query_res
        else {
            return Html(format!("Error, there is no job with id {build_id}"));
        };

    let status = JobStatus::from_i64(status);

    let tasks = sqlx::query_as::<_, TaskProperties>(
        "SELECT id, status, ret_code, task_type, output
        FROM tasks
        WHERE job_id = $1;",
    )
        .bind(build_id)
        .fetch_all(&db)
        .await;

    let Ok(tasks) = tasks else {
        return Html(format!(
            "Error, couldn't retrieve the tasks requested for {build_id}"
        ));
    };

    if tasks.is_empty() {
        return Html(format!("Suspicious: build {build_id} has no task. Did someone added a job without asking for running static_analyser/clang-tidy/compiling etc..."));
    };

    let mut all_tasks_str = String::from("");
    for task in tasks {
        let cur_task = format_task(&task, db.clone()).await;
        all_tasks_str = format!("{all_tasks_str}\n<br><br>\n{cur_task}")
    }

    if !is_valid_git_hash(commit_id.as_str()) { panic!() }
    let html_head = get_head_with_title(format!("build details for build id #{build_id}").as_str());
    Html(format!(
        "{DOCTYPE}<html lang=\"en-GB\">{html_head}<body>
<h1 class=\"post-title\">View build list</h1>
<a href=\"/\" class=\"link_button display_inline_block\">Click here to go back to the job list view</a>
<br>
<br>

<h1 class=\"post-title\">build with id {build_id}</h1>
<br>
Was added at {added_at} UTC
<br>
Building from commit {commit_id}
<br>
status is: {status:?}
<br>
<br>
{all_tasks_str}
</body>
</html>"
    ))
}
