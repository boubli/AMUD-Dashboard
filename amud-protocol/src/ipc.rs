use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
pub struct ChallengeMessage {
    pub challenge: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AgentAuthMessage {
    pub auth: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthProofMessage {
    pub auth: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigRequest {
    pub request: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pve_token_configured: Option<bool>,
}

/// First authenticated line from the agent — registers the session under `node_tag`.
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentHelloPayload {
    pub node_tag: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub platform: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentHelloMessage {
    pub hello: AgentHelloPayload,
}

/// SHA-256(secret ‖ nonce) — the raw secret never crosses the socket.
pub fn agent_auth_proof(secret: &str, nonce: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hasher.update(nonce.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_is_deterministic() {
        let a = agent_auth_proof("secret", "nonce");
        let b = agent_auth_proof("secret", "nonce");
        assert_eq!(a, b);
        assert_ne!(a, agent_auth_proof("secret", "other"));
    }
}
