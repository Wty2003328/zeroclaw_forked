//! Deferred built-in tool loading and unified deferred tool registry.
//!
//! Extends the MCP deferred loading pattern to built-in tools. Non-core
//! built-in tools are represented as lightweight stubs (name + description)
//! in the system prompt. The LLM must call `tool_search` to fetch full
//! schemas, which activates them for the rest of the conversation.
//!
//! The [`DeferredToolRegistry`] provides a unified search/activate interface
//! across both MCP and built-in deferred tools for [`ToolSearchTool`].

use std::sync::Arc;

use crate::tools::mcp_deferred::DeferredMcpToolSet;
use crate::tools::traits::{Tool, ToolSpec};

// ── DeferredBuiltinStub ─────────────────────────────────────────────────

/// A built-in tool whose schema is withheld from the LLM until activated.
/// Unlike MCP stubs, the tool is already fully constructed — we just defer
/// sending its schema to save context window.
#[derive(Clone)]
pub struct DeferredBuiltinStub {
    pub name: String,
    pub description: String,
    pub tool: Arc<dyn Tool>,
}

impl DeferredBuiltinStub {
    pub fn from_tool(tool: Arc<dyn Tool>) -> Self {
        Self {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            tool,
        }
    }
}

// ── Unified search result ───────────────────────────────────────────────

/// Metadata returned by registry searches, regardless of stub origin.
pub struct StubInfo<'a> {
    pub name: &'a str,
    pub description: &'a str,
}

// ── DeferredToolRegistry ────────────────────────────────────────────────

/// Unified registry for both MCP and built-in deferred tool stubs.
/// Provides search, lookup, and activation methods consumed by
/// [`ToolSearchTool`](super::tool_search::ToolSearchTool).
pub struct DeferredToolRegistry {
    mcp: Option<DeferredMcpToolSet>,
    builtin: Vec<DeferredBuiltinStub>,
}

impl DeferredToolRegistry {
    pub fn new(mcp: Option<DeferredMcpToolSet>, builtin: Vec<DeferredBuiltinStub>) -> Self {
        Self { mcp, builtin }
    }

    /// Create a registry with only MCP stubs (no built-in deferred tools).
    pub fn mcp_only(mcp: DeferredMcpToolSet) -> Self {
        Self {
            mcp: Some(mcp),
            builtin: Vec::new(),
        }
    }

    /// Create a registry with only built-in deferred stubs (no MCP).
    pub fn builtin_only(builtin: Vec<DeferredBuiltinStub>) -> Self {
        Self {
            mcp: None,
            builtin,
        }
    }

    /// Total number of deferred stubs (MCP + built-in).
    pub fn len(&self) -> usize {
        self.mcp.as_ref().map_or(0, |m| m.len()) + self.builtin.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// All stub names with descriptions, for rendering in the system prompt.
    pub fn stub_names_with_descriptions(&self) -> Vec<(&str, &str)> {
        let mut out = Vec::with_capacity(self.len());
        if let Some(ref mcp) = self.mcp {
            for stub in &mcp.stubs {
                out.push((stub.prefixed_name.as_str(), stub.description.as_str()));
            }
        }
        for stub in &self.builtin {
            out.push((stub.name.as_str(), stub.description.as_str()));
        }
        out
    }

    /// Look up a deferred tool by exact name (MCP or built-in).
    pub fn get_by_name(&self, name: &str) -> Option<StubInfo<'_>> {
        if let Some(ref mcp) = self.mcp {
            if let Some(stub) = mcp.get_by_name(name) {
                return Some(StubInfo {
                    name: &stub.prefixed_name,
                    description: &stub.description,
                });
            }
        }
        for stub in &self.builtin {
            if stub.name == name {
                return Some(StubInfo {
                    name: &stub.name,
                    description: &stub.description,
                });
            }
        }
        None
    }

    /// Keyword search across all deferred stubs. Returns results ranked by
    /// number of matching terms (descending), up to `max_results`.
    pub fn search(&self, query: &str, max_results: usize) -> Vec<StubInfo<'_>> {
        let terms: Vec<String> = query
            .split_whitespace()
            .map(|t| t.to_ascii_lowercase())
            .collect();
        if terms.is_empty() {
            return self
                .stub_names_with_descriptions()
                .into_iter()
                .take(max_results)
                .map(|(name, desc)| StubInfo {
                    name,
                    description: desc,
                })
                .collect();
        }

        let mut scored: Vec<(StubInfo<'_>, usize)> = Vec::new();

        // Score MCP stubs
        if let Some(ref mcp) = self.mcp {
            for stub in &mcp.stubs {
                let haystack = format!(
                    "{} {}",
                    stub.prefixed_name.to_ascii_lowercase(),
                    stub.description.to_ascii_lowercase()
                );
                let hits = terms.iter().filter(|t| haystack.contains(t.as_str())).count();
                if hits > 0 {
                    scored.push((
                        StubInfo {
                            name: &stub.prefixed_name,
                            description: &stub.description,
                        },
                        hits,
                    ));
                }
            }
        }

        // Score built-in stubs
        for stub in &self.builtin {
            let haystack = format!(
                "{} {}",
                stub.name.to_ascii_lowercase(),
                stub.description.to_ascii_lowercase()
            );
            let hits = terms.iter().filter(|t| haystack.contains(t.as_str())).count();
            if hits > 0 {
                scored.push((
                    StubInfo {
                        name: &stub.name,
                        description: &stub.description,
                    },
                    hits,
                ));
            }
        }

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored
            .into_iter()
            .take(max_results)
            .map(|(info, _)| info)
            .collect()
    }

    /// Activate a deferred tool by name, returning an `Arc<dyn Tool>`.
    /// For built-in tools, this clones the existing Arc. For MCP tools,
    /// this constructs a new wrapper via the registry.
    pub fn activate(&self, name: &str) -> Option<Arc<dyn Tool>> {
        // Check built-in first (cheaper — no wrapper construction)
        for stub in &self.builtin {
            if stub.name == name {
                return Some(Arc::clone(&stub.tool));
            }
        }
        // Then MCP
        if let Some(ref mcp) = self.mcp {
            if let Some(tool) = mcp.activate(name) {
                return Some(Arc::from(tool));
            }
        }
        None
    }

    /// Return the full [`ToolSpec`] for a deferred tool.
    pub fn tool_spec(&self, name: &str) -> Option<ToolSpec> {
        // Check built-in first
        for stub in &self.builtin {
            if stub.name == name {
                return Some(stub.tool.spec());
            }
        }
        // Then MCP
        if let Some(ref mcp) = self.mcp {
            return mcp.tool_spec(name);
        }
        None
    }
}

// ── System prompt helper ────────────────────────────────────────────────

/// Build the `<available-deferred-tools>` section for the system prompt.
/// Lists tool names and descriptions for both MCP and built-in deferred
/// tools. Instructs the LLM to call `tool_search` to activate them.
pub fn build_deferred_tools_section(registry: &DeferredToolRegistry) -> String {
    if registry.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str("## Deferred Tools\n\n");
    out.push_str(
        "The tools listed below are available but NOT yet loaded. \
         To use any of them you MUST first call the `tool_search` tool \
         to fetch their full schemas. Use `\"select:name1,name2\"` for \
         exact tools or keywords to search. Once activated, the tools \
         become callable for the rest of the conversation.\n\n",
    );
    out.push_str("<available-deferred-tools>\n");
    for (name, desc) in registry.stub_names_with_descriptions() {
        out.push_str(name);
        out.push_str(" - ");
        out.push_str(desc);
        out.push('\n');
    }
    out.push_str("</available-deferred-tools>\n");
    out
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::traits::ToolResult;
    use async_trait::async_trait;

    struct FakeTool {
        tool_name: &'static str,
        tool_desc: &'static str,
    }

    #[async_trait]
    impl Tool for FakeTool {
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
        let tool = Arc::new(FakeTool {
            tool_name: name,
            tool_desc: desc,
        });
        DeferredBuiltinStub::from_tool(tool)
    }

    #[test]
    fn builtin_stub_from_tool() {
        let stub = make_builtin_stub("git_operations", "Run git commands");
        assert_eq!(stub.name, "git_operations");
        assert_eq!(stub.description, "Run git commands");
    }

    #[test]
    fn registry_len_combines_both() {
        let registry = DeferredToolRegistry::new(
            None,
            vec![
                make_builtin_stub("a", "Tool A"),
                make_builtin_stub("b", "Tool B"),
            ],
        );
        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
    }

    #[test]
    fn registry_empty_when_no_stubs() {
        let registry = DeferredToolRegistry::new(None, vec![]);
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn get_by_name_finds_builtin() {
        let registry = DeferredToolRegistry::builtin_only(vec![
            make_builtin_stub("cron_add", "Add a cron job"),
            make_builtin_stub("http_request", "Make HTTP requests"),
        ]);
        let info = registry.get_by_name("http_request").unwrap();
        assert_eq!(info.name, "http_request");
        assert_eq!(info.description, "Make HTTP requests");
        assert!(registry.get_by_name("nonexistent").is_none());
    }

    #[test]
    fn search_ranks_by_hits() {
        let registry = DeferredToolRegistry::builtin_only(vec![
            make_builtin_stub("cron_add", "Add a scheduled cron job"),
            make_builtin_stub("cron_list", "List all cron jobs"),
            make_builtin_stub("http_request", "Make HTTP requests"),
        ]);
        // "cron job" matches cron_add (2 hits: cron + job) and cron_list (2 hits)
        let results = registry.search("cron job", 10);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.name.starts_with("cron_")));
    }

    #[test]
    fn search_no_match_returns_empty() {
        let registry =
            DeferredToolRegistry::builtin_only(vec![make_builtin_stub("cron_add", "Add cron")]);
        let results = registry.search("zzzzz", 10);
        assert!(results.is_empty());
    }

    #[test]
    fn activate_returns_builtin_tool() {
        let registry = DeferredToolRegistry::builtin_only(vec![make_builtin_stub(
            "git_operations",
            "Git ops",
        )]);
        let tool = registry.activate("git_operations");
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name(), "git_operations");
    }

    #[test]
    fn activate_nonexistent_returns_none() {
        let registry = DeferredToolRegistry::builtin_only(vec![]);
        assert!(registry.activate("nonexistent").is_none());
    }

    #[test]
    fn tool_spec_returns_full_schema() {
        let registry =
            DeferredToolRegistry::builtin_only(vec![make_builtin_stub("cron_add", "Add cron job")]);
        let spec = registry.tool_spec("cron_add").unwrap();
        assert_eq!(spec.name, "cron_add");
        assert_eq!(spec.description, "Add cron job");
    }

    #[test]
    fn build_section_empty_when_no_stubs() {
        let registry = DeferredToolRegistry::new(None, vec![]);
        assert!(build_deferred_tools_section(&registry).is_empty());
    }

    #[test]
    fn build_section_lists_builtin_tools() {
        let registry = DeferredToolRegistry::builtin_only(vec![
            make_builtin_stub("cron_add", "Add a cron job"),
            make_builtin_stub("browser", "Browse the web"),
        ]);
        let section = build_deferred_tools_section(&registry);
        assert!(section.contains("<available-deferred-tools>"));
        assert!(section.contains("cron_add - Add a cron job"));
        assert!(section.contains("browser - Browse the web"));
        assert!(section.contains("tool_search"));
        assert!(section.contains("## Deferred Tools"));
    }

    #[test]
    fn stub_names_with_descriptions_includes_all() {
        let registry = DeferredToolRegistry::builtin_only(vec![
            make_builtin_stub("a", "Tool A"),
            make_builtin_stub("b", "Tool B"),
        ]);
        let names = registry.stub_names_with_descriptions();
        assert_eq!(names.len(), 2);
        assert_eq!(names[0], ("a", "Tool A"));
        assert_eq!(names[1], ("b", "Tool B"));
    }
}
