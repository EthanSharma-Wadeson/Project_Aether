# SECURITY_MODEL.md — Project Aether

## Purpose

Define assets to protect, adversaries, trust assumptions, and mitigations across identity, payments, reputation, privacy, and consensus.

**Status:** Draft — updated after PROTO-0 through PROTO-3 security review and Control Plane Milestone 1 (2026-07-29)

Evidence references:

- [PROTO_0_RESULTS.md](../Project_Phases/phase_1/PROTO_0_RESULTS.md)
- [PROTO_0_SECURITY_REVIEW.md](../Project_Phases/phase_1/PROTO_0_SECURITY_REVIEW.md)
- [PROTO_1_RESULTS.md](../Project_Phases/phase_1/PROTO_1_RESULTS.md)
- [PROTO_NET_0_RESULTS.md](../Project_Phases/phase_2/PROTO_NET_0_RESULTS.md)
- [PROTO_NET_0_SECURITY_REVIEW.md](../Project_Phases/phase_2/PROTO_NET_0_SECURITY_REVIEW.md)
- [PROTO_3_SECURITY_REVIEW.md](../Project_Phases/phase_2/PROTO_3_SECURITY_REVIEW.md)
- [CONTROL_PLANE_MILESTONE_1_RESULTS.md](../Project_Phases/phase_3/CONTROL_PLANE_MILESTONE_1_RESULTS.md)

A passing prototype suite is evidence for specific claims only. It is **not** a global proof that Aether is secure.

---

## Protected Assets

| Asset | Why it matters |
|-------|----------------|
| Agent keys | Control of identity actions and funds |
| Channel state integrity | Theft via stale or forged updates |
| Bonded value | Direct economic loss |
| Attestation validity | Wrongful escrow release / reputation poison |
| Capability envelopes | Unauthorized spend or action scope |
| Private payloads | Model IP, data, strategy |
| Protocol liveness | Agents must settle and dispute |

## Adversary Classes

1. **Malicious agent** — cheats counterparties, farms reputation, griefs disputes  
2. **Compromised agent host** — malware extracts operational keys  
3. **Sybil operator** — many identities to overwhelm discovery/reputation  
4. **Colluding cartel** — wash activity, censorship of rivals (if they run infrastructure)  
5. **Network adversary** — delay, partition, replay  
6. **Malicious principal** — configures agents to externalize harm while shielding funds  
7. **Curious counterparty** — extracts private data from overdisclosure  

---

## Signature Semantics (Invariant)

> **A signature proves control of an authorised key. It does not prove that an agent's output is correct, truthful, or safe.**

A valid signature authenticates:

- key control
- message integrity under a domain-separated signing context

A valid signature does **not** authenticate:

- output correctness
- task quality
- policy compliance by itself
- settlement finality
- evidence availability

Secondary signatures (including payment-like signatures) must not bypass capability checks.

---

## Validated by PROTO-0

The following identity and capability properties were exercised by the PROTO-0 local deterministic prototype and adversarial suite:

| Area | Evidenced behaviour |
|------|---------------------|
| Identity verification | Registration, AgentId derivation, public-key authentication, malformed/invalid rejection |
| Capability verification | Signed `CapabilityGrant` envelope verification before semantic trust |
| Delegation narrowing | Child capabilities may reduce authority, never expand it |
| Revoke / freeze behaviour | Explicit capability revoke, parent-revoke cascade, identity freeze, identity revoke |
| Stale root rejection | Capabilities bound to prior `root_version` fail current-state authorisation after authorised root update |
| Signature verification pipeline | Domain-separated preimage, canonical CBOR body, context mismatch fail-closed |

Also evidenced:

- identity alone cannot authorise a simulated economic action
- capability enforcement occurs before simulated economic authorisation
- expired capabilities cannot authorise new actions

PROTO-0 does **not** convert these into production network guarantees. It validates the smallest local authority primitive under controlled assumptions below.

---

## Validated by PROTO-1 (local simulator)

The following bilateral channel properties were exercised by the PROTO-1 local deterministic simulator and adversarial suite (post-remediation):

| Area | Evidenced behaviour |
|------|---------------------|
| Dual-signed transitions | Open, activate, update, close, and abort require both party signatures over identical canonical bodies |
| Commitment-chain updates | Normal `apply_update` enforces `previous_state_commitment` continuity and `sequence = previous + 1` |
| Commitment-chain disputes | Dispute raise requires a direct chain extension; resolution only admits candidates that chain from the applied baseline |
| Replay / stale rejection | Stale, skipped, and replayed updates rejected on the apply path; orphaned high-sequence dispute evidence ignored |
| PROTO-0 integration on channel ops | Identity status + capability grants required before channel transitions (except see terminal trust boundaries) |
| Soft vs hard finality | `soft_local_agreement` distinct from `hard_settlement_placeholder` (always false in PROTO-1) |
| Terminal operation auth | `finalize_close` requires both parties' `channel.close` capability and elapsed dispute window; `resolve_dispute` requires participant `channel.dispute` capability |
| Store integrity | `ChannelStore` does not expose direct mutable access; state changes go through transition functions |

PROTO-1 **does not** convert these into production network, settlement, or distributed dispute guarantees.

### PROTO-1 terminal-operation trust boundaries

| Operation | Auth required | Notes |
|-----------|---------------|-------|
| `open_channel` / `activate_channel` / `apply_update` / `begin_close` / `abort_open` | Dual signatures + both parties' capability grants + PROTO-0 checks | Normative hot path |
| `raise_dispute` | Dual-signed evidence + raiser `channel.dispute` + direct chain extension | Evidence must be `sequence = applied + 1` |
| `finalize_close` | Both parties' `channel.close` + `now >= dispute_deadline` | Cooperative terminal step after `begin_close` |
| `resolve_dispute` | Resolver `channel.dispute` + participant membership | Selects highest **chained** dual-signed state among candidates |
| `ReceiptV0` | Not independently verified | Audit copy of signatures from accepted transitions; not a standalone proof object |

---

## Validated by PROTO-2 (local simulator)

The following escrow and receipt properties were exercised by the PROTO-2 local deterministic simulator and adversarial suite:

| Area | Evidenced behaviour |
|------|---------------------|
| Dual-signed terms | Escrow creation requires both payer and provider signatures over identical canonical `EscrowTermsV0` |
| Provider-signed receipts | `SettlementReceiptV0` verified independently via PROTO-0 signing pipeline; distinct from PROTO-1 `ReceiptV0` |
| Receipt binding | `escrow_id`, `terms_version`, monotonic `receipt_nonce`; replay rejected |
| PROTO-0 integration | Every economic operation requires capability grant before escrow transition rules |
| Value conservation | Checked arithmetic; no double release/refund; fee within reserved budget |
| Timeout paths | `fund_before`, `receipt_before`, post-receipt dispute window enforced with logical `now` |
| Dispute resolution | Trusted-local deterministic rule: valid receipt → release; else refund |
| PROTO-1 isolation | Separate `EscrowStore`; `ChannelStore` unchanged by escrow operations |
| Economic finality | `EconomicFinalityViewV0.finalized` on terminal status; `hard_settlement_placeholder` always false |

PROTO-2 **does not** convert these into production escrow safety, real-money custody, or blockchain finality.

### PROTO-2 trust boundaries

| Operation | Auth required | Notes |
|-----------|---------------|-------|
| `create_escrow` | Dual-signed terms + both parties' `escrow.create` | No unilateral terms |
| `fund_escrow` | Payer-signed funding + `escrow.fund` + simulated balance | No real-money transfer |
| `submit_receipt` | Provider signature + `escrow.submit_receipt` | Proves bounded claim only, not correctness |
| `release_escrow` | Either party `escrow.release` + elapsed dispute window + bound receipt | Not automatic on receipt alone |
| `refund_escrow` | Payer `escrow.refund` (cooperative or post-expiry) | Provider cannot self-refund |
| `resolve_dispute` | Resolver `escrow.resolve` | Local trusted function, not distributed arbitration |
| `SettlementReceiptV0` | Independently verified provider signature | Does not prove output truth or usefulness |

---

## Validated by PROTO-NET-0 (local transport simulation)

The following authenticated-messaging properties were exercised by the PROTO-NET-0 local deterministic simulator and adversarial suite (see [PROTO_NET_0_SECURITY_REVIEW.md](../Project_Phases/phase_2/PROTO_NET_0_SECURITY_REVIEW.md)):

| Area | Evidenced behaviour |
|------|---------------------|
| Agent authentication | Hello and envelope signatures verified under PROTO-0 operational keys; unknown / inactive identities rejected |
| Dual-party session establishment | Both `net.hello` and `net.hello.accept` required; unilateral establishment rejected |
| Deterministic session IDs | Same `session_id` derived independently on both agent stores |
| Envelope authenticity | Tampered fields and forged signatures fail closed |
| Replay resistance (local) | Hello nonce tracking + per-session `message_id` rejection |
| Receiver binding | Wrong `expected_receiver` rejected |
| Payload commitment | Delivered bytes must match signed `payload_commitment` |
| Carrier boundary | Network does not call `authorise_action` and does not mutate channel/escrow state |
| Signature ≠ correctness | Envelope proves sender key control only; not payload truth or economic authorisation |

PROTO-NET-0 **does not** evidence: real network security, encrypted transport, distributed discovery, partition tolerance, or key-compromise recovery.

### PROTO-NET-0 trust boundaries

| Operation | Auth required | Notes |
|-----------|---------------|-------|
| Directory register | Active PROTO-0 identity | Local map only; no Sybil cost |
| `initiate_hello` / `accept_hello` / `complete_hello` | Signed hello(s) + Active identities | Dual authentication for Established |
| `deliver_envelope` | Sender signature + Active identities + Established session | Carrier only — not economic authorisation |
| Envelope payload | Integrity via commitment | Consuming protocol layers must still enforce PROTO-0/1/2 rules |

---

## Validated by PROTO-4 (local settlement binding)

The following settlement-binding properties were exercised by the PROTO-4 local deterministic prototype and adversarial suite (see [PROTO_4_RESULTS.md](../Project_Phases/phase_2/PROTO_4_RESULTS.md), [PROTO_4_SECURITY_REVIEW.md](../Project_Phases/phase_2/PROTO_4_SECURITY_REVIEW.md) — post-remediation **APPROVE WITH DOCUMENTED LIMITATIONS**):

| Area | Evidenced behaviour |
|------|---------------------|
| Authority split | Adapter evidence cannot overwrite PROTO-2 escrow status or conservation |
| Capability gate | `settlement.bind` / `settle` / `query` / `cancel` required before mutations and adapter side effects |
| Soft ≠ hard finality | `EconomicFinalityViewV0.finalized` distinct from `hard_settlement_placeholder` |
| Hard flag on intended path | `finalize_settlement` requires Confirmed binding, Confirmed caller report, destination check, **and** fresh adapter `Confirmed` query |
| Fake confirmation | Missing/wrong mock proof token fails closed; hard flag remains false |
| Duplicate / replay | Correlation idempotency; cross-binding external-ref replay rejected |
| Conflict handling | Post-confirm adapter reversal + query → `DisputedExternal` + hard flag cleared |
| NET isolation | Transport session/envelope alone does not authorise settlement |
| Hard API exposure closed | `promote_verified_hard_settlement` is crate-private; no public hard-promotion export (P4-SEC-R01) |
| Stale finalize blocked | Cached Confirmed report after adapter reverse cannot finalize (P4-SEC-R02) |

### Validated by PROTO-4 Remediation

- Hard finality cannot be promoted without the validated settlement finalization path
- Stale adapter evidence cannot create hard finality
- External settlement evidence remains separate from Aether protocol truth
- Adapter responses remain evidence, not authority

PROTO-4 **does not** evidence: production banking security, legal settlement finality, or honesty of a compromised live adapter.

### PROTO-4 trust boundaries

| Operation | Auth required | Notes |
|-----------|---------------|-------|
| Account bind | `settlement.bind` + Active identity | External account ref is opaque; not KYC proof |
| Settle request / submit / finalize | `settlement.settle` + Active identity | Escrow must be terminal with matching outcome; finalize re-queries adapter |
| Query | `settlement.query` | May refresh settlement status; must not mutate escrow economics |
| Cancel | `settlement.cancel` | Only before Confirmed |
| Adapter response | Structured evidence + proof token for Confirmed | Provider is evidence source only |

### Hard finality limitations

- `hard_settlement_placeholder` is a **protocol view bit**, not legal/payment finality
- Promotion is only via crate-private `promote_verified_hard_settlement` from `finalize_settlement`
- Finalize requires fresh live adapter `Confirmed` attestation in addition to a valid caller report
- Adapter confirmation ≠ banking finality; signature ≠ correctness
- Still assumes trusted local mock adapter and in-process host for store handles

---

## Not Yet Validated

The following remain **unvalidated** and must not be treated as evidenced:

- payment security on real settlement backends (PROTO-4 proves binding to a **mock** ledger only)
- settlement finality and on-chain enforcement
- distributed revocation propagation
- watchtowers, offline safety, and slash economics
- reputation systems and anti-wash scoring (PROTO-3 **implemented** — 34 tests pass — anti-wash heuristics remain OPEN)
- key rotation hierarchy (root / recovery / operational / channel keys)
- privacy selective-disclosure flows
- multi-party authority and production Merkle permission structures
- network adversary resistance (delay, partition, replay across peers)
- production dispute adjudication beyond local in-memory resolution

Partially exercised locally but **not** production-validated:

- payment channels and channel-state machines (PROTO-1 simulator only)
- disputes and dispute windows (local logical-time enforcement only; no distributed slash outcomes)
- authenticated agent messaging / sessions (PROTO-NET-0 local simulator only; no real sockets or encryption)

---

## Trust Assumptions (Draft)

### General

- Cryptographic primitives behave as specified  
- Settlement security inherited from selected backend; layered fault model per [CONSENSUS_AND_SETTLEMENT.md](CONSENSUS_AND_SETTLEMENT.md) §4–5  
- For BFT verifier committees: fewer than one-third Byzantine (`n ≥ 3f + 1`) where applicable  
- Agents verify capabilities and proofs; do not trust free-text claims  
- Off-chain channel safety depends on timely dispute participation or watchtowers  
- Evidence availability is a separate obligation from integrity commitments — see [CONSENSUS_AND_SETTLEMENT.md](CONSENSUS_AND_SETTLEMENT.md) §10–11  
- TEEs / ZK systems, if used, carry their own trust/complexity assumptions explicitly  

### PROTO-0 trust assumptions (explicit)

These assumptions were required by the PROTO-0 verifier and remain in force for v0 identity/capability validation until replaced by stronger mechanisms:

1. **Coherent local state** — the verifier’s identity registry, capability store, and revocation map are coherent for the evaluation; missing revoke/root state must fail closed rather than invent authority.
2. **Trusted logical time input** — expiry uses a caller-supplied logical `now`; PROTO-0 does not secure wall-clock time.
3. **Current-state validation** — normal authorisation evaluates against the active permission root / `root_version`; historical authority validation is a separate mode not yet defined.
4. **Subject binding** — the acting agent must be the capability `subject` for action authorisation.
5. **Canonical CBOR rules** — schema-locked field order is protocol-critical; non-canonical encodings are rejected.
6. **Single operational key (v0)** — one operational key authorises identity and capability grants until hierarchy/rotation is implemented and validated.
7. **Local revocation only (v0)** — revocation is not yet distributed; verifiers that lack revoke records may diverge unless state is shared.

---

## Security Properties

| Property | Intent |
|----------|--------|
| Authentication | Messages bound to authorized keys |
| Authorization | Capabilities constrain actions before economic effects |
| Integrity | Settlements match authorized latest states |
| Accountability | Fraud attributable and slashable |
| Availability | Honest parties can eventually finalize |
| Privacy | Selective disclosure; see PRIVACY_MODEL |
| Non-repudiation | Signed updates and attestations bind parties |

---

## Capability / Identity Authorization Rules

Normative verification order for capability-gated actions:

1. **Verify signature first** — reconstruct canonical body bytes, verify the enclosing `SignedMessage`, and reject signing-context mismatch.
2. **Validate semantics second** — only after authentication, evaluate identity status, permission-root binding, capability constraints, and parent chain.
3. **Fail closed** on:
   - revoked capability (including revoked parent)
   - frozen or revoked identity
   - stale permission root / root-version mismatch
   - malformed or non-canonical objects
4. **Children cannot expand authority** — delegated capabilities must remain a subset of parent scope, limits, expiry, counterparties, rate limits, and delegation depth.

Capability checks MUST occur before an externally visible economic action is authorised. Payment signatures alone are insufficient.

---

## Key Risk Mitigations

### Validated or partially present at v0 / PROTO-0 / PROTO-1

- Capability scopes with low blast radius  
- Identity freeze / revoke emergency stop  
- Permission-root version invalidation for stale grants  
- Fail-closed signature and context checks  
- Local bilateral channel dual-signature enforcement (PROTO-1)  
- Commitment-chain dispute filtering (PROTO-1 remediation)  

### Later-stage mitigations (not yet validated)

- Hierarchical keys + rapid operational-key rotation  
- Hardware / enclave custody where principals require it  
- Distinct root/recovery-holder freeze paths  
- Avoid single long-lived hot root key on agent hosts  
- Distributed revocation propagation  

---

## Payment & Dispute Security

- Monotonic sequence numbers; reject stale state on the apply path
- Dispute evidence must form a valid commitment chain from the applied baseline (PROTO-1 remediation)
- Dispute windows enforced on cooperative `finalize_close` via caller-supplied logical `now`
- Watchtowers or principal backups for offline agents
- Slash for submitting revoked/fraudulent state
- Capability checks prevent “both parties agree to violate principal limits”

**Status:** local simulator behaviour evidenced by PROTO-1; distributed/production dispute safety remains design intent only.

## Reputation Security

- Evidence rooted in economically consequential events  
- Anti-wash graph and stake weighting  
- Cost to reset identity trust  
- Careful handling of ambiguous faults  

**Status:** PROTO-3 implemented — 34 acceptance tests pass (2026-07-29). Security review **APPROVE WITH DOCUMENTED LIMITATIONS**.

Implemented: `ReputationEventV0`, `EvidenceRefV0`, `AgentMetricsV0`, `ReputationStore` (append-only + materialised cache), `ReputationIndexer` (read-only derivation from PROTO-2/4 artifacts), `ReputationQueryV0/ResultV0`, enterprise `LocalOnly` mode.

Validated by implementation:
- Authority boundary preserved — indexer has no write path to identity, capability, escrow, or settlement stores
- Evidence-first derivation — every event cites verifiable `EvidenceRefV0` with SHA-256 commitments
- Deterministic recomputation — `full_recompute()` matches materialised cache (P3-R002)
- Idempotent ingestion — duplicate events rejected by `event_id` (P3-T015)
- Settlement credit requires hard finality (P3-DEC-008)
- Enterprise local-only mode blocks public queries (P3-E01)
- Revoked identity stops positive accrual (P3-T010)

P3-SEC-001 resolved: `capability.denied` excluded from v0 event set (no signed evidence source).
P3-SEC-002 documented: `event_log_root` proves consistency, not completeness — `CompletenessQualification::SelfAttested` exposed.
P3-SEC-003 documented: commitment-only mode protects payload content, not relationship graph.

**OPEN:** Sybil resistance (no bonds), indexer honesty enforcement, collusion with real economic work, counterparty graph privacy, work quality verification. See [PROTO_3_SECURITY_REVIEW.md](../Project_Phases/phase_2/PROTO_3_SECURITY_REVIEW.md).

## Verification Security

- Fail closed on unknown proof types  
- Pin proof system versions  
- Separate “attested by TEE X” from “mathematically proven” in APIs so agents do not confuse assurance levels  
- Separate “signature verified” from “output correct / safe”

## Consensus / Infrastructure Security

- Minimize MEV relevance of agent micro-updates by keeping them off-chain  
- Censorship resistance for dispute submissions especially  
- DA guarantees for evidence blobs needed in disputes  

## Secure Development Practices (Project Norms)

- Threat model before feature design  
- Prefer simplicity on the hot path  
- Explicit status of cryptographic assumptions in release notes  
- No “AI safety theater” substituting for protocol security  
- Prototype tests evidence specific claims; they do not replace threat modelling or later formal analysis  

## Incident Response (Protocol Level)

- Identity freeze / mass revoke procedures  
- Channel emergency close  
- Parameter circuit breakers for fee/bond misconfigurations  
- Disclosure process for critical vulnerabilities  

---

## Open Questions

### Carried forward

- Watchtower economics and default availability assumptions  
- Formal verification targets for channel and capability logic  
- Quantum agility timeline  

### Raised / sharpened by PROTO-0

- How is **revocation distributed** so honest verifiers cannot miss a revoke?
- What is the authoritative **clock / logical-time source** for expiry?
- When is **historical authority validation** (non-current root) allowed, and how is it bounded?
- When does **operational-key rotation / key hierarchy** become mandatory versus optional for v0→v1?
- How should subject-binding evolve for delegated presentation in payment flows?

---

## Control Plane (Milestone 1 — Read-Only Observatory)

**Status:** Application layer implemented; Milestone 1 security remediation complete (2026-07-29). Awaiting security review approval before Milestone 2.

Evidence: [CONTROL_PLANE_MILESTONE_1_RESULTS.md](../Project_Phases/phase_3/CONTROL_PLANE_MILESTONE_1_RESULTS.md), [CONTROL_PLANE_SECURITY_MODEL.md](../Project_Phases/phase_3/CONTROL_PLANE_SECURITY_MODEL.md)

| Area | Evidenced behaviour |
|------|---------------------|
| Authority boundary | No PROTO-0 write routes; no escrow/settlement/reputation mutation paths |
| Authentication | JWT access tokens (`iss`/`aud`/`typ`), bcrypt passwords, HttpOnly refresh cookies |
| Login abuse protection | Progressive backoff by IP + username; generic `invalid credentials` errors |
| Refresh lifecycle | Rotation on every refresh; reuse detection revokes all operator sessions |
| Browser headers | CSP, X-Content-Type-Options, X-Frame-Options, Referrer-Policy |
| Read isolation | Protocol adapters expose getters only; dashboard matches protocol store state |
| Audit logging | Login, refresh, logout, and observation actions recorded append-only |
| Regression | 311 protocol tests unchanged; 18 Control Plane integration tests pass |

**Remaining before Milestone 2:**

- CSRF protection (required for write routes)
- MFA / per-operator PROTO-0 key delegation
- TLS for non-localhost deployment
- DB file permissions hardening (deployment checklist)

A compromised Control Plane remains an **application-layer** incident. Protocol cryptographic guarantees (capability chain, escrow validity, settlement proofs, reputation event_id binding) are enforced by `aether-core`, not by the dashboard.

---

## Related

- [CONSENSUS_AND_SETTLEMENT.md](CONSENSUS_AND_SETTLEMENT.md)
- [PRIVACY_MODEL.md](PRIVACY_MODEL.md)
- [CONSENSUS_DESIGN.md](CONSENSUS_DESIGN.md)
- [PROTOCOL_DESIGN.md](PROTOCOL_DESIGN.md)
- `research/cryptography.md`
- [PROTO_0_SECURITY_REVIEW.md](../Project_Phases/phase_1/PROTO_0_SECURITY_REVIEW.md)
