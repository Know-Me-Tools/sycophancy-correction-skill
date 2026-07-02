# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

A Rust MCP server implementing the `sycophancy.correction` agentskills.io skill. It detects and corrects sycophantic patterns (S-01 through S-08) in LLM completions, prompts, agent descriptors, and pipeline configs. Distributed as a Claude Code plugin.

## Build & Run

```bash
cargo build --release                    # build the MCP server binary
cargo build                              # debug build
cargo check                              # type-check without linking
cargo test                               # run all workspace tests
cargo test -p sycophancy-core            # test core library only
cargo test -p sycophancy-mcp             # test MCP server only
cargo test -p sycophancy-core -- <name>  # run a single test by name
```

The binary is `target/release/sycophancy-correction`. It communicates over stdio (JSON-RPC). Its LLM client speaks the OpenAI-compatible `/chat/completions` wire format and defaults `llm.base_url` (in `skill.toml`) to a local openai-proxy at `http://localhost:8181/v1` — no API key needed by default. Set `SYCOPHANCY_LLM_API_KEY` only if `llm.base_url` is repointed at a provider that validates the Authorization header.

Logging goes to stderr (MCP requirement — stdout is reserved for JSON-RPC). Control verbosity via `RUST_LOG=debug`.

## Architecture

Two-crate workspace:

**`sycophancy-core`** (library) — all domain logic, no network I/O:
- `config` — `SkillConfig` deserialized from `skill.toml` (detection thresholds, LLM models, hook toggles, audit backend)
- `skill::patterns` — `Pattern` trait + `PatternRegistry` with 8 built-in heuristics. Each pattern runs regex-based detection against content and returns `Vec<HeuristicMatch>`. Extend by implementing `Pattern` and calling `registry.register()`.
- `skill::detector` — `Detector` orchestrates the pattern registry, applies severity overrides and disabled-patterns from config, delegates scoring to `Scorer`
- `skill::scorer` — severity-weighted score in [0.0, 1.0], clamped against theoretical ceiling
- `skill::corrector` — `Corrector` builds critic/red-team system prompts per the spec and delegates to any `LlmClient` implementor. The `LlmClient` trait is the integration seam for swapping LLM providers.
- `hooks` — `Hook` trait with 9 lifecycle events (`before_detect` through `on_error`). `HookRegistry` dispatches in priority order. Hooks return `HookResult` (Continue/Mutate/Abort/SkipRemaining). Mutations are declarative via `HookMutation` — the executor applies them, not the hook.
- `pmpo::PmpoExecutor` — top-level orchestrator implementing the 6-phase PMPO loop (Specify, Plan, Execute, Reflect, Persist, Terminate). Wires detector + corrector + hooks. This is the entry point for all skill invocations.

**`sycophancy-mcp`** (binary) — thin MCP transport layer:
- `main.rs` — config loading, hook registration, executor construction, server launch
- `server.rs` — `SycophancyServer` implements `rmcp::ServerHandler`. Contains `ProxyLlmClient`, a real OpenAI-compatible `/chat/completions` client (default: local openai-proxy, :8181). Four tool handlers delegate to `PmpoExecutor`.
- `tools.rs` — MCP tool schema definitions and string-to-enum parse helpers

**Key data flow:** MCP tool call -> `SycophancyServer::call_tool` -> `PmpoExecutor::execute` -> hooks + detector + scorer + corrector -> `SkillOutput`

## Key Types

- `SkillInput` / `SkillOutput` — the input/output contract (matches agentskills.json schema)
- `TargetType` — `Prompt | Completion | AgentDescriptor | Pipeline`
- `CorrectionMode` — `DetectOnly | Annotate | Rewrite | FullRestructure`
- `Strictness` — `Permissive | Standard | Strict`
- `Severity` — `Low | Medium | High | Critical` with weighted scoring via `.weight()`
- `DetectionResult` — score + classifications + correction_mandatory flag
- `HookResult` / `HookMutation` — hook return semantics

## Important Invariants

- The `ProxyLlmClient` in `server.rs` makes real HTTP calls to `llm.base_url` (`skill.toml`) using the OpenAI-compatible `/chat/completions` format. `rewrite_model` is defined in config but not yet wired to a distinct client — both critic and rewrite passes currently use `critic_model`.
- In `FullRestructure` mode, the corrected artifact runs through a second detection pass. If score >= `clean_threshold` (0.1), it loops. Max 2 passes, then `SkillError::CorrectionFailed`.
- Critical classifications or score >= 0.6 mark correction as mandatory for callers, but `DetectOnly` remains report-only.
- S-07 (Scope Creep Flattery) is suppressed at `Permissive` strictness.
- S-05 (Context Bleed) requires `prior_completions` in the input context — it's a no-op without them.

## Configuration

All runtime behavior is driven by `skill.toml`. Key sections: `[detection]` (disabled patterns, severity overrides, divergence thresholds), `[correction]` (mandatory threshold, max passes), `[hooks]` (enable builtins), `[audit]` (backend: stdout/file/surreal_db), `[llm]` (models, max tokens).

## MCP Tools

| Tool | Purpose |
|------|---------|
| `detect_sycophancy` | Score + classify only (read-only) |
| `correct_sycophancy` | Detect + rewrite in one call |
| `analyze_reflect_phase` | S-08 specialist — enforces Delta/RootCause/Actions structure |
| `skill_info` | Returns metadata, no args |

## Hook System

9 lifecycle events, priority-ordered (lower runs first). Builtin hooks: `builtin.tracing` (priority -100), `builtin.audit` (priority 100). Custom hooks implement the `Hook` trait and register via `executor.hooks_mut().register()`. See `hooks/examples/` for patterns: custom pattern injection, SurrealDB audit, webhook notifications.

## Manifest Files

- `skill.toml` — runtime config + agentskills.io manifest
- `agentskills.json` — agentskills.io marketplace schema (input/output contracts, acceptance criteria, patterns)
- `claude-plugin.json` — Claude Code marketplace metadata
- `.mcp.json` — Claude Code project-level MCP server registration
