//! Parse `.jsonl` replay scripts into phased event streams.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use ratakit::agent::recording::{RecordingEntry, RecordingReader};
use ratakit::agent::state::{AgentEvent, PermissionRequest};

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

/// Load a versioned or legacy `.jsonl` file with submit markers separating phases.
pub fn load_script(path: &Path) -> Result<ReplayScript, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut reader = RecordingReader::new(BufReader::new(file));
    let mut phases: Vec<Vec<ReplayStep>> = vec![Vec::new()];
    while let Some(entry) = reader.next_entry().map_err(|error| error.to_string())? {
        let step = match entry {
            RecordingEntry::Event(event) => ReplayStep::Event(event),
            RecordingEntry::Permission(permission) => ReplayStep::Permission(permission),
            RecordingEntry::AwaitSubmit => ReplayStep::AwaitSubmit,
        };
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
