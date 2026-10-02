use rein_ch01_helloworld::rein::{
    run_maintenance_session, ControlSignal, Extension02Executor, Observation,
};
use serde_json::Value;
use std::{
    env,
    io::{self, Write},
    path::PathBuf,
};

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn command_generator(observation: &Observation) -> Option<String> {
    let package = observation
        .source_contents
        .iter()
        .find(|(path, _)| path == "package.json")?
        .1
        .clone();
    let json: Value = serde_json::from_str(&package).ok()?;
    let scripts = json.get("scripts")?.as_object()?;
    if scripts.len() != 1 {
        return None;
    }
    let only = scripts.keys().next()?.to_owned();
    let mut output = observation.original.clone();
    let mut cursor = 0;
    let mut changed = false;
    while let Some(offset) = output[cursor..].find("npm run ") {
        let start = cursor + offset;
        let name_start = start + "npm run ".len();
        let name_end = output[name_start..]
            .find(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-' && ch != ':')
            .map(|n| name_start + n)
            .unwrap_or(output.len());
        let name = &output[name_start..name_end];
        if !scripts.contains_key(name) {
            output.replace_range(name_start..name_end, &only);
            changed = true;
            cursor = name_start + only.len();
        } else {
            cursor = name_end;
        }
        if cursor >= output.len() {
            break;
        }
    }
    changed.then_some(output)
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let workspace = arg_value(&args, "--workspace")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("current directory"));
    let target = arg_value(&args, "--target").unwrap_or_else(|| "README.md".into());
    let approve_demo = args.iter().any(|arg| arg == "--approve-demo");
    let executor = Extension02Executor::new(&workspace);
    let signal = ControlSignal::new();
    let report = run_maintenance_session(
        &executor,
        &signal,
        &target,
        "command-v1",
        command_generator,
        move |candidate| {
            if approve_demo {
                return Some(candidate.approve());
            }
            eprint!(
                "{}\nApprove replacement for {}? [y/N] ",
                candidate.diff(),
                candidate.path().display()
            );
            let _ = io::stderr().flush();
            let mut answer = String::new();
            io::stdin().read_line(&mut answer).ok()?;
            answer
                .trim()
                .eq_ignore_ascii_case("y")
                .then(|| candidate.approve())
        },
    )
    .await;
    match report {
        Ok(report) => {
            let completed = report.stop_reason
                == rein_ch01_helloworld::rein::maintenance_session::StopReason::Completed;
            let records: Vec<_> = executor
                .records()
                .into_iter()
                .map(|record| {
                    serde_json::json!({
                        "taskId": record.task_id,
                        "requestId": record.request_id,
                        "callId": record.call_id,
                        "prepared": record.prepared,
                        "dispatched": record.dispatched,
                        "terminal": record.terminal,
                        "reason": record.reason,
                        "childPid": record.child_pid,
                        "exitCode": record.exit_code,
                        "reaped": record.reaped,
                    })
                })
                .collect();
            let output = serde_json::json!({
                "mode": "offline_fixture",
                "report": report,
                "records": records,
                "model": null,
                "tokens": null,
                "cost": null,
                "latencyMs": null,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output).expect("report JSON")
            );
            if !completed {
                std::process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("maintenance session failed: {error:?}");
            std::process::exit(1);
        }
    }
}
