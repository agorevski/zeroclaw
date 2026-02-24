//! Provider subsystem for model inference backends.
//!
//! This module implements the factory pattern for AI model providers. Each provider
//! implements the [`Provider`] trait defined in [`traits`], and is registered in the
//! factory function [`create_provider`] by its canonical string key.
//!
//! # Extension
//!
//! To add a new provider, implement [`Provider`] in a new submodule and register it
//! in [`create_provider_with_url`]. See `AGENTS.md` §7.1 for the full change playbook.

pub mod openai;
pub mod traits;

#[allow(unused_imports)]
pub use traits::{
    ChatMessage, ChatRequest, ChatResponse, ConversationMessage, Provider, ProviderCapabilityError,
    ToolCall, ToolResultMessage,
};

use std::path::PathBuf;

const MAX_API_ERROR_CHARS: usize = 200;

#[derive(Debug, Clone)]
pub struct ProviderRuntimeOptions {
    pub auth_profile_override: Option<String>,
    pub zeroclaw_dir: Option<PathBuf>,
    pub secrets_encrypt: bool,
    pub reasoning_enabled: Option<bool>,
}

impl Default for ProviderRuntimeOptions {
    fn default() -> Self {
        Self {
            auth_profile_override: None,
            zeroclaw_dir: None,
            secrets_encrypt: true,
            reasoning_enabled: None,
        }
    }
}

fn is_secret_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':')
}

fn token_end(input: &str, from: usize) -> usize {
    let mut end = from;
    for (i, c) in input[from..].char_indices() {
        if is_secret_char(c) {
            end = from + i + c.len_utf8();
        } else {
            break;
        }
    }
    end
}

/// Scrub known secret-like token prefixes from provider error strings.
///
/// Redacts tokens with prefixes like `sk-`, `xoxb-`, `xoxp-`, `ghp_`, `gho_`,
/// `ghu_`, and `github_pat_`.
pub fn scrub_secret_patterns(input: &str) -> String {
    const PREFIXES: [&str; 7] = [
        "sk-",
        "xoxb-",
        "xoxp-",
        "ghp_",
        "gho_",
        "ghu_",
        "github_pat_",
    ];

    let mut scrubbed = input.to_string();

    for prefix in PREFIXES {
        let mut search_from = 0;
        loop {
            let Some(rel) = scrubbed[search_from..].find(prefix) else {
                break;
            };

            let start = search_from + rel;
            let content_start = start + prefix.len();
            let end = token_end(&scrubbed, content_start);

            if end == content_start {
                search_from = content_start;
                continue;
            }

            scrubbed.replace_range(start..end, "[REDACTED]");
            search_from = start + "[REDACTED]".len();
        }
    }

    scrubbed
}

/// Sanitize API error text by scrubbing secrets and truncating length.
pub fn sanitize_api_error(input: &str) -> String {
    let scrubbed = scrub_secret_patterns(input);

    if scrubbed.chars().count() <= MAX_API_ERROR_CHARS {
        return scrubbed;
    }

    let mut end = MAX_API_ERROR_CHARS;
    while end > 0 && !scrubbed.is_char_boundary(end) {
        end -= 1;
    }

    format!("{}...", &scrubbed[..end])
}

/// Build a sanitized provider error from a failed HTTP response.
pub async fn api_error(provider: &str, response: reqwest::Response) -> anyhow::Error {
    let status = response.status();
    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<failed to read provider error body>".to_string());
    let sanitized = sanitize_api_error(&body);
    anyhow::anyhow!("{provider} API error ({status}): {sanitized}")
}

/// Resolve API key for a provider from config and environment variables.
fn resolve_provider_credential(name: &str, credential_override: Option<&str>) -> Option<String> {
    if let Some(raw_override) = credential_override {
        let trimmed_override = raw_override.trim();
        if !trimmed_override.is_empty() {
            return Some(trimmed_override.to_owned());
        }
    }

    let provider_env_candidates: Vec<&str> = match name {
        "openai" => vec!["OPENAI_API_KEY"],
        _ => vec![],
    };

    for env_var in provider_env_candidates {
        if let Ok(value) = std::env::var(env_var) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    for env_var in ["ZEROCLAW_API_KEY", "API_KEY"] {
        if let Ok(value) = std::env::var(env_var) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    None
}

/// Factory: create the right provider from config (without custom URL)
pub fn create_provider(name: &str, api_key: Option<&str>) -> anyhow::Result<Box<dyn Provider>> {
    create_provider_with_options(name, api_key, &ProviderRuntimeOptions::default())
}

/// Factory: create provider with runtime options (auth profile override, state dir).
pub fn create_provider_with_options(
    name: &str,
    api_key: Option<&str>,
    options: &ProviderRuntimeOptions,
) -> anyhow::Result<Box<dyn Provider>> {
    let _ = options;
    create_provider_with_url_and_options(name, api_key, None, options)
}

/// Factory: create the right provider from config with optional custom base URL
pub fn create_provider_with_url(
    name: &str,
    api_key: Option<&str>,
    api_url: Option<&str>,
) -> anyhow::Result<Box<dyn Provider>> {
    create_provider_with_url_and_options(name, api_key, api_url, &ProviderRuntimeOptions::default())
}

/// Factory: create provider with optional base URL and runtime options.
fn create_provider_with_url_and_options(
    name: &str,
    api_key: Option<&str>,
    api_url: Option<&str>,
    _options: &ProviderRuntimeOptions,
) -> anyhow::Result<Box<dyn Provider>> {
    let resolved_credential = resolve_provider_credential(name, api_key)
        .map(|v| String::from_utf8(v.into_bytes()).unwrap_or_default());
    #[allow(clippy::option_as_ref_deref)]
    let key = resolved_credential.as_ref().map(String::as_str);

    match name {
        "openai" => Ok(Box::new(openai::OpenAiProvider::with_base_url(api_url, key))),
        _ => anyhow::bail!(
            "Unknown provider: {name}. Only \"openai\" is currently supported."
        ),
    }
}

/// Create provider chain with retry and fallback behavior.
///
/// Reliability config was removed during the minimal-binary strip. This now
/// delegates directly to `create_provider_with_url`.
pub fn create_resilient_provider(
    primary_name: &str,
    api_key: Option<&str>,
    api_url: Option<&str>,
) -> anyhow::Result<Box<dyn Provider>> {
    create_provider_with_url(primary_name, api_key, api_url)
}

/// Create provider chain with retry/fallback behavior and auth runtime options.
pub fn create_resilient_provider_with_options(
    primary_name: &str,
    api_key: Option<&str>,
    api_url: Option<&str>,
    options: &ProviderRuntimeOptions,
) -> anyhow::Result<Box<dyn Provider>> {
    create_provider_with_url_and_options(primary_name, api_key, api_url, options)
}

/// Create a routed or standard provider. Without routing support, this falls
/// back to a simple provider.
pub fn create_routed_provider(
    primary_name: &str,
    api_key: Option<&str>,
    api_url: Option<&str>,
) -> anyhow::Result<Box<dyn Provider>> {
    create_resilient_provider(primary_name, api_key, api_url)
}

/// Create a routed provider using explicit runtime options.
pub fn create_routed_provider_with_options(
    primary_name: &str,
    api_key: Option<&str>,
    api_url: Option<&str>,
    options: &ProviderRuntimeOptions,
) -> anyhow::Result<Box<dyn Provider>> {
    create_resilient_provider_with_options(primary_name, api_key, api_url, options)
}

/// Information about a supported provider for display purposes.
pub struct ProviderInfo {
    /// Canonical name used in config (e.g. `"openai"`)
    pub name: &'static str,
    /// Human-readable display name
    pub display_name: &'static str,
    /// Alternative names accepted in config
    pub aliases: &'static [&'static str],
    /// Whether the provider runs locally (no API key required)
    pub local: bool,
}

/// Return the list of all known providers for display in `zeroclaw providers list`.
pub fn list_providers() -> Vec<ProviderInfo> {
    vec![
        ProviderInfo {
            name: "openai",
            display_name: "OpenAI",
            aliases: &[],
            local: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_openai() {
        assert!(create_provider("openai", Some("provider-test-credential")).is_ok());
    }

    #[test]
    fn factory_unknown_provider_errors() {
        let p = create_provider("nonexistent", None);
        assert!(p.is_err());
        let msg = p.err().unwrap().to_string();
        assert!(msg.contains("Unknown provider"));
    }

    #[test]
    fn factory_empty_name_errors() {
        assert!(create_provider("", None).is_err());
    }

    #[test]
    fn listed_providers_have_unique_ids_and_aliases() {
        let providers = list_providers();
        let mut canonical_ids = std::collections::HashSet::new();
        let mut aliases = std::collections::HashSet::new();

        for provider in providers {
            assert!(
                canonical_ids.insert(provider.name),
                "Duplicate canonical provider id: {}",
                provider.name
            );

            for alias in provider.aliases {
                assert_ne!(
                    *alias, provider.name,
                    "Alias must differ from canonical id: {}",
                    provider.name
                );
                assert!(
                    !canonical_ids.contains(alias),
                    "Alias conflicts with canonical provider id: {}",
                    alias
                );
                assert!(aliases.insert(alias), "Duplicate provider alias: {}", alias);
            }
        }
    }

    #[test]
    fn listed_providers_and_aliases_are_constructible() {
        for provider in list_providers() {
            assert!(
                create_provider(provider.name, Some("provider-test-credential")).is_ok(),
                "Canonical provider id should be constructible: {}",
                provider.name
            );

            for alias in provider.aliases {
                assert!(
                    create_provider(alias, Some("provider-test-credential")).is_ok(),
                    "Provider alias should be constructible: {} (for {})",
                    alias,
                    provider.name
                );
            }
        }
    }

    // ── API error sanitization ───────────────────────────────

    #[test]
    fn sanitize_scrubs_sk_prefix() {
        let input = "request failed: sk-1234567890abcdef";
        let out = sanitize_api_error(input);
        assert!(!out.contains("sk-1234567890abcdef"));
        assert!(out.contains("[REDACTED]"));
    }

    #[test]
    fn sanitize_scrubs_multiple_prefixes() {
        let input = "keys sk-abcdef xoxb-12345 xoxp-67890";
        let out = sanitize_api_error(input);
        assert!(!out.contains("sk-abcdef"));
        assert!(!out.contains("xoxb-12345"));
        assert!(!out.contains("xoxp-67890"));
    }

    #[test]
    fn sanitize_truncates_long_error() {
        let long = "a".repeat(400);
        let result = sanitize_api_error(&long);
        assert!(result.len() <= 203);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn sanitize_no_secret_no_change() {
        let input = "simple upstream timeout";
        let result = sanitize_api_error(input);
        assert_eq!(result, input);
    }

    #[test]
    fn scrub_github_personal_access_token() {
        let input = "auth failed with token ghp_abc123def456";
        let result = scrub_secret_patterns(input);
        assert_eq!(result, "auth failed with token [REDACTED]");
    }

    #[test]
    fn scrub_github_fine_grained_pat() {
        let input = "failed: github_pat_11AABBC_xyzzy789";
        let result = scrub_secret_patterns(input);
        assert_eq!(result, "failed: [REDACTED]");
    }

    #[test]
    fn resolve_provider_credential_prefers_explicit_argument() {
        let resolved = resolve_provider_credential("openai", Some("  explicit-key  "));
        assert_eq!(resolved, Some("explicit-key".to_string()));
    }
}
