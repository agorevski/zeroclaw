# Operational Runbook

Production operations guide for ZeroClaw deployments. This document covers log
level semantics, `RUST_LOG` configuration, recommended alerting thresholds, and
common failure patterns.

---

## Log Level Semantics

ZeroClaw uses the standard Rust `tracing` levels. Each level has a specific
operational meaning in this project:

| Level   | Meaning                                                      | Action Required |
|---------|--------------------------------------------------------------|-----------------|
| `error` | Service degraded or action failed; operator attention needed | Investigate immediately |
| `warn`  | Degraded but functional; may need attention soon             | Review within SLA |
| `info`  | Normal operational events (startup, shutdown, config loaded) | None (baseline) |
| `debug` | Troubleshooting detail (request/response flow, decisions)    | Use during incident investigation |
| `trace` | Deep debugging (high-volume per-message/per-field detail)    | Use only for targeted debugging sessions |

### What Each Level Captures

- **`error`**: Provider call failures after retries exhausted, tool execution
  panics, security policy violations, configuration errors that prevent startup.
- **`warn`**: Retryable provider errors, rate-limit approaching, tool execution
  timeout, deprecated config keys detected, memory backend degraded.
- **`info`**: Agent start/stop, memory backend initialized, peripheral tools
  loaded, channel connected, provider resolved, observer backend active.
- **`debug`**: Per-turn LLM request/response metadata (no prompt content),
  tool call decisions, history trimming, tool registry composition.
- **`trace`**: Raw message counts, token-level details, full tool argument
  schemas (sanitized), span entry/exit for orchestration loop internals.

---

## `RUST_LOG` Configuration

ZeroClaw uses [`tracing-subscriber`](https://docs.rs/tracing-subscriber) with
the `env-filter` feature. The `RUST_LOG` environment variable controls
per-module log levels using filter directives.

### Common Configurations

```bash
# Default production — operational events only
RUST_LOG=zeroclaw=info

# Debug provider issues (LLM calls, retries, routing)
RUST_LOG=zeroclaw::providers=debug,zeroclaw=info

# Deep channel debugging (message parsing, transport)
RUST_LOG=zeroclaw::channels=trace,zeroclaw=info

# Debug agent orchestration loop (turn flow, tool decisions)
RUST_LOG=zeroclaw::agent=debug,zeroclaw=info

# Debug tool execution (call details, timing)
RUST_LOG=zeroclaw::tools=debug,zeroclaw=info

# Debug security policy decisions
RUST_LOG=zeroclaw::security=debug,zeroclaw=info

# Debug memory backend operations
RUST_LOG=zeroclaw::memory=debug,zeroclaw=info

# Full debug across all subsystems (noisy — use sparingly)
RUST_LOG=zeroclaw=debug

# Trace everything (very high volume — targeted sessions only)
RUST_LOG=zeroclaw=trace
```

### Filter Syntax Reference

```bash
# Multiple modules at different levels
RUST_LOG=zeroclaw::agent=debug,zeroclaw::providers=trace,zeroclaw=info

# Include dependency crate logs (e.g., reqwest HTTP client)
RUST_LOG=zeroclaw=info,reqwest=debug

# Span-based filtering (show only spans named "agent_turn")
RUST_LOG=zeroclaw[agent_turn]=debug
```

---

## Recommended Alerting Thresholds

These thresholds are starting points for OTLP-based alerting. Tune them based
on your deployment profile, model provider latency, and traffic patterns.

### Core Metrics

| Metric                          | Warning     | Critical    | Notes |
|---------------------------------|-------------|-------------|-------|
| LLM latency P99                 | > 15s       | > 30s       | Varies by model; adjust for large-context models |
| LLM error rate (5 min window)   | > 2%        | > 5%        | Sustained errors indicate provider issue |
| Tool call failure rate           | > 5%        | > 15%       | Distinguish transient vs. persistent failures |
| Tool call latency P99            | > 10s       | > 30s       | Shell tools may have higher baseline |
| Agent turn count per message     | > 7         | > 9         | Approaching MAX_TOOL_ITERATIONS (10) |
| Queue depth                      | > 50        | > 100       | If using channel message queuing |
| Memory usage (process RSS)       | > 70%       | > 90%       | Monitor for memory leaks in long-running mode |

### Security Metrics

| Metric                          | Warning     | Critical    | Notes |
|---------------------------------|-------------|-------------|-------|
| Policy violations (5 min)        | > 3         | > 10        | May indicate misconfiguration or abuse |
| Auth failures (5 min)            | > 5         | > 20        | Brute-force indicator |
| Approval denials (1 hr)          | > 10        | > 50        | Review autonomy policy if frequent |

### Infrastructure Metrics

| Metric                          | Warning     | Critical    | Notes |
|---------------------------------|-------------|-------------|-------|
| Gateway response latency P99     | > 2s        | > 5s        | Webhook processing delay |
| Open file descriptors            | > 80% limit | > 95% limit | Long-running deployments |
| Disk usage (workspace dir)       | > 80%       | > 95%       | Memory backend file growth |

---

## Common Failure Patterns

### 1. LLM Provider Timeout

**Symptom**: Agent hangs or returns slowly; `LlmResponse` duration spikes.

**Likely Cause**: Provider API degradation, large context window, or network
latency.

**Resolution**:
1. Check provider status page.
2. Review `RUST_LOG=zeroclaw::providers=debug` output for retry patterns.
3. Consider switching to a faster model via `default_model` config or
   `--model` CLI override.
4. Check `reliability` config for timeout and retry settings.

### 2. Tool Execution Failures

**Symptom**: `ToolCall` events show `success: false` repeatedly.

**Likely Cause**: Missing dependencies in the execution environment, permission
issues, or malformed tool arguments from the LLM.

**Resolution**:
1. Enable `RUST_LOG=zeroclaw::agent=debug` to see tool call arguments
   (input sizes, tool names).
2. Check the specific tool's error message in the `ToolCall` observer event.
3. Verify the execution environment has required binaries and permissions.
4. Review security policy for overly restrictive tool allowlists.

### 3. Agent Exceeds Maximum Tool Iterations

**Symptom**: Error `Agent exceeded maximum tool iterations (10)`.

**Likely Cause**: LLM is stuck in a loop, repeatedly calling tools without
producing a final answer.

**Resolution**:
1. Review conversation history for circular tool-call patterns.
2. Check if the system prompt is too vague or conflicting.
3. Consider adjusting `temperature` (lower values produce more deterministic
   behavior).
4. Monitor `agent_turn` tracing spans for iteration counts approaching the
   limit.

### 4. Memory Backend Errors

**Symptom**: `Memory initialized` log missing or errors during memory
operations.

**Likely Cause**: File permissions on workspace directory, corrupted SQLite
database, or missing embedding model configuration.

**Resolution**:
1. Enable `RUST_LOG=zeroclaw::memory=debug` for backend diagnostics.
2. Verify workspace directory exists and is writable.
3. For SQLite backend, check for database lock contention.
4. For embedding-based memory, verify the API key and model availability.

### 5. Channel Connection Failures

**Symptom**: Channel does not receive or send messages; health check fails.

**Likely Cause**: Invalid credentials, network connectivity, or API rate
limits.

**Resolution**:
1. Enable `RUST_LOG=zeroclaw::channels=debug` for transport-level detail.
2. Verify channel credentials (bot token, webhook URL) are correctly
   configured and not expired.
3. Check for rate-limit headers in channel API responses.
4. Ensure the gateway bind address is reachable if using webhooks.

### 6. Security Policy Violations

**Symptom**: Tools denied with policy violation errors; approval prompts
appearing unexpectedly.

**Likely Cause**: Security policy is stricter than expected, or tool names
changed after policy was configured.

**Resolution**:
1. Enable `RUST_LOG=zeroclaw::security=debug` to see policy evaluation.
2. Review `autonomy` config section for tool allowlists and risk levels.
3. Verify tool names in the policy match the current tool registry.
4. Check approval manager configuration if interactive approval is enabled.

---

## Tracing and Spans

ZeroClaw instruments the agent orchestration loop and tool execution with
`tracing` events. These events are emitted at `debug` level and include
structured attributes for filtering and analysis. They complement the
`ObserverEvent` system which handles higher-level operational metrics.

### Key Tracing Events

| Event Name         | Location              | Attributes                         |
|--------------------|-----------------------|------------------------------------|
| `agent_turn`       | Agent orchestration   | `iteration`, `provider`, `model`   |
| `tool_execution`   | Tool call handling    | `tool.name`, `input_size`          |

### Viewing Tracing Output

- **OTLP backend**: Events are exported to your configured collector (Jaeger,
  Tempo, etc.) and can be viewed in the trace UI.
- **Log backend**: Events appear as structured log entries with context.
- **Noop backend**: Events are discarded (zero overhead in production if
  observability is disabled).

---

## Related Documentation

- [Audit Logging](audit-logging.md) — Tamper-evident audit trail design
- [Resource Limits](resource-limits.md) — Runtime resource constraints
- [Security Roadmap](security-roadmap.md) — Security feature planning
- [Sandboxing](sandboxing.md) — Execution sandboxing design
