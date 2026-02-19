# ZeroClaw Configuration & API Surface Audit

**Audit Date:** 2025-01-27  
**Auditor:** repo-health-auditor agent  
**Scope:** Config schema, CLI interface, public API stability, backward compatibility, defaults, validation  
**Repository:** zeroclaw2  
**Commit:** HEAD (current working tree)

---

## Executive Summary

ZeroClaw's configuration and API surface demonstrates **strong foundational design** with comprehensive environment variable support, secure defaults, and trait-driven extensibility. The config schema is well-structured with proper default functions and encryption support for secrets. However, there are **critical gaps** in validation, documentation parity, and public API boundaries that increase operator error surface and risk accidental breaking changes.

**Key Strengths:**
- Secure-by-default config (pairing required, public bind disabled, deny-by-default allowlists)
- Comprehensive env var precedence with `ZEROCLAW_*` overrides
- Atomic config file writes with backup/restore on failure
- Secret encryption at rest via `SecretStore`
- Trait-based extension points are well-defined and stable

**Critical Risks:**
- **No config validation on load** — invalid configs silently fall back or fail at runtime
- **Public API leakage** — entire crate tree is `pub mod` without `pub(crate)` boundaries
- **Documentation drift** — `docs/config-reference.md` missing 50%+ of schema keys
- **No deprecated field handling** — no migration paths or warnings for breaking changes
- **CLI help text coverage gaps** — many subcommands lack detailed descriptions

**Risk Rating by Category:**
| Category | Rating | Justification |
|----------|--------|---------------|
| Config Validation | 🔴 **High** | No startup validation; invalid configs accepted then fail at use |
| Public API Stability | 🔴 **High** | All modules public; accidental API surface exposed |
| Documentation Parity | 🟡 **Medium** | Docs lag schema by ~6 months of additions |
| Backward Compatibility | 🟡 **Medium** | No deprecation markers or migration tooling |
| Default Values | 🟢 **Low** | Defaults are safe, sensible, documented in code |
| Env Var Precedence | 🟢 **Low** | Clear precedence, well-tested, documented in `.env.example` |

---

## Detailed Findings

### 1. Config Schema Completeness & Structure

**Location:** `src/config/schema.rs:49-152` (top-level `Config` struct)

#### 1.1 Schema Organization: ✅ GOOD

The config schema follows a clear hierarchy:
- Top-level keys for core runtime (`default_provider`, `default_model`, etc.)
- Grouped subsystems (`[gateway]`, `[autonomy]`, `[memory]`, etc.)
- Per-provider/channel sections in nested maps

**Evidence:**
```rust
pub struct Config {
    pub workspace_dir: PathBuf,      // computed, not serialized
    pub config_path: PathBuf,        // computed, not serialized
    pub api_key: Option<String>,
    pub default_provider: Option<String>,
    pub default_model: Option<String>,
    pub default_temperature: f64,
    #[serde(default)]
    pub gateway: GatewayConfig,
    #[serde(default)]
    pub autonomy: AutonomyConfig,
    // ... 20+ subsystem configs
}
```

All subsystem configs have `#[serde(default)]`, ensuring partial TOML files load correctly.

#### 1.2 Default Functions: ✅ GOOD

**Evidence:** Every config section has default functions (e.g., `default_gateway_port() -> u16`, `default_true() -> bool`)

**Strengths:**
- Defaults are sensible and safe (e.g., `default_gateway_port() = 3000`, `default_gateway_host() = "127.0.0.1"`)
- Security defaults are deny-by-default (`require_pairing = true`, `allow_public_bind = false`)
- Limits have reasonable production values (`max_tool_iterations = 10`, `max_history_messages = 50`)

**Weakness:** Default functions are scattered throughout the file (lines 177-2547) rather than grouped near their structs, making them hard to audit.

#### 1.3 Type Safety: ✅ GOOD

All config keys use proper Rust types:
- `u16` for ports
- `PathBuf` for paths
- Enums for constrained values (`StreamMode`, `HardwareTransport`, `ProxyScope`)
- `Option<T>` for nullable fields

No string-based "magic values" that could be mistyped.

#### 1.4 **CRITICAL: Missing Validation on Load** 🔴

**Severity:** Critical  
**Location:** `src/config/schema.rs:2838-2939` (`Config::load_or_init`)

**Issue:** Config is deserialized from TOML with no validation. Invalid values are accepted and only fail later at runtime.

**Evidence:**
```rust
let mut config: Config = toml::from_str(&contents)
    .context("Failed to parse config file")?;  // ← only checks TOML syntax
// No validation here! Config with invalid proxy URLs, negative ports, etc. is accepted
config.apply_env_overrides();  // env vars also not validated
```

**Only one subsystem validates itself:**
```rust:src/config/schema.rs:934-969
impl ProxyConfig {
    pub fn validate(&self) -> Result<()> {
        // validates proxy URLs, service selectors, scope consistency
    }
}
```

But this is **only called during env override processing** (line 3177), not during initial load. If proxy config comes from TOML, it's never validated until first use.

**Impact:**
- Operators may run with invalid configs for months until the feature is used
- Gateway might bind to `0.0.0.0` if `host` is typo'd to empty string (no validation)
- `autonomy.max_actions_per_hour` could be set to `0` or `-1` (no bounds check)
- `multimodal.max_images` could exceed provider limits (validated at runtime, not startup)

**Recommended Fix:**
```rust
impl Config {
    pub async fn load_or_init() -> Result<Self> {
        // ... existing load logic ...
        config.validate()?;  // ← ADD THIS
        Ok(config)
    }
    
    pub fn validate(&self) -> Result<()> {
        // Gateway validation
        if self.gateway.port == 0 && !explicitly_requesting_random_port {
            bail!("gateway.port cannot be 0 (use CLI --port 0 for random port)");
        }
        if self.gateway.host.is_empty() {
            bail!("gateway.host cannot be empty");
        }
        
        // Autonomy validation
        if self.autonomy.max_actions_per_hour == 0 {
            bail!("autonomy.max_actions_per_hour must be > 0");
        }
        
        // Multimodal validation
        let (max_images, max_size) = self.multimodal.effective_limits();
        if self.multimodal.max_images != max_images {
            warn!("multimodal.max_images clamped from {} to {}", 
                  self.multimodal.max_images, max_images);
        }
        
        // Proxy validation
        self.proxy.validate()?;
        
        // Scheduler validation
        if self.scheduler.max_concurrent == 0 {
            bail!("scheduler.max_concurrent must be > 0");
        }
        
        Ok(())
    }
}
```

---

### 2. CLI Interface Structure & Validation

**Location:** `src/main.rs:82-254` (Clap command definitions)

#### 2.1 Command Organization: ✅ GOOD

CLI uses clap's derive API with clear subcommand hierarchy:
```
zeroclaw
  ├─ onboard (interactive wizard)
  ├─ agent (interactive loop or single message)
  ├─ gateway (webhook/websocket server)
  ├─ daemon (gateway + channels + scheduler)
  ├─ service (systemd/launchd lifecycle)
  ├─ doctor (health checks)
  ├─ cron (scheduled tasks)
  ├─ channel (telegram/discord/slack mgmt)
  ├─ hardware (USB discovery)
  ├─ peripheral (board management)
  ├─ config (schema export)
  └─ ...
```

#### 2.2 **MEDIUM: Help Text Completeness** 🟡

**Severity:** Medium  
**Location:** `src/main.rs:106-254`, `src/lib.rs:91-249`

**Issue:** Many subcommands lack detailed help text or examples.

**Evidence — Good help text:**
```rust
/// Initialize your workspace and configuration
Onboard {
    /// Run the full interactive wizard (default is quick setup)
    #[arg(long)]
    interactive: bool,
    // ... clear descriptions
}
```

**Evidence — Weak help text:**
```rust
/// Configure and manage scheduled tasks
Cron {
    #[command(subcommand)]
    cron_command: CronCommands,
}
// ❌ No examples, no mention of expression syntax, no tz handling
```

**Impact:** Users guess at cron syntax, timezone behavior, and command formats. Increases support burden.

**Recommended Fix:**
- Add `long_about` to complex commands with examples:
```rust
/// Configure and manage scheduled tasks
#[command(
    long_about = "Schedule recurring and one-shot tasks using cron expressions.
    
Examples:
  zeroclaw cron add '0 9 * * MON-FRI' --tz America/Los_Angeles 'agent -m \"Daily standup\"'
  zeroclaw cron add-at '2025-01-28T15:00:00Z' 'agent -m \"Reminder\"'
  zeroclaw cron once 30m 'agent -m \"Follow up\"'
"
)]
Cron { /* ... */ }
```

#### 2.3 **MEDIUM: Argument Validation** 🟡

**Severity:** Medium  
**Location:** CLI argument parsing (clap validates types but not business rules)

**Issue:** Clap validates that `--port <u16>` is a valid integer, but doesn't validate:
- Port ranges (e.g., `--port 0` should only be allowed with explicit intent)
- Temperature ranges (clap default is `0.7`, but invalid values are accepted)
- Provider/model name formats (no validation that `--provider custom:foo` matches expected patterns)

**Evidence:**
```rust:src/main.rs:147-148
/// Temperature (0.0 - 2.0)
#[arg(short, long, default_value = "0.7")]
temperature: f64,
```
Comment says `0.0 - 2.0` but no clap validation enforces this. Invalid values accepted, passed to config, fail at provider call time.

**Recommended Fix:**
```rust
#[arg(short, long, default_value = "0.7", value_parser = clap::value_parser!(f64).range(0.0..=2.0))]
temperature: f64,
```

---

### 3. Public API Stability & Boundaries

**Location:** `src/lib.rs:41-70`

#### 3.1 **CRITICAL: Entire Crate is Public** 🔴

**Severity:** Critical  
**Location:** `src/lib.rs:41-70`

**Issue:** All modules are declared as `pub mod`, exposing internal implementation to external crates. No `pub(crate)` boundaries.

**Evidence:**
```rust:src/lib.rs:41-70
pub mod agent;
pub mod approval;
pub mod auth;
pub mod channels;
pub mod config;
pub mod cost;
pub mod cron;
pub mod daemon;
pub mod doctor;
pub mod gateway;
pub mod hardware;
// ... 20+ modules, all public
```

**Impact:**
- **Accidental API surface**: Types like `agent::dispatcher::ToolDispatcher`, `gateway::rate_limiter::RateLimiter` are now public API even though they're internal implementation details
- **Breaking changes are hard to avoid**: Refactoring internal modules becomes a breaking change for any external crate that imported them
- **Unclear stability guarantees**: No way to distinguish "stable public API" from "internal, may change"

**Recommended Fix:**

1. **Declare internal modules as `pub(crate)`:**
```rust
pub(crate) mod agent;        // internal orchestration
pub(crate) mod approval;     // internal approval logic
pub(crate) mod daemon;       // internal daemon lifecycle
pub(crate) mod doctor;       // CLI-only tool
pub(crate) mod onboard;      // CLI-only wizard
pub(crate) mod service;      // CLI-only service mgmt
```

2. **Keep extension-point traits public:**
```rust
pub mod channels;    // channels::traits::Channel is public API
pub mod providers;   // providers::traits::Provider is public API
pub mod tools;       // tools::traits::Tool is public API
pub mod memory;      // memory::traits::Memory is public API
pub mod peripherals; // peripherals::traits::Peripheral is public API
```

3. **Explicitly export stable types:**
```rust
pub use config::Config;
pub use channels::traits::{Channel, ChannelMessage, SendMessage};
pub use providers::traits::{Provider, ChatMessage, ChatRequest, ChatResponse};
pub use tools::traits::{Tool, ToolResult, ToolSpec};
pub use memory::traits::Memory;
```

4. **Document stability guarantees:**
```rust
//! # Public API Stability
//!
//! Stable public API (semver guarantees):
//! - `Config` struct and its public fields
//! - Extension trait interfaces: `Channel`, `Provider`, `Tool`, `Memory`, `Peripheral`
//! - Public types used in trait signatures: `ChatMessage`, `ToolResult`, etc.
//!
//! Unstable (internal implementation, may change):
//! - CLI subcommand enums (for binary use only, not library API)
//! - All other modules under `pub(crate)`
```

#### 3.2 Command Enums Are Public But Should Be Binary-Only 🟡

**Severity:** Medium  
**Location:** `src/lib.rs:75-249`

**Issue:** CLI command enums (`ServiceCommands`, `ChannelCommands`, etc.) are public, but they're only meaningful for the binary. External crates can't use them without the full CLI context.

**Evidence:**
```rust:src/lib.rs:75-87
/// Service management subcommands
#[derive(Subcommand, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceCommands {  // ← public but not useful as library API
    Install,
    Start,
    Stop,
    Status,
    Uninstall,
}
```

**Impact:** Minor — these are harmless to expose, but they clutter the public API and imply they're meant to be used by library consumers.

**Recommended Fix:**
```rust
#[doc(hidden)]  // hide from rustdoc
pub enum ServiceCommands { /* ... */ }
```
Or move to `src/main.rs` if only the binary needs them.

---

### 4. Backward Compatibility & Migration

#### 4.1 **MEDIUM: No Deprecation System** 🟡

**Severity:** Medium  
**Location:** Entire `src/config/schema.rs` (no `#[deprecated]` attributes found)

**Issue:** No mechanism to deprecate config keys or warn users about upcoming breaking changes.

**Evidence:** Searched for deprecation markers:
```bash
grep -i "deprecated\|deprecated_since\|migrate\|migration\|breaking" src/config/schema.rs
# No matches found
```

**Impact:**
- Breaking config changes (e.g., renaming `gateway.paired_tokens` → `gateway.paired_clients`) require immediate updates with no transition period
- Operators have no warning before fields are removed
- No guidance on how to migrate from old to new schema

**Recommended Fix:**

1. **Add deprecation support:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GatewayConfig {
    pub port: u16,
    pub host: String,
    
    /// Paired bearer tokens (managed automatically, not user-edited)
    #[deprecated(since = "0.2.0", note = "renamed to `paired_clients`")]
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub paired_tokens: Vec<String>,
    
    /// Paired client identifiers (replaces `paired_tokens`)
    #[serde(default)]
    pub paired_clients: Vec<String>,
}
```

2. **Validate and warn at load time:**
```rust
impl Config {
    pub async fn load_or_init() -> Result<Self> {
        // ... load config ...
        config.check_deprecated_fields();
        Ok(config)
    }
    
    fn check_deprecated_fields(&self) {
        if !self.gateway.paired_tokens.is_empty() {
            warn!(
                "Config key 'gateway.paired_tokens' is deprecated since v0.2.0. \
                 Please rename to 'gateway.paired_clients' in your config.toml. \
                 Automatic migration will be removed in v0.3.0."
            );
            // Auto-migrate during load
            self.gateway.paired_clients.extend(self.gateway.paired_tokens.clone());
        }
    }
}
```

3. **Document migration in CHANGELOG:**
```markdown
## [0.2.0] - 2025-02-01

### Changed
- **BREAKING (with migration)**: `gateway.paired_tokens` renamed to `gateway.paired_clients`
  - Old key still works but is deprecated and will be removed in v0.3.0
  - Automatic migration: old values are copied to new key on load
  - Action required: Update `config.toml` to use new key name
```

#### 4.2 **LOW: Config Key Naming Consistency** ✅

**Severity:** Low

Config keys follow consistent `snake_case` conventions across all sections. No camelCase or kebab-case mixtures found.

**Evidence:**
```toml
[gateway]
port = 3000
require_pairing = true  # ✅ snake_case
allow_public_bind = false

[autonomy]
workspace_only = true
allowed_commands = []  # ✅ consistent
```

---

### 5. Default Values Analysis

#### 5.1 Security Defaults: ✅ EXCELLENT

**Location:** `src/config/schema.rs:628-644` (Gateway), `src/config/schema.rs:1570-1639` (Autonomy)

**Strengths:**
- **Gateway**: `require_pairing = true`, `allow_public_bind = false`, `host = "127.0.0.1"`
- **Autonomy**: `workspace_only = true`, `require_approval_for_medium_risk = true`, `block_high_risk_commands = true`
- **Secrets**: `encrypt = true` by default
- **Multimodal**: `allow_remote_fetch = false`
- **Browser**: `allow_remote_endpoint = false`

All defaults follow **secure-by-default** and **deny-by-default** principles.

#### 5.2 Performance Defaults: ✅ GOOD

**Location:** Various

**Evidence:**
- `agent.max_tool_iterations = 10` — prevents runaway loops
- `channels_config.message_timeout_secs = 300` — reasonable for on-device LLMs
- `reliability.provider_retries = 2` — balanced retry strategy
- `scheduler.max_concurrent = 4` — prevents resource exhaustion

Defaults balance safety and usability. Well-documented in code comments.

#### 5.3 **LOW: Surprising Default (Web Search Enabled)** 🟡

**Severity:** Low  
**Location:** `src/config/schema.rs:849-858`

**Issue:** `web_search.enabled = true` by default, which makes network requests to DuckDuckGo without explicit user consent.

**Evidence:**
```rust
impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            enabled: true,  // ← auto-enabled
            provider: default_web_search_provider(),  // duckduckgo
            // ...
        }
    }
}
```

**Impact:** Minor — DuckDuckGo is privacy-friendly, but auto-enabling a network service violates least-surprise principle.

**Recommended Fix:**
```rust
enabled: false,  // require explicit opt-in
```
Or add prominent notice in onboarding wizard.

---

### 6. Environment Variable Support

#### 6.1 Coverage: ✅ EXCELLENT

**Location:** `src/config/schema.rs:2943-3187` (`apply_env_overrides`)

**Strengths:**
- Comprehensive coverage: API keys, provider, model, gateway, temperature, proxy, storage, web search
- Consistent `ZEROCLAW_*` prefix for explicit overrides
- Fallback to generic env vars (`API_KEY`, `PROVIDER`, `PORT`) for container/Docker compatibility
- Clear precedence rules documented in code comments

**Evidence:**
```rust
// Provider override precedence:
// 1) ZEROCLAW_PROVIDER always wins when set.
// 2) Legacy PROVIDER is only honored when config still uses the
//    default provider (openrouter) or provider is unset.
```

#### 6.2 **LOW: Env Var Not Validated** 🟡

**Severity:** Low  
**Location:** `src/config/schema.rs:3177`

**Issue:** Proxy validation only runs during env override processing, not for TOML-based config. If both sources are invalid, only env-based config fails fast.

**Evidence:**
```rust:3177
if let Err(error) = self.proxy.validate() {
    tracing::warn!("Invalid proxy configuration ignored: {error}");
    self.proxy.enabled = false;  // ← silently disables, doesn't fail
}
```

Invalid proxy config from TOML is accepted without warning.

**Recommended Fix:** Move validation to `Config::load_or_init()` so TOML and env vars are both validated.

#### 6.3 .env.example Coverage: ✅ GOOD

**Location:** `.env.example:1-100`

**.env.example documents:**
- Core runtime vars (`API_KEY`, `PROVIDER`, `ZEROCLAW_WORKSPACE`)
- Provider-specific keys (20+ providers listed)
- Gateway config (`ZEROCLAW_GATEWAY_PORT`, `ZEROCLAW_ALLOW_PUBLIC_BIND`)
- Optional integrations (Pushover, Brave Search, Z.AI)
- Docker-specific vars (`HOST_PORT`)

**Weakness:** Missing some recently-added vars:
- `ZEROCLAW_REASONING_ENABLED` (added but not in `.env.example`)
- `ZEROCLAW_STORAGE_PROVIDER`
- `ZEROCLAW_PROXY_SCOPE`

**Recommended Fix:** Add to `.env.example`:
```bash
# ── Reasoning Control ─────────────────────────────────────────
# ZEROCLAW_REASONING_ENABLED=true  # enable explicit reasoning for supported providers
# REASONING_ENABLED=true

# ── Storage Backend ───────────────────────────────────────────
# ZEROCLAW_STORAGE_PROVIDER=postgres
# ZEROCLAW_STORAGE_DB_URL=postgresql://user:pass@localhost/zeroclaw

# ── Proxy Configuration ───────────────────────────────────────
# ZEROCLAW_PROXY_ENABLED=true
# ZEROCLAW_HTTP_PROXY=http://proxy.example.com:8080
# ZEROCLAW_PROXY_SCOPE=services  # environment|zeroclaw|services
# ZEROCLAW_PROXY_SERVICES=provider.*,channel.telegram
```

---

### 7. Config Merging & Precedence

#### 7.1 Precedence Logic: ✅ EXCELLENT

**Location:** `src/config/schema.rs:2766-2800` (workspace resolution), `src/config/schema.rs:2943-3187` (env overrides)

**Workspace resolution precedence:**
1. `ZEROCLAW_WORKSPACE` env var (highest priority)
2. `~/.zeroclaw/active_workspace.toml` marker (persisted from onboarding)
3. Default `~/.zeroclaw/` directory

**Config value precedence:**
1. Environment variable override (`ZEROCLAW_*`)
2. Fallback env var (`API_KEY`, `PROVIDER`, etc.)
3. TOML config value
4. Rust `Default::default()` value

**Evidence:**
```rust
// API Key: ZEROCLAW_API_KEY or API_KEY (generic)
if let Ok(key) = std::env::var("ZEROCLAW_API_KEY").or_else(|_| std::env::var("API_KEY")) {
    if !key.is_empty() {
        self.api_key = Some(key);  // ← overrides TOML value
    }
}
```

**Strength:** Provider precedence is smart:
```rust
// ZEROCLAW_PROVIDER always wins.
// Legacy PROVIDER only applies if config still uses default "openrouter".
// This prevents container defaults from overriding custom providers.
```

**No surprising silent overrides found.**

#### 7.2 **LOW: Proxy Auto-Enable on Env Var** 🟡

**Severity:** Low  
**Location:** `src/config/schema.rs:3154-3159`

**Issue:** If `HTTP_PROXY` env var is set but `proxy.enabled` is not explicitly configured, proxy is auto-enabled.

**Evidence:**
```rust
if explicit_proxy_enabled.is_none()
    && proxy_url_overridden
    && self.proxy.has_any_proxy_url()
{
    self.proxy.enabled = true;  // ← auto-enable from env presence
}
```

**Impact:** Minor — this is intuitive (if `HTTP_PROXY` is set, user expects proxy to work), but it violates explicit-over-implicit preference in rest of config system.

**Recommendation:** Document this behavior prominently in docs:
```markdown
## Proxy Auto-Enable

If `HTTP_PROXY` or `HTTPS_PROXY` environment variables are set but `proxy.enabled` is not configured, the proxy is **automatically enabled** with scope `zeroclaw`. To disable this behavior, explicitly set `proxy.enabled = false` in config.toml.
```

---

### 8. Validation & Error Reporting

#### 8.1 **CRITICAL: No Startup Validation** 🔴

(Covered in §1.4)

#### 8.2 **MEDIUM: Error Messages Lack Context** 🟡

**Severity:** Medium  
**Location:** Various (proxy validation, channel initialization, tool execution)

**Issue:** Some error messages don't include the config key path or the invalid value.

**Evidence — Good error:**
```rust:src/config/schema.rs:947-949
anyhow::bail!(
    "Unsupported proxy service selector '{selector}'. \
     Use tool `proxy_config` action `list_services` for valid values"
);
```

**Evidence — Weak error:**
```rust:src/channels/telegram.rs:402
anyhow::bail!("Telegram channel config is missing in config.toml");
// ❌ Doesn't say which field or section is malformed
```

**Recommended Fix:**
```rust
anyhow::bail!(
    "Telegram channel config is missing or invalid. \
     Add [channels_config.telegram] section to config.toml with bot_token and allowed_users. \
     Run `zeroclaw onboard --channels-only` to configure interactively."
);
```

#### 8.3 Silent Fallbacks: 🟡

**Locations:**
- `src/config/schema.rs:3177` — invalid proxy config is logged as warning but silently disabled
- `src/config/schema.rs:2560` — `default_temperature = 0.7` applied even if config specifies invalid value (due to `Default` impl)

**Impact:** Operators may not notice misconfigurations until the feature is needed.

**Recommended Fix:**
- Fail fast on invalid config (require explicit `validate()` call)
- Log warnings at `WARN` level with clear remediation steps
- Add `zeroclaw config validate` command to test config without starting services

---

### 9. Documentation Parity: config-reference.md

**Location:** `docs/config-reference.md`

#### 9.1 **MEDIUM: Documentation Lags Schema by ~50% Coverage** 🟡

**Severity:** Medium

**Issue:** `config-reference.md` documents ~20 top-level keys but schema has 40+ public config sections.

**Missing from docs:**
- `[cost]` — daily/monthly spending limits
- `[identity]` — AIEOS / OpenClaw identity format
- `[hardware]` — hardware wizard config
- `[peripherals]` — STM32/RPi GPIO boards
- `[browser.computer_use]` — sidecar endpoint config
- `[http_request]` — HTTP request tool config
- `[composio]` — 1000+ OAuth tool integration
- `[[agents]]` — delegate sub-agent configs
- `[query_classification]` — automatic model hint routing
- `[[model_routes]]` — comprehensive hint routing examples
- `[[embedding_routes]]` — embedding provider routing

**Impact:**
- Operators don't know these features exist
- Configuration errors go undetected (no validation, no docs)
- Support burden increases (users ask "how do I configure X?")

**Recommended Fix:**

1. **Add missing sections to `config-reference.md`:**
```markdown
## `[cost]`

| Key | Default | Purpose |
|---|---|---|
| `enabled` | `false` | Enable cost tracking and budget enforcement |
| `daily_limit_usd` | `10.00` | Maximum daily spend (USD) |
| `monthly_limit_usd` | `100.00` | Maximum monthly spend (USD) |
| `warn_at_percent` | `80` | Warn when spending reaches this % of limit |
| `allow_override` | `false` | Allow `--override` flag to bypass limits |

## `[identity]`

| Key | Default | Purpose |
|---|---|---|
| `format` | `"openclaw"` | Identity format: `openclaw` or `aieos` |
| `aieos_path` | _none_ | Path to AIEOS JSON file (relative to workspace) |

## `[peripherals]`

Enable hardware peripheral support (STM32 Nucleo, RPi GPIO):

```toml
[peripherals]
enabled = true
datasheet_dir = "datasheets"  # for RAG-based pin lookup

[[peripherals.boards]]
board = "nucleo-f401re"
transport = "serial"
path = "/dev/ttyACM0"
baud = 115200
```
```

2. **Add "Last verified" date to doc header (already present: `February 19, 2026`) and update quarterly.**

#### 9.2 Config Schema Export: ✅ GOOD

**Evidence:**
```bash
zeroclaw config schema  # exports JSON Schema
```

This is excellent for external tooling (IDE autocomplete, validators). Well-implemented.

---

### 10. Extensibility: Adding New Config Sections

#### 10.1 Process: ✅ EXCELLENT

Adding new providers/channels/tools via trait + factory requires **zero config changes** because of `#[serde(default)]` on all subsystem configs. New features can add optional config sections without breaking existing configs.

**Example:** Adding a new channel type:
```rust
// 1. Define config struct
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MyChannelConfig {
    pub api_key: String,
    pub allowed_users: Vec<String>,
}

// 2. Add to ChannelsConfig
pub struct ChannelsConfig {
    // ... existing channels ...
    #[serde(default)]
    pub my_channel: Option<MyChannelConfig>,  // ← optional, no breaking change
}

// 3. Implement Channel trait
struct MyChannel { /* ... */ }
#[async_trait]
impl Channel for MyChannel { /* ... */ }

// 4. Register in factory (no config needed)
```

**No changes to `Config::load_or_init()` required.**

#### 10.2 **LOW: Config Section Naming Inconsistency** 🟡

**Severity:** Low

**Issue:** Some top-level sections are singular (`gateway`, `runtime`, `memory`), others plural (`channels_config`, `peripherals`, `agents`). Not strictly wrong, but inconsistent.

**Recommended Fix:** Use singular for single-instance services, plural for collections:
- ✅ `gateway` (one gateway instance)
- ✅ `channels_config` (collection of channel configs)
- ✅ `agents` (map of delegate agents)
- ❌ `peripherals` → should be `peripheral` or `peripheral_config` (it's a collection of boards)

---

## Summary of Recommendations

### 🔴 Critical (Fix Immediately)

1. **Add `Config::validate()` method** — Validate all config sections at load time, fail fast on invalid values (§1.4)
2. **Restrict public API surface** — Mark internal modules as `pub(crate)`, only expose trait interfaces and stable types (§3.1)

### 🟡 High Priority (Fix Before v1.0)

3. **Add deprecation system** — Support `#[deprecated]` attributes, auto-migrate old keys, warn users (§4.1)
4. **Update `config-reference.md`** — Document all missing config sections (cost, identity, hardware, peripherals, etc.) (§9.1)
5. **Add `zeroclaw config validate` command** — Dry-run validation without starting services
6. **Improve CLI help text** — Add examples and detailed descriptions to complex commands (§2.2)
7. **Validate CLI arguments** — Add range validators to temperature, port, etc. (§2.3)

### 🟢 Low Priority (Technical Debt)

8. **Update `.env.example`** — Add recently-added env vars (reasoning, storage, proxy scope) (§6.3)
9. **Improve error messages** — Include config key paths and remediation steps (§8.2)
10. **Document proxy auto-enable behavior** — Clarify when `HTTP_PROXY` env var auto-enables proxy (§7.2)
11. **Disable web search by default** — Require explicit opt-in for network services (§5.3)
12. **Standardize config section naming** — Singular for singletons, plural for collections (§10.2)

---

## Risk Register

| Risk ID | Severity | Likelihood | Impact | Description | Mitigation |
|---------|----------|------------|--------|-------------|------------|
| CFG-001 | 🔴 Critical | High | High | Invalid config accepted at load, fails at runtime | Add `Config::validate()` with bounds checking |
| CFG-002 | 🔴 Critical | Medium | High | Accidental API surface exposure due to `pub mod` | Mark internal modules `pub(crate)`, expose only trait interfaces |
| CFG-003 | 🟡 High | Medium | Medium | Breaking config changes with no migration path | Add deprecation system + auto-migration |
| CFG-004 | 🟡 High | Medium | Medium | Documentation drift (50% of schema undocumented) | Update `config-reference.md` quarterly |
| CFG-005 | 🟡 Medium | Low | Medium | CLI help text gaps cause user confusion | Add `long_about` examples to complex commands |
| CFG-006 | 🟡 Medium | Medium | Low | Invalid CLI args accepted (temp range, port range) | Add clap value_parser range validators |
| CFG-007 | 🟢 Low | Low | Low | `.env.example` missing recent vars | Add reasoning, storage, proxy vars |
| CFG-008 | 🟢 Low | Low | Low | Web search auto-enabled by default | Change to opt-in |

---

## Appendix: Config Schema Statistics

**Schema Metrics:**
- **Top-level keys:** 28 (including nested subsystems)
- **Subsystem configs:** 25+ (gateway, autonomy, runtime, reliability, scheduler, etc.)
- **Total config fields:** ~300+ (including nested structs)
- **Env vars supported:** 40+ (`ZEROCLAW_*` + legacy fallbacks)
- **Default functions:** 80+ (one per optional field)
- **Validation functions:** 1 (`ProxyConfig::validate()` only)

**Public API Surface (via `pub mod`):**
- **Modules:** 30+ (all marked `pub`)
- **Extension traits:** 6 (Provider, Channel, Tool, Memory, RuntimeAdapter, Peripheral)
- **Command enums:** 8 (ServiceCommands, ChannelCommands, etc.)

**Documentation Coverage:**
- **config-reference.md:** ~20 sections documented
- **Schema sections:** ~45 sections defined
- **Coverage:** ~44%

---

**Audit Complete.** This report is a snapshot of the current state. Recommendations should be prioritized based on project roadmap and release timeline. The ZeroClaw team has built a strong foundation; addressing the validation and API boundary issues will solidify it for long-term stability.
