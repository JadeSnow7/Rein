use super::stdio_executor::CallRecord;
use super::{
    estimated_units, ControlSignal, ExecutorResult, Message, StdioExecutor, ToolCall, ToolExecutor,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path, time::Instant};

#[derive(Clone, Debug, Deserialize)]
pub struct Index {
    pub unit: String,
    pub budget: usize,
    pub documents: Vec<Document>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Document {
    pub id: String,
    pub path: String,
    pub title: String,
    pub keywords: Vec<String>,
    pub order: usize,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Task {
    pub id: String,
    pub question: String,
    pub rules: Vec<String>,
    pub budget: usize,
}
#[derive(Clone, Debug, Deserialize)]
pub struct Tasks {
    pub tasks: Vec<Task>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Answer {
    #[serde(rename = "rawAnswer")]
    pub raw_answer: String,
    pub claims: Vec<Claim>,
    #[serde(rename = "insufficientEvidence")]
    pub insufficient_evidence: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct Claim {
    pub field: String,
    pub value: String,
    pub source: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Row {
    #[serde(rename = "taskId")]
    pub task_id: String,
    pub strategy: String,
    pub question: String,
    pub budget: usize,
    pub status: String,
    pub messages: Vec<Message>,
    #[serde(rename = "estimatedUnits")]
    pub estimated_units: usize,
    #[serde(rename = "selectedSources")]
    pub selected_sources: Vec<String>,
    pub operations: Vec<serde_json::Value>,
    pub answer: Option<Answer>,
    pub quality: Option<f64>,
    #[serde(rename = "unsupportedClaims")]
    pub unsupported_claims: Vec<serde_json::Value>,
    #[serde(rename = "elapsedMs")]
    pub elapsed_ms: u128,
    #[serde(rename = "serviceTokens")]
    pub service_tokens: Option<usize>,
    #[serde(rename = "modelCalls")]
    pub model_calls: usize,
    #[serde(rename = "callRecords")]
    pub call_records: Vec<CallRecord>,
}
#[derive(Clone, Debug)]
struct Paragraph {
    source: String,
    order: usize,
    paragraph_index: usize,
    text: String,
}
struct TaskView<'a> {
    question: &'a str,
    rules: &'a [String],
}

pub fn validate(index: &Index, tasks: &Tasks, root: &Path) -> Result<(), String> {
    if index.unit != "estimated-bytes-v1" {
        return Err("invalid metadata unit".into());
    }
    const MAX_SAFE: usize = 9_007_199_254_740_991;
    if index.budget > MAX_SAFE || tasks.tasks.iter().any(|t| t.budget > MAX_SAFE) {
        return Err("budget exceeds MAX_SAFE_INTEGER".into());
    }
    if index.documents.iter().any(|d| d.id.is_empty())
        || !unique(index.documents.iter().map(|d| &d.id))
    {
        return Err("duplicate or empty document id".into());
    }
    if !unique(index.documents.iter().map(|d| &d.order)) {
        return Err("duplicate document order".into());
    }
    for d in &index.documents {
        let p = Path::new(&d.path);
        if d.path.is_empty()
            || p.is_absolute()
            || d.path
                .split('/')
                .any(|x| x.is_empty() || x == "." || x == "..")
            || !root.join(p).starts_with(root)
        {
            return Err(format!("invalid path for {}", d.id));
        }
    }
    if tasks.tasks.iter().any(|t| t.id.is_empty()) || !unique(tasks.tasks.iter().map(|t| &t.id)) {
        return Err("duplicate or empty task id".into());
    }
    Ok(())
}
fn unique<'a, T: Eq + std::hash::Hash + 'a>(mut it: impl Iterator<Item = &'a T>) -> bool {
    let mut s = HashSet::new();
    it.all(|x| s.insert(x))
}
fn docs_for(index: &Index, v: &TaskView<'_>, strategy: &str) -> Vec<Document> {
    let mut d = index.documents.clone();
    if strategy == "on-demand" {
        d.retain(|x| {
            v.question.contains(&x.title) || x.keywords.iter().any(|k| v.question.contains(k))
        });
    }
    d.sort_by_key(|x| x.order);
    d
}
fn paragraphs(d: &Document, b: &str) -> Vec<Paragraph> {
    b.split("\n\n")
        .enumerate()
        .filter(|(_, s)| !s.trim().is_empty())
        .map(|(i, s)| Paragraph {
            source: d.id.clone(),
            order: d.order,
            paragraph_index: i,
            text: s.into(),
        })
        .collect()
}
fn facts(p: &Paragraph) -> String {
    p.text
        .lines()
        .filter(|l| l.contains('：') && !l.starts_with("背景"))
        .collect::<Vec<_>>()
        .join("\n")
}
fn terms(q: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut ascii = String::new();
    let mut chinese = Vec::new();
    let flush = |out: &mut Vec<String>, run: &mut Vec<char>| {
        for n in 2..=run.len().min(8) {
            for w in run.windows(n) {
                out.push(w.iter().collect());
            }
        }
        run.clear();
    };
    for ch in q.chars() {
        if ch.is_ascii_alphanumeric() {
            flush(&mut out, &mut chinese);
            ascii.push(ch.to_ascii_lowercase());
        } else {
            if !ascii.is_empty() {
                out.push(std::mem::take(&mut ascii));
            }
            if ch.is_alphabetic() && !ch.is_ascii() {
                chinese.push(ch);
            } else {
                flush(&mut out, &mut chinese);
            }
        }
    }
    if !ascii.is_empty() {
        out.push(ascii);
    }
    flush(&mut out, &mut chinese);
    out.sort();
    out.dedup();
    out
}
fn term_matches(body: &str, term: &str) -> bool {
    if term.chars().all(|c| c.is_ascii_alphanumeric()) {
        body.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| word.eq_ignore_ascii_case(term))
    } else {
        body.contains(term)
    }
}

fn fields(q: &str) -> Vec<(&'static str, Vec<&'static str>)> {
    let mut v = vec![];
    if q.contains("传输方式") {
        v.push(("传输方式", vec!["传输方式", "交换方式"]));
    }
    if q.contains("协议") {
        v.push(("协议版本", vec!["协议版本", "协议"]));
    }
    if q.contains("检查命令") {
        v.push(("检查命令", vec!["检查命令", "验收命令"]));
    }
    if q.contains("批准") {
        v.push(("批准状态", vec!["批准状态", "批准"]));
    }
    if q.contains("处理动作") || q.contains("怎么处理") || q.contains("如何处理") {
        v.push(("处理动作", vec!["处理动作", "恢复动作"]));
    }
    if q.contains("吞吐量") {
        v.push(("生产吞吐量", vec!["生产吞吐量", "吞吐量"]));
    }
    v
}
fn parse_claims(m: &[Message], q: &str) -> Vec<Claim> {
    let f = fields(q);
    let mut o = vec![];
    for x in m
        .iter()
        .filter(|x| x.role == "user" && x.content.starts_with("[来源:"))
    {
        let s = x
            .content
            .lines()
            .next()
            .and_then(|x| x.strip_prefix("[来源:"))
            .and_then(|x| x.strip_suffix(']'))
            .unwrap_or("");
        for l in x.content.lines().skip(1) {
            if let Some((k, v)) = l.split_once('：') {
                if let Some((canon, _)) = f.iter().find(|(_, a)| a.iter().any(|z| *z == k)) {
                    o.push(Claim {
                        field: (*canon).into(),
                        value: v.trim().into(),
                        source: s.into(),
                    });
                }
            }
        }
    }
    o
}
fn answer(m: &[Message]) -> Answer {
    let q = m
        .iter()
        .find(|x| x.role == "user" && !x.content.starts_with("[来源:"))
        .map(|x| x.content.as_str())
        .unwrap_or("");
    let c = parse_claims(m, q);
    Answer {
        raw_answer: c
            .iter()
            .map(|x| format!("{}：{}（来源：{}）", x.field, x.value, x.source))
            .collect::<Vec<_>>()
            .join("\n"),
        insufficient_evidence: c.is_empty(),
        claims: c,
    }
}
fn quality(root: &Path, id: &str, c: &[Claim]) -> Result<f64, String> {
    let text = std::fs::read_to_string(root.join("tasks.json"))
        .map_err(|_| "oracle unavailable".to_owned())?;
    let x: serde_json::Value =
        serde_json::from_str(&text).map_err(|_| "oracle invalid JSON".to_owned())?;
    let task = x["tasks"]
        .as_array()
        .and_then(|a| a.iter().find(|t| t["id"] == id))
        .ok_or_else(|| "oracle expectedFacts missing".to_owned())?;
    let f = task["expectedFacts"]
        .as_array()
        .ok_or_else(|| "oracle expectedFacts missing".to_owned())?;
    if f.is_empty() {
        if task["unknown"] != true {
            return Err("oracle has empty expectedFacts without unknown=true".into());
        }
        return Ok(if c.is_empty() { 1.0 } else { 0.0 });
    };
    Ok(f.iter()
        .filter(|z| {
            c.iter()
                .any(|a| z["field"] == a.field && z["value"] == a.value && z["source"] == a.source)
        })
        .count() as f64
        / f.len() as f64)
}
fn base(v: &TaskView<'_>) -> Vec<Message> {
    let mut m = v
        .rules
        .iter()
        .map(|c| Message {
            role: "system".into(),
            content: c.clone(),
            tool_call_id: None,
            tool_calls: vec![],
        })
        .collect::<Vec<_>>();
    m.push(Message {
        role: "user".into(),
        content: v.question.into(),
        tool_call_id: None,
        tool_calls: vec![],
    });
    m
}
fn empty(t: &Task, s: &str, b: usize, status: &str, start: Instant) -> Row {
    Row {
        task_id: t.id.clone(),
        strategy: s.into(),
        question: t.question.clone(),
        budget: b,
        status: status.into(),
        messages: vec![],
        estimated_units: 0,
        selected_sources: vec![],
        operations: vec![],
        answer: None,
        quality: None,
        unsupported_claims: vec![],
        elapsed_ms: start.elapsed().as_millis(),
        service_tokens: None,
        model_calls: 0,
        call_records: vec![],
    }
}

pub async fn run_one(
    index: &Index,
    task: &Task,
    strategy: &str,
    root: &Path,
    budget_override: Option<usize>,
) -> Row {
    run_one_with_executor(
        index,
        task,
        strategy,
        root,
        budget_override,
        StdioExecutor::new(root),
    )
    .await
}
pub async fn run_one_with_executor(
    index: &Index,
    task: &Task,
    strategy: &str,
    root: &Path,
    budget_override: Option<usize>,
    mut executor: StdioExecutor,
) -> Row {
    let start = Instant::now();
    let budget = budget_override.unwrap_or(task.budget);
    const MAX_SAFE: usize = 9_007_199_254_740_991;
    if budget > MAX_SAFE {
        return empty(task, strategy, budget, "error", start);
    }
    let v = TaskView {
        question: &task.question,
        rules: &task.rules,
    };
    let mut m = base(&v);
    if budget == 0 || estimated_units(&m) > budget {
        return empty(task, strategy, budget, "context_budget_exhausted", start);
    }
    if !matches!(strategy, "on-demand" | "window" | "summary" | "retrieval") {
        return empty(task, strategy, budget, "error", start);
    }
    let docs = docs_for(index, &v, strategy);
    let mut ps = vec![];
    let mut ops = vec![];
    executor.env.push((
        "REIN_HYBRID_WORKSPACE".into(),
        root.to_string_lossy().into_owned(),
    ));
    for (n, d) in docs.iter().enumerate() {
        let call = ToolCall {
            id: format!("call-{}", n + 1),
            name: "read_file".into(),
            arguments: serde_json::json!({"path":d.path}),
        };
        match executor.execute(&call, &ControlSignal::new(), None).await {
            ExecutorResult::Completed(r) if r.ok => {
                let p = paragraphs(d, &r.output.unwrap_or_default());
                ops.push(serde_json::json!({"type":"read_file","path":d.path,"callId":call.id,"strategy":strategy,"paragraphs":p.len(),"score":serde_json::Value::Null}));
                ps.extend(p)
            }
            _ => {
                let mut r = empty(task, strategy, budget, "error", start);
                r.messages = m;
                r.estimated_units = estimated_units(&r.messages);
                r.operations = ops;
                r.call_records = executor.records();
                return r;
            }
        }
    }
    let mut chosen = if strategy == "retrieval" {
        let ts = terms(v.question);
        let mut scored = ps
            .iter()
            .map(|p| {
                let n = ts.iter().filter(|t| term_matches(&p.text, t)).count();
                (n, p)
            })
            .collect::<Vec<_>>();
        scored.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then(a.1.order.cmp(&b.1.order))
                .then(a.1.paragraph_index.cmp(&b.1.paragraph_index))
        });
        for (n, p) in &scored {
            ops.push(serde_json::json!({"type":"retrieval","source":p.source,"order":p.order,"paragraphIndex":p.paragraph_index,"score":n,"text":p.text,"selected":false}));
        }
        scored
            .into_iter()
            .filter(|(n, _)| *n > 0)
            .map(|(_, p)| p.clone())
            .collect()
    } else if strategy == "summary" {
        ps.iter()
            .filter(|p| !facts(p).is_empty())
            .cloned()
            .collect()
    } else {
        ps.clone()
    };
    if strategy == "window" {
        chosen.reverse()
    }
    let mut accepted = vec![];
    for p in chosen {
        let text = if strategy == "summary" {
            facts(&p)
        } else {
            p.text.clone()
        };
        let msg = Message {
            role: "user".into(),
            content: format!("[来源:{}]\n{}", p.source, text),
            tool_call_id: None,
            tool_calls: vec![],
        };
        if estimated_units(&m) + estimated_units(std::slice::from_ref(&msg)) > budget {
            if strategy == "window" {
                break;
            } else {
                continue;
            }
        }
        accepted.push(p);
        m.push(msg);
    }
    if strategy == "window" {
        accepted.sort_by_key(|p| (p.order, p.paragraph_index));
        let start_i = task.rules.len() + 1;
        m.truncate(start_i);
        for p in &accepted {
            m.push(Message {
                role: "user".into(),
                content: format!(
                    "[来源:{}]\n{}",
                    p.source,
                    if strategy == "summary" {
                        facts(p)
                    } else {
                        p.text.clone()
                    }
                ),
                tool_call_id: None,
                tool_calls: vec![],
            });
        }
    }
    for op in ops.iter_mut().filter(|o| o["type"] == "retrieval") {
        op["selected"] = serde_json::json!(accepted.iter().any(|p| p.source == op["source"]
            && serde_json::Value::from(p.paragraph_index) == op["paragraphIndex"]));
    }
    let mut selected = vec![];
    for p in &accepted {
        if !selected.contains(&p.source) {
            selected.push(p.source.clone());
        }
    }
    let a = answer(&m);
    let q = match quality(root, &task.id, &a.claims) {
        Ok(v) => v,
        Err(e) => {
            let mut r = empty(task, strategy, budget, "error", start);
            r.messages = m;
            r.estimated_units = estimated_units(&r.messages);
            r.selected_sources = selected;
            r.operations = ops;
            r.answer = Some(a);
            r.model_calls = 1;
            r.call_records = executor.records();
            r.unsupported_claims = vec![serde_json::json!({"error":e})];
            return r;
        }
    };
    Row {
        task_id: task.id.clone(),
        strategy: strategy.into(),
        question: task.question.clone(),
        budget,
        status: "completed".into(),
        estimated_units: estimated_units(&m),
        messages: m,
        selected_sources: selected,
        operations: ops,
        quality: Some(q),
        answer: Some(a),
        unsupported_claims: vec![],
        elapsed_ms: start.elapsed().as_millis(),
        service_tokens: None,
        model_calls: 1,
        call_records: executor.records(),
    }
}
