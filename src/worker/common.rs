use std::cell::OnceCell;
use std::fmt::{Debug, Formatter};
use std::string::String;
use std::ptr::hash;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use serde::de::Unexpected::Str;
use tokio::fs::read_to_string;

#[derive(Debug)]
pub(crate) enum Compiler {
    GccFromHardwareVendor,
    GccFromDistro,
}

#[derive(Debug, PartialEq)]
pub(crate) enum RequestedTest {
    AllTest,
    NoTestsOnlyCompile,
    AllExcept(Vec<String>),
    OnlySpecifiedTests(Vec<String>),
}

#[derive(Debug)]
pub(crate) struct TestSetup {
    pub test_setup_id: i64,
    pub compiler: Compiler,
    pub tests_to_run: RequestedTest,
    pub run_tests_on_qemu: bool,
    pub run_tests_on_real_hardware: bool,
}

#[derive(Debug)]
pub(crate) enum TaskKind {
    StaticAnalyser,
    ClangTidy,
    ClangFormat,
    Test(TestSetup),
}

#[derive(Debug)]
pub(crate) struct Task {
    id: i64,
    git_hash: String,
    task_type: TaskKind,

}

// useful to avoid re cloning the project for each build job.
pub(crate) const FOLDER_CONTAINING_A_GIT_DIR_TO_USE_AS_A_GIT_CACHE: &'static str = "/tmp/path/to/a/git/dir";
pub(crate) const MINICI_SERVER_REQUEST_URL: &'static str = "http://localhost:3000/request_task";
pub(crate) const MINICI_SERVER_REPORT_TEST_CHANGE: &'static str = "http://localhost:3000/report_test_change";
pub(crate) const MINICI_SERVER_ADD_TEST_TEST_LIST: &'static str = "http://localhost:3000/add_test_list_to_job";

const HOSTNAME: &'static str = "TODO_RETRIEVE_REAL_HOSTNAME";
pub(crate) const WORKER_CAPABILITIES: [(&'static str, &'static str);8] =
[("hostname", HOSTNAME),
("accept_static_analyser_task", "true"), // todo: hardcoded capabilities
("accept_clang_tidy_task", "true"),
("accept_clang_format_task", "true"),
("accept_compile_with_gcc_from_hardware_vendor", "true"),
("accept_compile_with_gcc_from_distro", "true"),
("accept_run_tests_on_qemu", "true"),
("accept_run_tests_on_real_hardware", "true")];

pub(crate) const STATIC_ANALYSER_SCRIPT_IN_TESTED_PROJECT: &'static str = "scripts/run_static_analyser.sh";
pub(crate) const RUN_CLANG_TIDY_SCRIPT_IN_TESTED_PROJECT: &'static str = "scripts/run_clang_tidy.sh";

impl Task {
    pub(crate) fn from_str(text: &str) -> Result<Self, String> {
        let lines = text
            .split("\r\n")
            .map(|x| x.split("\n"))
            .flatten()
            .collect::<Vec<_>>();

        if lines.len() < 3 {
            return Err(format!("Could not decode a task from input [{text}]. Need at least task id, commit, and task type"));
        }

        let line = lines[0];
        let res_id = if line.starts_with("Task id:") {
            let id = line.split(":")
                .nth(1);
            let Some(id) = id else {
                return Err(format!("Could not decode a task. No id after task id {text}"));
            };
            let id = id.trim();
            let id = id.parse::<i64>();
            let Ok(id) = id else {
                return Err(format!("Could not parse the id from task {text}, err: {}", id.err().unwrap()));
            };

            id
        } else {
            return Err(format!("Invalid task description {text}, shall start by task id"));
        };

        let line = lines[1];
        let git_hash = if line.starts_with("Git Hash:") {
            let hash = line.split(":")
                .nth(1);
            let Some(hash) = hash else {
                return Err(format!("Could not decode the git hash. No text after Git Hash in {text}"));
            };
            let hash = hash.trim();
            if !is_valid_git_hash(hash) {
                return Err(format!("Could not parse the git hash from task {text}"));
            };

            String::from(hash)
        } else {
            return Err(format!("Invalid task description {text}, shall start by task id"));
        };

        let task_kind = match lines[2] {
            "Type: StaticAnalyser" => TaskKind::StaticAnalyser,
            "Type: ClangTidy" => TaskKind::ClangTidy,
            "Type: ClangFormat" => TaskKind::ClangFormat,
            "Type: Tests" => {
                let test_setup = get_test_setup(lines[3..].as_ref())?;
                TaskKind::Test(test_setup)
            }
            _ => return Err(format!("Can't extract the task kind from {text}"))
        };

        let res = Task { id: res_id, git_hash, task_type: task_kind };
        Ok(res)
    }
    pub fn id(&self) -> i64 {
        self.id
    }
    pub fn git_hash(&self) -> &str {
        &self.git_hash
    }
    pub fn task_type(&self) -> &TaskKind {
        &self.task_type
    }
}

fn get_test_setup(lines: &[&str]) -> Result<TestSetup, String> {
    let nr_lines = lines.len();
    if nr_lines < 2 {
        return Err(String::from("Can't extract the test kind from the task"));
    }

    let test_setup_id = match lines[0] {
        x if x.starts_with("Test setup id: ") => {
            let id = x.split_whitespace()
                .into_iter()
                .nth(3)
                .unwrap();
            id.parse::<i64>().unwrap()
        }
        x => return Err(format!("Can't extract the test setup id from {x}"))
    };

    let requested_tests = match lines[1] {
        "Test type: NoTestOnlyCompile" => RequestedTest::NoTestsOnlyCompile,
        "Test type: AllTests" => RequestedTest::AllTest,
        x if x.starts_with("Test type: AllTestExcept(\"") && x.ends_with("\")") => {
            let to_filter_out = get_lines_from("Test type: AllTestExcept(\"", "\")", x)?;
            RequestedTest::AllExcept(to_filter_out)
        }
        x if x.starts_with("Test type: OnlySpecifiedTests(\"") && x.ends_with("\")") => {
            let to_keep = get_lines_from("Test type: OnlySpecifiedTests(\"", "\")", x)?;
            RequestedTest::OnlySpecifiedTests(to_keep)
        }
        x => return Err(format!("Can't extract the test kind from {x}"))
    };

    let compiler = match lines[2] {
        "Compiler: GccFromHardwareVendor" => Compiler::GccFromHardwareVendor,
        "Compiler: GccFromDistro" => Compiler::GccFromDistro,
        x => return Err(format!("Can't extract the compiler required from {x}")),
    };

    if requested_tests == RequestedTest::NoTestsOnlyCompile {
        return Ok(TestSetup { test_setup_id, tests_to_run: requested_tests, compiler, run_tests_on_qemu: false, run_tests_on_real_hardware: false });
    }

    if nr_lines < 5 {
        return Err(String::from("Can't extract if the tests has to run on qemu or real hardware"));
    }

    let run_on_qemu = lines[3..]
        .iter()
        .filter(|x| x.starts_with("Run tests on qemu: "))
        .take(1)
        .collect::<Vec<_>>();
    if run_on_qemu.len() != 1 {
        return Err(String::from("Can't extract if the tests has to run on qemu"));
    }
    let run_on_qemu = run_on_qemu[0].ends_with("true");

    let run_on_real_hardware = lines[3..]
        .iter()
        .filter(|x| x.starts_with("Run tests on real hardware: "))
        .take(1)
        .collect::<Vec<_>>();
    if run_on_real_hardware.len() != 1 {
        return Err(String::from("Can't extract if the tests has to run on real hardware"));
    }
    let run_on_real_hardware = run_on_real_hardware[0].ends_with("true");

    Ok(TestSetup { test_setup_id, tests_to_run: requested_tests, compiler, run_tests_on_qemu: run_on_qemu, run_tests_on_real_hardware: run_on_real_hardware })
}

fn get_lines_from(prefix: &str, suffix: &str, encoded_lines: &str) -> Result<Vec<String>, String> {
    let inner = encoded_lines.strip_prefix(prefix);
    let Some(inner) = inner else {
        return Err(format!("Error: can't extract lines from [{encoded_lines}] because it does not starts with prefix [${prefix}]"));
    };
    let inner = inner.strip_suffix(suffix);
    let Some(inner) = inner else {
        return Err(format!("Error: can't extract lines from [{encoded_lines}] because it does not ends with suffix [${prefix}]"));
    };

    let lines = inner
        .split("\\r\\n")
        .map(|x| x.split_whitespace())
        .flatten()
        .map(|x| String::from(x))
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Err(format!("Error: no lines got extracted from [{encoded_lines}]. Use AllTests or NoTestOnlyCompile if this is what you meant."));
    }
    Ok(lines)
}

pub(crate) fn is_valid_git_hash(hash: &str) -> bool {
    let l = hash.len();
    (2 < l) && (l <= 64) // with the transition to sha3-256, hashes can go to 64 hexa chars
        && hash.chars().all(|c| c.is_ascii_hexdigit())
}


pub(crate) fn report_task_data(task_id: i64, msg_str: &str) -> Result<(), String> {
    let client = reqwest::blocking::Client::new();
    let res = client
        .post("http://localhost:3000/update_task") // todo: hardcoded URL
        .form(&[("task_id", format!("{task_id}")),
            ("return_status", String::from("Running")),
            ("output", String::from(msg_str))])
        .send();
    let Ok(res) = res else {
        return Err(format!("failed update task with id: {task_id}, err:{}", res.err().unwrap()));
    };

    let inner_body = res.text_with_charset("utf-8");
    let Ok(inner_body) = inner_body else {
        return Err(format!("Error: failed to get text from request's reply. Err: {}", inner_body.err().unwrap()));
    };

    match inner_body.as_str() {
        "OK" => Ok(()),
        e => Err(format!("Error from server: {e}"))
    }
}

pub(crate) fn report_task_error(task_id: i64, err_str: &str, ret_code: i64) -> Result<(), String> {
    println!("Reporting task error: [{err_str}]");

    let client = reqwest::blocking::Client::new();
    let res = client
        .post("http://localhost:3000/update_task") // todo: hardcoded URL
        .form(&[("task_id", format!("{task_id}")),
            ("return_status", String::from("Failed")),
            ("ret_code", format!("{ret_code}")),
            ("output", String::from(err_str))])
        .send();
    let Ok(res) = res else {
        let err_msg = format!("failed update task with id: {task_id}, err:{}", res.err().unwrap());
        println!("Failed to report error: [{err_msg}]");
        return Err(err_msg);
    };

    let inner_body = res.text_with_charset("utf-8");
    let Ok(inner_body) = inner_body else {
        let err_msg = format!("Error: failed to get text from request's reply. Err: {}", inner_body.err().unwrap());
        println!("error {err_msg}");
        return Err(err_msg);
    };

    println!("Reported task error done. Answer from server: [{e}]", e = inner_body.as_str());

    match inner_body.as_str() {
        "OK" => Ok(()),
        e => Err(format!("Error from server: {e}"))
    }
}

pub(crate) enum FinishStatus {
    Success,
    Failed(i64),
    Timeout,
    Skipped,
}

impl FinishStatus {
    pub fn from_i32(ret_code: i32) -> FinishStatus {
        match ret_code {
            0 => FinishStatus::Success,
            124 => FinishStatus::Timeout,
            _ => FinishStatus::Failed(i64::from(ret_code))
        }
    }
}

impl Debug for FinishStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            FinishStatus::Success => { "Success" }
            FinishStatus::Failed(_) => { "Failed" }
            FinishStatus::Timeout => { "Timeout" }
            FinishStatus::Skipped => { "Skipped" }
        };
        write!(f, "{}", s)
    }
}

pub(crate) fn report_task_finish(task_id: i64, msg_str: &str, end_status: FinishStatus) -> Result<(), String> {
    println!("Reporting task finished: {msg_str}");

    let ret_code_str = match end_status {
        FinishStatus::Skipped
        | FinishStatus::Success => { String::from("0") }
        FinishStatus::Failed(e) => { format!("{e}") }
        FinishStatus::Timeout => { String::from("124") }
    };

    let client = reqwest::blocking::Client::new();
    let res = client
        .post("http://localhost:3000/update_task") // todo: hardcoded URL
        .form(&[("task_id", format!("{task_id}")),
            ("return_status", format!("{end_status:?}")),
            ("ret_code", ret_code_str),
            ("output", String::from(msg_str))])
        .send();
    let Ok(res) = res else {
        return Err(format!("failed update task with id: {task_id}, err:{}", res.err().unwrap()));
    };

    let inner_body = res.text_with_charset("utf-8");
    let Ok(inner_body) = inner_body else {
        return Err(format!("Error: failed to get text from request's reply. Err: {}", inner_body.err().unwrap()));
    };

    match inner_body.as_str() {
        "OK" => Ok(()),
        e => Err(format!("Error from server: {e}"))
    }
}

pub(crate) fn report_task_started(task_id: i64) -> Result<(), String> {
    let task_id = format!("{task_id}");
    let client = reqwest::blocking::Client::new();
    let res = client
        .post("http://localhost:3000/update_task") // todo: hardcoded URL
        .form(&[("task_id", task_id.as_str()),
            ("return_status", "Running"),
            ("output", "")])
        .send();
    let Ok(res) = res else {
        return Err(format!("failed update task with id: {task_id}, err:{}", res.err().unwrap()));
    };

    let inner_body = res.text_with_charset("utf-8");
    let Ok(inner_body) = inner_body else {
        return Err(format!("Error: failed to get text from request's reply. Err: {}", inner_body.err().unwrap()));
    };

    match inner_body.as_str() {
        "OK" => Ok(()),
        e => Err(format!("Error from server: {e}"))
    }
}

pub static mut TERM: OnceCell<AtomicU8> = OnceCell::new();

pub fn get_exit_request_counter() -> u8 {
    let counter = unsafe { TERM.get() };
    let Some(counter) = counter else { panic!() };
    let counter = counter.load(Ordering::SeqCst);
    counter
}

pub fn is_exit_requested() -> bool {
    let counter = get_exit_request_counter();
    let res = counter > 0;
    res
}

pub fn is_immediate_exit_requested() -> bool {
    let counter = get_exit_request_counter();
    let res = counter > 1;
    res
}