# AGENTS.md

## Repo Intent

This repository implements the `sycophancy.correction` skill as a Rust MCP server and Claude Code plugin.

## Sources Of Truth

- Treat `docs/skill-sycophancy-correction-v1.html` as the canonical external behavior and contract.
- Use `CLAUDE.md` for architecture, crate boundaries, and operational guidance.
- Keep `agentskills.json`, `claude-plugin.json`, `.mcp.json`, and `skill.toml` aligned with the Rust implementation.

## Working Rules

- Prefer spec-aligned public names at boundaries, but use compatibility shims instead of unnecessary breaking changes.
- Keep `sycophancy-core` free of transport concerns; MCP-specific behavior belongs in `crates/sycophancy-mcp`.
- Preserve stderr logging for server diagnostics; never write non-JSON-RPC output to stdout.
- When changing output contracts, keep audit metadata complete and use lowercase snake_case values where the spec expects them.
- For Reflect-phase handling, preserve the required `Delta -> Root Cause -> Corrective Actions` structure.

## Validation

- Start with focused checks such as `cargo fmt --all --check` and targeted `cargo check`/`cargo test` when dependencies are available.
- If network-restricted, prefer validations that do not require fetching crates and note any remaining limits in your handoff.

## Documentation

- Update docs when initialization changes project setup, public tool contracts, or developer workflow.
- Save design and implementation planning artifacts under `docs/plans/`.
