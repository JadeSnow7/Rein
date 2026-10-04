//! Fixed compile-and-run verification. Only a binary built in this run is executed.

use std::fs;
use std::io::ErrorKind as IoErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::{ErrorKind, HelloError, HelloResult};
use crate::process::{returncode, run_with_timeout};
use crate::workspace::{is_symlink, safe_read};

pub const COMPILE_TIMEOUT: Duration = Duration::from_secs(10);
pub const RUN_TIMEOUT: Duration = Duration::from_secs(2);
pub const EXPECTED_OUTPUT: &str = "Hello, world!\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub command: Vec<String>,
    pub compile_returncode: i32,
    pub run_returncode: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub passed: bool,
    pub run_timed_out: bool,
}

fn log_section(title: &str, code: &str, stdout: &str, stderr: &str) -> String {
    format!("{title} exit={code}\nstdout:\n{stdout}stderr:\n{stderr}")
}

/// A private temporary directory, removed when dropped.
struct TempDir(PathBuf);

impl TempDir {
    fn create() -> HelloResult<TempDir> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or_default();
        let path = std::env::temp_dir().join(format!("hello-world-{}-{nanos}", std::process::id()));
        fs::create_dir(&path).map_err(|e| HelloError::new(ErrorKind::WriteFailed, e.to_string()))?;
        Ok(TempDir(path))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn check_cpp(workspace: &Path, run_timeout: Duration) -> HelloResult<CheckResult> {
    safe_read(workspace, "hello.cpp")?;
    let log_path = workspace.join("compiler.log");
    if is_symlink(&log_path) || (log_path.exists() && !log_path.is_file()) {
        return Err(HelloError::new(ErrorKind::PathInvalid, "compiler.log must be a regular file"));
    }
    let temp = TempDir::create()?;
    let binary = temp.0.join("hello");
    let command: Vec<String> = ["c++", "-std=c++17", "hello.cpp", "-o"]
        .iter()
        .map(|s| s.to_string())
        .chain([binary.display().to_string()])
        .collect();
    let compiled =
        run_with_timeout(Command::new(&command[0]).args(&command[1..]).current_dir(workspace), COMPILE_TIMEOUT)
            .map_err(|e| match e.kind() {
                IoErrorKind::NotFound => HelloError::new(ErrorKind::CompilerMissing, "c++ was not found"),
                _ => HelloError::new(ErrorKind::CompilerMissing, e.to_string()),
            })?;
    let status =
        compiled.status.ok_or_else(|| HelloError::new(ErrorKind::CompileTimeout, "C++ compilation timed out"))?;
    let compile_code = returncode(status);
    let lossy = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
    let compile_log =
        log_section("compile", &compile_code.to_string(), &lossy(&compiled.stdout), &lossy(&compiled.stderr));
    let write_log =
        |text: &str| fs::write(&log_path, text).map_err(|e| HelloError::new(ErrorKind::WriteFailed, e.to_string()));
    write_log(&compile_log)?;
    if compile_code != 0 {
        return Ok(CheckResult {
            command,
            compile_returncode: compile_code,
            run_returncode: None,
            stdout: String::new(),
            stderr: compile_log,
            passed: false,
            run_timed_out: false,
        });
    }
    let run = run_with_timeout(Command::new(&binary).current_dir(workspace), run_timeout)
        .map_err(|e| HelloError::new(ErrorKind::CompilerMissing, e.to_string()))?;
    let stdout = lossy(&run.stdout);
    let mut stderr = lossy(&run.stderr);
    let run_code = run.status.map(returncode);
    let timed_out = run_code.is_none();
    if timed_out {
        stderr.push_str("execution timed out");
    }
    // The run result becomes evidence for the next diagnosis as well.
    let code_text = run_code.map(|c| c.to_string()).unwrap_or_else(|| "timeout".to_string());
    write_log(&(compile_log + &log_section("run", &code_text, &stdout, &stderr)))?;
    let passed = !timed_out && run_code == Some(0) && stdout == EXPECTED_OUTPUT;
    Ok(CheckResult {
        command,
        compile_returncode: compile_code,
        run_returncode: run_code,
        stdout,
        stderr,
        passed,
        run_timed_out: timed_out,
    })
}
