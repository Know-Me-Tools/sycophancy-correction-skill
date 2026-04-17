# Changelog

## [Unreleased]

### Changed
- `.mcp.json` now uses the installed `sycophancy-correction` binary from PATH instead of `cargo run`. Eliminates multi-minute compile hangs at every skill invocation. For skill development against live source, use the new `.mcp.dev.json`.
- `SKILLS.md` reduced to a clear redirect stub (was duplicating skill description from `SKILL.md`).
- `README.md` Installation section updated to describe binary-first install flow.
- `strictness: adversarial` removed as dead code — the `Adversarial` variant had no differentiated runtime behavior from `Strict` (empty if block in `detector.rs`).
- `skill.toml` fix: `divergence_threshold`, `sample_n`, `sample_temperature` were incorrectly nested inside `[detection.severity_overrides]` causing a TOML parse error on startup. Moved to `[detection]`.

### Added
- `.mcp.dev.json` — development MCP config using `cargo run` for live-source iteration.
- `scripts/smoke-test.sh` — verifies the MCP server starts and exposes expected tools. Exit 0 on pass, non-zero with specific diagnostic on fail.
