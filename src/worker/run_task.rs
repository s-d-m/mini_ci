use std::borrow::Cow;
use crate::common::{Compiler, FinishStatus, is_immediate_exit_requested, MINICI_SERVER_ADD_TEST_TEST_LIST, MINICI_SERVER_REPORT_TEST_CHANGE, report_task_data, report_task_error, report_task_finish, report_task_started, RequestedTest, RUN_CLANG_TIDY_SCRIPT_IN_TESTED_PROJECT, STATIC_ANALYSER_SCRIPT_IN_TESTED_PROJECT, Task, TaskKind, TestSetup};
use crate::update_git_repo::{
    get_commit_desc, get_git_checkout_in, run_git_clone_in, run_git_remote_update_in,
};
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, Read, Stdout};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{ExitCode, ExitStatus, Output, Stdio};
use tracing::error;
use crate::common;
use crate::run_command::{run_proc};


fn get_file_content(filename: &OsStr) -> Result<String, String> {
    let res = File::open(filename);
    let Ok(file) = res else {
        return Err(format!("Failed to open {filename:?} for reading. Err={}", res.err().unwrap()));
    };

    let mut buf_reader = BufReader::new(file);
    let mut contents = String::new();
    let res = buf_reader.read_to_string(&mut contents);
    let Ok(res) = res else {
        return Err(format!("Failed to read data from {filename:?} for reading. Err={}", res.err().unwrap()));
    };

    Ok(contents)
}

fn run_static_analyser_task(task_id: i64, task_dir: &Path) -> Result<FinishStatus, String> {
    println!("Running static_analyser in {}", String::from_utf8_lossy(task_dir.as_os_str().as_encoded_bytes()));

    let static_analyser_script = task_dir.join(Path::new(STATIC_ANALYSER_SCRIPT_IN_TESTED_PROJECT));
    let static_analyser_script = static_analyser_script.as_os_str();
    println!("Script path is {static_analyser_script:?}");

    let logs_file = task_dir.join(Path::new("static_analyser_logs_output"));
    let metrics_file = task_dir.join(Path::new("static_analyser_metrics_output"));

    let task_output = run_proc(task_id, static_analyser_script,
                               &["--output-log-file", logs_file.as_os_str().to_str().unwrap(),
                                 "--metric-output-path", metrics_file.as_os_str().to_str().unwrap()].as_ref());

    if !task_output.success() {
        report_task_data(task_id, "Failed to run static_analyser command with err")?;
        return Ok(FinishStatus::Failed(2));
    };

    let logs = get_file_content(logs_file.as_os_str());
    let Ok(logs) = logs else {
        report_task_data(task_id, logs.err().unwrap().as_str())?;
        return Ok(FinishStatus::Failed(2));
    };
    let logs = format!("Logs=[{logs}]");
    report_task_data(task_id, logs.as_str())?;

    let metrics = get_file_content(metrics_file.as_os_str());
    let Ok(metrics) = metrics else {
        report_task_data(task_id, metrics.err().unwrap().as_str()).expect("Failed to report task data");
        return Ok(FinishStatus::Failed(2));
    };
    let metrics = format!("Metrics=[{metrics}]");
    let _ = report_task_data(task_id, metrics.as_str());

    Ok(FinishStatus::Success)
}

fn run_clang_tidy_task(task_id: i64, task_dir: &Path) -> Result<FinishStatus, String> {
    println!("Running clang_tidy in {}", String::from_utf8_lossy(task_dir.as_os_str().as_encoded_bytes()));

    let clang_tidy_script = task_dir.join(Path::new(RUN_CLANG_TIDY_SCRIPT_IN_TESTED_PROJECT));
    let clang_tidy_script = clang_tidy_script.as_os_str();
    println!("Script path is {clang_tidy_script:?}");

    let task_output = run_proc(task_id, clang_tidy_script,
                               &[]);

    if !task_output.success() {
        report_task_data(task_id, "clang_tidy command failed")?;
        return Ok(FinishStatus::Failed(2));
    };

    Ok(FinishStatus::Success)
}

fn report_test_start(test_name: &str, task_id: i64, target: &str) -> Result<(), String> {
    let task_id = format!("{task_id}");
    let client = reqwest::blocking::Client::new();
    let res = client
        .post(MINICI_SERVER_REPORT_TEST_CHANGE)
        .form(&[("task_id", task_id.as_str()),
            ("test_name", test_name),
            ("operation", "Start"),
            ("target", target)])
        .send();
    let Ok(res) = res else {
        return Err(format!("failed to tell server we were starting a test {task_id}, err:{}", res.err().unwrap()));
    };

    match res.text_with_charset("utf-8").unwrap().as_str() {
        "OK" => (),
        e => return Err(format!("Error from server: {e}")),
    }
    Ok(())
}

fn report_test_finished(test_name: &str, task_id: i64, target: &str, status: FinishStatus) -> Result<(), String>
{
    let status_str = format!("{status:?}");
    let task_id = format!("{task_id}");

    println!("reporting test [{test_name}] finished with status [{status_str}] in task {task_id}");

    let client = reqwest::blocking::Client::new();
    let res = client
        .post(MINICI_SERVER_REPORT_TEST_CHANGE)
        .form(&[("task_id", task_id.as_str()),
            ("test_name", test_name),
            ("operation", "Finish"),
            ("status", &status_str),
            ("target", target)])
        .send();

    let Ok(res) = res else {
        let err_msg = format!("failed to tell server we finished a test {task_id}, err:{}", res.err().unwrap());
        println!("{}", err_msg);
        return Err(err_msg);
    };

    let reply = res.text_with_charset("utf-8").unwrap();
    let reply = reply.as_str();
    println!("Server replied with {reply}");

    match reply {
        "OK" => (),
        e => return Err(format!("Error from server: {e}")),
    }

    Ok(())
}

fn report_test_progress(test_name: &str, task_id: i64, target: &str, output: &str) -> Result<(), String>
{
    let task_id = format!("{task_id}");
    let client = reqwest::blocking::Client::new();
    let res = client
        .post(MINICI_SERVER_REPORT_TEST_CHANGE)
        .form(&[("task_id", task_id.as_str()),
            ("test_name", test_name),
            ("operation", "Progress"),
            ("output", output),
            ("target", target)])
        .send();

    let Ok(res) = res else {
        return Err(format!("failed to tell server add output to give it a test {task_id}, err:{}", res.err().unwrap()));
    };

    match res.text_with_charset("utf-8").unwrap().as_str() {
        "OK" => (),
        e => return Err(format!("Error from server: {e}")),
    }

    Ok(())
}


pub(crate) fn run_task(task: Task, git_mirror_path: &OsStr) -> Result<(), String> {
    let git_commit = task.git_hash();
    let task_id = task.id();

    let remote_update_success = run_git_remote_update_in(git_mirror_path);
    let git_commit_desc = get_commit_desc(git_mirror_path, git_commit);
    let Ok(git_commit_desc) = git_commit_desc else {
        let err_msg = if let Err(e) = remote_update_success {
            format!("Commit {git_commit} is not reachable. It might be on the server, but the fetch failed with {e}")
        } else {
            format!("Error: commit {git_commit} does not exists. Did you forget to push it to the server?")
        };
        return report_task_error(task_id, err_msg.as_str(), 2);
    };
    let msg = format!("Using commit {git_commit} with desc {git_commit_desc}");
    report_task_data(task_id, msg.as_str())?;

    let task_dir = temp_dir::TempDir::with_prefix("Dir_for_mini_worker_task_");
    let Ok(task_dir) = task_dir else {
        let err_msg = format!(
            "failed to create a temporary dir to run a task: {}",
            task_dir.err().unwrap()
        );
        return report_task_error(task_id, err_msg.as_str(), 2);
    };

    let path = task_dir.path();

    println!(
        "Creating directory [{d}] to run task with id {task_id} in it",
        d = String::from_utf8_lossy(path.as_os_str().as_encoded_bytes()),
    );

    let success = get_git_checkout_in(task_id, path.as_os_str(), git_mirror_path, git_commit);
    let Ok(_) = success else {
        let err_msg = success.err().unwrap();
        return report_task_error(task_id, err_msg.as_str(), 2);
    };


    report_task_started(task_id)?;
    let res = match task.task_type() {
        TaskKind::StaticAnalyser => run_static_analyser_task(task_id, path),
        TaskKind::ClangTidy => run_clang_tidy_task(task_id, path),
        TaskKind::ClangFormat => Ok(FinishStatus::Skipped),
        TaskKind::Test(setup) => run_tests_task(task_id, path, setup),
    };
    match res {
        Ok(status) => { report_task_finish(task_id, "", status)?; }
        Err(msg) => { report_task_finish(task_id, &msg, FinishStatus::Failed(2))? }
    }

    //  task_dir.leak();
    Ok(())
}

fn run_tests_task(task_id: i64, task_dir: &Path, test_setup: &TestSetup) -> Result<FinishStatus, String> {
    let TestSetup { test_setup_id, compiler, tests_to_run, run_tests_on_qemu, run_tests_on_real_hardware } = test_setup;

    let src_dir = task_dir;
    let src_dir_str = task_dir.as_os_str().to_str().unwrap();
    let build_dir = task_dir.join("build");
    let build_dir_str = build_dir.as_os_str().to_str().unwrap();
    let toolchain_file = src_dir.join("cmake/toolchain_for_target_hardware.cmake");
    let linker_script_path = src_dir.join("linkerscripts/matching_layout_from_vendor.ld");
    let linker_script_flags = format!("-DCMAKE_EXE_LINKER_FLAGS_INIT='-T{}'", linker_script_path.as_os_str().to_str().unwrap());
    let linker_script_flags = linker_script_flags.as_str();


    let args = match &test_setup.compiler {
        common::Compiler::GccFromHardwareVendor => vec![
            "-S", src_dir_str,
            "-B", build_dir_str,
            "-G", "Ninja",
            "--toolchain", toolchain_file.to_str().unwrap(),
            "--fresh",
        ],
        common::Compiler::GccFromDistro => {
            vec![
                "-S", src_dir_str,
                "-B", build_dir_str,
                "-G", "Ninja",
                "--toolchain", toolchain_file.to_str().unwrap(),
                "--fresh",
                linker_script_flags,
            ]
        }
    };

    let args_as_str = args
        .iter()
        .map(|a| format!("[{a}]"))
        .reduce(|a, b| format!("{a}\n{b}"))
        .unwrap();

    let cmd_as_str = format!("Running cmake with parameters {args_as_str}");

    report_task_data(task_id, cmd_as_str.as_str())?;

    let task_output = run_proc(task_id, PathBuf::from("cmake").as_os_str(), args.as_ref());
    if !task_output.success() {
        report_task_data(task_id, "cmake generation failed")?;
        return Ok(FinishStatus::Failed(2));
    };

    // now compiling
    // todo, compile only some tests if chosen "test only X, Y, Z"
    report_task_data(task_id, "now compiling using ninja --verbose all")?;

    let task_output = run_proc(task_id, PathBuf::from("ninja").as_os_str(),
                               &["-C", build_dir_str, "--verbose", "all"]);
    if !task_output.success() {
        report_task_data(task_id, "ninja command failed")?;
        return Ok(FinishStatus::Failed(2));
    };

    if let common::RequestedTest::NoTestsOnlyCompile = &test_setup.tests_to_run {
        report_task_data(task_id, "compilation finished")?;
        return Ok(FinishStatus::Success);
    };

    let available_tests = std::process::Command::new("ctest")
        .arg("--test-dir")
        .arg(build_dir_str)
        .arg("--show-only=human")
        .output();

    let Ok(available_tests) = available_tests else {
        let err_msg = format!(
            "Failed to call ctest to find available tests. Err={}",
            available_tests.err().unwrap()
        );
        report_task_data(task_id, err_msg.as_str())?;
        return Ok(FinishStatus::Failed(2));
    };

    let available_tests = String::from_utf8_lossy(&available_tests.stdout);
    let available_tests = get_available_tests(task_id, &available_tests)?;

    if let (common::RequestedTest::OnlySpecifiedTests(some_tests)) = &test_setup.tests_to_run {
        ensure_all_requested_tests_are_available(task_id, &available_tests, some_tests)?;
    }

    let tests_to_execute = get_tests_to_execute(&test_setup, &available_tests);
    drop(available_tests);


    println!("Will execute following tests {tests_to_execute:?}");

    let targets = match (test_setup.run_tests_on_qemu, test_setup.run_tests_on_real_hardware) {
        (true, true) => "qemu real_hardware",
        (true, false) => "qemu",
        (false, true) => "real_hardware",
        (false, false) => panic!(),
    };

    let client = reqwest::blocking::Client::new();
    let res = client
        .post(MINICI_SERVER_ADD_TEST_TEST_LIST)
        .form(&[("task_id", format!("{task_id}")),
            ("tests_to_add", tests_to_execute.join(" ")),
            ("targets", String::from(targets))])
        .send();
    let Ok(res) = res else {
        return Err(format!("failed to add tests to execute to task with id: {task_id}, err:{}", res.err().unwrap()));
    };

    let inner_body = res.text_with_charset("utf-8");
    let Ok(inner_body) = inner_body else {
        return Err(format!("Error: failed to get text from request's reply. Err: {}", inner_body.err().unwrap()));
    };

    match inner_body.as_str() {
        "OK" => (),
        e => return Err(format!("Error from server: {e}")),
    }

    let mut has_error = false;

    for test_name in tests_to_execute {
        if test_setup.run_tests_on_qemu {
            report_test_start(&test_name, task_id, "Qemu")?;
            if is_immediate_exit_requested() {
                has_error = true;
                let _ = report_test_progress(&test_name, task_id, "Qemu", "Not executing the test since the user requested to stop the worker immediately");
                let _ = report_test_finished(&test_name, task_id, "Qemu", FinishStatus::Failed(4));
            } else {
                let (tx, rx) = std::sync::mpsc::channel();
                let test_name_regexp = format!("^{test_name}$");
                let proc = std::process::Command::new(PathBuf::from("ctest").as_os_str())
                    .args(&[
                        "--test-dir",
                        build_dir_str,
                        "--verbose",
                        "--no-tests=error",
                        "--tests-regex",
                        &test_name_regexp])
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .unwrap();

                let thread_handle = std::thread::spawn(|| crate::run_command::push_messages(proc, tx));

                let mut has_timed_out = false;
                // todo: hardcoded string. Corresponds to the scripts that gets executed through ctest.
                let timeout_msg = "/usr/bin/timeout: sending signal TERM to command";
                loop {
                    match rx.recv() {
                        Ok(msg) => {
                            let mut msg = crate::run_command::prepend_channel(msg);
                            has_timed_out = has_timed_out || msg.contains(timeout_msg);

                            while let Ok(msg1) = rx.try_recv() {
                                let with_channel = crate::run_command::prepend_channel(msg1);
                                has_timed_out = has_timed_out || with_channel.contains(timeout_msg);
                                msg += with_channel.as_str();
                            }
                            report_test_progress(&test_name, task_id, "Qemu", msg.as_str()).expect("failed to update test");
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }

                let task_output = thread_handle.join().unwrap();
                let finish_status = if has_timed_out {
                    FinishStatus::Timeout
                } else {
                    if is_immediate_exit_requested() {
                        FinishStatus::Failed(3) // do not send skipped. Skipped is used to tell can't be executed
                        // on the hardware used, e.g. if it requires real hardware and is executed on qemu.
                    } else {
                        match task_output.code() {
                            None => { FinishStatus::Failed(2) }
                            Some(124) => {
                                has_timed_out = true;
                                has_error = true;
                                FinishStatus::Timeout
                            }
                            Some(0) => { FinishStatus::Success }
                            Some(n) => {
                                has_error = true;
                                FinishStatus::Failed(i64::from(n))
                            }
                        }
                    }
                };

                let _ = report_test_finished(&test_name, task_id, "Qemu", finish_status);
            }
        }

        if test_setup.run_tests_on_real_hardware {
            report_test_start(&test_name, task_id, "RealHardware")?;
            report_test_finished(&test_name, task_id, "RealHardware", FinishStatus::Skipped)?;
        }
    }

    let end_status = if has_error { FinishStatus::Failed(2) } else { FinishStatus::Success };
    let msg = format!("Done with task {task_id}");
    report_task_data(task_id, msg.as_str())?;
    Ok(end_status)
}

fn get_tests_to_execute(test_setup: &&TestSetup, available_tests: &Vec<&str>) -> Vec<String> {
    let tests_to_execute = match &test_setup.tests_to_run {
        RequestedTest::AllTest => {
            available_tests
                .iter()
                .map(|x| String::from(*x))
                .collect::<Vec<_>>()
        }
        RequestedTest::NoTestsOnlyCompile => panic!(),
        RequestedTest::AllExcept(x) => {
            let is_refused = |a: &str| x.iter().any(|x| x.eq(a));
            available_tests
                .iter()
                .filter(|x| !is_refused(x))
                .map(|x| String::from(*x))
                .collect::<Vec<_>>()
        }
        RequestedTest::OnlySpecifiedTests(x) => { x.clone() }
    };

    let unique_tests_to_execute = tests_to_execute
        .into_iter()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    unique_tests_to_execute
}

fn ensure_all_requested_tests_are_available(task_id: i64, available_tests: &Vec<&str>, some_tests: &Vec<String>) -> Result<(), String> {
    let is_available_test = |x: &str| available_tests
        .iter()
        .any(|a| x.eq(*a));

    let unknown_tests = some_tests
        .iter()
        .filter(|x| !is_available_test(&x))
        .collect::<Vec<_>>();
    if !unknown_tests.is_empty() {
        let err_msg = unknown_tests.iter()
            .map(|a| String::from(*a))
            .reduce(|a, b| format!("{a}\n{b}"))
            .unwrap();
        let msg = format!("Error: following tests requested but not found: [\n{err_msg}\n]");
        report_task_data(task_id, msg.as_str())?;
        Err(msg)
    } else {
        Ok(())
    }
}

fn get_available_tests<'a>(task_id: i64, available_tests: &'a Cow<'a, str>) -> Result<Vec<&'a str>, String> {
    let available_tests = available_tests
        .lines()
        .filter(|x| x.starts_with("  Test "))
        .map(|x| {
            let splitted = x.split_whitespace()
                .skip(2)
                .take(1)
                .collect::<Vec<_>>();
            let name = splitted.last().unwrap();
            name.clone()
        })
        .collect::<Vec<_>>();

    let available_tests_str = available_tests
        .iter()
        .map(|a| String::from(*a))
        .reduce(|a, b| format!("{a}\n{b}"))
        .unwrap();
    let available_tests_str = format!("Found following tests: [\n{available_tests_str}\n]");
    report_task_data(task_id, available_tests_str.as_str())?;
    drop(available_tests_str);
    Ok(available_tests)
}
