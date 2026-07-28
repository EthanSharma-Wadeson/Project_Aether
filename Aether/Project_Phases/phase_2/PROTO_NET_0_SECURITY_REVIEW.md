# PROTO-NET-0 Security Review vs SECURITY_MODEL.md

## Status

**Phase 2 — complete (documentation alignment applied; no implementation changes required)**

Sources reviewed:

- `Aether_docs/SECURITY_MODEL.md`
- `Project_Phases/phase_2/PROTO_NET_0_RESULTS.md`
- `Project_Phases/phase_2/MAINFRAME_THREAT_MODEL.md`
- Network module under `Aether/core/src/network/` (read-only)
- Adversarial suite `network_adversarial_tests.rs` (NET-A01–A14)

Date: 2026-07-28

**Outcome:** **APPROVE** — PROTO-NET-0 is ready as a foundation for the next prototype after this review is accepted. Do not start PROTO-4, networked PROTO-1/2, or reputation until that acceptance is explicit.

---

## Verdict

PROTO-NET-0 **supports** the authenticated-transport portions of the security model within its declared **local simulation** scope.

It correctly acts as a **carrier of authenticated messages**, not an authority layer:

- Reuses PROTO-0 identity registry and Active status checks
- Does **not** call `authorise_action` or bypass PROTO-0 capability gates
- Does **not** mutate or weaken PROTO-1 channel or PROTO-2 escrow state
- Preserves the signature invariant: **key control ≠ correctness / economic authorisation**

No blocking security contradiction requires remediation before review approval.

Passing tests are **not** evidence of production network security, encrypted transport, or Byzantine resilience.

---

## 1. SECURITY_MODEL Alignment

### Claims now evidenced by tests (local harness)

| SECURITY_MODEL / Phase 0 theme | PROTO-NET-0 evidence | Strength |
|--------------------------------|----------------------|----------|
| Signature proves key control | Ed25519 via DEC-004B `SignedMessage`; NET-A01, A10, A12 | Strong locally |
| Signature ≠ correctness | Envelope commits to `payload_commitment` only; payload semantics unchecked by transport | Strong (by design) |
| Authentication of agents | Hello/envelope verify under registry operational key; NET-A07, A09 | Strong locally |
| Identity Active requirement | `require_active_identity` on hello, accept, deliver | Strong locally |
| Fail-closed validation order | signature → identity → session → semantics → mutate | Strong locally |
| Replay resistance (local) | Hello nonce + envelope `message_id` sets; NET-A03, A04, A11 | Strong locally |
| Store integrity | `SessionStore` / directory: no public `get_mut`; transitions only | Strong locally |

### Claims that remain assumptions

| Claim | Why still assumed |
|-------|-------------------|
| Network adversary resistance | No real sockets; no delay/partition/MITM suite |
| Confidentiality of payloads | No encryption / TLS / Noise |
| Distributed discovery integrity | Directory is local curated map; no Sybil cost |
| Revocation propagation | Active check is local registry only |
| Key rotation mid-session | Not implemented |
| Wall-clock / clock sync | Logical `now` harness-trusted |
| Capability authorisation of transport | Intentionally **not** applied at NET layer (carrier) |

### Documentation vs implementation tensions (non-blocking)

| Item | Docs / design | Implementation | Assessment |
|------|---------------|----------------|------------|
| Session statuses | Design listed `HelloAccepted` as explicit step | Responder: accept → `Established`; initiator: `HelloSent` → `Established` | Residual design debt; dual authentication still required; not a bypass |
| Envelope `timestamp` | Present on wire model | Signed for integrity; **not** used for freshness (replay via `message_id`) | Acceptable for local sim; future transport should define skew policy |
| `SECURITY_MODEL.md` | Had no PROTO-NET-0 section | Behaviour matches network-carrier intent | Doc update applied with this review |

No direct contradiction with Phase 0 signature or authority invariants.

---

## 2. Authentication Review

### Can an attacker impersonate another `AgentId`?

**No (without that agent's operational key).**

`verify_hello_signature` / `verify_envelope_signature`:

1. Decode claimed `agent_id` / `sender_agent_id` from body  
2. Look up PROTO-0 registry entry  
3. Verify Ed25519 signature under **that** entry's operational public key  
4. Require `signer_key_id` matches claimed agent id  

Forging a hello that claims Agent B but is signed by Agent A fails verification under B's key (NET-A07, A09, A12).

### Can an attacker create a valid-looking hello?

Structurally yes; cryptographically no without a registered Active key. Unknown agents rejected (`UnknownNetworkAgent`).

### Can PROTO-0 identity checks be bypassed?

**No on the hot path.** Directory registration and all handshake/delivery paths call `require_active_identity` or equivalent Active checks. Network code does not construct alternate identity authority.

### Ed25519 verification order

Aligns with SECURITY_MODEL / DEC-004B:

```text
canonical body → domain-separated preimage → SHA-256 → Ed25519 verify
→ Active identity → session / semantics → mutate
```

---

## 3. Session Security Review

### Establishment

| Check | Result |
|-------|--------|
| `session_id` deterministic? | **Yes** — `SHA-256(canonical_cbor(ordered agents + both nonces + protocol_version))`; lifecycle test confirms both stores match |
| Both parties authenticated? | **Yes** — initiator signs `net.hello`; responder signs `net.hello.accept`; initiator verifies accept before Established |
| Unilateral establishment? | **Rejected** — responder needs valid peer hello + local accept; initiator needs pending HelloSent + peer accept |

### Lifecycle

| Check | Result | Evidence |
|-------|--------|----------|
| Invalid transitions | Envelope requires `Established`; pending-only cannot deliver | NET / lifecycle tests |
| Expired sessions | `now > expires_at` → `SessionExpired` | NET-A06 |
| Closed session reuse | Closed → envelopes rejected | NET-A13 |
| Duplicate sessions | Same `session_id` insert rejected; pending nonce reuse rejected | `SessionAlreadyExists` / `MessageReplay` |
| Replayed handshakes | Global/local nonce tracking | NET-A11 |

**Session hijacking (local):** Possession of `session_id` alone is insufficient; delivery requires valid sender signature and session participant binding. Real network hijacking via host compromise remains **OPEN**.

---

## 4. Envelope Security Review

### Field binding

| Field | Enforced? |
|-------|-----------|
| Sender binding | Signature under sender key + session participant check |
| Receiver binding | `expected_receiver` + session peer/local binding (NET-A02) |
| `message_id` uniqueness | Per-session `seen_message_ids` (NET-A03/A04) |
| Payload commitment | `SHA-256(payload)` must match (NET-A08) |
| Timestamp | Covered by signature; **not** freshness-checked |
| Signature coverage | All envelope body fields in canonical CBOR (tamper → NET-A10) |

### Signature semantics (confirmed intact)

Signature proves:

- sender operational key control  
- integrity of envelope fields under DEC-004B  

Signature does **not** prove:

- payload correctness or usefulness  
- economic authorisation (`escrow.*` / `channel.*`)  
- business validity of `message_type`  

Transport must not be treated as an authorisation gate. Economic messages carried as payload remain subject to PROTO-0/1/2 when those layers consume them.

---

## 5. Replay and Ordering Review

### Prevented locally

| Attack | Mitigation |
|--------|------------|
| Handshake hello replay | Per-store global nonce set |
| Envelope re-delivery | `message_id` set per session |
| Duplicate message IDs | Same |
| Closed/expired reuse | Status + TTL checks |

### Not guaranteed / requires future distributed transport

| Concern | Status |
|---------|--------|
| Cross-process nonce registry sync | OPEN |
| Ordering / gap detection (beyond uniqueness) | OPEN — no sequence number on envelopes |
| Delay until near TTL then deliver | OPEN — no Byzantine clock adversary |
| Partition during handshake | OPEN |
| Wire-level interception / confidentiality | OPEN |

---

## 6. PROTO-0 / PROTO-1 / PROTO-2 Boundary Review

| Layer | Bypass risk? | Finding |
|-------|--------------|---------|
| PROTO-0 identity | No | Registry + Active required |
| PROTO-0 capability | N/A at transport | Network does **not** call `authorise_action` — correct for carrier; economic ops must still gate later |
| PROTO-1 channels | No | No imports/mutations of `channel` |
| PROTO-2 escrow | No | No imports/mutations of `escrow` |

**Boundary conclusion:** PROTO-NET-0 is an authenticated messaging foundation. It must not be described as granting spend, channel, or escrow authority.

---

## 7. Threat Classification

| Threat | Status | Evidence |
|--------|--------|----------|
| Fake identity | **VALIDATED** (local) | Key-bound verify; NET-A07, A09 |
| Message forgery | **VALIDATED** (local) | NET-A01, A10, A12 |
| Replay | **VALIDATED** (local) | NET-A03, A04, A11 |
| Session hijacking | **PARTIALLY VALIDATED** | Needs peer key + store binding; host compromise / network MITM OPEN |
| Unknown agent | **VALIDATED** (local) | NET-A07; directory register requires Active |
| Version mismatch | **VALIDATED** (local) | NET-A05; feature intersection empty rejected |
| Malicious payload | **PARTIALLY VALIDATED** | Commitment mismatch rejected; content correctness intentionally out of scope |
| Network partition | **OPEN** | No distributed harness |
| Transport interception | **OPEN** | No encryption |
| Key compromise | **OPEN** | No rotation / recovery path in NET-0 |

---

## 8. Remaining Risks

Explicit residual risks (must not be overclaimed):

1. **No real network transport** — in-process message passing only  
2. **No encryption layer** — authenticity without confidentiality  
3. **No NAT / routing model** — endpoints are opaque simulation strings  
4. **No distributed discovery** — local directory; no DHT; no anti-Sybil registration bond  
5. **No Byzantine network testing** — delay, drop, reorder, partition untested  
6. **No key rotation** in the transport layer  
7. **No production revocation propagation** — Active check is local registry only  
8. **Logical timestamps remain trusted** — harness injects `now` / TTL  
9. **Envelope timestamp unused for freshness** — rely on `message_id` replay sets  
10. **Responder skips explicit `HelloAccepted` status** — design-doc mismatch; dual-auth still holds  

---

## 9. Recommendation

### APPROVE

**PROTO-NET-0 is ready as a foundation for the next prototype.**

Conditions of approval:

1. Treat all VALIDATED claims as **local-simulator evidence only**  
2. Do not claim production network or confidentiality security  
3. Next prototype (PROTO-4 **or** networked PROTO-1) must preserve: signature ≠ correctness; capability before economic auth; NET as carrier only  
4. Before public/multi-host deployment: address encryption, revocation sync, and clock/freshness policy  

### Not authorised by this review

- PROTO-4 settlement implementation  
- Networked channel / escrow integration  
- Reputation (PROTO-3)  
- Marketplace / Control Plane  

Those require a separate explicit go-ahead after this review is accepted.

---

## Follow-up Applied

- `SECURITY_MODEL.md` updated with **Validated by PROTO-NET-0** section  
- No Rust remediation required  

---

## Freeze Statement

> PROTO-NET-0 security review is complete. Implementation may proceed to the next Phase 2 prototype only after explicit acceptance of this APPROVE verdict.
