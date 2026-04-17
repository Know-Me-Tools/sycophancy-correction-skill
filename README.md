# sycophancy.correction — v1.0.0

An [agentskills.io](https://agentskills.io) compliant skill for detecting and correcting
sycophantic patterns in LLM completions, prompts, agent descriptors, and pipeline configurations.

Distributed as an MCP server binary with full Claude Code plugin and marketplace support.

Canonical Agent Skills metadata lives in `SKILL.md`; a compatibility index for tools that look for a root skills manifest lives in `SKILLS.md`.

---

## Installation

### Claude Code (Recommended)

1. Install the binary (one-time):

```bash
cargo build --release
cp target/release/sycophancy-correction ~/.local/bin/
# or: cargo install --path crates/sycophancy-mcp
```

2. The included `.mcp.json` invokes the installed binary from PATH:

```json
{
  "mcpServers": {
    "sycophancy-correction": {
      "command": "sycophancy-correction",
      "args": ["--config", "skill.toml"]
    }
  }
}
```

3. Verify the install:

```bash
./scripts/smoke-test.sh
```

For skill development against live source (recompiles on every invocation),
copy `.mcp.dev.json` over `.mcp.json`.

### Environment Variables

| Variable            | Required | Description                          |
|--------------------|----------|--------------------------------------|
| `ANTHROPIC_API_KEY` | Yes      | Used by the correction LLM calls     |
| `RUST_LOG`          | No       | Log level: `trace/debug/info/warn`   |
| `SKILL_CONFIG`      | No       | Path to skill.toml (default: `./skill.toml`) |

---

## Tools

| Tool                    | Description                                              |
|------------------------|----------------------------------------------------------|
| `detect_sycophancy`     | Score and classify patterns — no mutation, read-only     |
| `correct_sycophancy`    | Detect + rewrite in one call                             |
| `analyze_reflect_phase` | PMPO Reflect phase specialist — enforces Delta → Root Cause → Actions |
| `skill_info`            | Returns pattern library and capability metadata          |

---

## Detected Patterns

| ID   | Name                       | Default Severity |
|------|---------------------------|-----------------|
| S-01 | Unprompted Affirmation     | Medium          |
| S-02 | Agreement Without Grounding| High            |
| S-03 | Caveat Collapse            | Critical        |
| S-04 | Self-Rationalization       | Critical        |
| S-05 | Context Bleed Alignment    | High            |
| S-06 | Confidence Without Basis   | Medium          |
| S-07 | Scope Creep Flattery       | Low             |
| S-08 | Reflect Phase Inversion    | High            |

---

## Hook System

The skill exposes nine lifecycle hooks for extensibility without forking the core:

```
before_detect → after_detect → on_classify → on_score →
before_correct → after_correct → before_validate → on_complete / on_error
```

See [`hooks/README.md`](./hooks/README.md) for implementation guide and examples.

### Builtin Hooks

- **`builtin.tracing`** — structured tracing events at every lifecycle point (priority: -100)
- **`builtin.audit`** — writes JSON audit records to stdout/file/SurrealDB (priority: 100)

Enable in `skill.toml`:
```toml
[hooks]
tracing_hook = true
audit_hook   = true
```

---

## Configuration

All runtime behavior is driven by `skill.toml`. Key sections:

```toml
[detection]
disabled_patterns = ["S-07"]           # suppress specific patterns
severity_overrides = { "S-01" = "high" }

[correction]
mandatory_correction_threshold = 0.6   # flag that correction is mandatory for callers
max_passes = 2

[llm]
critic_model  = "claude-sonnet-4-6"
rewrite_model = "claude-sonnet-4-6"
```

---

## Crate Structure

```
crates/
  sycophancy-core/     # Library: detection, scoring, correction, hooks, PMPO executor
  sycophancy-mcp/      # Binary: MCP server, tool definitions, Anthropic client
hooks/
  examples/            # Drop-in hook implementations
agentskills.json       # agentskills.io marketplace manifest
claude-plugin.json     # Claude Code marketplace metadata
skill.toml             # Runtime config + agentskills.io manifest
.mcp.json              # Claude Code project-level MCP config
```

---

## Marketplace

- **agentskills.io:** `https://agentskills.io/skills/sycophancy-correction`
- **Namespace:** `prometheus-ags`
- **Validation contract:** `strict`

---

## Current Initialization Status

- Public manifests, MCP config, and Rust contracts are aligned toward the v1 skill spec.
- `detect_only` remains report-only and surfaces whether correction is mandatory.
- Reflect-phase correction now uses the dedicated reflect restructuring path.
- Root `SKILL.md` is present for Agent Skills compliance; root `SKILLS.md` is present as a compatibility index.
- The Anthropic client remains intentionally stubbed; real provider integration is still a follow-up task.

---

## License

MIT © Prometheus AGS — [travisjames.ai](https://travisjames.ai)
