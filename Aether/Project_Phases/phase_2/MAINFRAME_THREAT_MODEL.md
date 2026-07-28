# MAINFRAME_THREAT_MODEL.md — Phase 2 Expanded Threat Model

## Status

**Draft — extends [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md) for mainframe architecture**

Phase 1 validated local rule correctness. This document models adversaries and attacks for **distributed, economic-scale** deployment.

Every claim is tagged:

| Tag | Meaning |
|-----|---------|
| **VALIDATED** | Demonstrated by PROTO-0/1/2 tests |
| **DESIGNED** | Mitigation designed but not implemented |
| **ASSUMED** | Trust assumption; no prototype evidence |
| **OPEN** | No mitigation designed yet |

---

## 1. Threat Model Scope

### In scope

- Malicious and compromised agents
- Network adversaries (delay, replay, partition)
- Economic attacks (double spend, griefing, fee manipulation)
- Reputation manipulation and Sybil attacks
- Settlement backend failures
- Schema/version downgrade attacks

### Out of scope (Phase 2 design)

- Physical security of agent hosts
- Nation-state traffic analysis
- Quantum cryptanalysis
- Legal enforcement of protocol outcomes

---

## 2. Adversary Classes

| Adversary | Capability | Primary goal |
|-----------|------------|--------------|
| **Malicious agent** | Valid or stolen keys; follows protocol when profitable | Extract value, avoid payment, poison reputation |
| **Compromised agent host** | Read/sign with operational keys | Exfiltrate capabilities, forge messages |
| **Sybil operator** | Many identities, low per-identity cost | Flood discovery, farm reputation, dilute trust |
| **Dishonest provider** | Provider role in escrow/channel | Claim completion without work; release funds |
| **Dishonest client** | Payer role | Withhold payment after work; frivolous disputes |
| **Network adversary** | Delay, drop, replay, partition messages | Desynchronise state, extend dispute windows |
| **Malicious relay/hub** | Operates discovery or message relay | Censor, reorder, metadata leak |
| **Settlement backend adversary** | Compromised or malicious backend | False finality, censorship, fund theft |
| **Reputation manipulator** | Colluding agents + wash activity | Inflate scores without real work |
| **Malicious principal** | Configures agent policies | Externalise harm; shield funds from slash |

---

## 3. Attack Catalogue

### 3.1 Identity and capability attacks

| Attack | Description | Tag | Mitigation |
|--------|-------------|-----|------------|
| Identity-only authorisation | Act without capability grant | **VALIDATED** reject | PROTO-0 `authorise_action` |
| Forged capability grant | Sign grant without issuer key | **VALIDATED** reject | Grant signature verification |
| Escalated delegation | Child exceeds parent scope | **VALIDATED** reject | Delegation depth + narrowing |
| Stale grant after revoke | Use revoked capability | **VALIDATED** local | Revoke check in authorisation |
| Stale grant across network | Revoke not propagated to peer | **OPEN** | DEC-P2-008 revocation sync |
| Stale grant after root update | Grant bound to old permission root | **VALIDATED** reject | Root version check |
| Sybil registration flood | Many cheap identities | **OPEN** | Bonds + rate limits (DEC-P2-004) |
| Operational key theft | Attacker signs as agent | **ASSUMED** detect | Key rotation (DEC-P2-009); not evidenced |
| Root key compromise | Full identity takeover | **ASSUMED** | Recovery hierarchy (not built) |

### 3.2 Channel attacks (PROTO-1)

| Attack | Description | Tag | Mitigation |
|--------|-------------|-----|------------|
| Unilateral state update | One party signs alone | **VALIDATED** reject | Dual-signature requirement |
| Replay stale update | Re-submit old sequence | **VALIDATED** reject | Sequence monotonicity |
| Skipped sequence | Jump sequence without chain | **VALIDATED** reject | Sequence + commitment checks |
| Orphan dispute evidence | High-seq state not chained | **VALIDATED** reject | Chain validation (remediation) |
| Dispute without capability | Raise/resolve without grant | **VALIDATED** reject | Terminal auth |
| Premature finalize | Close before dispute window | **VALIDATED** reject | `DisputeWindowOpen` |
| Network desync | Peers disagree on channel state | **OPEN** | Networked PROTO-1 sync |
| Equivocation (networked) | Different states to different peers | **DESIGNED** | Commitment reveal + dispute |

### 3.3 Escrow and receipt attacks (PROTO-2)

| Attack | Description | Tag | Mitigation |
|--------|-------------|-----|------------|
| Forged receipt | Wrong-key signature | **VALIDATED** reject | Provider sig verification |
| Receipt replay | Reuse nonce | **VALIDATED** reject | Monotonic `receipt_nonce` |
| Wrong escrow binding | Receipt for different escrow | **VALIDATED** reject | Field binding checks |
| Payer-signed receipt | Wrong signer model | **VALIDATED** reject | Provider-only model |
| Release without capability | Identity-only release | **VALIDATED** reject | `escrow.release` grant |
| Double release / refund | Terminal op twice | **VALIDATED** reject | Status guards |
| Premature release | Before dispute window | **VALIDATED** reject | `DisputeWindowOpen` |
| False completion claim | Provider signs incorrect result | **VALIDATED** accept sig | Signature ≠ truth invariant |
| Colluding false receipt | Both parties agree to false claim | **ASSUMED** | Out of scope for crypto layer |
| Timeout manipulation | Adversarial clock | **ASSUMED** local | Trusted logical time in Phase 1 |
| Fee budget exhaustion | Consume more than reserved | **VALIDATED** reject | Fee ledger checks |

### 3.4 Network attacks (future)

| Attack | Description | Tag | Mitigation |
|--------|-------------|-----|------------|
| Message replay across sessions | Re-submit captured message | **OPEN** | Session nonce + expiry (MP-02) |
| Message delay | Hold message until timeout edge | **OPEN** | Deadline margins; clock sync policy |
| Partition attack | Split peers during dispute window | **OPEN** | Watchtower / principal mirror (Phase 2+) |
| Downgrade attack | Force old protocol version | **OPEN** | `ProtocolHelloV0` version floor |
| Metadata leak via relay | Relay observes traffic patterns | **OPEN** | E2E encryption in session layer |
| Directory poisoning | Register fake agent entries | **OPEN** | Bonded registration (DEC-P2-001) |

### 3.5 Economic attacks (future)

| Attack | Description | Tag | Mitigation |
|--------|-------------|-----|------------|
| Value creation | Mint funds in simulator | **VALIDATED** reject | Conservation checks |
| Double settlement | Settle same escrow twice on backend | **OPEN** | Idempotent settlement adapter |
| Griefing dispute | Raise disputes to lock funds | **DESIGNED** | Dispute bonds |
| Fee manipulation | Tamper fee quote after create | **VALIDATED** reject | Quote bound at create |
| Wash trading | Fake escrows for reputation | **OPEN** | Bonded evidence + PROTO-3 adversarial |
| Bond grinding | Minimal bond, maximal harm | **OPEN** | Task-class bond schedules |

### 3.6 Reputation attacks (future)

| Attack | Description | Tag | Mitigation |
|--------|-------------|-----|------------|
| Self-dealing reputation | Agents trade with own Sybils | **OPEN** | Graph analysis + bond cost |
| Score without evidence | Opaque platform rating | **DESIGNED** | Evidence refs mandatory |
| Indexer lie | Indexer returns false score | **OPEN** | Multi-indexer + evidence verify |
| Historical rewrite | Alter past events | **DESIGNED** | Signed immutable events |

---

## 4. Validated vs Assumed — Summary Matrix

| Security property | Phase 1 evidence | Distributed evidence |
|-------------------|------------------|----------------------|
| Signature verification | VALIDATED | ASSUMED (same crypto) |
| Capability before economics | VALIDATED | OPEN (needs revoke sync) |
| Dual-signed bilateral state | VALIDATED | OPEN (needs sync) |
| Escrow conservation | VALIDATED | OPEN (needs real settlement) |
| Receipt replay resistance | VALIDATED | OPEN (needs network nonce) |
| Dispute determinism | VALIDATED local | OPEN (needs evidence availability) |
| Sybil resistance | Not tested | OPEN |
| Reputation integrity | Not built | OPEN |
| Settlement finality | Not built | OPEN |
| Network partition safety | Not built | OPEN |

---

## 5. Trust Boundaries

```text
┌─────────────────────────────────────────┐
│  TRUSTED (harness / design assumption)   │
│  · Logical time (Phase 1)               │
│  · Local process memory                 │
│  · Dispute resolver (PROTO-2 local)       │
└─────────────────────────────────────────┘
          ↓ Phase 2 must replace with
┌─────────────────────────────────────────┐
│  VERIFIED (cryptographic / protocol)   │
│  · SignedMessage authenticity           │
│  · Capability grant validity            │
│  · State transition rules               │
│  · Receipt binding                      │
└─────────────────────────────────────────┘
          ↓ Phase 2 must add
┌─────────────────────────────────────────┐
│  ECONOMIC (backend / bond dependent)     │
│  · Settlement inclusion                 │
│  · Bond slash execution                 │
│  · Revocation registry freshness        │
└─────────────────────────────────────────┘
          ↓ Application layer
┌─────────────────────────────────────────┐
│  OPERATIONAL (human / org policy)        │
│  · Principal liability                  │
│  · Legal dispute resolution             │
│  · Custodian solvency                   │
└─────────────────────────────────────────┘
```

---

## 6. Security Invariants (Carry Forward)

Phase 2 must preserve all Phase 0 invariants. Additional mainframe invariants:

| # | Invariant | Status |
|---|-----------|--------|
| M1 | Economic messages always require full signed envelope | DESIGNED |
| M2 | Capability check precedes economic authorisation on every peer | DESIGNED |
| M3 | Revocation freshness bound is explicit and enforced | OPEN |
| M4 | Settlement finality stages are machine-readable | DESIGNED |
| M5 | Reputation scores cite verifiable evidence refs | DESIGNED |
| M6 | Discovery entries require anti-Sybil cost | OPEN |
| M7 | Protocol version downgrade is rejected | OPEN |
| M8 | Backend failure does not corrupt identity/capability state | DESIGNED |

---

## 7. Adversarial Test Requirements (Future)

Before networked pilot exits, require adversarial suites for:

| Suite | Target |
|-------|--------|
| NET-A01–A10 | Replay, delay, partition, downgrade |
| SET-A01–A10 | Double settlement, false finality, backend rollback |
| REP-A01–A10 | Sybil farm, wash trade, indexer lie |
| REV-A01–A05 | Stale grant across peers after revoke |

---

## 8. Relationship to SECURITY_MODEL.md

This document **extends** but does not replace [SECURITY_MODEL.md](../../Aether_docs/SECURITY_MODEL.md).

- SECURITY_MODEL.md: project-wide assets, adversaries, Phase 1 validated scope
- MAINFRAME_THREAT_MODEL.md: distributed deployment threats and Phase 2+ mitigations

On conflict, SECURITY_MODEL.md security invariants take precedence.

---

## Freeze Statement

> This threat model is for architecture review. It does not claim distributed security is evidenced until corresponding adversarial suites pass.
