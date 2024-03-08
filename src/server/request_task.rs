use crate::common::{is_valid_git_hash, Compiler, RequiredTests, TaskType};
use axum::extract::State;
use axum::Form;
use serde::Deserialize;
use sqlx::{FromRow, Pool, Sqlite};

fn return_false() -> bool {
    false
}

#[derive(Debug, Deserialize, Clone, Eq, Hash, PartialEq)]
pub struct AcceptJobForm {
    #[serde(default = "return_false")]
    accept_static_analyser_task: bool,
    #[serde(default = "return_false")]
    accept_clang_tidy_task: bool,
    #[serde(default = "return_false")]
    accept_clang_format_task: bool,
    #[serde(default = "return_false")]
    accept_compile_with_gcc_from_hardware_vendor: bool,
    #[serde(default = "return_false")]
    accept_compile_with_gcc_from_distro: bool,
    #[serde(default = "return_false")]
    accept_run_tests_on_qemu: bool,
    #[serde(default = "return_false")]
    accept_run_tests_on_real_hardware: bool,
    hostname: String,
}

#[derive(FromRow, Debug)]
struct TaskProperties {
    id: i64,
    task_type: i64,
    test_setup_id: Option<i64>,
    compiler_id: Option<i64>,
    required_tests: Option<i64>,
    mentioned_tests: Option<String>,
    run_tests_on_qemu: Option<i64>,
    run_tests_on_real_hardware: Option<i64>,
    git_hash: Option<String>,
}

pub async fn request_task(State(db): State<Pool<Sqlite>>, form: Form<AcceptJobForm>) -> String {
    let mut tx = db
        .begin()
        .await
        .expect("Error when starting a sql transaction");

    let query_res = sqlx::query_as::<_, TaskProperties>(
        "SELECT tasks.id, tasks.task_type, test_setup_id,
                compiler_id, required_tests, mentioned_tests, run_tests_on_qemu, run_tests_on_real_hardware,
                git_hash
    FROM tasks
    LEFT JOIN (SELECT test_setup.id as test_setup_id,
                      test_setup.task_id as test_setup_task_id,
                      test_setup.compiler_id as compiler_id,
                      test_setup.required_tests as required_tests,
                      test_setup.mentioned_tests as mentioned_tests,
                      test_setup.run_tests_on_qemu as run_tests_on_qemu,
                      test_setup.run_tests_on_real_hardware as run_tests_on_real_hardware
               FROM test_setup
               WHERE (    (     ((compiler_id = 1) AND ($4 = 1)) -- gcc_from_hardware_vendor
                             OR ((compiler_id = 2) AND ($5 = 1)) -- gcc_from_distro
                           )
                       AND
                          (    (required_tests = 2) -- no tests, only compile
                            OR (     ((run_tests_on_qemu = 0) OR ($6 = 1))
                                 AND ((run_tests_on_real_hardware = 0) OR ($7 = 1))
                               )
                          )
                     )
               )
    ON test_setup_task_id = tasks.id
    JOIN (SELECT jobs.commit_id as git_hash, jobs.id as id_from_job_table
          FROM jobs
          WHERE (status = 1) || (status = 2) -- shorten the search space
         )
    ON tasks.job_id = id_from_job_table
    WHERE status = 1 -- shortcut for pending
    AND (    ((tasks.task_type = 1) AND ($1 = 1)) -- static_analyser
          OR ((tasks.task_type = 2) AND ($2 = 1)) -- clang-format
          OR ((tasks.task_type = 3) AND ($3 = 1)) -- clang-tidy
          OR ((tasks.task_type = 4) AND (test_setup_id IS NOT NULL))-- tests and we already filtered the compilers
        )
    ORDER BY id
    LIMIT 1;")
        .bind(form.accept_static_analyser_task as u8)
        .bind(form.accept_clang_format_task as u8)
        .bind(form.accept_clang_tidy_task as u8)
        .bind(form.accept_compile_with_gcc_from_hardware_vendor as u8)
        .bind(form.accept_compile_with_gcc_from_distro as u8)
        .bind(form.accept_run_tests_on_qemu as u8)
        .bind(form.accept_run_tests_on_real_hardware as u8)
        .fetch_optional(&mut *tx)
        .await;

    let Ok(task_properties) = query_res else {
        return format!(
            "Error occurred while trying to find a fitting task: {:?}",
            query_res.err().unwrap()
        );
    };

    let Some(task_properties) = task_properties else {
        return String::from("no suitable task found. Maybe there are no tasks left to execute");
    };

    let task_id_to_run = task_properties.id;
    let task_type = TaskType::from_i64(task_properties.task_type);

    let Some(git_hash) = task_properties.git_hash else {
        return String::from("Couldn't retrieve the git hash from the task");
    };

    if !is_valid_git_hash(git_hash.as_str()) {
        return format!(
            "Error, retrieved task {task_id_to_run} but the git hash ({git_hash}) is invalid"
        );
    }

    let task_details = match task_type {
        TaskType::StaticAnalyser | TaskType::ClangFormat | TaskType::ClangTidy => {
            format!("Type: {task_type:?}")
        }
        TaskType::Tests => {
            let Some(required_tests) = task_properties.required_tests else {
                return String::from("Error: tests required but required tests are not specified");
            };
            let Some(test_setup_id) = task_properties.test_setup_id else {
                return String::from("Error: retrieved tests do not have an associated setup id");
            };
            let mentioned_tests = task_properties.mentioned_tests;
            let required_tests =
                RequiredTests::try_from_tag_and_string(required_tests, mentioned_tests);
            let Ok(required_tests_as_enum) = required_tests else {
                return String::from(required_tests.err().unwrap());
            };
            let Some(compiler_id) = task_properties.compiler_id else {
                return String::from(
                    "Error: no compiler is specified. How do you want to compile tests?",
                );
            };
            let compiler = Compiler::from_i64(compiler_id);

            let details = format!(
                "Type: Tests
Test setup id: {test_setup_id:?}
Test type: {required_tests_as_enum:?}
Compiler: {compiler:?}"
            );
            let details = if required_tests_as_enum != RequiredTests::NoTestOnlyCompile {
                let Some(run_tests_on_qemu) = task_properties.run_tests_on_qemu else {
                    return String::from("Error: tests required but couldn't find out if they were to run on qemu or not");
                };
                let Some(run_tests_on_real_hardware) = task_properties.run_tests_on_real_hardware
                    else {
                        return String::from("Error: tests required but couldn't find out if they were to run on real hardware or not");
                    };
                let run_tests_on_qemu = run_tests_on_qemu != 0;
                let run_tests_on_real_hardware = run_tests_on_real_hardware != 0;
                format!(
                    "{details}
Run tests on qemu: {run_tests_on_qemu:?}
Run tests on real hardware: {run_tests_on_real_hardware:?}"
                )
            } else {
                details
            };
            details
        }
    };

    let query_res = sqlx::query::<_>(
        "UPDATE tasks
             SET started_at = CURRENT_TIMESTAMP,
             status = 2, -- running
             executed_on = $1
             WHERE id = $2;",
    )
        .bind(&form.hostname)
        .bind(task_id_to_run)
        .execute(&mut *tx)
        .await;

    let Ok(_query_res) = query_res else {
        return format!(
            "Error: failed to update task {task_id_to_run} to set hostname to {h}.",
            h = form.hostname
        );
    };

    tx.commit()
        .await
        .expect("error occurred when trying to commit a transaction");

    format!(
        "Task id: {task_id_to_run}
Git Hash: {git_hash}
{task_details}
"
    )
}
