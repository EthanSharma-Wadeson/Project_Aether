//! PROTO-2 settlement receipt validation tests (P2-R###).

mod common;

use aether_core::crypto::signing::{sign_body, DOMAIN_TAG};
use aether_core::crypto::verify::SignedMessage;
use aether_core::error::Error;
use aether_core::escrow::{sign_receipt, submit_receipt, EscrowStatus, MSG_ESCROW_SUBMIT_RECEIPT};
use common::*;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

#[test]
fn p2_r001_valid_provider_signed_receipt_accepted() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let escrow = submit_receipt_ready(&mut pair, &terms, escrow_id, 1, 30);
    assert_eq!(escrow.status, EscrowStatus::ReceiptAccepted);
    assert!(pair
        .escrows
        .get(&escrow_id)
        .unwrap()
        .bound_receipt
        .is_some());
}

#[test]
fn p2_r002_forged_wrong_key_signature_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let receipt = sample_receipt(&terms, escrow_id, 1, 30);
    let mut csprng = OsRng;
    let wrong_key = SigningKey::generate(&mut csprng);
    let provider_id = pair.provider.identity.derived_agent_id();
    let mut signed = sign_receipt(&receipt, &wrong_key, &provider_id).unwrap();
    signed.signature = vec![1u8; 64];
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidSignature);
}

#[test]
fn p2_r003_receipt_wrong_escrow_id_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let wrong_id = [7u8; 32];
    let receipt = sample_receipt(&terms, wrong_id, 1, 30);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = sign_receipt(&receipt, &pair.provider.signing_key, &provider_id).unwrap();
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidReceipt);
}

#[test]
fn p2_r004_receipt_wrong_terms_version_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let mut bad_terms = terms.clone();
    bad_terms.terms_version = 99;
    let receipt = sample_receipt(&bad_terms, escrow_id, 1, 30);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = sign_receipt(&receipt, &pair.provider.signing_key, &provider_id).unwrap();
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidReceipt);
}

#[test]
fn p2_r005_receipt_wrong_claim_type_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let mut bad_terms = terms.clone();
    bad_terms.claim_type = "wrong".into();
    let receipt = sample_receipt(&bad_terms, escrow_id, 1, 30);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = sign_receipt(&receipt, &pair.provider.signing_key, &provider_id).unwrap();
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidReceipt);
}

#[test]
fn p2_r006_receipt_wrong_result_code_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let mut bad_terms = terms.clone();
    bad_terms.required_result_code = "fail".into();
    let receipt = sample_receipt(&bad_terms, escrow_id, 1, 30);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = sign_receipt(&receipt, &pair.provider.signing_key, &provider_id).unwrap();
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidReceipt);
}

#[test]
fn p2_r007_receipt_wrong_output_commitment_rejected() {
    let mut pair = setup_escrow_pair();
    let payer_id = pair.payer.identity.derived_agent_id();
    let provider_id = pair.provider.identity.derived_agent_id();
    let terms = sample_terms(&payer_id, &provider_id, 100, 10, 100, 500, 100);
    let mut terms_with_commit = terms.clone();
    terms_with_commit.accept_output_commitment = true;
    terms_with_commit.expected_output_commitment = Some(vec![1, 2, 3]);
    let escrow_id = terms_with_commit.escrow_id().unwrap();
    let dual = aether_core::escrow::transition::sign_terms_dual(
        &terms_with_commit,
        &pair.payer.signing_key,
        &pair.provider.signing_key,
        &payer_id,
        &provider_id,
    )
    .unwrap();
    aether_core::escrow::transition::create_escrow(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &dual,
        &pair.grant_payer,
        &pair.grant_provider,
        10,
    )
    .unwrap();
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let mut bad_terms = terms_with_commit.clone();
    bad_terms.expected_output_commitment = Some(vec![9, 9, 9]);
    let receipt = sample_receipt(&bad_terms, escrow_id, 1, 30);
    let signed = sign_receipt(&receipt, &pair.provider.signing_key, &provider_id).unwrap();
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidReceipt);
}

#[test]
fn p2_r008_receipt_replay_duplicate_nonce_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    submit_receipt_ready(&mut pair, &terms, escrow_id, 1, 30);
    let receipt = sample_receipt(&terms, escrow_id, 1, 31);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = sign_receipt(&receipt, &pair.provider.signing_key, &provider_id).unwrap();
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        31,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::ReceiptReplay | Error::InvalidEscrowStatus
    ));
    assert_eq!(
        pair.escrows.get(&escrow_id).unwrap().escrow.status,
        EscrowStatus::ReceiptAccepted
    );
}

#[test]
fn p2_r009_receipt_from_non_provider_signer_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let receipt = sample_receipt(&terms, escrow_id, 1, 30);
    let payer_id = pair.payer.identity.derived_agent_id();
    let signed = sign_receipt(&receipt, &pair.payer.signing_key, &payer_id).unwrap();
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        30,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidSignature | Error::InvalidReceipt
    ));
}

#[test]
fn p2_r010_non_canonical_receipt_body_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let receipt = sample_receipt(&terms, escrow_id, 1, 30);
    let body = receipt.encode().unwrap();
    let mut non_canonical = body.clone();
    non_canonical.push(0);
    let provider_id = pair.provider.identity.derived_agent_id();
    let (_, sig) = sign_body(
        &pair.provider.signing_key,
        DOMAIN_TAG,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
        MSG_ESCROW_SUBMIT_RECEIPT,
        &non_canonical,
    );
    let signed = SignedMessage {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        message_type: MSG_ESCROW_SUBMIT_RECEIPT.into(),
        body: non_canonical,
        signer_key_id: provider_id.clone(),
        signature: sig.to_bytes().to_vec(),
        domain_tag: DOMAIN_TAG.into(),
    };
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        30,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::MalformedObject(_) | Error::InvalidReceipt | Error::InvalidSignature
    ));
}

#[test]
fn p2_r011_tampered_body_after_signing_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let receipt = sample_receipt(&terms, escrow_id, 1, 30);
    let provider_id = pair.provider.identity.derived_agent_id();
    let mut signed = sign_receipt(&receipt, &pair.provider.signing_key, &provider_id).unwrap();
    signed.body[5] ^= 0xff;
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidSignature);
}

#[test]
fn p2_r012_receipt_claimed_amount_over_principal_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let mut receipt = sample_receipt(&terms, escrow_id, 1, 30);
    receipt.claimed_amount = terms.principal_amount + 1;
    receipt = receipt.with_computed_id().unwrap();
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = sign_receipt(&receipt, &pair.provider.signing_key, &provider_id).unwrap();
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::InvalidReceipt);
}

#[test]
fn p2_r013_payer_signed_receipt_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let receipt = sample_receipt(&terms, escrow_id, 1, 30);
    let payer_id = pair.payer.identity.derived_agent_id();
    let body = receipt.encode().unwrap();
    let (_, sig) = sign_body(
        &pair.payer.signing_key,
        DOMAIN_TAG,
        PROTOCOL_VERSION,
        SCHEMA_VERSION,
        MSG_ESCROW_SUBMIT_RECEIPT,
        &body,
    );
    let signed = SignedMessage {
        protocol_version: PROTOCOL_VERSION,
        schema_version: SCHEMA_VERSION,
        message_type: MSG_ESCROW_SUBMIT_RECEIPT.into(),
        body,
        signer_key_id: payer_id,
        signature: sig.to_bytes().to_vec(),
        domain_tag: DOMAIN_TAG.into(),
    };
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &pair.grant_provider,
        30,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidSignature | Error::InvalidReceipt
    ));
}

#[test]
fn p2_r014_receipt_without_submit_capability_rejected() {
    let mut pair = setup_escrow_pair();
    let (terms, escrow_id) = create_escrow_ready(&mut pair, 100, 10, 100, 500, 100, 10);
    fund_escrow_ready(&mut pair, escrow_id, 20);
    let receipt = sample_receipt(&terms, escrow_id, 1, 30);
    let provider_id = pair.provider.identity.derived_agent_id();
    let signed = sign_receipt(&receipt, &pair.provider.signing_key, &provider_id).unwrap();
    let bad_cap = direct_capability(&pair.provider, vec!["escrow.fund".into()], Some(10));
    let bad_grant = grant_and_store(&pair.provider, &pair.registry, &mut pair.caps, &bad_cap);
    let err = submit_receipt(
        &mut pair.escrows,
        &pair.registry,
        &pair.caps,
        &escrow_id,
        &signed,
        &bad_grant,
        30,
    )
    .unwrap_err();
    assert_eq!(err, Error::CapabilityDenied);
}
