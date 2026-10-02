//! Behavior checks that mirror the Python B03–B05 samples. Run with `cargo test`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use rein_hello_core::{
    apply_candidate, check_cpp, dispatch_tool, safe_read, unified_diff, ApplyOutcome, Decision, ErrorKind, ToolCall,
};
use serde_json::json;

const BROKEN: &str = "#include <iostream>\n\nint main() {\n    std::cout << \"Hello, world!\\n\"\n}\n";
const FIXED: &str = "#include <iostream>\n\nint main() {\n    std::cout << \"Hello, world!\\n\";\n}\n";

struct Workspace(PathBuf);

impl Workspace {
    fn new(source: &str) -> Workspace {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("rein-hello-test-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create workspace");
        fs::write(path.join("hello.cpp"), source).expect("write source");
        fs::write(path.join("compiler.log"), "error: expected ';'\n").expect("write log");
        Workspace(path)
    }
    fn path(&self) -> &Path {
        &self.0
    }
    fn backups(&self) -> usize {
        fs::read_dir(&self.0)
            .expect("list")
            .filter(|e| {
                e.as_ref().map(|e| e.file_name().to_string_lossy().starts_with("hello.cpp.bak-")).unwrap_or(false)
            })
            .count()
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn call(name: &str, arguments: serde_json::Value) -> ToolCall {
    ToolCall { id: "t1".into(), name: name.into(), arguments }
}

#[test]
fn b03_read_boundaries() {
    let ws = Workspace::new(BROKEN);
    assert_eq!(safe_read(ws.path(), "hello.cpp").expect("read").text, BROKEN);
    for bad in ["../hello.cpp", "/etc/passwd", "main.cpp"] {
        assert_eq!(safe_read(ws.path(), bad).unwrap_err().kind, ErrorKind::PathInvalid, "{bad}");
    }
    fs::write(ws.path().join("compiler.log"), vec![b'x'; 5000]).expect("write big log");
    assert_eq!(safe_read(ws.path(), "compiler.log").unwrap_err().kind, ErrorKind::FileTooLarge);
    fs::write(ws.path().join("compiler.log"), [0xff, 0xfe]).expect("write bad utf8");
    assert_eq!(safe_read(ws.path(), "compiler.log").unwrap_err().kind, ErrorKind::InvalidUtf8);
}

#[test]
fn b03_tool_requests_are_checked_before_running() {
    let ws = Workspace::new(BROKEN);
    let cases = [
        (call("write_file", json!({})), ErrorKind::ToolUnknown),
        (call("read_file", json!({"path": "hello.cpp", "extra": 1})), ErrorKind::ToolInvalid),
        (call("read_environment", json!({"secret": true})), ErrorKind::ToolInvalid),
        (call("bash", json!({"command": "cat compiler.log"})), ErrorKind::BashCommandInvalid),
        (call("bash", json!({"command": "ls -1; rm hello.cpp"})), ErrorKind::BashCommandInvalid),
    ];
    for (request, expected) in cases {
        assert_eq!(dispatch_tool(ws.path(), &request).unwrap_err().kind, expected, "{request:?}");
    }
    let listing = dispatch_tool(ws.path(), &call("bash", json!({"command": "ls -1"}))).expect("ls");
    assert!(listing["stdout"].as_str().unwrap_or_default().contains("hello.cpp"));
    assert_eq!(fs::read_to_string(ws.path().join("hello.cpp")).expect("source"), BROKEN);
}

#[test]
fn b04_reject_writes_nothing_even_if_source_changed() {
    let ws = Workspace::new(BROKEN);
    let stale = safe_read(ws.path(), "hello.cpp").expect("read").digest;
    fs::write(ws.path().join("hello.cpp"), format!("// edited\n{BROKEN}")).expect("edit");
    let outcome = apply_candidate(ws.path(), FIXED, &stale, Decision::Reject).expect("reject");
    assert_eq!(outcome, ApplyOutcome::Rejected);
    assert_eq!(ws.backups(), 0);
}

#[test]
fn b04_accept_checks_source_then_backs_up_and_writes() {
    let ws = Workspace::new(BROKEN);
    let digest = safe_read(ws.path(), "hello.cpp").expect("read").digest;
    fs::write(ws.path().join("hello.cpp"), format!("// edited\n{BROKEN}")).expect("edit");
    let stale = apply_candidate(ws.path(), FIXED, &digest, Decision::Accept).unwrap_err();
    assert_eq!(stale.kind, ErrorKind::SourceChanged);
    assert_eq!(ws.backups(), 0);

    fs::write(ws.path().join("hello.cpp"), BROKEN).expect("restore");
    let ApplyOutcome::Applied { backup } =
        apply_candidate(ws.path(), FIXED, &digest, Decision::Accept).expect("accept")
    else {
        panic!("expected an applied outcome");
    };
    assert_eq!(fs::read_to_string(backup).expect("backup"), BROKEN);
    assert_eq!(fs::read_to_string(ws.path().join("hello.cpp")).expect("source"), FIXED);
}

#[test]
fn b04_only_explicit_yes_accepts() {
    for answer in ["y", "Y\n", "yes", "接受"] {
        assert_eq!(Decision::from_answer(answer), Decision::Accept, "{answer}");
    }
    for answer in ["", "\n", "n", "no", "maybe"] {
        assert_eq!(Decision::from_answer(answer), Decision::Reject, "{answer}");
    }
}

#[test]
fn b04_diff_matches_python_difflib_for_the_sample() {
    let expected = [
        "--- hello.cpp\n",
        "+++ candidate.cpp\n",
        "@@ -1,5 +1,5 @@\n",
        " #include <iostream>\n",
        " \n",
        " int main() {\n",
        "-    std::cout << \"Hello, world!\\n\"\n",
        "+    std::cout << \"Hello, world!\\n\";\n",
        " }\n",
    ];
    assert_eq!(unified_diff(BROKEN, FIXED, "hello.cpp", "candidate.cpp"), expected);
    assert!(unified_diff(FIXED, FIXED, "a", "b").is_empty());
}

#[test]
fn b05_check_passes_only_for_correct_output() {
    let ws = Workspace::new(FIXED);
    let passed = check_cpp(ws.path(), Duration::from_secs(2)).expect("check");
    assert!(passed.passed, "{passed:?}");

    let broken = Workspace::new(BROKEN);
    let failed = check_cpp(broken.path(), Duration::from_secs(2)).expect("check");
    assert!(!failed.passed && failed.compile_returncode != 0 && failed.run_returncode.is_none());

    let wrong = Workspace::new(&FIXED.replace("Hello, world!", "Hi"));
    let result = check_cpp(wrong.path(), Duration::from_secs(2)).expect("check");
    assert_eq!((result.passed, result.compile_returncode, result.run_returncode), (false, 0, Some(0)));
}
