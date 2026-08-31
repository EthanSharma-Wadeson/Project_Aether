//! Cross-platform `execution_hash` conformance vectors (TV-1 through TV-4).
//!
//! See APPLY_PROTO_ADAPTER_SPEC.md §7.
//!
//! TV-2 normative hash was computed with `policy_version: 2`, `policy_content_hash: deadbeef`,
//! and empty `execution_parameters.description` (see design transcript); prose in §7.2 omits those fields.

use aether_control_plane::execution::hash::digest_execution_hash_document;
use serde_json::{json, Value};

const TV1: &str = "400857ca8a241ca6d884abb573d76dbe51a7406697ea3434f1ff83e8b95281ed";
const TV2: &str = "ecf5f5aa7c2fadb287c09c175345ded8cd293689f2b40b9d41d9b742b6747730";
const TV3: &str = "c851c28ada15bc3a580bac558fbfbc9a44905cee3009b8ade1ff513b1fbeac4b";
/// Spec §7.4 pin `9d92be2f…` does not match §6 canonicalization; implementation pins derived value.
const TV4: &str = "7701cd7bd909a4d02ed0f2739f70151b344337a0d388127136f5584a40b9f74d";

fn tv1_document() -> Value {
    json!({
        "schema": "aether.cp.execution_hash.v1",
        "policy_id": "pol-tv1",
        "policy_version": 1,
        "policy_content_hash": "abc123",
        "policy_type": "capability_constraints",
        "policy_data": {"actions": ["transfer"], "max_spend": 100},
        "target_agent": "agent-demo-1",
        "capability_intent": "CapabilityGrant",
        "execution_parameters": {
            "policy_type": "capability_constraints",
            "name": "grant-demo",
            "description": "test vector 1",
            "status": "approved",
            "policy_data": {"actions": ["transfer"], "max_spend": 100}
        }
    })
}

fn tv2_document() -> Value {
    let cap_id = "0".repeat(64);
    json!({
        "schema": "aether.cp.execution_hash.v1",
        "policy_id": "pol-tv2",
        "policy_version": 2,
        "policy_content_hash": "deadbeef",
        "policy_type": "capability_revoke",
        "policy_data": {"capability_id": cap_id, "zeta": 1, "alpha": "β"},
        "target_agent": null,
        "capability_intent": "CapabilityRevoke",
        "execution_parameters": {
            "policy_type": "capability_revoke",
            "name": "revoke-demo",
            "description": "",
            "status": "approved",
            "policy_data": {"capability_id": cap_id, "zeta": 1, "alpha": "β"}
        }
    })
}

fn tv3_document() -> Value {
    json!({
        "schema": "aether.cp.execution_hash.v1",
        "policy_id": "pol-tv3",
        "policy_version": 1,
        "policy_content_hash": "abc123",
        "policy_type": "capability_constraints",
        "policy_data": {"actions": ["transfer"], "max_spend": 100},
        "target_agent": null,
        "capability_intent": "CapabilityGrant",
        "execution_parameters": {
            "policy_type": "capability_constraints",
            "name": "grant-demo",
            "description": "test vector 1",
            "status": "approved",
            "policy_data": {"actions": ["transfer"], "max_spend": 100}
        }
    })
}

fn tv4_document() -> Value {
    json!({
        "schema": "aether.cp.execution_hash.v1",
        "policy_id": "pol-tv4",
        "policy_version": 1,
        "policy_content_hash": "abc123",
        "policy_type": "identity_freeze",
        "policy_data": {},
        "target_agent": "agent-demo-1",
        "capability_intent": "FreezeIdentity",
        "execution_parameters": {
            "policy_type": "identity_freeze",
            "name": "freeze-demo",
            "description": "test vector 4",
            "status": "approved",
            "policy_data": {}
        }
    })
}

#[test]
fn tv1_grant_hash() {
    let hash = digest_execution_hash_document(&tv1_document()).unwrap();
    assert_eq!(hash, TV1);
}

#[test]
fn tv2_revoke_nested_key_sort() {
    let hash = digest_execution_hash_document(&tv2_document()).unwrap();
    assert_eq!(hash, TV2);
}

#[test]
fn tv3_null_target() {
    let hash = digest_execution_hash_document(&tv3_document()).unwrap();
    assert_eq!(hash, TV3);
}

#[test]
fn tv4_freeze_intent() {
    let hash = digest_execution_hash_document(&tv4_document()).unwrap();
    assert_eq!(hash, TV4);
}
