//! Built-in `tool_search` tool for on-demand tool schema loading.
//!
//! When deferred loading is enabled (for MCP tools, built-in tools, or both),
//! this tool lets the LLM discover and activate deferred tools. Supports two
//! query modes:
//! - `select:name1,name2` — fetch exact tools by name.
//! - Free-text keyword search — returns the best-matching stubs.

use std::fmt::Write;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::tools::deferred_builtin::DeferredToolRegistry;
use crate::tools::mcp_deferred::ActivatedToolSet;
use crate::tools::traits::{Tool, ToolResult};

/// Default maximum number of search results.
const DEFAULT_MAX_RESULTS: usize = 5;

/// Built-in tool that fetches full schemas for deferred tools.
pub struct ToolSearchTool {
    deferred: DeferredToolRegistry,
    activated: Arc<Mutex<ActivatedToolSet>>,
}

impl ToolSearchTool {
    pub fn new(deferred: DeferredToolRegistry, activated: Arc<Mutex<ActivatedToolSet>>) -> Self {
        Self {
            deferred,
            activated,
        }
    }
}

#[async_trait]
impl Tool for ToolSearchTool {
    fn name(&self) -> &str {
        "tool_search"
    }

    fn description(&self) -> &str {
        "Fetch full schema definitions for deferred tools so they can be called. \
         Use \"select:name1,name2\" for exact match or keywords to search."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "description": "Query to find deferred tools. Use \"select:<tool_name>\" for direct selection, or keywords to search.",
                    "type": "string"
                },
                "max_results": {
                    "description": "Maximum number of results to return (default: 5)",
                    "type": "number",
                    "default": DEFAULT_MAX_RESULTS
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim();

        let max_results = args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| usize::try_from(v).unwrap_or(DEFAULT_MAX_RESULTS))
            .unwrap_or(DEFAULT_MAX_RESULTS);

        if query.is_empty() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("query parameter is required".into()),
            });
        }

        // Parse query mode
        if let Some(names_str) = query.strip_prefix("select:") {
            // Exact selection mode
            let names: Vec<&str> = names_str.split(',').map(str::trim).collect();
            return self.select_tools(&names);
        }

        // Keyword search mode
        let results = self.deferred.search(query, max_results);
        if results.is_empty() {
            return Ok(ToolResult {
                success: true,
                output: "No matching deferred tools found.".into(),
                error: None,
            });
        }

        // Activate and return full specs
        let mut output = String::from("<functions>\n");
        let mut activated_count = 0;
        let mut guard = self.activated.lock().unwrap();

        for stub_info in &results {
            if let Some(spec) = self.deferred.tool_spec(stub_info.name) {
                if !guard.is_activated(stub_info.name) {
                    if let Some(tool) = self.deferred.activate(stub_info.name) {
                        guard.activate(stub_info.name.to_string(), tool);
                        activated_count += 1;
                    }
                }
                let _ = writeln!(
                    output,
                    "<function>{{\"name\": \"{}\", \"description\": \"{}\", \"parameters\": {}}}</function>",
                    spec.name,
                    spec.description.replace('"', "\\\""),
                    spec.parameters
                );
            }
        }

        output.push_str("</functions>\n");
        drop(guard);

        tracing::debug!(
            "tool_search: query={query:?}, matched={}, activated={activated_count}",
            results.len()
        );

        Ok(ToolResult {
            success: true,
            output,
            error: None,
        })
    }
}

impl ToolSearchTool {
    fn select_tools(&self, names: &[&str]) -> anyhow::Result<ToolResult> {
        let mut output = String::from("<functions>\n");
        let mut not_found = Vec::new();
        let mut activated_count = 0;
        let mut guard = self.activated.lock().unwrap();

        for name in names {
            if name.is_empty() {
                continue;
            }
            match self.deferred.tool_spec(name) {
                Some(spec) => {
                    if !guard.is_activated(name) {
                        if let Some(tool) = self.deferred.activate(name) {
                            guard.activate(name.to_string(), tool);
                            activated_count += 1;
                        }
                    }
                    let _ = writeln!(
                        output,
                        "<function>{{\"name\": \"{}\", \"description\": \"{}\", \"parameters\": {}}}</function>",
                        spec.name,
                        spec.description.replace('"', "\\\""),
                        spec.parameters
                    );
                }
                None => {
                    not_found.push(*name);
                }
            }
        }

        output.push_str("</functions>\n");
        drop(guard);

        if !not_found.is_empty() {
            let _ = write!(output, "\nNot found: {}", not_found.join(", "));
        }

        tracing::debug!(
            "tool_search select: requested={}, activated={activated_count}, not_found={}",
            names.len(),
            not_found.len()
        );

        Ok(ToolResult {
            success: true,
            output,
            error: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::deferred_builtin::{DeferredBuiltinStub, DeferredToolRegistry};
    use crate::tools::mcp_client::McpRegistry;
    use crate::tools::mcp_deferred::{DeferredMcpToolSet, DeferredMcpToolStub};
    use crate::tools::mcp_protocol::McpToolDef;

    async fn make_mcp_deferred_set(stubs: Vec<DeferredMcpToolStub>) -> DeferredMcpToolSet {
        let registry = Arc::new(McpRegistry::connect_all(&[]).await.unwrap());
        DeferredMcpToolSet { stubs, registry }
    }

    fn make_mcp_stub(name: &str, desc: &str) -> DeferredMcpToolStub {
        let def = McpToolDef {
            name: name.to_string(),
            description: Some(desc.to_string()),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
        };
        DeferredMcpToolStub::new(name.to_string(), def)
    }

    struct FakeBuiltinTool {
        tool_name: &'static str,
        tool_desc: &'static str,
    }

    #[async_trait]
    impl Tool for FakeBuiltinTool {
        fn name(&self) -> &str {
            self.tool_name
        }
        fn description(&self) -> &str {
            self.tool_desc
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object", "properties": {}})
        }
        async fn execute(&self, _: serde_json::Value) -> anyhow::Result<ToolResult> {
            Ok(ToolResult {
                success: true,
                output: String::new(),
                error: None,
            })
        }
    }

    fn make_builtin_stub(name: &'static str, desc: &'static str) -> DeferredBuiltinStub {
        DeferredBuiltinStub::from_tool(Arc::new(FakeBuiltinTool {
            tool_name: name,
            tool_desc: desc,
        }))
    }

    #[tokio::test]
    async fn tool_metadata() {
        let tool = ToolSearchTool::new(
            DeferredToolRegistry::new(None, vec![]),
            Arc::new(Mutex::new(ActivatedToolSet::new())),
        );
        assert_eq!(tool.name(), "tool_search");
        assert!(!tool.description().is_empty());
        assert!(tool.parameters_schema()["properties"]["query"].is_object());
    }

    #[tokio::test]
    async fn empty_query_returns_error() {
        let tool = ToolSearchTool::new(
            DeferredToolRegistry::new(None, vec![]),
            Arc::new(Mutex::new(ActivatedToolSet::new())),
        );
        let result = tool
            .execute(serde_json::json!({"query": ""}))
            .await
            .unwrap();
        assert!(!result.success);
    }

    #[tokio::test]
    async fn select_nonexistent_tool_reports_not_found() {
        let tool = ToolSearchTool::new(
            DeferredToolRegistry::new(None, vec![]),
            Arc::new(Mutex::new(ActivatedToolSet::new())),
        );
        let result = tool
            .execute(serde_json::json!({"query": "select:nonexistent"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("Not found"));
    }

    #[tokio::test]
    async fn keyword_search_no_matches() {
        let tool = ToolSearchTool::new(
            DeferredToolRegistry::mcp_only(
                make_mcp_deferred_set(vec![make_mcp_stub("fs__read", "Read file")]).await,
            ),
            Arc::new(Mutex::new(ActivatedToolSet::new())),
        );
        let result = tool
            .execute(serde_json::json!({"query": "zzzzz_nonexistent"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("No matching"));
    }

    #[tokio::test]
    async fn keyword_search_finds_mcp_match() {
        let activated = Arc::new(Mutex::new(ActivatedToolSet::new()));
        let tool = ToolSearchTool::new(
            DeferredToolRegistry::mcp_only(
                make_mcp_deferred_set(vec![make_mcp_stub(
                    "fs__read",
                    "Read a file from disk",
                )])
                .await,
            ),
            Arc::clone(&activated),
        );
        let result = tool
            .execute(serde_json::json!({"query": "read file"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("<function>"));
        assert!(result.output.contains("fs__read"));
        assert!(activated.lock().unwrap().is_activated("fs__read"));
    }

    #[tokio::test]
    async fn keyword_search_finds_builtin_match() {
        let activated = Arc::new(Mutex::new(ActivatedToolSet::new()));
        let tool = ToolSearchTool::new(
            DeferredToolRegistry::builtin_only(vec![make_builtin_stub(
                "git_operations",
                "Run git commands like status, diff, log",
            )]),
            Arc::clone(&activated),
        );
        let result = tool
            .execute(serde_json::json!({"query": "git status"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("<function>"));
        assert!(result.output.contains("git_operations"));
        assert!(activated.lock().unwrap().is_activated("git_operations"));
    }

    #[tokio::test]
    async fn select_activates_builtin_tool() {
        let activated = Arc::new(Mutex::new(ActivatedToolSet::new()));
        let tool = ToolSearchTool::new(
            DeferredToolRegistry::builtin_only(vec![make_builtin_stub(
                "http_request",
                "Make HTTP requests",
            )]),
            Arc::clone(&activated),
        );
        let result = tool
            .execute(serde_json::json!({"query": "select:http_request"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("http_request"));
        assert!(activated.lock().unwrap().is_activated("http_request"));
    }

    #[tokio::test]
    async fn search_finds_across_mcp_and_builtin() {
        let activated = Arc::new(Mutex::new(ActivatedToolSet::new()));
        let mcp_set =
            make_mcp_deferred_set(vec![make_mcp_stub("srv__read_file", "Read a file")]).await;
        let builtin = vec![make_builtin_stub(
            "pdf_read",
            "Read and extract text from PDF files",
        )];
        let tool = ToolSearchTool::new(
            DeferredToolRegistry::new(Some(mcp_set), builtin),
            Arc::clone(&activated),
        );
        let result = tool
            .execute(serde_json::json!({"query": "read"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("srv__read_file"));
        assert!(result.output.contains("pdf_read"));
    }

    #[tokio::test]
    async fn multiple_servers_stubs_all_searchable() {
        let activated = Arc::new(Mutex::new(ActivatedToolSet::new()));
        let stubs = vec![
            make_mcp_stub("server_a__list_files", "List files on server A"),
            make_mcp_stub("server_a__read_file", "Read file on server A"),
            make_mcp_stub("server_b__query_db", "Query database on server B"),
            make_mcp_stub("server_b__insert_row", "Insert row on server B"),
        ];
        let tool = ToolSearchTool::new(
            DeferredToolRegistry::mcp_only(make_mcp_deferred_set(stubs).await),
            Arc::clone(&activated),
        );

        let result = tool
            .execute(serde_json::json!({"query": "file"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("server_a__list_files"));
        assert!(result.output.contains("server_a__read_file"));

        let result = tool
            .execute(serde_json::json!({"query": "database query"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("server_b__query_db"));
    }

    #[tokio::test]
    async fn select_activates_and_persists_across_calls() {
        let activated = Arc::new(Mutex::new(ActivatedToolSet::new()));
        let stubs = vec![
            make_mcp_stub("srv__tool_a", "Tool A"),
            make_mcp_stub("srv__tool_b", "Tool B"),
        ];
        let tool = ToolSearchTool::new(
            DeferredToolRegistry::mcp_only(make_mcp_deferred_set(stubs).await),
            Arc::clone(&activated),
        );

        tool.execute(serde_json::json!({"query": "select:srv__tool_a"}))
            .await
            .unwrap();
        assert!(activated.lock().unwrap().is_activated("srv__tool_a"));

        tool.execute(serde_json::json!({"query": "select:srv__tool_b"}))
            .await
            .unwrap();

        let guard = activated.lock().unwrap();
        assert!(guard.is_activated("srv__tool_a"));
        assert!(guard.is_activated("srv__tool_b"));
        assert_eq!(guard.tool_specs().len(), 2);
    }

    #[tokio::test]
    async fn reactivation_is_idempotent() {
        let activated = Arc::new(Mutex::new(ActivatedToolSet::new()));
        let tool = ToolSearchTool::new(
            DeferredToolRegistry::mcp_only(
                make_mcp_deferred_set(vec![make_mcp_stub("srv__tool", "A tool")]).await,
            ),
            Arc::clone(&activated),
        );

        tool.execute(serde_json::json!({"query": "select:srv__tool"}))
            .await
            .unwrap();
        tool.execute(serde_json::json!({"query": "select:srv__tool"}))
            .await
            .unwrap();

        assert_eq!(activated.lock().unwrap().tool_specs().len(), 1);
    }
}
