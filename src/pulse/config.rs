use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    #[serde(default)]
    pub collectors: CollectorsConfig,
    #[serde(default)]
    pub intelligence: IntelligenceConfig,
    #[serde(default)]
    pub interests: Vec<Interest>,
    #[serde(default)]
    pub dashboard: DashboardConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_base_path")]
    pub base_path: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_base_path() -> String {
    "/".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: String,
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
}

fn default_db_path() -> String {
    "./data/pulse.db".to_string()
}
fn default_retention_days() -> u32 {
    30
}

// --- Collectors ---

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CollectorsConfig {
    #[serde(default)]
    pub rss: Option<RssConfig>,
    #[serde(default)]
    pub hackernews: Option<HackerNewsConfig>,
    #[serde(default)]
    pub reddit: Option<RedditConfig>,
    #[serde(default)]
    pub stocks: Option<StocksConfig>,
    #[serde(default)]
    pub weather: Option<WeatherConfig>,
    #[serde(default)]
    pub github: Option<GitHubConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RssConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_30m")]
    pub interval: String,
    #[serde(default)]
    pub feeds: Vec<FeedEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeedEntry {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HackerNewsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_15m")]
    pub interval: String,
    #[serde(default = "default_hn_min_score")]
    pub min_score: u32,
}

fn default_hn_min_score() -> u32 {
    50
}

#[derive(Debug, Clone, Deserialize)]
pub struct RedditConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_30m")]
    pub interval: String,
    #[serde(default)]
    pub subreddits: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StocksConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_5m")]
    pub interval: String,
    #[serde(default)]
    pub symbols: Vec<String>,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeatherConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_1h")]
    pub interval: String,
    pub location: Option<String>,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_6h")]
    pub interval: String,
    #[serde(default)]
    pub watch_repos: Vec<String>,
    #[serde(default)]
    pub trending_languages: Vec<String>,
}

// --- Intelligence ---

#[derive(Debug, Clone, Default, Deserialize)]
pub struct IntelligenceConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub local: Option<LocalLlmConfig>,
    #[serde(default)]
    pub remote: Option<RemoteLlmConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocalLlmConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_ollama")]
    pub provider: String,
    #[serde(default = "default_local_model")]
    pub model: String,
    #[serde(default = "default_ollama_endpoint")]
    pub endpoint: String,
    #[serde(default = "default_relevance_threshold")]
    pub relevance_threshold: u32,
}

fn default_ollama() -> String {
    "ollama".to_string()
}
fn default_local_model() -> String {
    "llama3:8b".to_string()
}
fn default_ollama_endpoint() -> String {
    "http://localhost:11434".to_string()
}
fn default_relevance_threshold() -> u32 {
    4
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteLlmConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_claude")]
    pub provider: String,
    pub model: Option<String>,
    pub api_key: Option<String>,
    #[serde(default = "default_max_daily_calls")]
    pub max_daily_calls: u32,
    #[serde(default = "default_batch_size")]
    pub batch_size: u32,
}

fn default_claude() -> String {
    "claude".to_string()
}
fn default_max_daily_calls() -> u32 {
    100
}
fn default_batch_size() -> u32 {
    10
}

// --- Interests ---

#[derive(Debug, Clone, Deserialize)]
pub struct Interest {
    pub name: String,
    pub description: String,
    #[serde(default = "default_priority")]
    pub priority: String,
}

fn default_priority() -> String {
    "medium".to_string()
}

// --- Dashboard ---

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DashboardConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_refresh")]
    pub refresh_interval: String,
    #[serde(default)]
    pub widgets: Vec<WidgetConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WidgetConfig {
    #[serde(rename = "type")]
    pub widget_type: String,
    #[serde(default)]
    pub position: WidgetPosition,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct WidgetPosition {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

// Default value helpers
fn default_true() -> bool {
    true
}
fn default_theme() -> String {
    "dark".to_string()
}
fn default_refresh() -> String {
    "60s".to_string()
}
fn default_30m() -> String {
    "30m".to_string()
}
fn default_15m() -> String {
    "15m".to_string()
}
fn default_5m() -> String {
    "5m".to_string()
}
fn default_1h() -> String {
    "1h".to_string()
}
fn default_6h() -> String {
    "6h".to_string()
}
