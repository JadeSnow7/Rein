//! Durable runtime around the pure `rein-core` transition function.
#![forbid(unsafe_code)]
use rein_core::{
    ArtifactRef as CoreArtifactRef, HarnessSession, HarnessStep, Intent, Observation,
    ObservationKind, SessionStatus, StepResult, ToolCall,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use schemars::schema_for;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

pub mod model {
    #[derive(Clone, Debug, Default)]
    pub struct OfflineModel;
    impl OfflineModel {
        pub fn reads_named_fixture() -> Self {
            Self
        }
        pub fn complete(&self, result: Option<&[u8]>) -> ModelReply {
            match result {
                None => ModelReply::Tool {
                    call_id: "read-1".into(),
                    name: "read_file".into(),
                    path: "fixture.txt".into(),
                },
                Some(b) => ModelReply::Final(b.to_vec()),
            }
        }
        pub fn complete_named(&self, result: Option<&[u8]>, path: &str) -> ModelReply {
            match self.complete(result) {
                ModelReply::Tool { call_id, name, .. } => ModelReply::Tool {
                    call_id,
                    name,
                    path: path.into(),
                },
                final_reply => final_reply,
            }
        }
    }
    #[derive(Clone, Debug)]
    pub enum ModelReply {
        Tool {
            call_id: String,
            name: String,
            path: String,
        },
        Final(Vec<u8>),
    }
}
pub mod artifact;
pub mod owner;
pub mod tools;
pub mod verify;
pub use tools::ReadFileTool;
#[derive(Clone, Debug)]
pub struct RuntimeConfig {
    pub state_dir: PathBuf,
    pub fixture: PathBuf,
}
impl RuntimeConfig {
    pub fn for_fixture(d: impl AsRef<Path>, f: impl AsRef<Path>) -> Self {
        Self {
            state_dir: d.as_ref().into(),
            fixture: f.as_ref().into(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counts {
    pub session_revision: i64,
    pub events: i64,
    pub outbox: i64,
    pub observations: i64,
    pub artifacts: i64,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub enum EffectState {
    Pending,
    Claimed,
    OutcomeUnknown,
    Cancelled,
    Done,
}
#[derive(Clone, Debug)]
pub struct Effect {
    pub effect_id: String,
    pub kind: String,
    pub intent: Intent,
}
impl Effect {
    pub fn id(&self) -> &str {
        &self.effect_id
    }
}
pub mod store {
    use super::*;
    pub use super::{Effect, EffectState};
    pub struct SqliteStore {
        pub(crate) conn: Connection,
    }
    impl SqliteStore {
        pub fn open(p: impl AsRef<Path>) -> rusqlite::Result<Self> {
            if let Some(x) = p.as_ref().parent() {
                let _ = fs::create_dir_all(x);
            }
            let c = Connection::open(p)?;
            c.execute_batch("PRAGMA busy_timeout=5000; CREATE TABLE IF NOT EXISTS sessions(id TEXT PRIMARY KEY,revision INTEGER NOT NULL,state TEXT NOT NULL,budget INTEGER NOT NULL,body TEXT NOT NULL); CREATE TABLE IF NOT EXISTS observations(id TEXT PRIMARY KEY,effect_id TEXT,kind TEXT NOT NULL,body TEXT NOT NULL); CREATE TABLE IF NOT EXISTS events(id INTEGER PRIMARY KEY AUTOINCREMENT,revision INTEGER NOT NULL,kind TEXT NOT NULL); CREATE TABLE IF NOT EXISTS outbox(effect_id TEXT PRIMARY KEY,kind TEXT NOT NULL,intent TEXT NOT NULL,state TEXT NOT NULL,claim_count INTEGER NOT NULL DEFAULT 0,dispatch_count INTEGER NOT NULL DEFAULT 0); CREATE TABLE IF NOT EXISTS artifacts(hash TEXT PRIMARY KEY,kind TEXT NOT NULL,len INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS acceptance(status TEXT);")?;
            Ok(Self { conn: c })
        }
        pub fn connection(&self) -> &Connection {
            &self.conn
        }
        pub fn open_existing(p: impl AsRef<Path>, read_only: bool) -> rusqlite::Result<Self> {
            let flags = if read_only {
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
            } else {
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
            };
            Ok(Self {
                conn: Connection::open_with_flags(p, flags)?,
            })
        }
        pub fn init_session(&self, s: &HarnessSession) -> rusqlite::Result<()> {
            let tx =
                rusqlite::Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
            tx.execute(
                "INSERT OR IGNORE INTO sessions VALUES(?1,?2,?3,?4,?5)",
                params![
                    s.session_id,
                    s.revision,
                    state(&s.status),
                    s.tool_budget,
                    serde_json::to_string(s).unwrap()
                ],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO artifacts(hash,kind,len) VALUES(?1,'verification_plan',?2)",
                params![
                    s.verification_plan_ref.hash,
                    s.verification_plan_ref.len as i64
                ],
            )?;
            tx.commit()?;
            Ok(())
        }
        pub fn load_session(&self) -> rusqlite::Result<HarnessSession> {
            let b: String = self
                .conn
                .query_row("SELECT body FROM sessions", [], |r| r.get(0))?;
            serde_json::from_str(&b).map_err(|_| rusqlite::Error::InvalidQuery)
        }
        pub fn counts(&self) -> rusqlite::Result<Counts> {
            Ok(Counts {
                session_revision: self.conn.query_row(
                    "SELECT revision FROM sessions",
                    [],
                    |r| r.get(0),
                )?,
                events: self
                    .conn
                    .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))?,
                outbox: self
                    .conn
                    .query_row("SELECT COUNT(*) FROM outbox", [], |r| r.get(0))?,
                observations: self.conn.query_row(
                    "SELECT COUNT(*) FROM observations",
                    [],
                    |r| r.get(0),
                )?,
                artifacts: self
                    .conn
                    .query_row("SELECT COUNT(*) FROM artifacts", [], |r| r.get(0))?,
            })
        }
        pub fn pending_effect(&self) -> rusqlite::Result<Effect> {
            self.conn.query_row(
                "SELECT effect_id,kind,intent FROM outbox WHERE state='pending' ORDER BY rowid LIMIT 1",
                [],
                |r| {
                    Ok(Effect {
                        effect_id: r.get(0)?,
                        kind: r.get(1)?,
                        intent: serde_json::from_str(&r.get::<_, String>(2)?).map_err(|_| rusqlite::Error::InvalidQuery)?,
                    })
                },
            )
        }
        pub fn claim_next_effect(&self) -> rusqlite::Result<Option<Effect>> {
            let tx =
                rusqlite::Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
            let body: String =
                tx.query_row("SELECT body FROM sessions LIMIT 1", [], |r| r.get(0))?;
            let session: HarnessSession =
                serde_json::from_str(&body).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let e: Option<Effect> = if session.status == SessionStatus::Running {
                if let Some(active) = session.active_effect {
                    tx.query_row("SELECT effect_id,kind,intent FROM outbox WHERE state='pending' AND effect_id=?1 ORDER BY rowid LIMIT 1", params![active], |r| Ok(Effect { effect_id: r.get(0)?, kind: r.get(1)?, intent: serde_json::from_str(&r.get::<_, String>(2)?).map_err(|_| rusqlite::Error::InvalidQuery)? })).optional()?
                } else {
                    None
                }
            } else {
                None
            };
            if let Some(x) = &e {
                if tx.execute("UPDATE outbox SET state='claimed',claim_count=claim_count+1 WHERE effect_id=?1 AND state='pending'",params![x.effect_id])? != 1 { return Ok(None); }
            }
            tx.commit()?;
            Ok(e)
        }
        pub fn dispatch_claimed(&self, id: &str) -> rusqlite::Result<()> {
            let tx =
                rusqlite::Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
            let body: String =
                tx.query_row("SELECT body FROM sessions LIMIT 1", [], |r| r.get(0))?;
            let session: HarnessSession =
                serde_json::from_str(&body).map_err(|_| rusqlite::Error::InvalidQuery)?;
            if session.status != SessionStatus::Running
                || session.active_effect.as_deref() != Some(id)
            {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            let n = tx.execute("UPDATE outbox SET dispatch_count=dispatch_count+1 WHERE effect_id=?1 AND state='claimed' AND dispatch_count=0",params![id])?;
            if n != 1 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            tx.commit()?;
            Ok(())
        }
        pub fn dispatch_count_total(&self) -> rusqlite::Result<i64> {
            self.conn.query_row(
                "SELECT COALESCE(SUM(dispatch_count),0) FROM outbox",
                [],
                |r| r.get(0),
            )
        }
        pub fn dispatch_count(&self, id: &str) -> rusqlite::Result<i64> {
            self.conn.query_row(
                "SELECT dispatch_count FROM outbox WHERE effect_id=?1",
                params![id],
                |r| r.get(0),
            )
        }
        pub fn effect_dispatch_count(&self, k: &str) -> rusqlite::Result<i64> {
            self.conn.query_row(
                "SELECT COALESCE(SUM(dispatch_count),0) FROM outbox WHERE kind=?1",
                params![k],
                |r| r.get(0),
            )
        }
        pub fn effect_kinds(&self) -> Vec<String> {
            self.conn
                .prepare("SELECT kind FROM outbox ORDER BY rowid")
                .unwrap()
                .query_map([], |r| r.get(0))
                .unwrap()
                .filter_map(Result::ok)
                .collect()
        }
        pub fn state(&self) -> rusqlite::Result<String> {
            self.conn
                .query_row("SELECT state FROM sessions", [], |r| r.get(0))
        }
        pub fn tool_budget(&self) -> rusqlite::Result<i64> {
            self.conn
                .query_row("SELECT budget FROM sessions", [], |r| r.get(0))
        }
        pub fn outbox_count(&self) -> rusqlite::Result<i64> {
            self.conn
                .query_row("SELECT COUNT(*) FROM outbox", [], |r| r.get(0))
        }
        pub fn effect_state(&self, id: &str) -> rusqlite::Result<EffectState> {
            let s: String = self.conn.query_row(
                "SELECT state FROM outbox WHERE effect_id=?1",
                params![id],
                |r| r.get(0),
            )?;
            Ok(match s.as_str() {
                "pending" => EffectState::Pending,
                "claimed" => EffectState::Claimed,
                "unknown" => EffectState::OutcomeUnknown,
                "cancelled" => EffectState::Cancelled,
                "done" => EffectState::Done,
                _ => return Err(rusqlite::Error::InvalidQuery),
            })
        }
        pub fn last_observation(&self) -> rusqlite::Result<Observation> {
            let body: String = self.conn.query_row(
                "SELECT body FROM observations ORDER BY rowid DESC LIMIT 1",
                [],
                |r| r.get(0),
            )?;
            serde_json::from_str(&body).map_err(|_| rusqlite::Error::InvalidQuery)
        }
        pub fn acceptance(&self) -> rusqlite::Result<Option<verify::VerificationStatus>> {
            let x: Option<String> = self
                .conn
                .query_row("SELECT status FROM acceptance LIMIT 1", [], |r| r.get(0))
                .optional()?;
            Ok(x.map(|s| match s.as_str() {
                "passed" => verify::VerificationStatus::Passed,
                "failed" => verify::VerificationStatus::Failed,
                _ => verify::VerificationStatus::Undetermined,
            }))
        }
        pub fn commit_transition(
            &self,
            old: &HarnessSession,
            o: &Observation,
            r: &StepResult,
            refs: &[(String, String, i64)],
        ) -> rusqlite::Result<bool> {
            let StepResult::Transition {
                expected_revision,
                next_revision,
                session,
                next_intent,
                events,
            } = r
            else {
                return Ok(false);
            };
            if *expected_revision != old.revision {
                return Ok(false);
            }
            let tx =
                rusqlite::Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
            let cur: i64 = tx.query_row("SELECT revision FROM sessions", [], |x| x.get(0))?;
            if cur != old.revision as i64 {
                return Ok(false);
            }
            tx.execute(
                "INSERT INTO observations(id,effect_id,kind,body) VALUES(?1,?2,?3,?4)",
                params![
                    o.observation_id,
                    o.effect_id,
                    serde_json::to_string(&o.kind).unwrap(),
                    serde_json::to_string(o).unwrap()
                ],
            )?;
            for (x, k, l) in refs {
                tx.execute(
                    "INSERT OR IGNORE INTO artifacts VALUES(?1,?2,?3)",
                    params![x, k, l],
                )?;
            }
            for (x, k, l) in session_refs(session).into_iter().map(|r| {
                let kind = if session.final_ref.as_ref() == Some(&r) {
                    "final_answer"
                } else if session.tool_result_refs.iter().any(|x| x == &r) {
                    "tool_result"
                } else if session.active_intent.as_ref().is_some_and(|i| matches!(i, rein_core::ActiveIntent::Tool { args_ref, .. } if args_ref == &r)) {
                    "tool_args"
                } else {
                    "verification_plan"
                };
                (r.hash, kind.to_string(), r.len as i64)
            }) {
                tx.execute(
                    "INSERT OR IGNORE INTO artifacts VALUES(?1,?2,?3)",
                    params![x, k, l],
                )?;
            }
            if tx.execute(
                "UPDATE sessions SET revision=?1,state=?2,budget=?3,body=?4 WHERE revision=?5",
                params![
                    *next_revision as i64,
                    state(&session.status),
                    session.tool_budget,
                    serde_json::to_string(session).unwrap(),
                    old.revision as i64
                ],
            )? != 1
            {
                return Ok(false);
            }
            for e in events {
                tx.execute(
                    "INSERT INTO events(revision,kind)VALUES(?1,?2)",
                    params![*next_revision as i64, serde_json::to_string(e).unwrap()],
                )?;
            }
            if let Some(effect) = &old.active_effect {
                let terminal_state = match o.kind {
                    ObservationKind::Cancel => "cancelled",
                    ObservationKind::OutcomeUnknown => "unknown",
                    _ => "done",
                };
                let query = if matches!(o.kind, ObservationKind::Cancel) {
                    "UPDATE outbox SET state=?2 WHERE effect_id=?1 AND state IN ('claimed','pending','unknown')"
                } else {
                    "UPDATE outbox SET state=?2 WHERE effect_id=?1 AND state IN ('claimed','pending')"
                };
                if tx.execute(query, params![effect, terminal_state])? != 1 {
                    return Ok(false);
                }
            }
            if let ObservationKind::VerifierResult { status, .. } = &o.kind {
                let value = match status {
                    rein_core::VerificationStatus::Passed => "passed",
                    rein_core::VerificationStatus::Failed => "failed",
                    rein_core::VerificationStatus::Undetermined => "undetermined",
                };
                tx.execute("DELETE FROM acceptance", [])?;
                tx.execute("INSERT INTO acceptance VALUES(?1)", params![value])?;
            }
            if let Some(i) = next_intent {
                let (e, k) = match &i {
                    Intent::CallModel { effect_id } => (effect_id.clone(), "CallModel"),
                    Intent::ExecuteTool { effect_id, .. } => (effect_id.clone(), "ExecuteTool"),
                    Intent::Verify { effect_id, .. } => (effect_id.clone(), "Verify"),
                };
                tx.execute(
                    "INSERT INTO outbox(effect_id,kind,intent,state,claim_count,dispatch_count) VALUES(?1,?2,?3,'pending',0,0)",
                    params![e, k, serde_json::to_string(i).unwrap()],
                )?;
            }
            tx.commit()?;
            Ok(true)
        }
    }
    fn state(s: &SessionStatus) -> &'static str {
        match s {
            SessionStatus::Running => "running",
            SessionStatus::Cancelled => "cancelled",
            SessionStatus::Completed => "completed",
            SessionStatus::Accepted => "accepted",
            SessionStatus::OutcomeUnknown => "unknown",
        }
    }
}
pub struct RunResult {
    pub answer: String,
    pub acceptance: verify::VerificationStatus,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct EffectSnapshot {
    pub effect_id: String,
    pub kind: String,
    pub state: EffectState,
    pub dispatch_count: i64,
    pub reason: Option<String>,
}
#[derive(Clone, Debug)]
pub struct RuntimeSnapshot {
    pub session: HarnessSession,
    pub acceptance: Option<verify::VerificationStatus>,
    pub effects: Vec<EffectSnapshot>,
}

fn snapshot(store: &store::SqliteStore) -> Result<RuntimeSnapshot, Box<dyn std::error::Error>> {
    let tx =
        rusqlite::Transaction::new_unchecked(store.connection(), TransactionBehavior::Deferred)?;
    let body: String = tx.query_row("SELECT body FROM sessions", [], |row| row.get(0))?;
    let session: HarnessSession = serde_json::from_str(&body)?;
    let acceptance = tx
        .query_row("SELECT status FROM acceptance LIMIT 1", [], |row| {
            row.get::<_, String>(0)
        })
        .optional()?
        .map(|status| match status.as_str() {
            "passed" => verify::VerificationStatus::Passed,
            "failed" => verify::VerificationStatus::Failed,
            _ => verify::VerificationStatus::Undetermined,
        });
    let mut stmt =
        tx.prepare("SELECT effect_id,kind,state,dispatch_count FROM outbox ORDER BY rowid")?;
    let effects = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let kind: String = row.get(1)?;
            let state: String = row.get(2)?;
            let dispatch_count: i64 = row.get(3)?;
            let state = match state.as_str() {
                "pending" => EffectState::Pending,
                "claimed" => EffectState::Claimed,
                "unknown" => EffectState::OutcomeUnknown,
                "cancelled" => EffectState::Cancelled,
                "done" => EffectState::Done,
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            let reason =
                (state == EffectState::OutcomeUnknown).then(|| "outcome unknown".to_string());
            Ok(EffectSnapshot {
                effect_id: id,
                kind,
                state,
                dispatch_count,
                reason,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);
    tx.commit()?;
    Ok(RuntimeSnapshot {
        session,
        acceptance,
        effects,
    })
}
pub struct Runtime {
    _owner: owner::DriverOwner,
    store: store::SqliteStore,
    artifacts: artifact::ArtifactStore,
    model: model::OfflineModel,
    tool: ReadFileTool,
    fixture: PathBuf,
    session: HarnessSession,
}
impl Runtime {
    pub fn inspect(
        state_dir: impl AsRef<Path>,
    ) -> Result<RuntimeSnapshot, Box<dyn std::error::Error>> {
        let db = state_dir.as_ref().join("state.sqlite");
        if !db.is_file() {
            return Err("state database does not exist".into());
        }
        let store = store::SqliteStore::open_existing(db, true)?;
        snapshot(&store)
    }

    pub fn cancel_at(
        state_dir: impl AsRef<Path>,
    ) -> Result<RuntimeSnapshot, Box<dyn std::error::Error>> {
        let db = state_dir.as_ref().join("state.sqlite");
        if !db.is_file() {
            return Err("state database does not exist".into());
        }
        let store = store::SqliteStore::open_existing(db, false)?;
        let session = store.load_session()?;
        let observation = Observation {
            observation_id: format!("cancel-{}", session.revision),
            session_id: session.session_id.clone(),
            run_id: session.run_id.clone(),
            attempt_id: session.attempt_id.clone(),
            revision: session.revision,
            effect_id: session.active_effect.clone(),
            kind: ObservationKind::Cancel,
        };
        let result = HarnessStep::advance(&session, &observation);
        match result {
            StepResult::Transition { .. } => {
                if !store.commit_transition(&session, &observation, &result, &refs(&observation))? {
                    return Err("cancel commit failed".into());
                }
                snapshot(&store)
            }
            StepResult::Duplicate { .. } => snapshot(&store),
            StepResult::Rejected { code, .. } => Err(format!("rejected {code:?}").into()),
        }
    }

    pub fn open(
        c: RuntimeConfig,
        m: model::OfflineModel,
        mut t: ReadFileTool,
        v: verify::FixedVerifier,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let owner = owner::DriverOwner::acquire(&c.state_dir)?;
        let state_path = c.state_dir.join("state.sqlite");
        if state_path.exists() {
            return Self::reopen_owned(c.state_dir, owner);
        }
        let a = artifact::ArtifactStore::new(c.state_dir.join("artifacts"))?;
        let workdir = c.state_dir.join("workdir");
        fs::create_dir_all(&workdir)?;
        let fixture = workdir.join("fixture.txt");
        let mut source = fs::File::open(&c.fixture)?;
        let mut target = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&fixture)?;
        io::copy(&mut source, &mut target)?;
        target.sync_all()?;
        let mut fixture_permissions = fs::metadata(&fixture)?.permissions();
        fixture_permissions.set_readonly(true);
        fs::set_permissions(&fixture, fixture_permissions)?;
        let plan = v.plan_bytes()?;
        let p = a.put(&plan)?;
        let s = HarnessSession {
            session_id: "session-1".into(),
            run_id: "run-1".into(),
            attempt_id: "attempt-1".into(),
            revision: 0,
            status: SessionStatus::Running,
            tool_budget: 10,
            active_effect: None,
            active_intent: None,
            history: vec![],
            completed_effects: vec![],
            tool_result_refs: vec![],
            final_ref: None,
            verification_plan_ref: CoreArtifactRef {
                hash: p.hash.clone(),
                len: p.len as u64,
            },
        };
        let st = store::SqliteStore::open(c.state_dir.join("state.sqlite"))?;
        st.init_session(&s)?;
        let s = st.load_session()?;
        t.set_root(workdir.clone());
        Ok(Self {
            _owner: owner,
            store: st,
            artifacts: a,
            model: m,
            tool: t,
            fixture,
            session: s,
        })
    }
    pub fn reopen(state_dir: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let state_dir = state_dir.as_ref().to_path_buf();
        if !state_dir.join("state.sqlite").is_file() {
            return Err("state database does not exist".into());
        }
        let owner = owner::DriverOwner::acquire(&state_dir)?;
        Self::reopen_owned(state_dir, owner)
    }

    fn reopen_owned(
        state_dir: PathBuf,
        owner: owner::DriverOwner,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let st = store::SqliteStore::open(state_dir.join("state.sqlite"))?;
        let session = st.load_session()?;
        let artifacts = artifact::ArtifactStore::new(state_dir.join("artifacts"))?;
        let _ = verify::FixedVerifier::from_plan_bytes(
            &artifacts.read(&session.verification_plan_ref)?,
        )?;
        let workdir = state_dir.join("workdir");
        let fixture = workdir.join("fixture.txt");
        let mut tool = ReadFileTool::rooted_read_only();
        tool.set_root(workdir);
        Ok(Self {
            _owner: owner,
            store: st,
            artifacts,
            model: model::OfflineModel::reads_named_fixture(),
            tool,
            fixture,
            session,
        })
    }
    pub fn store(&self) -> &store::SqliteStore {
        &self.store
    }
    pub fn artifacts(&self) -> &artifact::ArtifactStore {
        &self.artifacts
    }
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.observe(Observation {
            observation_id: "start".into(),
            session_id: self.session.session_id.clone(),
            run_id: self.session.run_id.clone(),
            attempt_id: self.session.attempt_id.clone(),
            revision: self.session.revision,
            effect_id: None,
            kind: ObservationKind::Start,
        })
    }
    pub fn set_tool_budget<B>(&mut self, b: B) -> Result<(), Box<dyn std::error::Error>>
    where
        B: TryInto<u64>,
        B::Error: std::fmt::Display,
    {
        let budget = u32::try_from(
            b.try_into()
                .map_err(|e| format!("tool budget must be a valid u32: {e}"))?,
        )
        .map_err(|_| "tool budget must be a valid u32")?;
        let tx = rusqlite::Transaction::new_unchecked(
            self.store.connection(),
            TransactionBehavior::Immediate,
        )?;
        let body: String = tx.query_row("SELECT body FROM sessions", [], |r| r.get(0))?;
        let mut session: HarnessSession = serde_json::from_str(&body)?;
        if session.revision != 0 {
            return Err("tool budget can only be set at revision 0".into());
        }
        session.tool_budget = budget;
        if tx.execute(
            "UPDATE sessions SET budget=?1,body=?2 WHERE revision=0",
            params![budget, serde_json::to_string(&session)?],
        )? != 1
        {
            return Err("tool budget update lost CAS race".into());
        }
        tx.commit()?;
        self.session = session;
        Ok(())
    }
    pub fn cancel(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.observe(Observation {
            observation_id: format!("cancel-{}", self.session.revision),
            session_id: self.session.session_id.clone(),
            run_id: self.session.run_id.clone(),
            attempt_id: self.session.attempt_id.clone(),
            revision: self.session.revision,
            effect_id: self.session.active_effect.clone(),
            kind: ObservationKind::Cancel,
        })
    }
    pub fn observe(&mut self, o: Observation) -> Result<(), Box<dyn std::error::Error>> {
        self.session = self.store.load_session()?;
        let r = HarnessStep::advance(&self.session, &o);
        match &r {
            StepResult::Transition { session, .. } => {
                for reference in observation_refs(&o)
                    .into_iter()
                    .chain(session_refs(session).into_iter())
                {
                    self.artifacts.read(&reference)?;
                }
                if let ObservationKind::VerifierResult { status, output_ref } = &o.kind {
                    let receipt: verify::VerificationReceipt =
                        serde_json::from_slice(&self.artifacts.read(output_ref)?)?;
                    let plan_bytes = self.artifacts.read(&self.session.verification_plan_ref)?;
                    let verifier = verify::FixedVerifier::from_plan_bytes(&plan_bytes)?;
                    let final_ref = self
                        .session
                        .final_ref
                        .as_ref()
                        .ok_or("missing final reference")?;
                    let final_bytes = self.artifacts.read(final_ref)?;
                    verifier.validate_receipt(
                        &receipt,
                        &self.session.verification_plan_ref,
                        final_ref,
                        &final_bytes,
                    )?;
                    let expected = match receipt.status {
                        verify::VerificationStatus::Passed => rein_core::VerificationStatus::Passed,
                        verify::VerificationStatus::Failed => rein_core::VerificationStatus::Failed,
                        verify::VerificationStatus::Undetermined => {
                            rein_core::VerificationStatus::Undetermined
                        }
                    };
                    if &expected != status {
                        return Err("verification status does not match receipt".into());
                    }
                }
                if !self
                    .store
                    .commit_transition(&self.session, &o, &r, &refs(&o))?
                {
                    return Err("commit failed".into());
                }
                self.session = session.clone();
                Ok(())
            }
            StepResult::Duplicate { .. } => Ok(()),
            StepResult::Rejected { code, .. } => Err(format!("rejected {code:?}").into()),
        }
    }
    pub fn drive_one(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.session = self.store.load_session()?;
        let e = self.store.claim_next_effect()?.ok_or("no pending effect")?;
        self.store.dispatch_claimed(&e.effect_id)?;
        let intent = e.intent.clone();
        let (_p, kind) = match (e.kind.as_str(), intent) {
            ("CallModel", Intent::CallModel { .. }) => match self.model.complete_named(
                match self.session.tool_result_refs.last() {
                    Some(reference) => Some(self.read(reference)?),
                    None => None,
                }
                .as_deref(),
                self.fixture
                    .file_name()
                    .and_then(|x| x.to_str())
                    .unwrap_or("fixture.txt"),
            ) {
                model::ModelReply::Tool {
                    call_id,
                    name,
                    path,
                } => {
                    let a = self.artifacts.put(path.as_bytes())?;
                    (
                        path.into_bytes(),
                        ObservationKind::ModelTurn {
                            tool_call: Some(ToolCall {
                                call_id,
                                name,
                                args_ref: CoreArtifactRef {
                                    hash: a.hash,
                                    len: a.len,
                                },
                            }),
                            final_ref: None,
                        },
                    )
                }
                model::ModelReply::Final(b) => {
                    let a = self.artifacts.put(&b)?;
                    (
                        b,
                        ObservationKind::ModelTurn {
                            tool_call: None,
                            final_ref: Some(CoreArtifactRef {
                                hash: a.hash,
                                len: a.len,
                            }),
                        },
                    )
                }
            },
            ("ExecuteTool", Intent::ExecuteTool { call, .. }) => {
                let call_id = call.call_id;
                let name = call.name;
                let args_ref = call.args_ref;
                match self
                    .read(&args_ref)
                    .and_then(|bytes| String::from_utf8(bytes).map_err(|e| e.to_string()))
                {
                    Err(error) => {
                        let payload = serde_json::to_vec(
                            &serde_json::json!({"tool":name,"path":null,"error":error}),
                        )?;
                        let a = self.artifacts.put(&payload)?;
                        (
                            payload,
                            ObservationKind::ToolFailed {
                                call_id,
                                error_ref: a,
                            },
                        )
                    }
                    Ok(path) if name != "read_file" => {
                        let payload = serde_json::to_vec(
                            &serde_json::json!({"tool":name,"path":path,"error":"unsupported tool"}),
                        )?;
                        let a = self.artifacts.put(&payload)?;
                        (
                            payload,
                            ObservationKind::ToolFailed {
                                call_id,
                                error_ref: a,
                            },
                        )
                    }
                    Ok(path) => match self.tool.read(&path) {
                        Ok(b) => {
                            let a = self.artifacts.put(&b)?;
                            (
                                b,
                                ObservationKind::ToolResult {
                                    call_id,
                                    result_ref: a,
                                },
                            )
                        }
                        Err(error) => {
                            let payload = serde_json::to_vec(
                                &serde_json::json!({"tool":name,"path":path,"error":error.to_string()}),
                            )?;
                            let a = self.artifacts.put(&payload)?;
                            (
                                payload,
                                ObservationKind::ToolFailed {
                                    call_id,
                                    error_ref: a,
                                },
                            )
                        }
                    },
                }
            }
            ("Verify", Intent::Verify { final_ref, .. }) => {
                let a = self.read(&final_ref)?;
                let plan_bytes = self.artifacts.read(&self.session.verification_plan_ref)?;
                let verifier = verify::FixedVerifier::from_plan_bytes(&plan_bytes)?;
                let receipt =
                    verifier.receipt(&self.session.verification_plan_ref, &final_ref, &a)?;
                let s = receipt.status.clone();
                let b = serde_json::to_vec(&receipt)?;
                let r = self.artifacts.put(&b)?;
                let cs = match s {
                    verify::VerificationStatus::Passed => rein_core::VerificationStatus::Passed,
                    verify::VerificationStatus::Failed => rein_core::VerificationStatus::Failed,
                    verify::VerificationStatus::Undetermined => {
                        rein_core::VerificationStatus::Undetermined
                    }
                };
                (
                    b,
                    ObservationKind::VerifierResult {
                        status: cs,
                        output_ref: CoreArtifactRef {
                            hash: r.hash,
                            len: r.len,
                        },
                    },
                )
            }
            _ => return Err("effect mismatch".into()),
        };
        self.observe(Observation {
            observation_id: format!("obs-{}", e.effect_id),
            session_id: self.session.session_id.clone(),
            run_id: self.session.run_id.clone(),
            attempt_id: self.session.attempt_id.clone(),
            revision: self.session.revision,
            effect_id: Some(e.effect_id.clone()),
            kind,
        })?;
        Ok(())
    }
    fn read(&self, r: &CoreArtifactRef) -> Result<Vec<u8>, String> {
        self.artifacts.read(r).map_err(|e| e.to_string())
    }
    pub fn drive_to_completion(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for _ in 0..12 {
            self.session = self.store.load_session()?;
            if !matches!(self.session.status, SessionStatus::Running) {
                break;
            }
            match self.store.pending_effect() {
                Ok(_) => {}
                Err(rusqlite::Error::QueryReturnedNoRows) => break,
                Err(error) => return Err(error.into()),
            }
            self.drive_one()?;
            if !matches!(self.session.status, SessionStatus::Running) {
                break;
            }
        }
        Ok(())
    }
    pub fn run_to_acceptance(&mut self) -> Result<RunResult, Box<dyn std::error::Error>> {
        if self.session.revision == 0 {
            self.start()?
        }
        self.drive_to_completion()?;
        Ok(RunResult {
            answer: match self.session.final_ref.as_ref() {
                Some(reference) => String::from_utf8(self.read(reference)?)?,
                None => String::new(),
            },
            acceptance: self
                .store
                .acceptance()?
                .unwrap_or(verify::VerificationStatus::Undetermined),
        })
    }
    pub fn recover_explicitly(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.session = self.store.load_session()?;
        if let Some(effect_id) = self.session.active_effect.clone() {
            if self.store.effect_state(&effect_id)? != EffectState::Claimed {
                return Ok(());
            }
            self.observe(Observation {
                observation_id: format!("outcome-unknown-{}", self.session.revision),
                session_id: self.session.session_id.clone(),
                run_id: self.session.run_id.clone(),
                attempt_id: self.session.attempt_id.clone(),
                revision: self.session.revision,
                effect_id: Some(effect_id),
                kind: ObservationKind::OutcomeUnknown,
            })?;
        }
        Ok(())
    }
    pub fn claim_next_effect(&mut self) -> Result<Option<Effect>, Box<dyn std::error::Error>> {
        Ok(self.store.claim_next_effect()?)
    }
}
fn observation_refs(o: &Observation) -> Vec<CoreArtifactRef> {
    match &o.kind {
        ObservationKind::ModelTurn {
            tool_call,
            final_ref,
        } => tool_call
            .iter()
            .map(|x| x.args_ref.clone())
            .chain(final_ref.iter().cloned())
            .collect(),
        ObservationKind::ToolResult { result_ref, .. } => vec![result_ref.clone()],
        ObservationKind::ToolFailed { error_ref, .. } => vec![error_ref.clone()],
        ObservationKind::VerifierResult { output_ref, .. } => vec![output_ref.clone()],
        _ => vec![],
    }
}

fn refs(o: &Observation) -> Vec<(String, String, i64)> {
    let kind = match &o.kind {
        ObservationKind::ModelTurn { .. } => "tool_args",
        ObservationKind::ToolResult { .. } => "tool_result",
        ObservationKind::ToolFailed { .. } => "tool_error",
        ObservationKind::VerifierResult { .. } => "verify_receipt",
        _ => "observation",
    };
    observation_refs(o)
        .into_iter()
        .map(|r| (r.hash, kind.to_string(), r.len as i64))
        .collect()
}

fn session_refs(s: &HarnessSession) -> Vec<CoreArtifactRef> {
    std::iter::once(s.verification_plan_ref.clone())
        .chain(s.tool_result_refs.iter().cloned())
        .chain(s.final_ref.iter().cloned())
        .chain(s.active_intent.iter().filter_map(|intent| match intent {
            rein_core::ActiveIntent::Tool { args_ref, .. } => Some(args_ref.clone()),
            _ => None,
        }))
        .collect()
}
pub mod schema {
    use super::*;
    pub fn generate(d: &Path) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(d)?;
        fs::write(
            d.join("session.json"),
            serde_json::to_vec_pretty(&schema_for!(HarnessSession))?,
        )?;
        fs::write(
            d.join("observation.json"),
            serde_json::to_vec_pretty(&schema_for!(Observation))?,
        )?;
        fs::write(
            d.join("intent.json"),
            serde_json::to_vec_pretty(&schema_for!(Intent))?,
        )?;
        fs::write(
            d.join("step-result.json"),
            serde_json::to_vec_pretty(&schema_for!(StepResult))?,
        )?;
        Ok(())
    }
    pub fn check(d: &Path) -> Result<(), Box<dyn std::error::Error>> {
        for (name, bytes) in [
            (
                "session.json",
                serde_json::to_vec_pretty(&schema_for!(HarnessSession))?,
            ),
            (
                "observation.json",
                serde_json::to_vec_pretty(&schema_for!(Observation))?,
            ),
            (
                "intent.json",
                serde_json::to_vec_pretty(&schema_for!(Intent))?,
            ),
            (
                "step-result.json",
                serde_json::to_vec_pretty(&schema_for!(StepResult))?,
            ),
        ] {
            if fs::read(d.join(name))? != bytes {
                return Err(format!("schema drift: {name}").into());
            }
        }
        Ok(())
    }
}
