//! Stub runtime trace module — record_event is a no-op.

/// No-op record_event stub (runtime trace storage stripped).
#[allow(clippy::too_many_arguments)]
pub fn record_event(
    _event_type: &str,
    _channel: Option<&str>,
    _provider: Option<&str>,
    _model: Option<&str>,
    _turn_id: Option<&str>,
    _success: Option<bool>,
    _message: Option<&str>,
    _payload: serde_json::Value,
) {
}
