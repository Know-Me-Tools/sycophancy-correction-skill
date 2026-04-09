//! Example: Webhook Notifier Hook
//!
//! Fires an HTTP POST to a configured webhook URL when sycophancy score
//! exceeds a threshold. Useful for Slack/Teams alerts, PagerDuty, or
//! custom monitoring pipelines.
//!
//! # Registration
//! ```rust
//! executor.hooks_mut().register(
//!     WebhookNotifierHook::builder()
//!         .url("https://hooks.slack.com/services/XXX/YYY/ZZZ")
//!         .threshold(0.7)
//!         .include_content(false) // redact content from webhook payload
//!         .build()
//! );
//! ```

use async_trait::async_trait;
use sycophancy_core::{
    hooks::{Hook, HookContext, HookResult},
    skill::types::DetectionResult,
};

pub struct WebhookNotifierHook {
    url:             String,
    threshold:       f32,
    include_content: bool,
}

// ── Builder ───────────────────────────────────────────────────────────────────

pub struct WebhookNotifierHookBuilder {
    url:             Option<String>,
    threshold:       f32,
    include_content: bool,
}

impl WebhookNotifierHookBuilder {
    pub fn new() -> Self {
        Self { url: None, threshold: 0.6, include_content: false }
    }
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into()); self
    }
    pub fn threshold(mut self, t: f32) -> Self {
        self.threshold = t; self
    }
    pub fn include_content(mut self, v: bool) -> Self {
        self.include_content = v; self
    }
    pub fn build(self) -> WebhookNotifierHook {
        WebhookNotifierHook {
            url:             self.url.expect("webhook url is required"),
            threshold:       self.threshold,
            include_content: self.include_content,
        }
    }
}

impl WebhookNotifierHook {
    pub fn builder() -> WebhookNotifierHookBuilder {
        WebhookNotifierHookBuilder::new()
    }
}

// ── Hook impl ─────────────────────────────────────────────────────────────────

#[async_trait]
impl Hook for WebhookNotifierHook {
    fn name(&self)     -> &str { "example.webhook_notifier" }
    fn priority(&self) -> i32  { 50 }

    async fn after_detect(
        &self,
        ctx:    &mut HookContext,
        result: &DetectionResult,
    ) -> HookResult {
        if result.sycophancy_score < self.threshold {
            return HookResult::Continue;
        }

        let payload = serde_json::json!({
            "execution_id": ctx.execution_id.to_string(),
            "agent_did":    ctx.agent_did.as_deref().unwrap_or("unknown"),
            "score":        result.sycophancy_score,
            "has_critical": result.has_critical,
            "pattern_count": result.classifications.len(),
            "patterns": result.classifications
                .iter()
                .map(|c| &c.pattern_id)
                .collect::<Vec<_>>(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        // In production: fire-and-forget POST via reqwest
        // reqwest::Client::new().post(&self.url).json(&payload).send().await
        //
        // Stubbed here — add reqwest to your Cargo.toml to enable:
        let _ = (&self.url, &payload, self.include_content);
        tracing::info!(
            url   = &self.url,
            score = result.sycophancy_score,
            "webhook notification triggered (stubbed)"
        );

        ctx.log(format!(
            "WebhookNotifierHook: threshold {:.2} exceeded (score {:.2}) — notified {}",
            self.threshold, result.sycophancy_score, self.url
        ));

        HookResult::Continue
    }
}
