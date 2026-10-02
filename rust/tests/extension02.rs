use rein_ch01_helloworld::rein::{
    extension02, run_agent_loop_with_executor, ControlSignal, ExecutorResult, Extension02Executor,
    LoopEvent, LoopOptions, Message, ModelAdapter, ModelTurn, StopReason, ToolCall, ToolDefinition,
    ToolError, ToolExecutor, Workspace,
};
use std::{
    fs,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
static SEQ: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "rein-ext02-review-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("README.md"), "marker: alpha\n").unwrap();
        Self { root }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn read(id: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: "read_file".into(),
        arguments: serde_json::json!({"path":"README.md"}),
    }
}
fn fake(f: &Fixture, mode: &str) -> Extension02Executor {
    let ready = serde_json::json!({"protocol":extension02::PROTOCOL,"type":"ready","tools":extension02::readonly_declarations(),"capabilities":["read_file","search_files"],"maxMessageBytes":extension02::MAX_MESSAGE_BYTES});
    let template = r#"import {createInterface} from 'node:readline';
import {readFileSync,writeFileSync} from 'node:fs';import{createHash}from'node:crypto';import{join}from'node:path';
const mode=MODE;const root=process.env.REIN_HYBRID_WORKSPACE;const ready=READY;
if(mode==='bad_ready')ready.protocol='old';console.log(JSON.stringify(ready));
const r=createInterface({input:process.stdin});let x;
function stop(code=0){r.close();process.stdin.destroy();process.stdout.end(()=>{process.exitCode=code})}
r.on('line',line=>{const message=JSON.parse(line);
 if(message.type==='cancel'){writeFileSync(join(root,'cancel-received'),'yes');if(mode==='cancel'){console.log(JSON.stringify({...message,type:'terminal',status:'cancelled'}));stop()}return}
 x=message;writeFileSync(join(root,'invoked'),x.callId);
 const base={protocol:ready.protocol,sessionId:x.sessionId,requestId:x.requestId,taskId:x.taskId,callId:x.callId};
 console.log(JSON.stringify({...base,type:'started'}));
 if(mode==='timeout'||mode==='cancel'){return}
 if(mode==='oversize'){process.stdout.write('x'.repeat(300000));stop();return}
 const text=readFileSync(join(root,'README.md'),'utf8');const sha256=createHash('sha256').update(text).digest('hex');
 let terminal={...base,type:'terminal',status:'succeeded',result:{ok:true,output:text},evidence:{taskId:x.taskId,callId:x.callId,tool:x.tool,arguments:x.arguments,sources:[{path:'README.md',sha256}]}};
 if(mode==='bad_request')terminal.requestId='wrong';if(mode==='bad_version')terminal.protocol='old';
 if(mode==='missing_evidence')delete terminal.evidence;if(mode==='bad_evidence')terminal.evidence.sources[0].sha256='0'.repeat(64);
 if(mode==='lying_output')terminal.result.output='not actual text';
 if(mode==='bad_union')terminal.result.error={code:'bad',message:'contradictory'};
 if(mode==='unsolicited_cancel')terminal={...base,type:'terminal',status:'cancelled'};
 console.log(JSON.stringify(terminal));if(mode==='duplicate')console.log(JSON.stringify(terminal));if(mode==='tail')process.stdout.write('tail');stop(mode==='exit_error'?7:0)
});"#;
    let script = template
        .replace("MODE", &serde_json::to_string(mode).unwrap())
        .replace("READY", &ready.to_string());
    let path = f.root.join("host.mjs");
    fs::write(&path, script).unwrap();
    let mut e = Extension02Executor::new(&f.root);
    e.host_script = path;
    e.grace = Duration::from_millis(500);
    e
}
fn adopted(out: ExecutorResult) -> String {
    match out {
        ExecutorResult::Completed(r) if r.ok => r.output.unwrap(),
        x => panic!("expected adopted result, got {x:?}"),
    }
}
#[tokio::test]
async fn fake_positive_reaches_invoke_and_exits_cleanly() {
    let f = Fixture::new();
    let e = fake(&f, "ok");
    assert_eq!(
        adopted(e.execute(&read("c"), &ControlSignal::new(), None).await),
        "marker: alpha\n"
    );
    assert_eq!(fs::read_to_string(f.root.join("invoked")).unwrap(), "c");
    assert!(e.records()[0].reaped);
    assert_eq!(e.records()[0].exit_code, Some(0));
}
#[tokio::test]
async fn terminal_corruptions_reach_dispatch_and_are_unknown() {
    for mode in [
        "bad_request",
        "bad_version",
        "missing_evidence",
        "bad_evidence",
        "lying_output",
        "bad_union",
        "unsolicited_cancel",
        "duplicate",
        "tail",
        "exit_error",
        "oversize",
    ] {
        let f = Fixture::new();
        let e = fake(&f, mode);
        let out = e.execute(&read("c"), &ControlSignal::new(), None).await;
        assert_eq!(
            fs::read_to_string(f.root.join("invoked")).unwrap(),
            "c",
            "mode {mode} never invoked"
        );
        assert!(
            matches!(out, ExecutorResult::OutcomeUnknown),
            "{mode}: {out:?}"
        );
        assert!(e.records()[0].reaped, "unreaped {mode}");
    }
}
#[tokio::test]
async fn unsupported_ready_never_dispatches() {
    let f = Fixture::new();
    let e = fake(&f, "bad_ready");
    assert!(matches!(
        e.execute(&read("c"), &ControlSignal::new(), None).await,
        ExecutorResult::NotDispatched
    ));
    assert!(!f.root.join("invoked").exists());
    assert!(e.records()[0].reaped);
}
#[tokio::test]
async fn deadline_after_started_reaps_and_does_not_adopt() {
    let f = Fixture::new();
    let e = fake(&f, "timeout");
    let start = Instant::now();
    let out = e
        .execute(
            &read("c"),
            &ControlSignal::new(),
            Some(start + Duration::from_secs(1)),
        )
        .await;
    assert!(f.root.join("invoked").exists());
    assert!(matches!(out, ExecutorResult::OutcomeUnknown));
    assert!(start.elapsed() < Duration::from_secs(3));
    assert!(e.records()[0].reaped);
}
#[tokio::test]
async fn cancellation_sends_bound_cancel_and_reaps_ack() {
    let f = Fixture::new();
    let mut e = fake(&f, "cancel");
    let signal = ControlSignal::new();
    let trigger = signal.clone();
    e.on_started = Some(Arc::new(move || trigger.cancel()));
    let out = e.execute(&read("c"), &signal, None).await;
    assert!(matches!(out, ExecutorResult::Cancelled), "{out:?}");
    assert!(f.root.join("cancel-received").exists());
    assert!(e.records()[0].reaped);
}
#[tokio::test]
async fn real_host_reads_searches_and_tracks_unique_identity() {
    let f = Fixture::new();
    let e = Extension02Executor::new(&f.root);
    assert_eq!(
        adopted(e.execute(&read("read"), &ControlSignal::new(), None).await),
        "marker: alpha\n"
    );
    let call = ToolCall {
        id: "search".into(),
        name: "search_files".into(),
        arguments: serde_json::json!({"needle":"marker:"}),
    };
    assert_eq!(
        adopted(e.execute(&call, &ControlSignal::new(), None).await),
        "README.md"
    );
    let r = e.records();
    assert_eq!(r.len(), 2);
    assert_ne!(r[0].request_id, r[1].request_id);
    assert!(r.iter().all(|r| r.reaped && r.exit_code == Some(0)));
}
struct Model {
    calls: Vec<ToolCall>,
    received: Mutex<Vec<Vec<Message>>>,
}
impl ModelAdapter for Model {
    fn complete<'a>(
        &'a self,
        m: &'a [Message],
        _: &'a [ToolDefinition],
    ) -> Pin<Box<dyn Future<Output = Result<ModelTurn, ToolError>> + Send + 'a>> {
        Box::pin(async move {
            let mut history = self.received.lock().unwrap();
            history.push(m.to_vec());
            let calls = if history.len() == 1 {
                self.calls.clone()
            } else {
                vec![]
            };
            let text = if history.len() == 1 { "" } else { "done" };
            Ok(ModelTurn {
                message: Message {
                    role: "assistant".into(),
                    content: text.into(),
                    tool_call_id: None,
                    tool_calls: calls.clone(),
                },
                tool_calls: calls,
            })
        })
    }
}
#[tokio::test]
async fn same_turn_failure_continues_second_call_and_pairs_results() {
    let f = Fixture::new();
    let e = Extension02Executor::new(&f.root);
    let mut bad = read("bad");
    bad.arguments = serde_json::json!({"path":"missing.md"});
    let model = Model {
        calls: vec![bad, read("good")],
        received: Mutex::new(vec![]),
    };
    let out = run_agent_loop_with_executor(
        &model,
        &Workspace {
            root: f.root.clone(),
        },
        "read",
        LoopOptions::default(),
        &e,
    )
    .await;
    assert_eq!(out.reason, StopReason::FinalAnswer);
    let received = model.received.lock().unwrap();
    let tools: Vec<_> = received[1].iter().filter(|m| m.role == "tool").collect();
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0].tool_call_id.as_deref(), Some("bad"));
    assert_eq!(tools[1].tool_call_id.as_deref(), Some("good"));
    assert!(tools[0].content.contains("path_invalid"));
    assert_eq!(tools[1].content, "marker: alpha\n");
    assert_eq!(e.records().len(), 2);
}
#[tokio::test]
async fn unknown_stops_pending_calls_without_fake_results() {
    let f = Fixture::new();
    let e = fake(&f, "bad_request");
    let model = Model {
        calls: vec![read("one"), read("two")],
        received: Mutex::new(vec![]),
    };
    let out = run_agent_loop_with_executor(
        &model,
        &Workspace {
            root: f.root.clone(),
        },
        "read",
        LoopOptions::default(),
        &e,
    )
    .await;
    assert_eq!(out.reason, StopReason::OutcomeUnknown);
    assert_eq!(model.received.lock().unwrap().len(), 1);
    assert_eq!(e.records().len(), 1);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e,LoopEvent::ActionSkipped{call_id:Some(id),..} if id=="two")));
    assert!(!out.messages.iter().any(|m| m.role == "tool"));
}
