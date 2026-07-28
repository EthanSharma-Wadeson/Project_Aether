# Python Interop Verifier

Independent second-language verifier for the published v0 signing fixtures.

## Purpose

Validate that the Rust-generated fixture set can be reconstructed and verified in a separate language without using the published CBOR bytes, preimage bytes, or digest bytes as authoritative verification inputs.

## Runtime

- Python `3.13.5`
- `cryptography` `46.0.1`
- no third-party CBOR library required

## Run

```bash
python3 verify_fixtures.py
```

The verifier:

1. loads the fixture source object
2. reconstructs canonical CBOR from the object
3. reconstructs the signing preimage from the spec
4. computes SHA-256 independently
5. verifies Ed25519 signatures
6. compares reconstructed values to published fixture values

## Canonical ordering rule

The verifier uses an explicit protocol field order, not Python dict insertion order:

1. `protocol_version`
2. `schema_version`
3. `operational_public_key`
4. `permission_root`
5. `metadata_commitment`

If this order changes, fixture bytes change.
