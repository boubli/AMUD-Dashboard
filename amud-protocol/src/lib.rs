//! Shared JSON types for AMUD server ↔ agent communication.

pub mod ipc;
pub mod telemetry;

pub use ipc::{
    agent_auth_proof, AgentAuthMessage, AgentHelloMessage, AgentHelloPayload, AuthProofMessage,
    ChallengeMessage, ConfigRequest,
};
pub use telemetry::{AgentTelemetry, DiskMountTelemetry, LxcContainer, NetworkTelemetry};
