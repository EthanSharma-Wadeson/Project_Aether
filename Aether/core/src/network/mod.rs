//! PROTO-NET-0 — secure agent transport foundation (local simulation).
//!
//! Deterministic in-process simulation of authenticated agent-to-agent messaging.
//! No TCP/UDP/WebSockets, no encryption transport, no DHT, no real networking.

pub mod directory;
pub mod envelope;
pub mod hello;
pub mod model;
pub mod session;
pub mod verify;

pub use directory::{AgentDirectoryV0, DirectoryEntryV0};
pub use envelope::{
    deliver_envelope, make_envelope, sign_envelope, verify_and_open_envelope, MessageEnvelopeV0,
    MSG_NET_ENVELOPE,
};
pub use hello::{
    accept_hello, complete_hello, create_hello, initiate_hello, sign_hello, ProtocolHelloV0,
    MSG_NET_HELLO, MSG_NET_HELLO_ACCEPT,
};
pub use model::{
    NetworkFeature, SessionStatus, ENVELOPE_VERSION, MIN_PROTOCOL_VERSION,
    NETWORK_PROTOCOL_VERSION, NETWORK_SCHEMA_VERSION,
};
pub use session::{close_session, create_session, SecureSessionV0, SessionRecord, SessionStore};
pub use verify::{require_active_identity, verify_envelope_signature, verify_hello_signature};
