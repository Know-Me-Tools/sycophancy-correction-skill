# `/init` Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Initialize the repo so guidance, setup, manifests, and core behavior are aligned with the v1 sycophancy-correction spec.

**Architecture:** Keep the existing two-crate split. Make public contracts more spec-faithful at the MCP boundary, keep compatibility where cheap, and add focused correctness improvements in `sycophancy-core` instead of rewriting the stack.

**Tech Stack:** Rust workspace, `rmcp`, `serde`, `tokio`, TOML/JSON manifests, Claude Code MCP config.

---

### Task 1: Save repo guidance and planning artifacts

**Files:**
- Create: `AGENTS.md`
- Create: `docs/plans/2026-04-09-init-design.md`
- Create: `docs/plans/2026-04-09-init-implementation.md`

**Step 1: Add repo guidance**

Write `AGENTS.md` with repo-specific instructions derived from `CLAUDE.md` and `docs/skill-sycophancy-correction-v1.html`.

**Step 2: Save the approved design**

Write the design rationale, scope, non-goals, and validation approach to `docs/plans/2026-04-09-init-design.md`.

**Step 3: Save this implementation plan**

Write the plan to `docs/plans/2026-04-09-init-implementation.md`.

### Task 2: Fix repo bootstrap and manifest coherence

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/sycophancy-core/Cargo.toml`
- Modify: `crates/sycophancy-mcp/Cargo.toml`
- Modify: `.mcp.json`
- Modify: `README.md`

**Step 1: Fix workspace inheritance**

Make the crate manifests inherit version, edition, license, authors, repository, homepage, keywords, and categories from the workspace.

**Step 2: Make local MCP startup repo-friendly**

Update `.mcp.json` to run the server from the repo via Cargo instead of assuming a globally installed binary.

**Step 3: Update setup docs**

Reflect the local MCP startup behavior and current initialization status in `README.md`.

### Task 3: Align MCP inputs and outputs with the spec

**Files:**
- Modify: `crates/sycophancy-mcp/src/tools.rs`
- Modify: `crates/sycophancy-mcp/src/server.rs`
- Modify: `crates/sycophancy-core/src/skill/types.rs`

**Step 1: Prefer canonical boundary fields**

Accept nested `context` and preferred `correction_mode` inputs, while retaining compatibility with existing flattened or legacy fields where practical.

**Step 2: Normalize enum string output**

Add helpers so strictness and related values emit lowercase spec-friendly strings instead of debug-formatted variants.

**Step 3: Enrich tool metadata**

Improve `skill_info` and tool schemas so they better reflect the spec and current runtime behavior.

### Task 4: Patch high-value core behavior gaps

**Files:**
- Modify: `crates/sycophancy-core/src/pmpo/executor.rs`
- Modify: `crates/sycophancy-core/src/skill/corrector.rs`
- Modify: `crates/sycophancy-mcp/src/server.rs`

**Step 1: Preserve true `detect_only` semantics**

Return reports without silently escalating into rewrite mode, while still surfacing `correction_mandatory` in outputs.

**Step 2: Add output validation**

Enforce the highest-value acceptance criteria locally before returning `SkillOutput`.

**Step 3: Use reflect-specific correction flow**

Wire `analyze_reflect_phase` to the specialized reflect corrector path so the output structure is closer to the spec.

### Task 5: Validate and document remaining gaps

**Files:**
- Modify: `README.md`

**Step 1: Run formatting checks**

Run: `cargo fmt --all --check`

Expected: PASS, or a precise formatting diff to fix.

**Step 2: Attempt build validation within local constraints**

Run: `cargo check`

Expected: PASS if dependencies are available locally; otherwise document the network restriction and stop short of pretending the build was verified.

**Step 3: Summarize residual non-init gaps**

Document remaining limitations such as the stubbed Anthropic client.
