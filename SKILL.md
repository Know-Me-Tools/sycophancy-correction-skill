---
name: sycophancy-correction
description: Detect and correct sycophantic patterns in prompts, completions, agent descriptors, pipeline configs, and PMPO Reflect outputs. Use when an artifact is overly agreeable, ungrounded, structurally flattering, or insufficiently critical.
license: MIT
compatibility: Designed for Agent Skills-compatible tools, Claude-compatible skill loaders, and repository scanners. Local execution requires Rust and Cargo; first build may require network access to fetch crates.
metadata:
  author: Prometheus AGS
  version: "1.0.0"
  skill_id: sycophancy.correction
  namespace: prometheus-ags
---

# Sycophancy Correction

## When to use this skill

Use this skill when you need to inspect or rewrite an artifact that may be too agreeable, flattering, scope-expanding, or structurally unwilling to challenge assumptions.

Typical triggers:

- completions that open with approval instead of reasoning
- prompts or agent specs that suppress critique or reward agreement over truth
- PMPO Reflect outputs that summarize success instead of naming deltas and root causes
- multi-turn outputs that drift toward prior user framing without grounding

## What this skill does

- Detects the eight canonical sycophancy patterns `S-01` through `S-08`
- Scores artifacts on a `0.0` to `1.0` sycophancy scale
- Returns classifications with severity, rationale, and audit metadata
- Supports `detect_only`, `annotate`, `rewrite`, and `full_restructure`
- Applies specialized restructuring for Reflect-phase analysis

## Repository map

- `docs/skill-sycophancy-correction-v1.html` is the canonical external contract
- `skill.toml` contains runtime configuration
- `agentskills.json` and `claude-plugin.json` define marketplace and plugin metadata
- `crates/sycophancy-core` contains detection, correction, hooks, and PMPO orchestration
- `crates/sycophancy-mcp` exposes the MCP server and tool boundary

## Key constraints

- Keep transport logic in `crates/sycophancy-mcp`; keep domain logic in `crates/sycophancy-core`
- Preserve stderr-only diagnostics for MCP server logging
- Keep audit output complete and use spec-aligned lowercase enum values at public boundaries
- For Reflect-phase correction, preserve `Delta -> Root Cause -> Corrective Actions`

## Validation notes

- Prefer `cargo fmt --all --check`, then `cargo check` and `cargo test`
- If dependencies are not cached locally, Cargo validation may need network access
- The Anthropic client is currently stubbed; detection and contract validation are the main initialized behaviors
