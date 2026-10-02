use rein_ch01_helloworld::rein::context_methods::{
    run_one_with_executor, validate, Document, Index, Task, Tasks,
};
use rein_ch01_helloworld::rein::{estimated_units, Message, StdioExecutor};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{self, Command},
    sync::atomic::{AtomicUsize, Ordering},
};

static SEQ: AtomicUsize = AtomicUsize::new(0);
fn book() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}
fn unique_root() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "rein-ch08-v3-{}-{}",
        process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&p).unwrap();
    fs::create_dir(p.join("docs")).unwrap();
    p
}
fn fixture() -> (PathBuf, Index, Vec<Task>) {
    let p = unique_root();
    let docs = [
        (
            "early",
            "背景早期\n\n传输方式：JSON Lines\n协议版本：rein-extension/0.1\n",
        ),
        ("latest", "背景发布\n\n检查命令：npm test\n"),
        ("approved", "批准状态：approved\n"),
        ("failed", "处理动作：deploy\n"),
        ("background", "背景自然段\n"),
        ("other", "无关事实：never\n"),
    ];
    for (id, body) in docs {
        fs::write(p.join("docs").join(format!("{id}.md")), body).unwrap();
    }
    let documents = [
        "early",
        "latest",
        "approved",
        "failed",
        "background",
        "other",
    ]
    .into_iter()
    .enumerate()
    .map(|(n, id)| Document {
        id: id.into(),
        path: format!("docs/{id}.md"),
        title: id.into(),
        keywords: vec![id.into()],
        order: n + 1,
    })
    .collect();
    let index = Index {
        unit: "estimated-bytes-v1".into(),
        budget: 2400,
        documents,
    };
    let rules = vec![
        "只根据实际读取的资料回答".into(),
        "每个事实必须给出来源".into(),
    ];
    let tasks = vec![
        Task {
            id: "task-01".into(),
            question: "传输方式和协议版本是什么？".into(),
            rules: rules.clone(),
            budget: 2400,
        },
        Task {
            id: "task-02".into(),
            question: "检查命令是什么？".into(),
            rules: rules.clone(),
            budget: 2400,
        },
        Task {
            id: "task-03".into(),
            question: "批准状态和处理动作是什么？".into(),
            rules: rules.clone(),
            budget: 2400,
        },
        Task {
            id: "task-04".into(),
            question: "生产吞吐量是多少？".into(),
            rules,
            budget: 2400,
        },
    ];
    fs::write(p.join("tasks.json"), r#"{"tasks":[{"id":"task-01","expectedFacts":[{"field":"传输方式","value":"JSON Lines","source":"early"},{"field":"协议版本","value":"rein-extension/0.1","source":"early"}]},{"id":"task-02","expectedFacts":[{"field":"检查命令","value":"npm test","source":"latest"}]},{"id":"task-03","expectedFacts":[{"field":"批准状态","value":"approved","source":"approved"},{"field":"处理动作","value":"deploy","source":"failed"}]},{"id":"task-04","expectedFacts":[],"unknown":true}]}"#).unwrap();
    (p, index, tasks)
}
fn executor(p: &Path, fault: Option<&str>) -> StdioExecutor {
    let mut e = StdioExecutor::new(p);
    e.code_root = book();
    if let Some(mode) = fault {
        e.host_script = book().join("ts/tests/fixtures/hybrid-fault-host.ts");
        e.env.push(("REIN_HYBRID_FAULT_MODE".into(), mode.into()));
    }
    e
}
async fn run(
    i: &Index,
    t: &Task,
    s: &str,
    p: &Path,
    b: Option<usize>,
) -> rein_ch01_helloworld::rein::context_methods::Row {
    run_one_with_executor(i, t, s, p, b, executor(p, None)).await
}

#[tokio::test]
async fn default_matrix_preserves_rules_and_exact_estimates() {
    let (p, i, ts) = fixture();
    for t in &ts {
        for s in ["on-demand", "window", "summary", "retrieval"] {
            let r = run(&i, t, s, &p, None).await;
            assert_eq!(r.status, "completed");
            assert_eq!(r.estimated_units, estimated_units(&r.messages));
            assert!(r.estimated_units <= t.budget);
            assert_eq!(
                r.messages
                    .iter()
                    .filter(|m| m.role == "system")
                    .map(|m| &m.content)
                    .collect::<Vec<_>>(),
                t.rules.iter().collect::<Vec<_>>()
            );
            assert_eq!(
                r.messages
                    .iter()
                    .find(|m| m.role == "user" && !m.content.starts_with("[来源:"))
                    .unwrap()
                    .content,
                t.question
            );
            assert_eq!(r.service_tokens, None);
            assert_eq!(r.model_calls, 1);
            if t.id == "task-04" {
                let a = r.answer.unwrap();
                assert!(a.claims.is_empty() && a.insufficient_evidence);
                assert_eq!(r.quality, Some(1.0));
            }
        }
    }
}

#[tokio::test]
async fn jsonl_mutation_flows_through_http_source_and_oracle() {
    let (p, mut i, ts) = fixture();
    fs::write(
        p.join("docs/early.md"),
        "背景删除\n\n传输方式：HTTP\n协议版本：rein-extension/0.1\n真实背景应被summary删除\n",
    )
    .unwrap();
    i.documents[0].keywords = vec!["传输".into(), "协议".into()];
    let before = fs::read(p.join("tasks.json")).unwrap();
    let r = run(&i, &ts[0], "summary", &p, None).await;
    assert_eq!(fs::read(p.join("tasks.json")).unwrap(), before);
    let a = r.answer.unwrap();
    assert!(a.raw_answer.contains("HTTP") && a.raw_answer.contains("0.1"));
    assert_eq!(r.quality, Some(0.5));
    assert!(r
        .messages
        .iter()
        .filter(|m| m.role == "user" && m.content.starts_with("[来源:"))
        .all(|m| !m.content.contains("真实背景")));
}

#[tokio::test]
async fn budget_boundaries_are_empty_and_do_not_dispatch() {
    let (p, i, mut ts) = fixture();
    ts[0].rules = vec!["r".into()];
    ts[0].question = "q".into();
    let base = estimated_units(&[
        Message {
            role: "system".into(),
            content: "r".into(),
            tool_call_id: None,
            tool_calls: vec![],
        },
        Message {
            role: "user".into(),
            content: "q".into(),
            tool_call_id: None,
            tool_calls: vec![],
        },
    ]);
    for s in ["on-demand", "window", "summary", "retrieval"] {
        for b in [0, base - 1] {
            let r = run(&i, &ts[0], s, &p, Some(b)).await;
            assert_eq!(r.status, "context_budget_exhausted");
            assert!(
                r.messages.is_empty()
                    && r.operations.is_empty()
                    && r.call_records.is_empty()
                    && r.answer.is_none()
            );
        }
    }
}

#[tokio::test]
async fn window_budget_keeps_exact_latest_two_complete_source_messages() {
    let (p, mut i, mut ts) = fixture();
    i.documents.truncate(1);
    ts[0].rules = vec!["r".into()];
    ts[0].question = "无匹配词".into();
    fs::write(
        p.join("docs/early.md"),
        "第一段\n\n第二段\n\n第三段\n\n第四段\n",
    )
    .unwrap();
    let base = vec![
        Message {
            role: "system".into(),
            content: "r".into(),
            tool_call_id: None,
            tool_calls: vec![],
        },
        Message {
            role: "user".into(),
            content: "无匹配词".into(),
            tool_call_id: None,
            tool_calls: vec![],
        },
    ];
    let wanted = ["第三段", "第四段"]
        .iter()
        .enumerate()
        .map(|(n, text)| Message {
            role: "user".into(),
            content: if n == 0 {
                format!("[来源:early]\n{text}")
            } else {
                format!("[来源:early]\n{text}\n")
            },
            tool_call_id: None,
            tool_calls: vec![],
        })
        .collect::<Vec<_>>();
    let budget = estimated_units(&base) + estimated_units(&wanted);
    let r = run(&i, &ts[0], "window", &p, Some(budget)).await;
    let sources = r
        .messages
        .iter()
        .filter(|m| m.role == "user" && m.content.starts_with("[来源:"))
        .map(|m| m.content.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        sources,
        wanted.iter().map(|m| m.content.clone()).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn retrieval_scores_body_only_and_unknown_query_is_empty() {
    let (p, mut i, mut ts) = fixture();
    ts[0].question = "稀有词".into();
    i.documents[0].keywords = vec!["metadata".into()];
    i.documents[5].keywords = vec!["稀有词".into()];
    fs::write(p.join("docs/early.md"), "metadata-only\n").unwrap();
    fs::write(p.join("docs/other.md"), "正文稀有词\n").unwrap();
    let r = run(&i, &ts[0], "retrieval", &p, None).await;
    let ops = r
        .operations
        .iter()
        .filter(|o| o["type"] == "retrieval")
        .collect::<Vec<_>>();
    assert!(ops.iter().all(|o| o["text"].is_string()
        && o["order"].is_number()
        && o["score"].is_number()
        && o["selected"].is_boolean()));
    let early = ops.iter().find(|o| o["source"] == "early").unwrap()["score"]
        .as_u64()
        .unwrap();
    let other = ops.iter().find(|o| o["source"] == "other").unwrap()["score"]
        .as_u64()
        .unwrap();
    assert_eq!(early, 0);
    assert!(other > early);
    ts[3].question = "zzqnomatch902".into();
    let empty = run(&i, &ts[3], "retrieval", &p, None).await;
    assert!(empty.selected_sources.is_empty());
    let a = empty.answer.unwrap();
    assert!(a.claims.is_empty() && a.insufficient_evidence);
    assert_eq!(empty.quality, Some(1.0));
}

#[tokio::test]
async fn unknown_oracle_still_scores_real_throughput_claim() {
    let (p, mut i, mut ts) = fixture();
    fs::write(p.join("docs/latest.md"), "生产吞吐量：100 requests/s\n").unwrap();
    ts[3].question = "生产吞吐量是多少？".into();
    i.documents[1].keywords = vec!["生产".into(), "吞吐量".into()];
    let r = run(&i, &ts[3], "retrieval", &p, None).await;
    let a = r.answer.unwrap();
    assert!(a.claims.iter().any(|c| c.value == "100 requests/s") && !a.insufficient_evidence);
    assert_eq!(r.quality, Some(0.0));
}

#[tokio::test]
async fn duplicate_bodies_keep_document_identity_and_retrieval_changes_with_body() {
    let (p, mut i, mut ts) = fixture();
    i.documents.truncate(2);
    ts[0].rules = vec!["r".into()];
    ts[0].question = "common".into();
    fs::write(p.join("docs/early.md"), "common\n").unwrap();
    fs::write(p.join("docs/latest.md"), "common\n").unwrap();
    i.documents.reverse();
    let window = run(&i, &ts[0], "window", &p, None).await;
    let window_ids = window
        .messages
        .iter()
        .filter(|m| m.content.starts_with("[来源:"))
        .map(|m| m.content.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        window_ids,
        vec!["[来源:early]\ncommon\n", "[来源:latest]\ncommon\n"]
    );
    let r = run(&i, &ts[0], "retrieval", &p, None).await;
    let ids = r
        .operations
        .iter()
        .filter(|o| o["type"] == "retrieval" && o["selected"] == true)
        .map(|o| o["source"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["early", "latest"]);
    assert!(r
        .operations
        .iter()
        .filter(|o| o["type"] == "retrieval" && o["selected"] == true)
        .all(|o| o["score"] == 1));
    fs::write(p.join("docs/early.md"), "changed\n").unwrap();
    let changed = run(&i, &ts[0], "retrieval", &p, None).await;
    let early = changed
        .operations
        .iter()
        .find(|o| o["type"] == "retrieval" && o["source"] == "early")
        .unwrap();
    assert_eq!(early["score"], 0);
}

#[test]
fn metadata_and_native_api_reject_unsafe_inputs() {
    let (_, mut i, ts) = fixture();
    i.documents[1].id = i.documents[0].id.clone();
    assert!(validate(&i, &Tasks { tasks: ts.clone() }, Path::new("/tmp")).is_err());
    let (_, mut i, ts) = fixture();
    i.documents[0].path = "../escape.md".into();
    assert!(validate(&i, &Tasks { tasks: ts }, Path::new("/tmp")).is_err());
    let r = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--locked",
            "--manifest-path",
            book().join("rust/Cargo.toml").to_str().unwrap(),
            "--example",
            "ch08_context",
            "--",
            "fixtures/ch08-context",
            "--budget=9007199254740992",
        ])
        .current_dir(book())
        .output()
        .unwrap();
    assert!(!r.status.success());
    assert!(String::from_utf8_lossy(&r.stderr).contains("MAX_SAFE_INTEGER"));
}

#[tokio::test]
async fn node_faults_and_missing_files_keep_real_records() {
    let (p, i, mut ts) = fixture();
    ts[0].question = "early 传输方式和协议版本是什么？".into();
    for mode in ["invalid_json", "crash_after_invoke", "wrong_id"] {
        let r = run_one_with_executor(&i, &ts[0], "on-demand", &p, None, executor(&p, Some(mode)))
            .await;
        assert_eq!(r.status, "error");
        assert!(r.answer.is_none() && r.model_calls == 0 && !r.call_records.is_empty());
        let c = r.call_records.last().unwrap();
        assert!(c.dispatched && c.child_pid.is_some() && c.reaped && c.terminal.is_some());
        println!("fault {mode}: {c:?}");
        if mode == "crash_after_invoke" {
            assert_eq!(c.exit_code, Some(17));
        }
        assert_eq!(c.terminal.as_deref(), Some("unknown"));
    }
    fs::remove_file(p.join("docs/early.md")).unwrap();
    let r = run(&i, &ts[0], "on-demand", &p, None).await;
    assert_eq!(r.status, "error");
    assert!(r.selected_sources.is_empty());
    let c = r.call_records.last().unwrap();
    assert!(c.prepared && !c.dispatched && c.child_pid.is_none());
}

#[tokio::test]
async fn oracle_errors_are_explicit_after_answering() {
    let (p, i, ts) = fixture();
    for contents in [
        r#"{"tasks":[{"id":"task-01","expectedFacts":[],"unknown":false}]}"#,
        "{invalid json",
    ] {
        fs::write(p.join("tasks.json"), contents).unwrap();
        let r = run(&i, &ts[0], "on-demand", &p, None).await;
        assert_eq!(r.status, "error");
        assert!(r.answer.is_some());
        assert_eq!(r.model_calls, 1);
        assert_eq!(r.quality, None);
        assert!(!r.unsupported_claims.is_empty());
    }
}

#[tokio::test]
async fn direct_max_budget_and_zero_budget_skip_deleted_selected_file() {
    let (p, mut i, ts) = fixture();
    let r = run(&i, &ts[0], "on-demand", &p, Some(usize::MAX)).await;
    assert_eq!(r.status, "error");
    assert!(r.answer.is_none() && r.model_calls == 0 && r.call_records.is_empty());
    i.documents[0].keywords = vec!["传输".into()];
    fs::remove_file(p.join("docs/early.md")).unwrap();
    let r = run(&i, &ts[0], "on-demand", &p, None).await;
    assert_eq!(r.status, "error");
    assert!(!r.call_records.is_empty());
    let r = run(&i, &ts[0], "on-demand", &p, Some(0)).await;
    assert_eq!(r.status, "context_budget_exhausted");
    assert!(r.call_records.is_empty() && r.operations.is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn symlink_escape_is_pre_dispatch_with_prepared_record() {
    let (p, mut i, ts) = fixture();
    std::os::unix::fs::symlink("/etc/hosts", p.join("docs/link.md")).unwrap();
    i.documents[0].path = "docs/link.md".into();
    i.documents[0].keywords = vec!["传输".into()];
    let r = run(&i, &ts[0], "on-demand", &p, None).await;
    assert_eq!(r.status, "error");
    let c = r.call_records.first().unwrap();
    assert!(c.prepared && !c.dispatched && c.child_pid.is_none());
}

#[tokio::test]
async fn missing_oracle_id_and_ascii_word_boundaries() {
    let (p, mut i, mut ts) = fixture();
    ts[0].id = "renamed".into();
    let r = run(&i, &ts[0], "on-demand", &p, None).await;
    assert_eq!(r.status, "error");
    assert!(r.answer.is_some() && r.model_calls == 1 && r.quality.is_none());
    let mut oracle = serde_json::from_str::<serde_json::Value>(
        &fs::read_to_string(p.join("tasks.json")).unwrap(),
    )
    .unwrap();
    oracle["tasks"][0]["id"] = serde_json::json!("renamed");
    fs::write(p.join("tasks.json"), serde_json::to_vec(&oracle).unwrap()).unwrap();
    i.documents[0].keywords = vec!["传输".into()];
    let r = run(&i, &ts[0], "on-demand", &p, None).await;
    assert_eq!(r.status, "completed");
    assert!(r.answer.unwrap().raw_answer.contains("JSON Lines"));
    assert_eq!(r.quality, Some(1.0));
    ts[3].question = "at".into();
    fs::write(p.join("docs/early.md"), "metadata\n\ncat\n").unwrap();
    i.documents[0].keywords = vec!["metadata".into()];
    let r = run(&i, &ts[3], "retrieval", &p, None).await;
    let ops = r
        .operations
        .iter()
        .filter(|o| o["type"] == "retrieval")
        .collect::<Vec<_>>();
    let scores = ops
        .iter()
        .filter(|o| o["source"] == "early")
        .map(|o| o["score"].as_u64().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(scores, vec![0, 0]);
    ts[3].question = "metadata".into();
    let changed = run(&i, &ts[3], "retrieval", &p, None).await;
    assert!(changed
        .operations
        .iter()
        .any(|o| o["type"] == "retrieval" && o["score"] == 1));
}

#[test]
fn metadata_rejects_task_duplicates_orders_absolute_and_unsafe_budget() {
    let (_, i, mut ts) = fixture();
    ts.push(ts[0].clone());
    assert!(validate(&i, &Tasks { tasks: ts }, Path::new("/tmp")).is_err());
    let (_, mut i, ts) = fixture();
    i.documents[1].order = i.documents[0].order;
    assert!(validate(&i, &Tasks { tasks: ts }, Path::new("/tmp")).is_err());
    let (_, mut i, ts) = fixture();
    i.documents[0].path = "/tmp/x.md".into();
    assert!(validate(&i, &Tasks { tasks: ts }, Path::new("/tmp")).is_err());
    let (_, mut i, ts) = fixture();
    i.budget = usize::MAX;
    assert!(validate(&i, &Tasks { tasks: ts }, Path::new("/tmp")).is_err());
}
