extern crate libc;

use libc::{fcntl, F_GETFL, F_SETFL, O_NONBLOCK};

use nix::unistd::Pid;
use nix::sys::signal::{self, Signal};
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::io::{BufRead, BufReader, Read};
use std::os::fd::AsRawFd;
use std::os::unix::process::ExitStatusExt;
use std::os::unix::raw::pid_t;
use std::process::{Child, ExitStatus};
use std::sync::mpsc::{RecvError, Sender};
use std::thread::sleep;
use std::time::Duration;
use kill_tree::blocking::{kill_tree, kill_tree_with_config};
use nix::errno::Errno;
use nix::errno::Errno::ESRCH;
use tracing::error;
use crate::common::{is_immediate_exit_requested, report_task_data, report_task_error};

#[derive(Clone, Copy)]
enum ChannelTag {
    STDOUT,
    STDERR,
}

pub enum Message {
    STDOUT(String),
    STDERR(String),
}

fn is_process_running(child: &mut std::process::Child) -> bool {
    match child.try_wait() {
        Ok(Some(_)) => false,
        Ok(None) => true,
        Err(e) => {
            println!("Failed to check if process {a} is still running. Error is [{e:?}] Assuming it isn't.",
                     a = child.id()
            );
            false
        }
    }
}

fn push_one_line_of_leftover(
    leftovers: &mut VecDeque<u8>,
    tag: ChannelTag,
    channel: &std::sync::mpsc::Sender<Message>,
) {
    let mut line = Default::default();

    if let Ok(num) = leftovers.read_line(&mut line) {
        if num > 0 {
            let line = if line.ends_with("\n") {
                line[0..(line.len() - 1)].as_ref()
            } else {
                line.as_str()
            };
            let line = String::from_utf8_lossy(line.as_bytes()).into_owned();
            match tag {
                ChannelTag::STDOUT => channel.send(Message::STDOUT(line)),
                ChannelTag::STDERR => channel.send(Message::STDERR(line)),
            }
                .expect("sending message in channel failed")
        }
    }
}

fn add_to_leftovers<T: Read>(
    leftovers: &mut VecDeque<u8>,
    buf_reader: &mut BufReader<T>,
    tag: ChannelTag,
    channel: &std::sync::mpsc::Sender<Message>,
) {
    if let Ok(true) = buf_reader.has_data_left() {
        let nr_bytes_available = buf_reader.buffer().len();
        if nr_bytes_available == 0 {
            return;
        }
        let mut buf_reader_data = Vec::with_capacity(nr_bytes_available);
        let _ = buf_reader
            .by_ref()
            .take(nr_bytes_available as u64)
            .read_to_end(&mut buf_reader_data)
            .unwrap();
        for byte in buf_reader_data.into_iter() {
            leftovers.push_back(byte);
        }

        while leftovers.contains(&('\n' as u8)) {
            push_one_line_of_leftover(leftovers, tag, channel);
        }
    }
}

fn kill_process_group(pid_to_kill: u32, signal: String) {
  kill_tree_with_config(pid_to_kill, &kill_tree::Config { signal, include_target: true }).unwrap();
}

pub fn push_messages(mut proc: Child, tx: Sender<Message>) -> ExitStatus {
    let mut stdout_leftovers: VecDeque<u8> = Default::default();
    let mut stderr_leftovers: VecDeque<u8> = Default::default();

    let stdout = proc.stdout.take().unwrap();
    let stderr = proc.stderr.take().unwrap();

    set_nonblocking(&stdout, true).unwrap();
    set_nonblocking(&stderr, true).unwrap();

    let mut stdout = BufReader::new(stdout);
    let mut stderr = BufReader::new(stderr);

    let mut was_process_manually_stopped = false;
    if is_process_running(&mut proc) {
        loop {
            add_to_leftovers(&mut stdout_leftovers, &mut stdout, ChannelTag::STDOUT, &tx);
            add_to_leftovers(&mut stderr_leftovers, &mut stderr, ChannelTag::STDERR, &tx);
            if is_process_running(&mut proc) {
                if is_immediate_exit_requested() {
                    kill_process_group(proc.id(), String::from("SIGTERM"));
                    was_process_manually_stopped = true;
                    break;
                }
                sleep(Duration::from_millis(10)) // don't suck 100% CPU
            } else {
                break;
            }
        }
    }

    if !stdout_leftovers.is_empty() {
        push_one_line_of_leftover(&mut stdout_leftovers, ChannelTag::STDOUT, &tx);
    }

    if !stderr_leftovers.is_empty() {
        push_one_line_of_leftover(&mut stderr_leftovers, ChannelTag::STDERR, &tx);
    }

    // we can exit the loop in two conditions:
    // 1. the process finished by itself
    // 2. we got asked to stop immediately. In which case we sent the term signal to the process
    if is_process_running(&mut proc) && is_immediate_exit_requested() {
        // if we get here, it means the process is still doing its cleanup.
        // give a bit a leeway to the process to handle its termination.

        // wait up to 50 ms for the process to finish its cleanup
        for _ in 1..=10 {
            if is_process_running(&mut proc) {
                sleep(Duration::from_millis(5));
            }
        }
        // we gave what should be more than enough for a process to cleanup
        if is_process_running(&mut proc) {
            // just kill it
            kill_process_group(proc.id(), String::from("SIGKILL"));
            was_process_manually_stopped = true; // shouldn't be necessary
        }
    }

    if was_process_manually_stopped {
        tx.send(Message::STDERR(String::from("Stopping process due to user request to stop the worker"))).expect("failed to send message into channel");
    }
    drop(tx);

    let ret_code = proc.wait().unwrap();

    ret_code
}

// taken from https://stackoverflow.com/a/66292796
fn set_nonblocking<H>(handle: &H, nonblocking: bool) -> std::io::Result<()>
    where
        H: Read + AsRawFd,
{
    let fd = handle.as_raw_fd();
    let flags = unsafe { fcntl(fd, F_GETFL, 0) };
    if flags < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let flags = if nonblocking {
        flags | O_NONBLOCK
    } else {
        flags & !O_NONBLOCK
    };
    let res = unsafe { fcntl(fd, F_SETFL, flags) };
    if res != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

pub fn prepend_channel(msg: Message) -> String {
    match msg {
        Message::STDOUT(s) => {
            format!("stdout: {s}\n")
        }
        Message::STDERR(s) => {
            format!("stderr: {s}\n")
        }
    }
}

pub fn run_proc(task_id: i64, command: &OsStr, params: &[&str]) -> ExitStatus {
    if is_immediate_exit_requested() {
        return ExitStatus::from_raw(3);
    }

    let proc = std::process::Command::new(command)
        .args(params)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    let Ok(proc) = proc else {
        let err = proc.err().unwrap();
        let err_msg = format!("{err}");
        report_task_error(task_id, err_msg.as_str(), 2).expect("Err: failed to report error");
        return ExitStatus::from_raw(3);
    };


    let (tx, rx) = std::sync::mpsc::channel();
    let thread_handle = std::thread::spawn(|| push_messages(proc, tx));

    loop {
        match rx.recv() {
            Ok(msg) => {
                let mut msg = prepend_channel(msg);
                while let Ok(msg1) = rx.try_recv() {
                    msg += prepend_channel(msg1).as_str();
                }
                report_task_data(task_id, msg.as_str()).expect("Error occurred when reporting data");
            }
            Err(_) => {
                break;
            }
        }
    }

    thread_handle.join().unwrap()
}