use rein_ch01_helloworld::rein::context_methods::{run_one, validate, Index, Row, Tasks};
use serde::Serialize;
use std::{env, fs, path::PathBuf};

#[derive(Serialize)]
struct Output {
    unit: String,
    #[serde(rename = "serviceTokens")]
    service_tokens: Option<usize>,
    results: Vec<Row>,
}
fn main() {
    let mut args = env::args().skip(1);
    let root = PathBuf::from(
        args.next()
            .unwrap_or_else(|| "fixtures/ch08-context".into()),
    );
    let mut budget: Option<usize> = None;
    let mut strategy: Option<String> = None;
    while let Some(arg) = args.next() {
        let (key, value) = arg.split_once('=').unwrap_or((arg.as_str(), ""));
        match key {
            "--budget" if budget.is_none() && !value.is_empty() => {
                let parsed: usize = value.parse().map_err(|_| "invalid budget").unwrap();
                if parsed > 9_007_199_254_740_991 {
                    panic!("invalid budget: exceeds MAX_SAFE_INTEGER")
                }
                budget = Some(parsed);
            }
            "--strategy" if strategy.is_none() && !value.is_empty() => {
                if !matches!(value, "on-demand" | "window" | "summary" | "retrieval") {
                    panic!("invalid strategy")
                }
                strategy = Some(value.to_owned());
            }
            _ => panic!("invalid or duplicate argument: {arg}"),
        }
    }
    let index: Index =
        serde_json::from_str(&fs::read_to_string(root.join("index.json")).expect("read index"))
            .expect("valid index");
    let tasks: Tasks =
        serde_json::from_str(&fs::read_to_string(root.join("tasks.json")).expect("read tasks"))
            .expect("valid tasks");
    validate(&index, &tasks, &root).expect("invalid metadata");
    let strategies: Vec<&str> = strategy
        .as_deref()
        .map(|s| vec![s])
        .unwrap_or_else(|| vec!["on-demand", "window", "summary", "retrieval"]);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let results = rt.block_on(async {
        let mut out = Vec::new();
        for task in &tasks.tasks {
            for s in &strategies {
                out.push(run_one(&index, task, s, &root, budget).await);
            }
        }
        out
    });
    println!(
        "{}",
        serde_json::to_string(&Output {
            unit: index.unit,
            service_tokens: None,
            results
        })
        .unwrap()
    );
}
