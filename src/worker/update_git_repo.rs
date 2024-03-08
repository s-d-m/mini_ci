use crate::common::report_task_data;
use crate::run_command::run_proc;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::ExitStatus;

pub fn run_git_clone_in(dst_dir: &OsStr, git_source_url: &OsStr) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .arg("clone")
        .arg("--mirror")
        .arg("--")
        .arg(git_source_url)
        .arg(dst_dir)
        .output();

    let Ok(output) = output else {
        return Err(format!(
            "fail to run git clone {git_source_url:?} in {dst_dir:?}"
        ));
    };

    println!(
        "stdout: {s}",
        s = String::from_utf8_lossy(output.stdout.as_ref())
    );
    println!(
        "stderr: {s}",
        s = String::from_utf8_lossy(output.stderr.as_ref())
    );

    if !output.status.success() {
        return Err(format!(
            "running git clone failed with {e}, ret_code={c}",
            e = String::from_utf8_lossy(output.stderr.as_ref()),
            c = output.status
        ));
    }

    Ok(())
}

pub fn get_git_checkout_in(
    task_id: i64,
    dst_dir: &OsStr,
    git_source_url: &OsStr,
    commit: &str,
) -> Result<(), String> {
    // todo: use worktree
    let task_output = run_proc(
        task_id,
        PathBuf::from("git").as_os_str(),
        &["clone", git_source_url.to_str().unwrap(),
            dst_dir.to_str().unwrap()],
    );
    if !task_output.success() {
        report_task_data(task_id, "git clone failed")?;
        return Err(format!(
            "fail to run git clone {git_source_url:?} in {dst_dir:?} for commit {commit}"
        ));
    };

    let task_output = run_proc(
        task_id,
        PathBuf::from("git").as_os_str(),
        &[
            "-c",
            "advice.detachedHead=false",
            "-C",
            dst_dir.to_str().unwrap(),
            "checkout",
            commit,
        ],
    );
    if !task_output.success() {
        report_task_data(task_id, "git checkout failed")?;
        return Err(String::from("fail to run git checkout"));
    };

    Ok(())
}

pub fn run_git_remote_update_in(directory_with_git: &OsStr) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(directory_with_git)
        .arg("remote")
        .arg("update")
        .output();

    let Ok(output) = output else {
        return Err(format!(
            "fail to run git remote update in {directory_with_git:?}: error={x}",
            x = output.err().unwrap()
        ));
    };

    println!(
        "stdout: {s}",
        s = String::from_utf8_lossy(output.stdout.as_ref())
    );
    println!(
        "stderr: {s}",
        s = String::from_utf8_lossy(output.stderr.as_ref())
    );

    if !output.status.success() {
        return Err(format!("running git remote update failed with {e}, ret_code={c}, directory={directory_with_git:?}",
                           e = String::from_utf8_lossy(output.stderr.as_ref()), c = output.status));
    }

    Ok(())
}

pub fn get_commit_desc(path_to_git_mirror: &OsStr, git_hash: &str) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(path_to_git_mirror)
        .arg("cat-file")
        .arg("commit")
        .arg(git_hash)
        .output();
    match output {
        Ok(x) if x.status.success() => Ok(String::from_utf8_lossy(x.stdout.as_ref()).into()),
        Ok(x) => Err(format!(
            "git cat-file failed with error={a} and output={b}",
            a = x.status.code().unwrap(),
            b = String::from_utf8_lossy(x.stderr.as_ref())
        )),
        Err(x) => Err(String::from("Failed to run git cat-file")),
    }
}
