# PROTO-NET-0 Results — Phase 2

## Status

**PROTO-NET-0 implementation complete — awaiting security review before further Phase 2 prototypes**

Command:

```bash
cd Aether/core && cargo fmt --check && cargo test && cargo clippy --all-targets -- -D warnings
```

Date: 2026-07-28

Do **not** begin PROTO-4, reputation, or marketplace work from this result set.

---

## Implemented Scope

Local deterministic simulation of authenticated agent-to-agent transport under `Aether/core/src/network/`:

| Module | Role |
|--------|------|
| `model.rs` | `SessionStatus`, feature tokens, version constants |
| `hello.rs` | `ProtocolHelloV0`, initiate / accept / complete handshake |
| `envelope.rs` | `MessageEnvelopeV0`, sign, deliver, payload commitment |
| `session.rs` | `SecureSessionV0`, `SessionStore` (no public `get_mut`) |
| `directory.rs` | `AgentDirectoryV0` — local `AgentId → endpoint` map |
| `verify.rs` | Signature + PROTO-0 active-identity checks |

### What was built

1. **ProtocolHelloV0** — signed introduction with features, nonce, version validation  
2. **MessageEnvelopeV0** — authenticated transport container with payload commitment  
3. **SecureSessionV0** — `Created → HelloSent → Established → Closed` (responder lands on Established after accept)  
4. **AgentDirectoryV0** — local discovery only (no DHT / internet)

### Validation order (enforced)

```text
1. Verify signature
2. Verify identity (PROTO-0 registry, Active status)
3. Verify session state
4. Verify message semantics
5. Mutate state
```

### Architecture choices

- Reuses DEC-004B `SignedMessage`, schema-locked CBOR, SHA-256, Ed25519  
- Session ID = `SHA-256(canonical_cbor(ordered agents + both nonces + protocol_version))`  
- Each agent has its own `SessionStore` (simulates independent processes)  
- Envelope signature proves sender key control only — **not** payload correctness  
- Does not modify PROTO-0 / PROTO-1 / PROTO-2 semantics

---

## Test Results

| Suite | Passed | Failed |
|-------|--------|--------|
| Unit (`agent_id`) | 2 | 0 |
| `identity_tests` (PROTO-0) | 16 | 0 |
| `capability_tests` (PROTO-0) | 20 | 0 |
| `adversarial_tests` (PROTO-0) | 12 | 0 |
| `channel_tests` (PROTO-1) | 27 | 0 |
| `channel_adversarial_tests` (PROTO-1) | 24 | 0 |
| Escrow suites (PROTO-2) | 82 | 0 |
| `network_tests` (PROTO-NET-0) | 12 | 0 |
| `network_adversarial_tests` (PROTO-NET-0) | 14 | 0 |
| **Total** | **209** | **0** |

Clippy: clean (`--all-targets -- -D warnings`)  
Fmt: clean

### Adversarial coverage

| ID | Case | Result |
|----|------|--------|
| NET-A01 | Invalid signature | Reject |
| NET-A02 | Wrong receiver | Reject |
| NET-A03 | Replay / re-deliver | Reject |
| NET-A04 | Duplicate message ID | Reject |
| NET-A05 | Unsupported protocol version | Reject |
| NET-A06 | Expired session | Reject |
| NET-A07 | Unknown agent | Reject |
| NET-A08 | Payload commitment mismatch | Reject |
| NET-A09 | Session identity mismatch | Reject |
| NET-A10 | Modified envelope field | Reject |

Additional: hello nonce replay, forged hello, envelope after close, empty feature intersection.

---

## Security Claims Evidenced

### Locally evidenced

- Agent A can discover Agent B via local directory (PROTO-0 identity required to register)
- Agents establish an authenticated session through dual hello exchange
- Session IDs are deterministic across independent stores
- Signed envelopes deliver only when session is Established
- Invalid signatures, replays, wrong receivers, and commitment mismatches fail closed
- Unknown / inactive identities rejected via PROTO-0 registry
- Closed and expired sessions reject envelopes
- Existing PROTO-0/1/2 regression suite remains green

### Signature semantics preserved

> Envelope signature proves sender key control over envelope fields.  
> It does **not** prove payload correctness, business validity, or economic authorisation.

---

## Assumptions

| Assumption | Notes |
|------------|-------|
| Logical time trusted | `now` / TTL injected by harness |
| Separate in-memory stores | Simulate two agents; no shared process required for correctness |
| No transport encryption | Authenticity only; confidentiality out of scope |
| Directory is local | Curated map; no Sybil cost |
| Feature intersection non-empty | Handshake requires at least one shared feature |

---

## Non-Goals (intentionally excluded)

- Real networking (TCP / UDP / WebSockets)
- Encryption / TLS / Noise
- DHT or internet discovery
- Blockchain, tokens, settlement
- Reputation or marketplace
- Production identity federation
- Watchtowers or offline delivery

---

## Remaining Risks

- No network adversary (delay, partition, MITM) beyond in-process message forging
- No encrypted transport — observers of a future wire path would see payloads
- Directory has no anti-Sybil registration cost
- Session TTL depends on trusted logical time
- Revocation of identity mid-session not fully exercised beyond Active check at delivery
- HelloAccept skips explicit `HelloAccepted` status on responder (goes Created→Established); initiator still passes HelloSent→Established

---

## Next Step Recommendation

**Security review complete:** [PROTO_NET_0_SECURITY_REVIEW.md](PROTO_NET_0_SECURITY_REVIEW.md) — **APPROVE**.

Do not begin PROTO-4, networked PROTO-1/2, or reputation until that APPROVE verdict is **explicitly accepted**.

PROTO-NET-0 is the first independent agent-to-agent communication primitive. Further Phase 2 work depends on acceptance of the review.
