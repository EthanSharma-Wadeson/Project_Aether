//! PROTO-1 bilateral channel simulator.
//!
//! Local deterministic shared-state agreement between two agents.
//! No networking, settlement, tokens, or real payments.
//!
//! ## Trust boundaries
//!
//! - State-changing operations require dual signatures and PROTO-0 capability checks.
//! - `ChannelStore` is read-only to external callers; mutations go through transition APIs.
//! - `finalize_close` and `resolve_dispute` are terminal operations with explicit auth
//!   (see `SECURITY_MODEL.md` § Validated by PROTO-1).

mod chain;
pub mod dispute;
pub mod model;
pub mod state;
pub mod transition;
pub mod verify;

pub use dispute::{raise_dispute, resolve_dispute, DisputeEvidenceV0};
pub use model::{
    ChannelOpenMaterialV0, ChannelStatus, ChannelV0, DualSignedUpdate, FinalityViewV0, ReceiptV0,
    StateUpdateV0, ACTION_ACTIVATE, ACTION_CLOSE, ACTION_DISPUTE, ACTION_OPEN, ACTION_UPDATE,
    MSG_CHANNEL_ACTIVATE, MSG_CHANNEL_CLOSE, MSG_CHANNEL_DISPUTE, MSG_CHANNEL_OPEN,
    MSG_CHANNEL_UPDATE,
};
pub use state::ChannelStateV0;
pub use transition::{
    abort_open, activate_channel, apply_update, begin_close, finalize_close, open_channel,
    sign_open_dual, sign_update_dual, ChannelStore,
};
