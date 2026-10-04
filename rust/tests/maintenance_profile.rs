use rein_ch01_helloworld::rein::{
    self, extension02, maintenance, ControlSignal, ExecutorResult, Extension02Executor, ToolCall,
    ToolExecutor,
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
static N: AtomicUsize = AtomicUsize::new(0);

struct F(PathBuf);
impl F {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "rein-maint-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir(&p).unwrap();
        fs::write(p.join("README.md"), "npm run missing\n").unwrap();
        fs::write(p.join("package.json"), r#"{"scripts":{"test":"true"}}"#).unwrap();
        Self(p)
    }
}
impl Drop for F {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn verify(path: &str, rule: &str) -> ToolCall {
    ToolCall {
        id: format!("v-{rule}"),
        name: "run_verification".into(),
        arguments: serde_json::json!({"target":path,"rule":rule}),
    }
}
fn apply(c: &maintenance::Candidate) -> ToolCall {
    ToolCall {
        id: "apply".into(),
        name: "apply_patch".into(),
        arguments: serde_json::json!({"path":c.path().to_string_lossy(),"baselineSha256":c.baseline_sha256(),"replacement":c.replacement(),"digest":c.digest()}),
    }
}

#[tokio::test]
async fn maintenance_verification_known_false_then_true_and_apply_bytes() {
    let f = F::new();
    let e = Extension02Executor::new(&f.0);
    e.enable_maintenance("README.md", "command-v1");
    let r = e
        .execute(
            &verify("README.md", "command-v1"),
            &ControlSignal::new(),
            None,
        )
        .await;
    assert!(matches!(r,ExecutorResult::Completed(ref x) if x.ok));
    let out = match r {
        ExecutorResult::Completed(x) => x.output.unwrap(),
        _ => String::new(),
    };
    assert!(out.contains("\"ok\":false"));
    let c = maintenance::propose(&f.0, "README.md", "npm run test\n".into()).unwrap();
    let a = c.approve();
    assert!(e.authorize_candidate(&c, &a));
    let r = e.execute(&apply(&c), &ControlSignal::new(), None).await;
    assert!(matches!(r,ExecutorResult::Completed(x) if x.ok));
    assert_eq!(
        fs::read_to_string(f.0.join("README.md")).unwrap(),
        "npm run test\n"
    );
    let r = e
        .execute(
            &verify("README.md", "command-v1"),
            &ControlSignal::new(),
            None,
        )
        .await;
    assert!(matches!(r,ExecutorResult::Completed(ref x) if x.ok));
    let out = match r {
        ExecutorResult::Completed(x) => x.output.unwrap(),
        _ => String::new(),
    };
    assert!(out.contains("\"ok\":true"));
}

#[tokio::test]
async fn unapproved_cross_target_and_stale_baseline_never_dispatch() {
    let f = F::new();
    let e = Extension02Executor::new(&f.0);
    e.enable_maintenance("README.md", "command-v1");
    let c = maintenance::propose(&f.0, "README.md", "npm run test\n".into()).unwrap();
    assert!(
        matches!(e.execute(&apply(&c),&ControlSignal::new(),None).await,ExecutorResult::Completed(x) if !x.ok)
    );
    let cross_candidate = maintenance::propose(
        &f.0,
        "package.json",
        r#"{"scripts":{"test":"true","lint":"true"}}"#.into(),
    )
    .unwrap();
    assert!(e.authorize_candidate(&cross_candidate, &cross_candidate.approve()));
    let cross = apply(&cross_candidate);
    assert!(
        matches!(e.execute(&cross,&ControlSignal::new(),None).await,ExecutorResult::Completed(x) if !x.ok)
    );
    let a = c.approve();
    assert!(e.authorize_candidate(&c, &a));
    fs::write(f.0.join("README.md"), "changed\n").unwrap();
    assert!(
        matches!(e.execute(&apply(&c),&ControlSignal::new(),None).await,ExecutorResult::Completed(x) if !x.ok)
    );
    assert!(e.records().iter().all(|record| !record.dispatched));
}

#[tokio::test]
async fn fake_success_that_does_not_write_is_unknown_and_approval_not_reusable() {
    let f = F::new();
    let script = f.0.join("host.mjs");
    let ready=serde_json::to_string(&serde_json::json!({"protocol":extension02::PROTOCOL,"type":"ready","tools":extension02::maintenance_declarations(),"capabilities":["read_file","search_files","run_verification","apply_patch"],"maxMessageBytes":extension02::MAX_MESSAGE_BYTES})).unwrap();
    fs::write(&script,format!(r#"import crypto from 'node:crypto'; const r={ready}; console.log(JSON.stringify(r)); process.stdin.on('data',b=>{{let x=JSON.parse(b); const replacement=x.arguments.replacement; const sha=crypto.createHash('sha256').update(replacement).digest('hex'); console.log(JSON.stringify({{protocol:x.protocol,type:'started',sessionId:x.sessionId,requestId:x.requestId,taskId:x.taskId,callId:x.callId}})); console.log(JSON.stringify({{protocol:x.protocol,type:'terminal',status:'succeeded',sessionId:x.sessionId,requestId:x.requestId,taskId:x.taskId,callId:x.callId,result:{{ok:true,output:JSON.stringify({{ok:true,path:'README.md',postSha256:sha}})}},evidence:{{taskId:x.taskId,callId:x.callId,tool:x.tool,arguments:x.arguments,sources:[{{path:'README.md',sha256:sha}}]}}}})); setTimeout(()=>process.exit(),50)}})"#,ready=ready)).unwrap();
    assert!(std::process::Command::new("node")
        .arg("--check")
        .arg(&script)
        .status()
        .unwrap()
        .success());
    let mut e = Extension02Executor::new(&f.0);
    e.host_script = script;
    e.grace = Duration::from_millis(500);
    e.enable_maintenance("README.md", "command-v1");
    let c = maintenance::propose(&f.0, "README.md", "npm run test\n".into()).unwrap();
    assert!(e.authorize_candidate(&c, &c.approve()));
    let r = e.execute(&apply(&c), &ControlSignal::new(), None).await;
    assert!(
        matches!(r, ExecutorResult::OutcomeUnknown),
        "{r:?} records={:?}",
        e.records()
    );
    assert!(e
        .records()
        .last()
        .is_some_and(|record| record.dispatched && record.reaped));
    assert_eq!(
        fs::read_to_string(f.0.join("README.md")).unwrap(),
        "npm run missing\n"
    );
    assert!(
        matches!(e.execute(&apply(&c),&ControlSignal::new(),None).await,ExecutorResult::Completed(x) if !x.ok)
    );
}

#[tokio::test]
async fn verification_source_change_is_unknown() {
    let f = F::new();
    let mut e = Extension02Executor::new(&f.0);
    e.enable_maintenance("README.md", "command-v1");
    let root = f.0.clone();
    e.on_started = Some(std::sync::Arc::new(move || {
        fs::write(
            root.join("package.json"),
            r#"{"scripts":{"test":"changed"}}"#,
        )
        .unwrap();
    }));
    let r = e
        .execute(
            &verify("README.md", "command-v1"),
            &ControlSignal::new(),
            None,
        )
        .await;
    assert!(matches!(r, ExecutorResult::OutcomeUnknown));
    assert!(e
        .records()
        .last()
        .is_some_and(|record| record.dispatched && record.reaped));
}
