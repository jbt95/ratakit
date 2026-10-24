//! Parse `.jsonl` replay scripts into phased event streams.

use std::fs;
use std::path::Path;

use ratakit::agent::state::{AgentEvent, PermissionRequest};
use serde::Deserialize;
use serde_json::Value;

/// One line in a replay script.
#[derive(Debug, Clone)]
pub enum ReplayStep {
    Event(AgentEvent),
    Permission(PermissionRequest),
    AwaitSubmit,
}

/// A script split into phases; each phase runs after the user submits once.
#[derive(Debug, Clone)]
pub struct ReplayScript {
    pub phases: Vec<Vec<ReplayStep>>,
}

#[derive(Deserialize)]
struct AwaitSubmitLine {
    await_submit: bool,
}

#[derive(Deserialize)]
struct PermissionLine {
    permission: PermissionRequest,
}

/// Load a script from a `.jsonl` file (phases separated by `{"await_submit":true}`).
pub fn load_script(path: &Path) -> Result<ReplayScript, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut phases: Vec<Vec<ReplayStep>> = vec![Vec::new()];
    for (lineno, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let step = parse_line(trimmed).map_err(|e| format!("line {}: {e}", lineno + 1))?;
        if matches!(step, ReplayStep::AwaitSubmit) {
            phases.push(Vec::new());
            continue;
        }
        phases.last_mut().unwrap().push(step);
    }
    while phases.last().is_some_and(Vec::is_empty) {
        phases.pop();
    }
    if phases.is_empty() {
        phases.push(Vec::new());
    }
    Ok(ReplayScript { phases })
}

fn parse_line(line: &str) -> Result<ReplayStep, String> {
    let value: Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
    if let Ok(await_line) = serde_json::from_value::<AwaitSubmitLine>(value.clone()) {
        if await_line.await_submit {
            return Ok(ReplayStep::AwaitSubmit);
        }
    }
    if let Ok(perm) = serde_json::from_value::<PermissionLine>(value.clone()) {
        return Ok(ReplayStep::Permission(perm.permission));
    }
    let event: AgentEvent = serde_json::from_value(value).map_err(|e| e.to_string())?;
    Ok(ReplayStep::Event(event))
}

/// Drives phased replay against the mock app's layout state.
#[derive(Debug)]
pub struct ScriptDriver {
    script: ReplayScript,
    phase: usize,
    pending: Vec<ReplayStep>,
}

impl ScriptDriver {
    pub fn new(script: ReplayScript) -> Self {
        let pending = script.phases.first().cloned().unwrap_or_default();
        Self {
            script,
            phase: 0,
            pending,
        }
    }

    pub fn phase(&self) -> usize {
        self.phase
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Pop the next step, if any.
    pub fn pop_step(&mut self) -> Option<ReplayStep> {
        if self.pending.is_empty() {
            return None;
        }
        Some(self.pending.remove(0))
    }

    /// User submitted: advance to the next phase and queue its steps.
    pub fn on_user_submit(&mut self) {
        self.phase += 1;
        self.pending = self
            .script
            .phases
            .get(self.phase)
            .cloned()
            .unwrap_or_default();
    }
}
