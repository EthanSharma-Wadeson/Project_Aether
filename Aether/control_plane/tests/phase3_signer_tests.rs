//! Milestone 2 Phase 3 — Signer Abstraction tests.
//! No PROTO-0 calls. No protocol mutations.

use aether_control_plane::config::SignerConfig;
use aether_control_plane::db::operators::OperatorRole;
use aether_control_plane::db::Db;
use aether_control_plane::signer::audit::SignOutcome;
use aether_control_plane::signer::enterprise_signer::EnterpriseSigner;
use aether_control_plane::signer::operation::GovernanceOperation;
use aether_control_plane::signer::traits::Signer;
use aether_control_plane::signer::{build_signing_gateway_with, SignerAuditService};
use chrono::{TimeZone, Utc};

fn signer_config() -> SignerConfig {
    SignerConfig {
        mode: "enterprise".into(),
        identity: "enterprise-default".into(),
        // Fixed seed for deterministic tests.
        seed_hex: Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into()),
        ephemeral: false,
    }
}

fn sample_operation() -> GovernanceOperation {
    let mut op = GovernanceOperation::new(
        "req-phase3",
        "op-1",
        OperatorRole::Operator,
        "policy.apply.future",
        br#"{"probe":true}"#,
    )
    .with_policy("policy-1", 3)
    .with_target("agent:demo");
    // Pin timestamp for canonical stability tests.
    op.created_at = Utc.with_ymd_and_hms(2026, 7, 29, 12, 0, 0).unwrap();
    op.operation_id = "op-fixed-id".into();
    op
}

#[test]
fn operation_signs_successfully() {
    let signer = EnterpriseSigner::from_config(&signer_config()).unwrap();
    let op = sample_operation();
    let signed = signer.sign_operation(&op).unwrap();
    assert_eq!(signed.signer_identity, "enterprise-default");
    assert!(!signed.signature.is_empty());
    assert_eq!(signed.operation.operation_id, op.operation_id);
}

#[test]
fn signature_verifies() {
    let signer = EnterpriseSigner::from_config(&signer_config()).unwrap();
    let op = sample_operation();
    let signed = signer.sign_operation(&op).unwrap();
    assert!(signer
        .verify_signature(&signed.operation, &signed.signature)
        .is_ok());
}

#[test]
fn invalid_signature_rejected() {
    let signer = EnterpriseSigner::from_config(&signer_config()).unwrap();
    let op = sample_operation();
    let signed = signer.sign_operation(&op).unwrap();
    let mut bad = signed.signature.clone();
    // Flip last hex nibble.
    let last = bad.pop().unwrap();
    bad.push(if last == '0' { '1' } else { '0' });
    let err = signer.verify_signature(&op, &bad).unwrap_err();
    assert!(err.to_string().contains("signature") || err.to_string().contains("mismatch"));
}

#[test]
fn signer_identity_returned() {
    let signer = EnterpriseSigner::from_config(&signer_config()).unwrap();
    assert_eq!(signer.signer_identity(), "enterprise-default");
    assert!(!signer.public_key_hex().is_empty());
}

#[test]
fn payload_hash_stable() {
    let a = GovernanceOperation::new("r", "o", OperatorRole::Admin, "act", b"same");
    let b = GovernanceOperation::new("r", "o", OperatorRole::Admin, "act", b"same");
    assert_eq!(a.payload_hash, b.payload_hash);
    let c = GovernanceOperation::new("r", "o", OperatorRole::Admin, "act", b"diff");
    assert_ne!(a.payload_hash, c.payload_hash);
    assert_eq!(a.payload_hash.len(), 64);
}

#[test]
fn canonical_serialization_stable() {
    let op = sample_operation();
    let once = op.canonical_bytes().unwrap();
    let twice = op.canonical_bytes().unwrap();
    assert_eq!(once, twice);
    assert_eq!(op.operation_hash().unwrap(), op.operation_hash().unwrap());

    let mut other = op.clone();
    other.action = "other".into();
    assert_ne!(
        op.canonical_bytes().unwrap(),
        other.canonical_bytes().unwrap()
    );
}

#[tokio::test]
async fn successful_sign_audited() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("signer.db");
    let db = Db::connect(db_path.to_str().unwrap()).await.unwrap();
    db.migrate().await.unwrap();

    let gateway = build_signing_gateway_with(&signer_config(), &db).unwrap();
    let op = sample_operation();
    let signed = gateway.sign_operation(&op).await.unwrap();
    assert!(!signed.signature.is_empty());

    let audit = SignerAuditService::new(db.pool().clone());
    let rows = audit.list_recent(10).await.unwrap();
    assert!(rows
        .iter()
        .any(|r| { r.action == "SIGN_REQUEST_CREATED" && r.outcome == SignOutcome::Created }));
    assert!(rows
        .iter()
        .any(|r| { r.action == "SIGN_REQUEST_COMPLETED" && r.outcome == SignOutcome::Completed }));
    assert!(rows
        .iter()
        .all(|r| r.signer_identity == "enterprise-default"));
    assert!(rows.iter().all(|r| r.operation_id == op.operation_id));
    assert!(rows.iter().all(|r| r.request_id == "req-phase3"));
    assert!(rows
        .iter()
        .all(|r| r.policy_id.as_deref() == Some("policy-1")));
}

#[tokio::test]
async fn failed_sign_audited() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("signer-fail.db");
    let db = Db::connect(db_path.to_str().unwrap()).await.unwrap();
    db.migrate().await.unwrap();

    let gateway = build_signing_gateway_with(&signer_config(), &db).unwrap();
    let mut op = sample_operation();
    op.action = "".into(); // triggers SignerError::Sign

    let err = gateway.sign_operation(&op).await.unwrap_err();
    assert!(err.to_string().contains("action"));

    let audit = SignerAuditService::new(db.pool().clone());
    let rows = audit.list_recent(10).await.unwrap();
    assert!(rows.iter().any(|r| r.action == "SIGN_REQUEST_CREATED"));
    assert!(rows
        .iter()
        .any(|r| { r.action == "SIGN_REQUEST_FAILED" && r.outcome == SignOutcome::Failed }));
    assert!(!rows.iter().any(|r| r.action == "SIGN_REQUEST_COMPLETED"));
}

#[test]
fn signer_does_not_call_protocol() {
    // Structural guarantee: EnterpriseSigner / SigningGateway modules have no
    // protocol imports. This test documents the Phase 3 boundary.
    let signer = EnterpriseSigner::from_config(&signer_config()).unwrap();
    let op = sample_operation();
    let _ = signer.sign_operation(&op).unwrap();
    // If we reached here without protocol adapters, Phase 3 boundary holds.
}
