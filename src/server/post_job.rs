use crate::common::{DOCTYPE, get_head_with_title};
use crate::common::is_valid_git_hash;
use crate::post_job::TestsToRun::{NoTestsOnlyCompile, NotEvenCompile};
use axum::extract::State;
use axum::response::Html;
use axum::Form;
use serde::Deserialize;
use sqlx::{FromRow, Pool, Sqlite, SqliteConnection, SqliteExecutor};
use std::fmt::Debug;

#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq)]
pub enum TestsToRun {
    // needs to match test_type in sql database
    AllTests = 1,
    NoTestsOnlyCompile = 2,
    NotEvenCompile = 3,
    AllTestsExcept = 4,
    OnlySpecifiedTests = 5,
}

fn return_false() -> bool {
    false
}

#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq)]
pub struct PostJobForm {
    commit_to_use: String,
    tests_to_run: TestsToRun,
    #[serde(default)]
    explicitly_disabled_tests: String,
    #[serde(default)]
    explicitly_enabled_tests: String,
    #[serde(default = "return_false")]
    compile_with_gcc_from_hardware_vendor: bool,
    #[serde(default = "return_false")]
    compile_with_gccFromDistro: bool,
    #[serde(default = "return_false")]
    run_tests_on_qemu: bool,
    #[serde(default = "return_false")]
    run_tests_on_real_hardware: bool,
    #[serde(default = "return_false")]
    run_static_analyser: bool,
    #[serde(default = "return_false")]
    run_clang_tidy: bool,
    #[serde(default = "return_false")]
    run_clang_format: bool,
    #[serde(default)]
    email_to_notify_on_completion: String,
}

#[derive(FromRow)]
struct RowID {
    id: i64,
}

async fn add_task(
    tx: impl SqliteExecutor<'_>,
    add_task_b: bool,
    job_id: i64,
    job_kind: i64,
    job_kind_str: &'static str,
) -> Result<(), Html<String>> {
    if !add_task_b {
        return Ok(());
    }

    let query_res = sqlx::query(
        "INSERT INTO tasks(job_id, task_type)
        VALUES ($1, $2);",
    )
        .bind(job_id)
        .bind(job_kind) // shortcut for select id from tasks_kind where name = 'job_kind_as_human_str'
        .execute(tx)
        .await;

    match query_res {
        Ok(_) => Ok(()),
        Err(err_msg) => Err(Html(format!(
            "Error occurred while inserting a {job_kind_str} task into database: {err_msg:?}")
        )),
    }
}

async fn add_static_analyser_task(
    tx: impl SqliteExecutor<'_>,
    add_static_analyser_task: bool,
    job_id: i64,
) -> Result<(), Html<String>> {
    add_task(tx, add_static_analyser_task, job_id, 1, "static_analyser").await
}

async fn add_clang_format_task(
    tx: impl SqliteExecutor<'_>,
    add_clang_format_task: bool,
    job_id: i64,
) -> Result<(), Html<String>> {
    add_task(tx, add_clang_format_task, job_id, 2, "clang-format").await
}

async fn add_clang_tidy_task(
    tx: impl SqliteExecutor<'_>,
    add_clang_tidy_task: bool,
    job_id: i64,
) -> Result<(), Html<String>> {
    add_task(tx, add_clang_tidy_task, job_id, 3, "clang-tidy").await
}

async fn add_test_setup(
    tx: &mut SqliteConnection,
    form: &Form<PostJobForm>,
    job_id: i64,
) -> Result<(), Html<String>> {
    if form.tests_to_run == NotEvenCompile {
        return Ok(());
    }

    let is_compiler_specified = form.compile_with_gccFromDistro || form.compile_with_gcc_from_hardware_vendor;
    if !is_compiler_specified {
        return Err(Html(String::from(
            "Error: compilation needed, but no compiler chosen",
        )));
    }

    let need_target = match form.tests_to_run {
        TestsToRun::AllTests => true,
        TestsToRun::NoTestsOnlyCompile => false,
        NotEvenCompile => {
            panic!()
        }
        TestsToRun::AllTestsExcept => true,
        TestsToRun::OnlySpecifiedTests => true,
    };

    let has_target = form.run_tests_on_qemu || form.run_tests_on_real_hardware;
    if need_target && (!has_target) {
        return Err(Html(String::from("Error: asking to run tests, but no target specified. You need to select at least one between qemu and real_hardware")));
    }

    if (form.tests_to_run == TestsToRun::AllTestsExcept)
        && (form.explicitly_disabled_tests.is_empty())
    {
        return Err(Html(String::from("Error: asking to run all tests except some, but the list of tests to avoid is empty. If you want to run all tests, use the AllTests option.")));
    }

    if (form.tests_to_run == TestsToRun::OnlySpecifiedTests)
        && (form.explicitly_enabled_tests.is_empty())
    {
        return Err(Html(String::from("Error: asking to run only some tests, but the list of tests to run is empty. If you do not want to run any tests, use the OnlyCompile option, or NotEvenCompile.")));
    }

    let mentioned_tests = match form.tests_to_run {
        TestsToRun::AllTests | TestsToRun::NoTestsOnlyCompile => None,
        NotEvenCompile => {
            panic!()
        }
        TestsToRun::AllTestsExcept => Some(&form.explicitly_disabled_tests),
        TestsToRun::OnlySpecifiedTests => Some(&form.explicitly_enabled_tests),
    };

    let query_res = sqlx::query_as::<_, RowID>(
        "INSERT INTO tasks(job_id, task_type)
        VALUES ($1, $2)
        RETURNING id;",
    )
        .bind(job_id)
        .bind(4) // shortcut for select id from tasks_kind where name = 'tests'
        .fetch_one(&mut *tx)
        .await;

    let Ok(RowID {
               id: task_id_for_tests,
           }) = query_res
        else {
            // no need to manually call rollback. It is done automatically on Drop
            return Err(Html(format!(
                "Error occurred while inserting a test task into database: {:?}",
                query_res.err()
            )));
        };

    let run_on_qemu = match form.tests_to_run {
        NoTestsOnlyCompile => None,
        NotEvenCompile => panic!(),
        TestsToRun::AllTests | TestsToRun::AllTestsExcept | TestsToRun::OnlySpecifiedTests => {
            Some(form.run_tests_on_qemu)
        }
    };

    let run_on_real_hardware = match form.tests_to_run {
        NoTestsOnlyCompile => None,
        NotEvenCompile => panic!(),
        TestsToRun::AllTests | TestsToRun::AllTestsExcept | TestsToRun::OnlySpecifiedTests => {
            Some(form.run_tests_on_real_hardware)
        }
    };

    if form.compile_with_gccFromDistro {
        let query_res = sqlx::query_as::<_, RowID>(
            "INSERT INTO test_setup(task_id, compiler_id, required_tests, mentioned_tests, run_tests_on_qemu, run_tests_on_real_hardware)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id;")
            .bind(task_id_for_tests)
            .bind(2) // shortcut for select id from compilers where name = 'gccFromDistro'
            .bind((form.tests_to_run.clone()) as i64)
            .bind(mentioned_tests)
            .bind(run_on_qemu)
            .bind(run_on_real_hardware)
            .fetch_one(&mut *tx)
            .await;

        let Ok(RowID { id: _test_setup_id }) = query_res else {
            // no need to manually call rollback. It is done automatically on Drop
            return Err(Html(format!(
                "Error occurred while inserting a test setup with gccFromDistro into database: {:?}",
                query_res.err()
            )));
        };

        if form.compile_with_gcc_from_hardware_vendor {
            let query_res = sqlx::query_as::<_, RowID>(
                "INSERT INTO tasks(job_id, task_type)
        VALUES ($1, $2)
        RETURNING id;",
            )
                .bind(job_id)
                .bind(4) // shortcut for select id from tasks_kind where name = 'tests'
                .fetch_one(&mut *tx)
                .await;

            let Ok(RowID {
                       id: task_id_for_compile_only,
                   }) = query_res
                else {
                    // no need to manually call rollback. It is done automatically on Drop
                    return Err(Html(format!("Error occurred while inserting a test task into database (one for only compile): {:?}", query_res.err())));
                };

            // if both compilers are requested, only run tests on code compiled with gccFromDistro
            // but still compile with gcc from hardware_vendor
            let query_res = sqlx::query_as::<_, RowID>(
                "INSERT INTO test_setup(task_id, compiler_id, required_tests)
        VALUES ($1, $2, $3)
        RETURNING id;",
            )
                .bind(task_id_for_compile_only)
                .bind(1) // shortcut for select id from compilers where name = 'gcc_from_hardware_vendor'
                .bind(NoTestsOnlyCompile as i64) // no tests, only compile
                .fetch_one(&mut *tx)
                .await;

            let Ok(RowID { id: _test_setup_id }) = query_res else {
                // no need to manually call rollback. It is done automatically on Drop
                return Err(Html(format!("Error occurred while inserting a compilation test setup with gcc from hardware_vendor (not adding tests) into database: {:?}", query_res.err())));
            };
        }

        return Ok(());
    }

    if form.compile_with_gcc_from_hardware_vendor {
        let query_res = sqlx::query_as::<_, RowID>(
            "INSERT INTO test_setup(task_id, compiler_id, required_tests, mentioned_tests, run_tests_on_qemu, run_tests_on_real_hardware)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id;")
            .bind(task_id_for_tests)
            .bind(1) // shortcut for select id from compilers where name = 'gcc_from_hardware_vendor'
            .bind(form.tests_to_run.clone() as i64)
            .bind(mentioned_tests)
            .bind(run_on_qemu)
            .bind(run_on_real_hardware)
            .fetch_one(&mut *tx)
            .await;

        let Ok(RowID { id: _test_setup_id }) = query_res else {
            // no need to manually call rollback. It is done automatically on Drop
            return Err(Html(format!("Error occurred while inserting a test setup with gcc_from_hardware_vendor into database: {:?}", query_res.err())));
        };
    }

    Ok(())
}

pub async fn post_job(State(db): State<Pool<Sqlite>>, form: Form<PostJobForm>) -> Html<String> {
    if (!form.run_static_analyser)
        && (!form.run_clang_tidy)
        && (!form.run_clang_format)
        && (form.tests_to_run == NotEvenCompile)
    {
        return Html(String::from("Error: posting a job but nothing requested."));
    }

    if !is_valid_git_hash(form.commit_to_use.as_str()) {
        return Html(String::from("Error: invalid git hash given."));
    }

    let email = if form.email_to_notify_on_completion.is_empty() {
        None
    } else {
        Some(&form.email_to_notify_on_completion)
    };

    let mut tx = db
        .begin()
        .await
        .expect("Error when starting a sql transaction");

    let query_res = sqlx::query_as::<_, RowID>(
        "INSERT INTO jobs(commit_id, email)
            VALUES($1, $2)
            RETURNING id;",
    )
        .bind(&form.commit_to_use)
        .bind(email)
        .fetch_one(&mut *tx)
        .await;

    let Ok(RowID { id: job_id }) = query_res else {
        // no need to manually call rollback. It is done automatically on Drop
        return Html(format!(
            "Error occurred while inserting job into database: {:?}",
            query_res.err()
        ));
    };

    let query_res = add_static_analyser_task(&mut *tx, form.run_static_analyser, job_id).await;
    if let Err(e) = query_res {
        return e;
    };

    let query_res = add_clang_format_task(&mut *tx, form.run_clang_format, job_id).await;
    if let Err(e) = query_res {
        return e;
    };

    let query_res = add_clang_tidy_task(&mut *tx, form.run_clang_tidy, job_id).await;
    if let Err(e) = query_res {
        return e;
    };

    let query_res = add_test_setup(&mut *tx, &form, job_id).await;
    if let Err(e) = query_res {
        return e;
    };

    tx.commit()
        .await
        .expect("error occurred when trying to commit a transaction");

    let x = job_id;

    let html_head = get_head_with_title(format!("Information about the added job (ID={x})").as_str());
    let formatted_form = format!("{form:?}");
    let formatted_form = html_escape::encode_safe(formatted_form.as_str());
    Html(format!(
        "{DOCTYPE}<html lang=\"en-GB\">{html_head}<body>\
<h1 class=\"post-title\">Go back to the build list</h1>
<a href=\"/\" class=\"link_button\">Click here to go back to the job list view</a>
<br><br>
<h1 class=\"post-title\">Information about the added job</h1>
    added job with id {x} <br>
    <blockquote><pre>form: {formatted_form}</pre></blockquote>
<br>
<a href=\"/build/{x}\" class=\"link_button\">View build details</a>

 </body></html>"))
}
