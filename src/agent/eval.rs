use serde::{Deserialize, Serialize};

use schemars::JsonSchema;

// ── Complexity estimation ───────────────────────────────────────

/// Coarse complexity tier for a user message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComplexityTier {
    /// Short, simple query (greetings, yes/no, lookups).
    Simple,
    /// Typical request — not trivially simple, not deeply complex.
    Standard,
    /// Long or reasoning-heavy request (code, multi-step, analysis).
    Complex,
}

// ── Signal-based complexity scoring ─────────────────────────────

/// Multi-signal complexity features extracted in a single pass.
#[derive(Debug, Default)]
struct ComplexitySignals {
    word_count: usize,
    line_count: usize,
    sentence_count: usize,
    question_marks: usize,
    code_fences: usize,
    backtick_spans: usize,
    indented_lines: usize,
    reasoning_keywords: usize,
    technical_keywords: usize,
    action_keywords: usize,
    list_markers: usize,
    url_count: usize,
    file_path_indicators: usize,
    has_multi_paragraph: bool,
    max_indent_depth: usize,
}

/// Reasoning-heavy phrases (case-insensitive substring match).
const REASONING_KEYWORDS: &[&str] = &[
    "explain",
    "why",
    "analyze",
    "compare",
    "design",
    "trade-off",
    "tradeoff",
    "reasoning",
    "step by step",
    "think through",
    "evaluate",
    "critique",
    "pros and cons",
    "what if",
    "how does",
    "how do",
    "how would",
    "root cause",
    "diagnose",
    "investigate",
];

/// Technical / code-related keywords.
const TECHNICAL_KEYWORDS: &[&str] = &[
    "function",
    "class",
    "struct",
    "module",
    "api",
    "database",
    "query",
    "schema",
    "deploy",
    "pipeline",
    "endpoint",
    "migration",
    "docker",
    "kubernetes",
    "terraform",
    "async",
    "thread",
    "mutex",
    "trait",
    "generic",
    "lifetime",
    "borrow",
    "pointer",
    "algorithm",
    "recursion",
    "complexity",
    "sql",
    "regex",
    "json",
    "yaml",
    "toml",
    "config",
    "server",
    "client",
    "socket",
    "http",
    "grpc",
    "webhook",
    "oauth",
    "token",
    "encrypt",
    "hash",
    "certificate",
];

/// Action verbs that signal agentic / write-heavy tasks.
const ACTION_KEYWORDS: &[&str] = &[
    "implement",
    "refactor",
    "debug",
    "optimize",
    "fix",
    "rewrite",
    "migrate",
    "build",
    "create",
    "add",
    "remove",
    "delete",
    "update",
    "change",
    "modify",
    "set up",
    "configure",
    "install",
    "upgrade",
    "patch",
    "deploy",
    "ship",
    "test",
    "benchmark",
    "profile",
    "audit",
    "review",
    "architect",
];

/// Extract all complexity signals from a message in ~one pass.
fn extract_signals(message: &str) -> ComplexitySignals {
    let mut s = ComplexitySignals::default();
    let lower = message.to_lowercase();

    // ── Line-level signals ──────────────────────────────────────
    let mut consecutive_blanks = 0u32;
    for line in message.lines() {
        s.line_count += 1;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            consecutive_blanks += 1;
            if consecutive_blanks >= 1 {
                s.has_multi_paragraph = true;
            }
            continue;
        }
        consecutive_blanks = 0;

        // Indentation depth (leading spaces / 4 or tabs)
        let leading_spaces = line.len() - line.trim_start().len();
        let indent = if line.starts_with('\t') {
            line.bytes().take_while(|&b| b == b'\t').count()
        } else {
            leading_spaces / 4
        };
        if indent > 0 {
            s.indented_lines += 1;
            if indent > s.max_indent_depth {
                s.max_indent_depth = indent;
            }
        }

        // List markers: "- ", "* ", "1. ", "2. ", etc.
        if trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed
                .bytes()
                .next()
                .map_or(false, |b| b.is_ascii_digit())
                && trimmed.contains(". ")
        {
            s.list_markers += 1;
        }
    }

    // ── Character / word-level signals ──────────────────────────
    let mut in_word = false;
    for ch in message.chars() {
        if ch.is_whitespace() {
            if in_word {
                s.word_count += 1;
                in_word = false;
            }
        } else {
            in_word = true;
        }
        match ch {
            '?' => s.question_marks += 1,
            '.' | '!' => s.sentence_count += 1,
            _ => {}
        }
    }
    if in_word {
        s.word_count += 1;
    }
    // At least 1 sentence if there are words
    if s.word_count > 0 && s.sentence_count == 0 {
        s.sentence_count = 1;
    }

    // ── Code indicators ─────────────────────────────────────────
    s.code_fences = message.matches("```").count() / 2; // pairs
    // Inline backtick spans (not fences): count single ` not part of ```
    let single_bt = message.matches('`').count();
    let triple_bt = message.matches("```").count() * 3;
    s.backtick_spans = single_bt.saturating_sub(triple_bt) / 2;

    // ── Keyword scanning (on lowered text) ──────────────────────
    for kw in REASONING_KEYWORDS {
        if lower.contains(kw) {
            s.reasoning_keywords += 1;
        }
    }
    for kw in TECHNICAL_KEYWORDS {
        if lower.contains(kw) {
            s.technical_keywords += 1;
        }
    }
    for kw in ACTION_KEYWORDS {
        if lower.contains(kw) {
            s.action_keywords += 1;
        }
    }

    // ── URL & file path detection ───────────────────────────────
    for word in message.split_whitespace() {
        if word.starts_with("http://") || word.starts_with("https://") {
            s.url_count += 1;
        }
        // File path heuristic: contains / or \ with an extension
        if (word.contains('/') || word.contains('\\'))
            && (word.contains('.') || word.ends_with('/'))
            && !word.starts_with("http")
        {
            s.file_path_indicators += 1;
        }
    }

    s
}

/// Compute a 0.0–1.0 complexity score from extracted signals.
///
/// Weight budget (sums to 1.0):
///   Length 0.20 | Structure 0.10 | Code 0.20 | Reasoning 0.20
///   Technical 0.15 | Action 0.10 | Context 0.05
fn compute_complexity_score(s: &ComplexitySignals) -> f64 {
    let mut score = 0.0_f64;

    // ── Length signal (0–0.20) ───────────────────────────────────
    // Scale: 1 word ≈ 0.01, 10 words ≈ 0.10, 20+ words saturates.
    score += (s.word_count as f64 / 20.0).min(1.0) * 0.20;

    // ── Structure signal (0–0.10) ───────────────────────────────
    let structure = (s.line_count as f64 / 6.0).min(1.0) * 0.4
        + if s.has_multi_paragraph { 0.3 } else { 0.0 }
        + (s.list_markers as f64 / 3.0).min(1.0) * 0.3;
    score += structure.min(1.0) * 0.10;

    // ── Code signal (0–0.20) ────────────────────────────────────
    let code = if s.code_fences > 0 {
        1.0
    } else {
        let backtick_signal = (s.backtick_spans as f64 / 2.0).min(1.0) * 0.5;
        let indent_signal = (s.indented_lines as f64 / 3.0).min(1.0) * 0.3;
        let path_signal = (s.file_path_indicators as f64 / 2.0).min(1.0) * 0.2;
        backtick_signal + indent_signal + path_signal
    };
    score += code.min(1.0) * 0.20;

    // ── Reasoning keyword signal (0–0.20) ───────────────────────
    // 1 keyword ≈ 0.10, 2+ saturates.
    score += (s.reasoning_keywords as f64 / 2.0).min(1.0) * 0.20;

    // ── Technical keyword signal (0–0.15) ───────────────────────
    // 1 keyword ≈ 0.05, 3+ saturates.
    score += (s.technical_keywords as f64 / 3.0).min(1.0) * 0.15;

    // ── Action keyword signal (0–0.10) ──────────────────────────
    score += (s.action_keywords as f64 / 2.0).min(1.0) * 0.10;

    // ── Contextual extras (0–0.05) ──────────────────────────────
    let extras = (s.url_count as f64 / 2.0).min(1.0) * 0.4
        + (s.question_marks as f64 / 2.0).min(1.0) * 0.3
        + (s.sentence_count as f64 / 4.0).min(1.0) * 0.3;
    score += extras.min(1.0) * 0.05;

    // ── Strong-signal floor ─────────────────────────────────────
    // Certain signals are decisive regardless of message length:
    // code fences or 2+ reasoning keywords guarantee at least Standard+.
    if s.code_fences > 0 || s.reasoning_keywords >= 2 {
        score = score.max(0.40);
    }

    score.clamp(0.0, 1.0)
}

/// Estimate the complexity of a user message without an LLM call.
///
/// Uses a weighted multi-signal scorer across length, structure, code
/// indicators, reasoning/technical/action keywords, and contextual
/// features (URLs, questions, file paths). All computed in a single
/// pass — sub-millisecond even on constrained hardware.
///
/// Score thresholds: < 0.10 → Simple, 0.10–0.35 → Standard, > 0.35 → Complex.
pub fn estimate_complexity(message: &str) -> ComplexityTier {
    score_to_tier(compute_complexity_score(&extract_signals(message)))
}

/// Like [`estimate_complexity`] but also returns the raw score for observability.
pub fn estimate_complexity_scored(message: &str) -> (ComplexityTier, f64) {
    let score = compute_complexity_score(&extract_signals(message));
    (score_to_tier(score), score)
}

fn score_to_tier(score: f64) -> ComplexityTier {
    if score > 0.35 {
        ComplexityTier::Complex
    } else if score < 0.10 {
        ComplexityTier::Simple
    } else {
        ComplexityTier::Standard
    }
}

// ── Auto-classify config ────────────────────────────────────────

/// Configuration for automatic complexity-based classification.
///
/// When the rule-based classifier in `QueryClassificationConfig` produces no
/// match, the eval layer can fall back to `estimate_complexity` and map the
/// resulting tier to a routing hint.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AutoClassifyConfig {
    /// Hint to use for `Simple` complexity tier (e.g. `"fast"`).
    #[serde(default)]
    pub simple_hint: Option<String>,
    /// Hint to use for `Standard` complexity tier.
    #[serde(default)]
    pub standard_hint: Option<String>,
    /// Hint to use for `Complex` complexity tier (e.g. `"reasoning"`).
    #[serde(default)]
    pub complex_hint: Option<String>,
    /// Hint prefix for cost-optimized routing (default: `"cost-optimized"`).
    #[serde(default = "default_cost_optimized_hint")]
    pub cost_optimized_hint: String,
}

fn default_cost_optimized_hint() -> String {
    "cost-optimized".to_string()
}

impl Default for AutoClassifyConfig {
    fn default() -> Self {
        Self {
            simple_hint: None,
            standard_hint: None,
            complex_hint: None,
            cost_optimized_hint: default_cost_optimized_hint(),
        }
    }
}

impl AutoClassifyConfig {
    /// Map a complexity tier to the configured hint, if any.
    pub fn hint_for(&self, tier: ComplexityTier) -> Option<&str> {
        match tier {
            ComplexityTier::Simple => self.simple_hint.as_deref(),
            ComplexityTier::Standard => self.standard_hint.as_deref(),
            ComplexityTier::Complex => self.complex_hint.as_deref(),
        }
    }
}

// ── Post-response eval ──────────────────────────────────────────

/// Configuration for the post-response quality evaluator.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvalConfig {
    /// Enable the eval quality gate.
    #[serde(default)]
    pub enabled: bool,
    /// Minimum quality score (0.0–1.0) to accept a response.
    /// Below this threshold, a retry with a higher-tier model is suggested.
    #[serde(default = "default_min_quality_score")]
    pub min_quality_score: f64,
    /// Maximum retries with escalated models before accepting whatever we get.
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

fn default_min_quality_score() -> f64 {
    0.5
}

fn default_max_retries() -> u32 {
    1
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_quality_score: default_min_quality_score(),
            max_retries: default_max_retries(),
        }
    }
}

/// Result of evaluating a response against quality heuristics.
#[derive(Debug, Clone)]
pub struct EvalResult {
    /// Aggregate quality score from 0.0 (terrible) to 1.0 (excellent).
    pub score: f64,
    /// Individual check outcomes (for observability).
    pub checks: Vec<EvalCheck>,
    /// If score < threshold, the suggested higher-tier hint for retry.
    pub retry_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EvalCheck {
    pub name: &'static str,
    pub passed: bool,
    pub weight: f64,
}

/// Code-related keywords in user queries.
const CODE_KEYWORDS: &[&str] = &[
    "code",
    "function",
    "implement",
    "class",
    "struct",
    "module",
    "script",
    "program",
    "bug",
    "error",
    "compile",
    "syntax",
    "refactor",
];

/// Evaluate a response against heuristic quality checks. No LLM call.
///
/// Checks:
/// 1. **Non-empty**: response must not be empty.
/// 2. **Not a cop-out**: response must not be just "I don't know" or similar.
/// 3. **Sufficient length**: response length should be proportional to query complexity.
/// 4. **Code presence**: if the query mentions code keywords, the response should
///    contain a code block.
pub fn evaluate_response(
    query: &str,
    response: &str,
    complexity: ComplexityTier,
    auto_classify: Option<&AutoClassifyConfig>,
) -> EvalResult {
    let mut checks = Vec::new();

    // Check 1: Non-empty
    let non_empty = !response.trim().is_empty();
    checks.push(EvalCheck {
        name: "non_empty",
        passed: non_empty,
        weight: 0.3,
    });

    // Check 2: Not a cop-out
    let lower_resp = response.to_lowercase();
    let cop_out_phrases = [
        "i don't know",
        "i'm not sure",
        "i cannot",
        "i can't help",
        "as an ai",
    ];
    let is_cop_out = cop_out_phrases
        .iter()
        .any(|phrase| lower_resp.starts_with(phrase));
    let not_cop_out = !is_cop_out || response.len() > 200; // long responses with caveats are fine
    checks.push(EvalCheck {
        name: "not_cop_out",
        passed: not_cop_out,
        weight: 0.25,
    });

    // Check 3: Sufficient length for complexity
    let min_len = match complexity {
        ComplexityTier::Simple => 5,
        ComplexityTier::Standard => 20,
        ComplexityTier::Complex => 50,
    };
    let sufficient_length = response.len() >= min_len;
    checks.push(EvalCheck {
        name: "sufficient_length",
        passed: sufficient_length,
        weight: 0.2,
    });

    // Check 4: Code presence when expected
    let query_lower = query.to_lowercase();
    let expects_code = CODE_KEYWORDS.iter().any(|kw| query_lower.contains(kw));
    let has_code = response.contains("```") || response.contains("    "); // code block or indented
    let code_check_passed = !expects_code || has_code;
    checks.push(EvalCheck {
        name: "code_presence",
        passed: code_check_passed,
        weight: 0.25,
    });

    // Compute weighted score
    let total_weight: f64 = checks.iter().map(|c| c.weight).sum();
    let earned: f64 = checks.iter().filter(|c| c.passed).map(|c| c.weight).sum();
    let score = if total_weight > 0.0 {
        earned / total_weight
    } else {
        1.0
    };

    // Determine retry hint: if score is low, suggest escalating
    let retry_hint = if score <= default_min_quality_score() {
        // Try to escalate: Simple→Standard→Complex
        let next_tier = match complexity {
            ComplexityTier::Simple => Some(ComplexityTier::Standard),
            ComplexityTier::Standard => Some(ComplexityTier::Complex),
            ComplexityTier::Complex => None, // already at max
        };
        next_tier.and_then(|tier| {
            auto_classify
                .and_then(|ac| ac.hint_for(tier))
                .map(String::from)
        })
    } else {
        None
    };

    EvalResult {
        score,
        checks,
        retry_hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── estimate_complexity ─────────────────────────────────────

    #[test]
    fn simple_short_message() {
        assert_eq!(estimate_complexity("hi"), ComplexityTier::Simple);
        assert_eq!(estimate_complexity("hello"), ComplexityTier::Simple);
        assert_eq!(estimate_complexity("yes"), ComplexityTier::Simple);
        assert_eq!(estimate_complexity("ok"), ComplexityTier::Simple);
        assert_eq!(estimate_complexity("thanks"), ComplexityTier::Simple);
    }

    #[test]
    fn simple_short_question() {
        assert_eq!(estimate_complexity("what time is it?"), ComplexityTier::Simple);
    }

    #[test]
    fn complex_code_fence() {
        let msg = "Here is some code:\n```rust\nfn main() {}\n```";
        assert_eq!(estimate_complexity(msg), ComplexityTier::Complex);
    }

    #[test]
    fn complex_multiple_reasoning_keywords() {
        let msg = "Please explain why this design is better and analyze the trade-off";
        assert_eq!(estimate_complexity(msg), ComplexityTier::Complex);
    }

    #[test]
    fn complex_long_structured_request() {
        let msg = "I need you to implement a new authentication middleware that:\n\
                   - Validates JWT tokens from the OAuth provider\n\
                   - Checks the database for session validity\n\
                   - Handles token refresh when expired\n\
                   - Returns proper 401/403 error responses\n\n\
                   The current code in `src/auth/mod.rs` needs refactoring.";
        assert_eq!(estimate_complexity(msg), ComplexityTier::Complex);
    }

    #[test]
    fn complex_code_with_backticks_and_paths() {
        let msg = "The `parse_config()` function in src/config/schema.rs is returning \
                   None when the `api_key` field has shell expansion like `${API_KEY}`. \
                   Debug this and fix the root cause.";
        assert_eq!(estimate_complexity(msg), ComplexityTier::Complex);
    }

    #[test]
    fn standard_medium_message() {
        let msg = "Can you help me find a good restaurant nearby?";
        assert_eq!(estimate_complexity(msg), ComplexityTier::Standard);
    }

    #[test]
    fn standard_short_with_one_keyword() {
        let msg = "explain this";
        assert_eq!(estimate_complexity(msg), ComplexityTier::Standard);
    }

    #[test]
    fn standard_moderate_question() {
        let msg = "What is the difference between TCP and UDP and when should I use each?";
        assert_eq!(estimate_complexity(msg), ComplexityTier::Standard);
    }

    #[test]
    fn scored_returns_score_with_tier() {
        let (tier, score) = estimate_complexity_scored("hi");
        assert_eq!(tier, ComplexityTier::Simple);
        assert!(score < 0.10, "simple message score {score} should be < 0.10");

        let (tier, score) = estimate_complexity_scored(
            "Refactor the authentication module to use async traits and implement \
             proper error handling with the `thiserror` crate. The current code in \
             `src/auth/mod.rs` has several issues:\n\
             - No timeout on token validation\n\
             - Panics on malformed JWTs\n\
             - Missing rate limiting",
        );
        assert_eq!(tier, ComplexityTier::Complex);
        assert!(score > 0.35, "complex message score {score} should be > 0.35");
    }

    #[test]
    fn signals_detect_urls_and_paths() {
        let msg = "Check https://api.example.com/health and compare with src/gateway/mod.rs";
        let signals = extract_signals(msg);
        assert!(signals.url_count >= 1);
        assert!(signals.file_path_indicators >= 1);
    }

    #[test]
    fn signals_detect_list_markers() {
        let msg = "Do these things:\n- first\n- second\n- third";
        let signals = extract_signals(msg);
        assert!(signals.list_markers >= 3);
    }

    // ── auto_classify ───────────────────────────────────────────

    #[test]
    fn auto_classify_maps_tiers_to_hints() {
        let ac = AutoClassifyConfig {
            simple_hint: Some("fast".into()),
            standard_hint: None,
            complex_hint: Some("reasoning".into()),
            ..Default::default()
        };
        assert_eq!(ac.hint_for(ComplexityTier::Simple), Some("fast"));
        assert_eq!(ac.hint_for(ComplexityTier::Standard), None);
        assert_eq!(ac.hint_for(ComplexityTier::Complex), Some("reasoning"));
    }

    // ── evaluate_response ───────────────────────────────────────

    #[test]
    fn empty_response_scores_low() {
        let result = evaluate_response("hello", "", ComplexityTier::Simple, None);
        assert!(result.score <= 0.5, "empty response should score low");
    }

    #[test]
    fn good_response_scores_high() {
        let result = evaluate_response(
            "what is 2+2?",
            "The answer is 4.",
            ComplexityTier::Simple,
            None,
        );
        assert!(
            result.score >= 0.9,
            "good simple response should score high, got {}",
            result.score
        );
    }

    #[test]
    fn cop_out_response_penalized() {
        let result = evaluate_response(
            "explain quantum computing",
            "I don't know much about that.",
            ComplexityTier::Standard,
            None,
        );
        assert!(
            result.score < 1.0,
            "cop-out should be penalized, got {}",
            result.score
        );
    }

    #[test]
    fn code_query_without_code_response_penalized() {
        let result = evaluate_response(
            "write a function to sort an array",
            "You should use a sorting algorithm.",
            ComplexityTier::Standard,
            None,
        );
        // "code_presence" check should fail
        let code_check = result.checks.iter().find(|c| c.name == "code_presence");
        assert!(
            code_check.is_some() && !code_check.unwrap().passed,
            "code check should fail"
        );
    }

    #[test]
    fn retry_hint_escalation() {
        let ac = AutoClassifyConfig {
            simple_hint: Some("fast".into()),
            standard_hint: Some("default".into()),
            complex_hint: Some("reasoning".into()),
            ..Default::default()
        };
        // Empty response for a Simple query → should suggest Standard hint
        let result = evaluate_response("hello", "", ComplexityTier::Simple, Some(&ac));
        assert_eq!(result.retry_hint, Some("default".into()));
    }

    #[test]
    fn no_retry_when_already_complex() {
        let ac = AutoClassifyConfig {
            simple_hint: Some("fast".into()),
            standard_hint: Some("default".into()),
            complex_hint: Some("reasoning".into()),
            ..Default::default()
        };
        // Empty response for Complex → no escalation possible
        let result =
            evaluate_response("explain everything", "", ComplexityTier::Complex, Some(&ac));
        assert_eq!(result.retry_hint, None);
    }

    #[test]
    fn max_retries_defaults() {
        let config = EvalConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.max_retries, 1);
        assert!((config.min_quality_score - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn cost_optimized_hint_default() {
        let config = AutoClassifyConfig::default();
        assert_eq!(config.cost_optimized_hint, "cost-optimized");
    }
}
