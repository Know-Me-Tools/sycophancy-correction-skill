# `/init` Design

**Goal:** Initialize the repository so Codex-facing guidance, manifests, MCP setup, and core Rust contracts all reflect the v1 skill spec closely enough to make future work predictable.

**Chosen approach:** Balanced, spec-first initialization. The HTML spec remains the source of truth for public behavior, while internal Rust changes stay incremental and compatibility-minded.

## Scope

- Add a root `AGENTS.md` derived from `CLAUDE.md` and the skill spec.
- Normalize bootstrap files and manifests so the repo is coherent for local development.
- Align public contracts across manifests, MCP tools, and core Rust types.
- Implement the highest-value missing behavior for initialization, especially around detect-only semantics, reflect-phase handling, and output validation.

## Non-Goals

- Full Anthropic API integration.
- A complete rewrite of the detection/correction engine.
- Solving every future roadmap item described in the spec.

## Key Decisions

### 1. Spec is canonical at public boundaries

The HTML skill spec defines the expected target types, correction modes, audit output, reflect-phase behavior, and acceptance criteria. Public JSON contracts and tool schemas should therefore prefer spec-aligned names and values.

### 2. Compatibility shims beat gratuitous breakage

Where the repo already exposes different field names, initialization should accept both the current and the preferred names when practical. This keeps the repo usable while still nudging it toward the canonical shape.

### 3. `/init` should improve real behavior, not only metadata

Initialization is not just documentation. The repo should also gain a few high-value implementation improvements that make the skill safer and more internally consistent:

- preserve true `detect_only` semantics,
- expose a more spec-faithful reflect-phase path,
- validate output invariants before returning results,
- fix workspace manifest inheritance so the crates use the intended Rust edition and package metadata.

## Planned Changes

### Repo guidance and setup

- Add `AGENTS.md` at the repo root.
- Save this design plus an implementation plan under `docs/plans/`.
- Update `.mcp.json` so local development works from the repo without requiring a globally installed binary.

### Manifest and contract alignment

- Fix workspace package inheritance in the crate manifests.
- Keep marketplace/plugin manifests aligned with the current runtime setup.
- Prefer `correction_mode` and nested `context` at tool boundaries while accepting legacy flattened fields when reasonable.

### Core behavior upgrades

- Make `detect_only` return a report without silently escalating into rewrite mode.
- Normalize enum serialization/display values to the lowercase forms expected by the spec.
- Add output validation checks for major acceptance-criteria invariants.
- Use the specialized reflect-phase corrector in the reflect-specific flow.

## Validation Strategy

- Run formatting checks locally.
- Run build checks only if dependencies are locally available; otherwise document the network restriction.
- Summarize any remaining gaps explicitly instead of hiding them.
