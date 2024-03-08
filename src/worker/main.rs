#![feature(buf_read_has_data_left)]

mod update_git_repo;
mod common;
mod run_task;
mod run_command;

use std::ffi::OsStr;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};
use reqwest;
use reqwest::header::TE;
use crate::common::{FOLDER_CONTAINING_A_GIT_DIR_TO_USE_AS_A_GIT_CACHE, get_exit_request_counter, is_exit_requested, MINICI_SERVER_REQUEST_URL, Task, TaskKind, TERM, WORKER_CAPABILITIES};
use crate::run_task::run_task;
use crate::update_git_repo::{run_git_clone_in, run_git_remote_update_in};

fn set_signal_handler() -> Result<(), ExitCode> {
    let term = unsafe { TERM.get_or_init(|| { AtomicU8::new(0) }) };
    let action = move || { term.fetch_add(1, Ordering::SeqCst); };
    for signal in [signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT] {
        let signal_setup = unsafe { signal_hook::low_level::register(signal, action) };
        if signal_setup.is_err() {
            println!("Could not setup the signal handler");
            return Err(ExitCode::FAILURE);
        }
    }
    Ok(())
}


fn main() -> ExitCode {
    // set the signal handler, first thing before doing anything else
    set_signal_handler().unwrap();
    std::thread::spawn(|| {
        let mut last_seen_counter = 0;

        loop {
            let current_counter_value = get_exit_request_counter();
            if current_counter_value != last_seen_counter {
                if current_counter_value == 1 {
                    println!("Exit requested. Will wait for the ongoing task to finish.")
                } else {
                    println!("Exit requested. Will cancel ongoing task and finish right now.");
                    return;
                }
            }
            last_seen_counter = current_counter_value;
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    let tmp_git_mirror_dir = temp_dir::TempDir::with_prefix("Dir_for_mini_worker_gir_mirror_");
    let Ok(tmp_git_mirror_dir) = tmp_git_mirror_dir else {
        println!("failed to create a temporary dir: {}", tmp_git_mirror_dir.err().unwrap());
        return ExitCode::from(2);
    };
    println!("Creating directory [{d}] to use as git mirror", d = String::from_utf8_lossy(tmp_git_mirror_dir.path().as_os_str().as_encoded_bytes()));

    if is_exit_requested() { return ExitCode::SUCCESS; };

    let git_mirror_path = tmp_git_mirror_dir.path().as_os_str();
    let success = run_git_clone_in(git_mirror_path, OsStr::new(FOLDER_CONTAINING_A_GIT_DIR_TO_USE_AS_A_GIT_CACHE));
    let Ok(()) = success else {
        println!("Failed to create a git mirror");
        return ExitCode::from(2);
    };

    'wait_loop: loop {
        'work_loop: loop {
            if is_exit_requested() { return ExitCode::SUCCESS; };

            let client = reqwest::blocking::Client::new();
            let res = client
                .post(MINICI_SERVER_REQUEST_URL)
                .form(&WORKER_CAPABILITIES)
                .send();
            let Ok(res) = res else {
                println!("failed to ask for a task: {}", res.err().unwrap());
                break 'work_loop;
            };

            let task_str = res.text_with_charset("utf-8");
            let Ok(task_str) = task_str else {
                println!("Error: failed to get text from request's reply. Err: {}", task_str.err().unwrap());
                break 'work_loop;
            };

            println!("received task: {:?}", task_str);
            let task = Task::from_str(task_str.as_str());
            let Ok(task) = task else {
                println!("Error while parsing task: {}", task.err().unwrap());
                break 'work_loop;
            };

            println!("INFO: task parsed as {task:?}");
            let res = run_task(task, git_mirror_path);
            if let Err(e) = res {
                println!("task failed with {e}");
            }
        }
        // wait before asking a new task to avoid flooding the server with requests
        // but keep checking for the flag telling us to quit
        let max_time_to_sleep = Duration::from_secs(5);
        let instant_before_sleep = Instant::now();
        while (Instant::now() - instant_before_sleep) < max_time_to_sleep {
            sleep(Duration::from_millis(20));
            if is_exit_requested() { return ExitCode::SUCCESS; };
        }
    }
    return ExitCode::SUCCESS;
}
