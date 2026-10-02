use rein_runtime::model::OfflineModel;
use rein_runtime::verify::FixedVerifier;
use rein_runtime::{schema, ReadFileTool, Runtime, RuntimeConfig};
use std::path::PathBuf;

fn usage() -> ! {
    eprintln!("rein-runtime demo|show|resume|cancel|schema");
    std::process::exit(2)
}
fn take<I: Iterator<Item = String>>(it: &mut I) -> String {
    it.next().unwrap_or_else(|| usage())
}
fn state_arg<I: Iterator<Item = String>>(it: &mut I) -> PathBuf {
    let mut state = None;
    while let Some(arg) = it.next() {
        if arg == "--state-dir" && state.is_none() {
            state = Some(PathBuf::from(take(it)));
        } else {
            usage();
        }
    }
    state.unwrap_or_else(|| usage())
}
fn out(v: serde_json::Value) {
    println!("{}", serde_json::to_string(&v).unwrap());
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut a = std::env::args().skip(1);
    let cmd = a.next().unwrap_or_else(|| usage());
    let result: Result<(), Box<dyn std::error::Error>> = match cmd.as_str() {
        "schema" => {
            let mut d = None;
            let mut check = false;
            while let Some(x) = a.next() {
                match x.as_str() {
                    "--output" if d.is_none() => d = Some(PathBuf::from(take(&mut a))),
                    "--check" if !check => check = true,
                    _ => usage(),
                }
            }
            let d = d.unwrap_or_else(|| usage());
            if check {
                schema::check(&d)?
            } else {
                schema::generate(&d)?
            }
            out(serde_json::json!({"ok":true,"output":d}));
            Ok(())
        }
        "demo" => {
            let mut d = None;
            let mut f = None;
            let mut e = None;
            let mut budget = None;
            let mut prepare = false;
            while let Some(x) = a.next() {
                match x.as_str() {
                    "--state-dir" if d.is_none() => d = Some(PathBuf::from(take(&mut a))),
                    "--fixture" if f.is_none() => f = Some(PathBuf::from(take(&mut a))),
                    "--expected" if e.is_none() => e = Some(PathBuf::from(take(&mut a))),
                    "--budget" if budget.is_none() => budget = Some(take(&mut a).parse::<u32>()?),
                    "--prepare-only" if !prepare => prepare = true,
                    _ => usage(),
                }
            }
            let d = d.unwrap_or_else(|| usage());
            let f = f.unwrap_or_else(|| usage());
            if d.join("state.sqlite").exists() {
                return Err("state already exists".into());
            }
            let expected = e.unwrap_or_else(|| f.clone());
            let mut r = Runtime::open(
                RuntimeConfig::for_fixture(&d, &f),
                OfflineModel::reads_named_fixture(),
                ReadFileTool::rooted_read_only(),
                FixedVerifier::expected_file(expected),
            )?;
            if let Some(n) = budget {
                r.set_tool_budget(n)?
            }
            if prepare {
                r.start()?
            } else {
                let x = r.run_to_acceptance()?;
                out(
                    serde_json::json!({"ok":true,"answer":x.answer,"acceptance":x.acceptance,"session":r.store().load_session()?}),
                );
                return Ok(());
            }
            out(serde_json::json!({"ok":true,"prepared":true,"session":r.store().load_session()?}));
            Ok(())
        }
        "show" => {
            let d = state_arg(&mut a);
            let snapshot = Runtime::inspect(&d)?;
            out(
                serde_json::json!({"ok":true,"session":snapshot.session,"acceptance":snapshot.acceptance,"effects":snapshot.effects}),
            );
            Ok(())
        }
        "resume" => {
            let d = state_arg(&mut a);
            let mut r = Runtime::reopen(&d)?;
            r.recover_explicitly()?;
            r.drive_to_completion()?;
            drop(r);
            let snapshot = Runtime::inspect(&d)?;
            out(
                serde_json::json!({"ok":true,"session":snapshot.session,"acceptance":snapshot.acceptance,"effects":snapshot.effects}),
            );
            Ok(())
        }
        "cancel" => {
            let d = state_arg(&mut a);
            let snapshot = Runtime::cancel_at(&d)?;
            out(
                serde_json::json!({"ok":true,"session":snapshot.session,"acceptance":snapshot.acceptance,"effects":snapshot.effects}),
            );
            Ok(())
        }
        _ => usage(),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1)
    }
    Ok(())
}
