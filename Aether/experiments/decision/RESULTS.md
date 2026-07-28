# Decision Experiment Results

**Last run:** 2026-07-28  
**Environment:** rustc 1.97.1 (stable-aarch64-apple-darwin)

## rust_sign_verify (DEC-001 / DEC-004A)

**Location:** `rust_sign_verify/`  
**Command:** `cargo test && cargo run`

| Check | Result |
|-------|--------|
| `cargo test` | 2/2 passed |
| Ed25519 sign/verify on SHA-256 digest | PASS |
| Wrong-payload rejection | PASS |

**Evidence recorded:**

- `ed25519-dalek` 2.x compiles and verifies in the target environment without custom crypto.
- Signing was performed over a **fixed stub payload hash**, not canonical wire bytes (DEC-004B remains blocked on DEC-003).

**Impact:**

- Supports keeping DEC-001 (Rust) and provisional DEC-004A (Ed25519 primitive).
- Does not justify locking DEC-004B signed-byte construction.

---

## serialization_compare (DEC-003)

**Location:** `serialization_compare/`  
**Command:** `cargo test && cargo run`

### Criterion matrix

| Criterion | Canonical JSON (sorted keys) | CBOR (ciborium + serde) | serde_json default |
|-----------|------------------------------|-------------------------|-------------------|
| Deterministic encoding | PASS | PASS | PASS* |
| Field-order independence | PASS | **FAIL** | FAIL |
| Encode/decode roundtrip | PASS | PASS | PASS |
| Unknown fields (`deny_unknown_fields`) | PASS | PASS | — |
| Integer edge cases (u32::MAX, u64::MAX) | PASS | PASS | — |
| Binary key handling | Arrays of u8 in JSON (not base64 in this harness) | Compact byte strings | — |
| Human auditability | HIGH (readable UTF-8) | LOW (binary) | HIGH |
| Cross-language feasibility | HIGH (JSON/JCS ecosystem) | MEDIUM-HIGH (RFC 8949; field order must be spec-locked) | LOW for wire |

\*serde_json default is deterministic only if field order is fixed in a single struct definition — not safe across implementations reordering fields.

### Key findings

1. **Sorted-key canonical JSON** is field-order independent and deterministic in tests.
2. **CBOR via serde** is deterministic **only when struct field order is fixed in the schema spec** — reordering struct fields changes bytes (test `cbor_field_order_dependent_with_serde` confirms).
3. **Binary fields:** serde_json emitted byte arrays as JSON integer arrays in this harness; production wire JSON would need an explicit rule (e.g. base64url) — affects size and cross-language parsers.
4. **Size (stub message):** canonical JSON 355 bytes; CBOR 214 bytes (~40% smaller on this fixture).
5. **Fixture hashes (stub message):**
   - canonical_json_sha256: `ebdd86fbc0d830280daffd61ae5536b9054fc0959656140502a8d4812d08f817`
   - cbor_serde_sha256: `6ce7472385d71eea01503e01241ef4ac5666a335b7d1b7c2bbf921075f3d1276`

### Impact on DEC-003

Neither format is "winner by preference." Evidence supports:

- **Reject** serde_json default as canonical wire format.
- **Shortlist** (a) schema-locked canonical CBOR, (b) sorted-key canonical JSON with explicit binary encoding rules.
- Final selection must be made jointly with DEC-004B, DEC-006, DEC-007, and consistency review — not locked from this experiment alone.

---

## core scaffold (DEC-002)

**Location:** `../../core/`  
**Command:** `cargo build`  
**Result:** PASS (empty scaffold compiles)

---

## signing_pipeline (DEC-004B)

**Location:** `signing_pipeline/`  
**Command:** `cargo run && cargo test`

### Exact signing construction

The verified v0 signing pipeline is:

```text
Canonical protocol object
  → schema-locked canonical CBOR bytes
  → domain-separated signing preimage
  → SHA-256 digest
  → Ed25519 sign / verify
```

#### Canonical protocol object used in the experiment

```text
IdentityRegisterV0 {
  protocol_version: u32,
  schema_version: u32,
  operational_public_key: bytes,
  permission_root: bytes,
  metadata_commitment: optional bytes | null,
}
```

#### Canonical CBOR body rules

- CBOR map encoded in **normative schema order**:
  1. `protocol_version`
  2. `schema_version`
  3. `operational_public_key`
  4. `permission_root`
  5. `metadata_commitment`
- Keys are UTF-8 text strings.
- Integers are unsigned.
- Binary values are CBOR byte strings.
- `metadata_commitment = null` encodes as CBOR `null`.
- Unknown fields are rejected during strict decode in tests.

#### Signing preimage construction

```text
preimage =
  UTF8("aether:v0:sign:v1")
  || 0x00
  || protocol_version_u32be
  || 0x00
  || schema_version_u32be
  || 0x00
  || message_type_len_u16be
  || UTF8(message_type)
  || 0x00
  || canonical_cbor_body_len_u32be
  || canonical_cbor_body
```

Where:

- `message_type` for the valid fixture = `identity.register`
- `protocol_version_u32be` = 4-byte big-endian unsigned integer
- `schema_version_u32be` = 4-byte big-endian unsigned integer
- `message_type_len_u16be` = 2-byte big-endian unsigned integer
- `canonical_cbor_body_len_u32be` = 4-byte big-endian unsigned integer

Then:

```text
digest = SHA-256(preimage)
signature = Ed25519_sign(signing_key, digest)
verify = Ed25519_verify(public_key, digest, signature)
```

### Test matrix

| Check | Result |
|-------|--------|
| Identical objects produce identical canonical bytes | PASS |
| Equivalent construction paths produce identical canonical bytes | PASS |
| Encode → decode → encode remains byte-identical | PASS |
| Valid signatures verify | PASS |
| Modified message data fails verification | PASS |
| Modified domain fails verification | PASS |
| Modified message type fails verification | PASS |
| Modified signature fails verification | PASS |
| Wrong public key fails verification | PASS |
| Tests load fixtures from disk and verify expected result | PASS |

### Fixture inventory

Generated under `Aether/schemas/v0/fixtures/`:

1. `identity_register_valid.fixture.json`
2. `identity_register_modified_message_data.fixture.json`
3. `identity_register_modified_domain.fixture.json`
4. `identity_register_modified_message_type.fixture.json`
5. `identity_register_modified_signature.fixture.json`
6. `identity_register_wrong_public_key.fixture.json`
7. `manifest.json`

### Key findings

1. The previously ambiguous domain-binding bug was real: hardcoding the domain tag caused the negative domain test to pass incorrectly on first run. Fixing verification to use the fixture's domain tag resolved this.
2. The domain tag, protocol version, schema version, message type, and canonical body are now all explicitly bound into the signing preimage with unambiguous byte boundaries.
3. Canonical CBOR fixtures are reproducible from disk and verify as expected.
4. This satisfies the DEC-004B requirement for testing real canonical bytes, but does **not** by itself justify locking the decision group.

### Impact on DEC-004B

- Supports keeping DEC-004B as **Provisional** with higher evidence.
- Removes two prior open gaps:
  - real canonical-CBOR sign/verify test
  - committed fixture set under `schemas/v0/fixtures/`
- Remaining reasons not to lock:
  - `permission_root` root-authority commitment shape is still provisional
  - `registered_at` / time semantics still unresolved
  - no additional non-Python second-language fixture consumer yet

---

## python_interop (DEC-003 / DEC-004B)

**Location:** `python_interop/`  
**Command:** `python3 verify_fixtures.py`

### Dependency versions

- Python `3.13.5`
- `cryptography` `46.0.1`
- no external CBOR package required

### Independent verification scope

The Python verifier independently:

1. loaded each fixture source object
2. reconstructed canonical CBOR from the object
3. reconstructed the signing preimage from the spec
4. computed SHA-256 independently
5. verified the Ed25519 signature
6. compared reconstructed values with published fixture values afterward

### Canonical ordering investigation

The Python verifier does **not** rely on Python dict insertion order or Rust map insertion behavior.

Instead, it encodes the CBOR map using an explicit protocol field order:

1. `protocol_version`
2. `schema_version`
3. `operational_public_key`
4. `permission_root`
5. `metadata_commitment`

**Result:** independently reconstructed CBOR bytes matched the published Rust fixtures for all cases. Equivalent construction paths also produced identical bytes.

### Test / comparison results

| Check | Result |
|-------|--------|
| Reconstructed CBOR matches published fixture bytes | PASS (all fixtures) |
| Reconstructed preimage matches published fixture preimage | PASS (all fixtures) |
| Reconstructed SHA-256 digest matches published digest | PASS (all fixtures) |
| Valid fixture verifies | PASS |
| Modified message data fails | PASS |
| Modified domain fails | PASS |
| Modified message type fails | PASS |
| Modified signature fails | PASS |
| Wrong public key fails | PASS |
| Equivalent construction path reproduces same CBOR bytes | PASS |

### Impact

- Removes the interoperability concern that the current fixture set might only be reproducible inside Rust.
- Strengthens DEC-003 and DEC-004B, but does **not** justify locking the group yet.
- Confirms the protocol must specify **explicit schema order** rather than relying on generic map canonicalization or implementation insertion behavior.

### Remaining reasons to stay Provisional

- `permission_root` root-authority commitment shape is still provisional
- `registered_at` / time semantics still unresolved
- only one independent second-language verifier exists so far

---

## Re-run instructions

```bash
source "$HOME/.cargo/env"
cd Aether/experiments/decision/rust_sign_verify && cargo test && cargo run
cd ../serialization_compare && cargo test && cargo run
cd ../signing_pipeline && cargo run && cargo test
python3 ../python_interop/verify_fixtures.py
cd ../../../core && cargo build
```

Update this file when experiments are re-run. Reference commit/date in DECISIONS.md evidence sections.
