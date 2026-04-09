use std::collections::HashMap;
use uuid::Uuid;

/// Mutable state passed to every hook invocation.
///
/// Hooks may read from and write to the metadata map freely.
/// Structural mutations (content overrides, score overrides) are declared
/// via `HookMutation` in the `HookResult` instead of being applied directly —
/// this keeps the core executor as the single owner of pipeline state.
#[derive(Debug, Clone)]
pub struct HookContext {
    /// Unique execution ID for this skill invocation
    pub execution_id: Uuid,
    /// Current pass number (1-indexed; 2 = validation pass in full_restructure)
    pub pass: u32,
    /// Accumulated log entries from hooks (surfaced in audit trail)
    pub log: Vec<String>,
    /// Arbitrary key/value metadata for hook-to-hook communication
    pub metadata: HashMap<String, String>,
    /// DID of the invoking agent, if provided
    pub agent_did: Option<String>,
}

impl HookContext {
    pub fn new(agent_did: Option<String>) -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            pass: 1,
            log: Vec::new(),
            metadata: HashMap::new(),
            agent_did,
        }
    }

    /// Write a log entry that will appear in the AuditTrail.hook_log
    pub fn log(&mut self, entry: impl Into<String>) {
        self.log.push(entry.into());
    }

    /// Retrieve typed metadata inserted by another hook.
    pub fn get_meta(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// Store typed metadata for downstream hooks.
    pub fn set_meta(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }
}
