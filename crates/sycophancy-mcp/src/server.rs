//! MCP server handler.
//!
//! Implements `rmcp::ServerHandler` and exposes four tools:
//!   - detect_sycophancy
//!   - correct_sycophancy
//!   - analyze_reflect_phase
//!   - skill_info

use anyhow::Result;
use rmcp::{
    model::{
        CallToolRequestParam, CallToolResult, Content, Implementation, InitializeResult,
        ListToolsResult, PaginatedRequestParam, ProtocolVersion, ServerCapabilities,
        ToolsCapability,
    },
    service::{RequestContext, ServiceExt},
    ServerHandler,
};
use serde_json::{json, Value};
use std::sync::Arc;
use sycophancy_core::{config::SkillConfig, pmpo::PmpoExecutor};
use tokio::sync::Mutex;

use crate::tools::{AnalyzeReflectPhaseInput, CorrectSycophancyInput, DetectSycophancyInput};

// ── Shared LLM client (Anthropic) ────────────────────────────────────────────

pub struct AnthropicClient {
    base_url: String,
    model: String,
}

impl AnthropicClient {
    pub fn new(base_url: String, model: String) -> Self {
        Self { base_url, model }
    }
}

#[async_trait::async_trait]
impl sycophancy_core::skill::corrector::LlmClient for AnthropicClient {
    async fn complete(
        &self,
        _system: &str,
        user: &str,
        max_tokens: u32,
    ) -> sycophancy_core::error::SkillResult<String> {
        // In a real deployment: call the Anthropic messages API.
        // The API key is read from ANTHROPIC_API_KEY env var.
        // Stubbed here to keep the binary dependency-light.
        let _ = (&self.base_url, &self.model, max_tokens);
        let response = format!(
            "<reasoning>\nStub correction — replace AnthropicClient::complete \
             with a real HTTP call to {}/v1/messages using model {}.\n</reasoning>\n\n\
             [CORRECTED ARTIFACT — stub]\n{user}",
            self.base_url, self.model
        );
        Ok(response)
    }
}

// ── Server Handler ────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SycophancyServer {
    executor: Arc<Mutex<PmpoExecutor>>,
    client: Arc<AnthropicClient>,
    config: SkillConfig,
}

impl SycophancyServer {
    pub fn new(executor: PmpoExecutor, config: SkillConfig) -> Self {
        let client = Arc::new(AnthropicClient::new(
            config.llm.base_url.clone(),
            config.llm.critic_model.clone(),
        ));
        Self {
            executor: Arc::new(Mutex::new(executor)),
            client,
            config,
        }
    }
}

#[async_trait::async_trait]
impl ServerHandler for SycophancyServer {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                ..Default::default()
            },
            server_info: Implementation {
                name: "sycophancy-correction".into(),
                version: self.config.skill.version.clone(),
            },
            instructions: Some(
                "Detects and corrects sycophantic patterns in LLM completions, \
                 prompts, agent descriptors, and pipeline configurations. \
                 Use detect_sycophancy first, then correct_sycophancy if needed."
                    .into(),
            ),
        }
    }

    fn list_tools(
        &self,
        _request: PaginatedRequestParam,
        _ctx: RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, rmcp::Error>> + Send + '_ {
        std::future::ready(Ok(ListToolsResult {
            next_cursor: None,
            tools: crate::tools::tool_definitions(),
        }))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        _ctx: RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, rmcp::Error>> + Send + '_ {
        async move {
            let args = request.arguments.map(Value::Object).unwrap_or(json!({}));

            match request.name.as_ref() {
                "detect_sycophancy" => {
                    let input: DetectSycophancyInput = serde_json::from_value(args)
                        .map_err(|e| rmcp::Error::invalid_params(e.to_string(), None))?;
                    self.handle_detect(input).await
                }
                "correct_sycophancy" => {
                    let input: CorrectSycophancyInput = serde_json::from_value(args)
                        .map_err(|e| rmcp::Error::invalid_params(e.to_string(), None))?;
                    self.handle_correct(input).await
                }
                "analyze_reflect_phase" => {
                    let input: AnalyzeReflectPhaseInput = serde_json::from_value(args)
                        .map_err(|e| rmcp::Error::invalid_params(e.to_string(), None))?;
                    self.handle_reflect(input).await
                }
                "skill_info" => self.handle_info().await,
                _ => Err(rmcp::Error::invalid_params("tool not found", None)),
            }
        }
    }
}

// ── Tool Handlers ─────────────────────────────────────────────────────────────

impl SycophancyServer {
    async fn handle_detect(
        &self,
        input: DetectSycophancyInput,
    ) -> Result<CallToolResult, rmcp::Error> {
        use sycophancy_core::skill::types::{CorrectionMode, SkillInput};

        let target = input.target.parse_target();
        let content = input.content.clone();
        let context = input.input_context();
        let strictness = input.parse_strictness();
        let agent_did = input.agent_did.clone();

        let skill_input = SkillInput {
            target,
            content,
            context,
            correction_mode: CorrectionMode::DetectOnly,
            strictness,
        };

        let correction_mandatory_threshold = self.config.correction.mandatory_correction_threshold;

        let executor = self.executor.lock().await;
        let output = executor
            .execute(skill_input, self.client.as_ref(), agent_did)
            .await
            .map_err(|e| rmcp::Error::internal_error(e.to_string(), None))?;

        let result = json!({
            "sycophancy_score": output.sycophancy_score,
            "classifications":  output.classifications,
            "correction_mandatory": output.sycophancy_score >= correction_mandatory_threshold
                || output.classifications.iter().any(|c| c.severity == sycophancy_core::skill::types::Severity::Critical),
            "delta_summary":    output.delta_summary,
            "audit_trail":      output.audit_trail,
        });

        Ok(CallToolResult {
            content: vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_default(),
            )],
            is_error: Some(false),
        })
    }

    async fn handle_correct(
        &self,
        input: CorrectSycophancyInput,
    ) -> Result<CallToolResult, rmcp::Error> {
        use sycophancy_core::skill::types::SkillInput;

        let target = input.target.parse_target();
        let content = input.content.clone();
        let context = input.input_context();
        let correction_mode = input.parse_mode();
        let strictness = input.parse_strictness();
        let agent_did = input.agent_did.clone();

        let skill_input = SkillInput {
            target,
            content,
            context,
            correction_mode,
            strictness,
        };

        let executor = self.executor.lock().await;
        let output = executor
            .execute(skill_input, self.client.as_ref(), agent_did)
            .await
            .map_err(|e| rmcp::Error::internal_error(e.to_string(), None))?;

        let result = json!({
            "sycophancy_score":    output.sycophancy_score,
            "corrected_artifact":  output.corrected_artifact,
            "delta_summary":       output.delta_summary,
            "classifications":     output.classifications,
            "audit_trail":         output.audit_trail,
        });

        Ok(CallToolResult {
            content: vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_default(),
            )],
            is_error: Some(false),
        })
    }

    async fn handle_reflect(
        &self,
        input: AnalyzeReflectPhaseInput,
    ) -> Result<CallToolResult, rmcp::Error> {
        let executor = self.executor.lock().await;
        let output = executor
            .execute_reflect_phase(
                input.reflect_output,
                self.client.as_ref(),
                None,
                input.correct.unwrap_or(true),
            )
            .await
            .map_err(|e| rmcp::Error::internal_error(e.to_string(), None))?;

        let result = json!({
            "s08_detected":       output.classifications.iter().any(|c| c.pattern_id == "S-08"),
            "sycophancy_score":   output.sycophancy_score,
            "corrected_reflect":  output.corrected_artifact,
            "delta_summary":      output.delta_summary,
        });

        Ok(CallToolResult {
            content: vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_default(),
            )],
            is_error: Some(false),
        })
    }

    async fn handle_info(&self) -> Result<CallToolResult, rmcp::Error> {
        let info = json!({
            "skill_id":   "sycophancy.correction",
            "version":    &self.config.skill.version,
            "author":     &self.config.skill.author,
            "patterns":   ["S-01","S-02","S-03","S-04","S-05","S-06","S-07","S-08"],
            "modes":      ["detect_only","annotate","rewrite","full_restructure"],
            "strictness": ["permissive","standard","strict"],
            "pmpo":       "compliant",
            "uar":        "compatible",
            "validation_contract": "strict",
            "capabilities": ["llm.invoke", "tool.run", "memory.read", "memory.write"],
            "hooks": [
                "before_detect", "after_detect", "on_classify", "on_score",
                "before_correct", "after_correct", "before_validate",
                "on_complete", "on_error"
            ],
            "homepage":   "https://agentskills.io/skills/sycophancy-correction",
        });
        Ok(CallToolResult {
            content: vec![Content::text(
                serde_json::to_string_pretty(&info).unwrap_or_default(),
            )],
            is_error: Some(false),
        })
    }
}

// ── Serve ─────────────────────────────────────────────────────────────────────

pub async fn serve(executor: PmpoExecutor, config: SkillConfig) -> Result<()> {
    use rmcp::transport::io::stdio;

    let server = SycophancyServer::new(executor, config);

    tracing::info!("sycophancy-correction MCP server starting on stdio");
    server.serve(stdio()).await?.waiting().await?;
    Ok(())
}
