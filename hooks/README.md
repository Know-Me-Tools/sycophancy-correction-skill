# Hook Examples

Drop-in hook implementations demonstrating the three most common extension patterns.
Copy any of these into your project and register via `executor.hooks_mut().register(...)`.

## Available Examples

### `custom_pattern_hook.rs`
Injects a domain-specific pattern at runtime without touching the core library.
Use when your evaluation domain has sycophancy signals the canonical S-01..S-08 set
doesn't cover (e.g. medical flattery, financial over-validation).

### `prometheus_audit_hook.rs`
Writes execution records to `surreal-memory-server` under the agent's DID namespace.
Also includes `CedarGovernanceHook` — enforces the `architect` role requirement for
`full_restructure` mode using metadata set by the UAR Cedar PEP.

### `webhook_notifier_hook.rs`
Fires an HTTP POST when sycophancy score exceeds a threshold.
Includes a fluent builder. Activate via:
```rust
executor.hooks_mut().register(
    WebhookNotifierHook::builder()
        .url("https://hooks.slack.com/services/XXX")
        .threshold(0.7)
        .build()
);
```

## Hook Priority Guidelines

| Priority Range | Use Case                                      |
|---------------|-----------------------------------------------|
| < -100        | Content normalization, Cedar gates            |
| -100 to 0     | Tracing, observability (builtin.tracing = -100) |
| 0 to 100      | Business logic, pattern injection, webhooks   |
| > 100         | Audit writes, persistence (builtin.audit = 100) |

## Implementing a New Hook

```rust
use sycophancy_core::hooks::{Hook, HookContext, HookResult};
use async_trait::async_trait;

pub struct MyHook;

#[async_trait]
impl Hook for MyHook {
    fn name(&self) -> &str { "my_namespace.my_hook" }
    fn priority(&self) -> i32 { 25 }

    async fn after_detect(
        &self,
        ctx: &mut HookContext,
        result: &sycophancy_core::skill::types::DetectionResult,
    ) -> HookResult {
        // Your logic here
        ctx.log(format!("MyHook ran, score={:.2}", result.sycophancy_score));
        HookResult::Continue
    }
}
```

Register it:
```rust
executor.hooks_mut().register(MyHook);
```
