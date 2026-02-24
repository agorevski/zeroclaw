//! Gateway handler and protocol traits for pluggable request handling.

use async_trait::async_trait;
use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Frame types in the gateway protocol (matching OpenClaw: request, response, event).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrameType {
    Request,
    Response,
    Event,
}

/// A protocol frame exchanged between gateway clients and server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub frame_type: FrameType,
    pub id: Option<String>,
    pub method: Option<String>,
    pub payload: serde_json::Value,
}

/// Protocol encoder/decoder for gateway communication.
///
/// Implement this trait to support different wire formats (JSON, MessagePack, etc.)
pub trait Protocol: Send + Sync {
    fn encode(&self, frame: &Frame) -> Result<Vec<u8>>;
    fn decode(&self, data: &[u8]) -> Result<Frame>;
    fn name(&self) -> &str;
}

/// JSON-based protocol implementation.
#[derive(Debug, Clone, Default)]
pub struct JsonProtocol;

impl Protocol for JsonProtocol {
    fn encode(&self, frame: &Frame) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(frame)?)
    }
    fn decode(&self, data: &[u8]) -> Result<Frame> {
        Ok(serde_json::from_slice(data)?)
    }
    fn name(&self) -> &str { "json" }
}

/// Context provided to gateway handlers for each request.
#[derive(Debug, Clone)]
pub struct GatewayRequestContext {
    pub client_id: String,
    pub auth_token: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Response from a gateway handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayResponse {
    pub status: u16,
    pub body: serde_json::Value,
}

/// Gateway handler for processing domain-specific requests.
///
/// Matching OpenClaw's 25+ handler modules pattern. Each handler owns
/// a domain (agents, channels, models, cron, etc.) and processes
/// requests within that domain.
#[async_trait]
pub trait GatewayHandler: Send + Sync {
    /// The domain this handler manages (e.g., "agents", "channels", "models").
    fn domain(&self) -> &str;
    /// Handle a request within this domain.
    async fn handle(&self, method: &str, payload: serde_json::Value, context: &GatewayRequestContext) -> Result<GatewayResponse>;
    /// List supported methods.
    fn methods(&self) -> Vec<&str>;
    fn name(&self) -> &str;
}

/// Gateway event broadcaster for pushing events to connected clients.
#[async_trait]
pub trait EventBroadcaster: Send + Sync {
    /// Broadcast an event to all connected clients.
    async fn broadcast(&self, event: Frame) -> Result<()>;
    /// Broadcast to clients matching a scope filter.
    async fn broadcast_scoped(&self, event: Frame, scope: &str) -> Result<()>;
    /// Get count of connected clients.
    fn client_count(&self) -> usize;
    fn name(&self) -> &str;
}

/// Auth provider for gateway connections.
#[async_trait]
pub trait GatewayAuth: Send + Sync {
    /// Validate an auth token and return client identity.
    async fn authenticate(&self, token: &str) -> Result<Option<String>>;
    /// Check if a client has a specific permission.
    async fn authorize(&self, client_id: &str, permission: &str) -> Result<bool>;
    fn name(&self) -> &str;
}
