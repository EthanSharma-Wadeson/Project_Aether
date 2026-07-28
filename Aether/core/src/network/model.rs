//! Network model constants and shared enums.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Created,
    HelloSent,
    HelloAccepted,
    Established,
    Closed,
}

impl SessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::HelloSent => "hello_sent",
            Self::HelloAccepted => "hello_accepted",
            Self::Established => "established",
            Self::Closed => "closed",
        }
    }

    pub fn parse(s: &str) -> crate::error::Result<Self> {
        match s {
            "created" => Ok(Self::Created),
            "hello_sent" => Ok(Self::HelloSent),
            "hello_accepted" => Ok(Self::HelloAccepted),
            "established" => Ok(Self::Established),
            "closed" => Ok(Self::Closed),
            _ => Err(crate::error::Error::MalformedObject("session status")),
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Closed)
    }

    pub fn allows_envelope(self) -> bool {
        matches!(self, Self::Established)
    }
}

/// Supported feature tokens advertised in ProtocolHelloV0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkFeature {
    EnvelopeV0,
    SessionV0,
    Custom(String),
}

impl NetworkFeature {
    pub fn as_str(&self) -> &str {
        match self {
            Self::EnvelopeV0 => "net.envelope.v0",
            Self::SessionV0 => "net.session.v0",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "net.envelope.v0" => Self::EnvelopeV0,
            "net.session.v0" => Self::SessionV0,
            other => Self::Custom(other.into()),
        }
    }
}

pub const NETWORK_PROTOCOL_VERSION: u32 = 1;
pub const NETWORK_SCHEMA_VERSION: u32 = 1;
pub const ENVELOPE_VERSION: u32 = 1;
pub const MIN_PROTOCOL_VERSION: u32 = 1;
