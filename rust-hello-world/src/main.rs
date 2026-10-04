//! `rein-hello check | tool | edit` — the same observable behavior as the Python CLI.

use std::io::{self, BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use rein_hello_core::{
    apply_candidate, check_cpp, dispatch_tool, render_review, safe_read, ApplyOutcome, CheckResult, Color, Decision,
    EditOutcome, HelloError, Proposal, ToolCall,
};
use serde::Serialize;

#[derive(Serialize)]
struct CheckReport<'a> {
    passed: bool,
    compile_returncode: i32,
    run_returncode: Option<i32>,
    stdout: &'a str,
    stderr: &'a str,
}

fn print_check(result: &CheckResult) {
    let report = CheckReport {
        passed: result.passed,
        compile_returncode: result.compile_returncode,
        run_returncode: result.run_returncode,
        stdout: &result.stdout,
        stderr: &result.stderr,
    };
    println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
}

enum Cli {
    Check { workspace: PathBuf },
    Tool { workspace: PathBuf, call: String },
    Edit { workspace: PathBuf, proposal: PathBuf, color: Color },
}

const USAGE: &str = "usage: rein-hello check --workspace DIR
       rein-hello tool --workspace DIR --call JSON
       rein-hello edit --workspace DIR --proposal FILE [--color auto|always|never]";

fn parse(args: &[String]) -> Option<Cli> {
    let option = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let workspace = PathBuf::from(option("--workspace")?);
    match args.first()?.as_str() {
        "check" => Some(Cli::Check { workspace }),
        "tool" => Some(Cli::Tool { workspace, call: option("--call")? }),
        "edit" => {
            let color = Color::parse(&option("--color").unwrap_or_else(|| "auto".to_string()))?;
            Some(Cli::Edit { workspace, proposal: PathBuf::from(option("--proposal")?), color })
        }
        _ => None,
    }
}

fn run_edit(workspace: &Path, proposal: &Proposal, color: Color) -> Result<EditOutcome, HelloError> {
    let original = safe_read(workspace, "hello.cpp")?;
    println!("{}", render_review(&original.text, &proposal.code, color));
    print!("接受修改？[y/N] ");
    let _ = io::stdout().flush();
    let mut answer = String::new();
    let _ = io::stdin().lock().read_line(&mut answer); // EOF leaves the answer empty: reject
    if !io::stdin().is_terminal() {
        println!(); // keep the status on its own line when the answer was piped in
    }
    match apply_candidate(workspace, &proposal.code, &proposal.source_digest, Decision::from_answer(&answer))? {
        ApplyOutcome::Rejected => {
            println!("rejected");
            Ok(EditOutcome::Rejected)
        }
        ApplyOutcome::Applied { backup } => {
            println!("accepted");
            println!("backup={}", backup.display());
            let check = check_cpp(workspace, rein_hello_core::check::RUN_TIMEOUT)?;
            print_check(&check);
            Ok(EditOutcome::Verified { backup, check })
        }
    }
}

fn run(cli: Cli) -> Result<bool, HelloError> {
    match cli {
        Cli::Check { workspace } => {
            let result = check_cpp(&workspace, rein_hello_core::check::RUN_TIMEOUT)?;
            print_check(&result);
            Ok(result.passed)
        }
        Cli::Tool { workspace, call } => {
            let value: serde_json::Value = serde_json::from_str(&call)
                .map_err(|_| HelloError::new(rein_hello_core::ErrorKind::ToolInvalid, "tool call is not JSON"))?;
            let result = dispatch_tool(&workspace, &ToolCall::from_value(&value))?;
            println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
            Ok(true)
        }
        Cli::Edit { workspace, proposal, color } => {
            let text = std::fs::read_to_string(&proposal)
                .map_err(|e| HelloError::new(rein_hello_core::ErrorKind::ResponseInvalid, e.to_string()))?;
            let outcome = run_edit(&workspace, &Proposal::from_json(&text)?, color)?;
            Ok(match outcome {
                EditOutcome::Rejected => true,
                EditOutcome::Verified { check, .. } => check.passed,
            })
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(cli) = parse(&args) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match run(cli) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(error) => {
            let report = serde_json::json!({"status": error.kind.code(), "detail": error.detail});
            eprintln!("{report}");
            ExitCode::from(1)
        }
    }
}
