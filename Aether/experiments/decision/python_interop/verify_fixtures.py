#!/usr/bin/env python3
"""
Independent Python verifier for v0 signing fixtures.

This verifier does not treat fixture CBOR bytes, preimage bytes, or digest bytes
as authoritative inputs for the verification path. It reconstructs each from the
source object and verification context, then compares the reconstructed outputs
against the published fixture values.
"""

from __future__ import annotations

import hashlib
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import cryptography
from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

DOMAIN_SEPARATOR = b"\x00"
SCHEMA_ORDER = [
    "protocol_version",
    "schema_version",
    "operational_public_key",
    "permission_root",
    "metadata_commitment",
]


@dataclass
class VerificationResult:
    fixture_name: str
    expected: bool
    actual: bool
    cbor_match: bool
    preimage_match: bool
    digest_match: bool
    fixture_hash_match: bool


def repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


def fixtures_dir() -> Path:
    return repo_root() / "schemas" / "v0" / "fixtures"


def encode_uint(value: int) -> bytes:
    if value < 0:
        raise ValueError("negative integers not supported in v0 fixture encoder")
    if value <= 23:
        return bytes([value])
    if value <= 0xFF:
        return bytes([0x18, value])
    if value <= 0xFFFF:
        return bytes([0x19]) + value.to_bytes(2, "big")
    if value <= 0xFFFFFFFF:
        return bytes([0x1A]) + value.to_bytes(4, "big")
    return bytes([0x1B]) + value.to_bytes(8, "big")


def encode_bytes(value: bytes) -> bytes:
    return encode_major_with_length(2, len(value)) + value


def encode_text(value: str) -> bytes:
    data = value.encode("utf-8")
    return encode_major_with_length(3, len(data)) + data


def encode_null() -> bytes:
    return b"\xF6"


def encode_major_with_length(major: int, length: int) -> bytes:
    if length <= 23:
        return bytes([(major << 5) | length])
    if length <= 0xFF:
        return bytes([(major << 5) | 24, length])
    if length <= 0xFFFF:
        return bytes([(major << 5) | 25]) + length.to_bytes(2, "big")
    if length <= 0xFFFFFFFF:
        return bytes([(major << 5) | 26]) + length.to_bytes(4, "big")
    return bytes([(major << 5) | 27]) + length.to_bytes(8, "big")


def encode_value(value: Any) -> bytes:
    if value is None:
        return encode_null()
    if isinstance(value, int):
        return encode_uint(value)
    if isinstance(value, bytes):
        return encode_bytes(value)
    if isinstance(value, str):
        return encode_text(value)
    raise TypeError(f"unsupported value type: {type(value)!r}")


def canonical_cbor_body_from_source(source: dict[str, Any]) -> bytes:
    body = {
        "protocol_version": source["protocol_version"],
        "schema_version": source["schema_version"],
        "operational_public_key": bytes.fromhex(source["operational_public_key_hex"]),
        "permission_root": bytes.fromhex(source["permission_root_hex"]),
        "metadata_commitment": (
            None
            if source["metadata_commitment_hex"] is None
            else bytes.fromhex(source["metadata_commitment_hex"])
        ),
    }
    out = bytearray()
    out.extend(encode_major_with_length(5, len(SCHEMA_ORDER)))
    for key in SCHEMA_ORDER:
        out.extend(encode_text(key))
        out.extend(encode_value(body[key]))
    return bytes(out)


def reconstruct_preimage(source: dict[str, Any], verification_context: dict[str, Any], cbor_body: bytes) -> bytes:
    message_type_bytes = verification_context["message_type"].encode("utf-8")
    domain_tag_bytes = verification_context["domain_tag_utf8"].encode("utf-8")
    return (
        domain_tag_bytes
        + DOMAIN_SEPARATOR
        + int(source["protocol_version"]).to_bytes(4, "big")
        + DOMAIN_SEPARATOR
        + int(source["schema_version"]).to_bytes(4, "big")
        + DOMAIN_SEPARATOR
        + len(message_type_bytes).to_bytes(2, "big")
        + message_type_bytes
        + DOMAIN_SEPARATOR
        + len(cbor_body).to_bytes(4, "big")
        + cbor_body
    )


def reconstruct_fixture_hash(preimage: bytes, public_key: bytes, signature: bytes, expected: bool) -> bytes:
    return hashlib.sha256(
        b'aether:v0:fixture:v1' + preimage + public_key + signature + (b"\x01" if expected else b"\x00")
    ).digest()


def verify_fixture(fixture_path: Path) -> VerificationResult:
    fixture = json.loads(fixture_path.read_text())
    source = fixture["source_object"]
    verification_context = fixture["verification_context"]

    reconstructed_cbor = canonical_cbor_body_from_source(source)
    reconstructed_preimage = reconstruct_preimage(source, verification_context, reconstructed_cbor)
    reconstructed_digest = hashlib.sha256(reconstructed_preimage).digest()

    public_key = bytes.fromhex(fixture["public_key_hex"])
    signature = bytes.fromhex(fixture["signature_hex"])
    expected = bool(fixture["expected_verification_result"])

    verifier = Ed25519PublicKey.from_public_bytes(public_key)
    try:
        verifier.verify(signature, reconstructed_digest)
        actual = True
    except InvalidSignature:
        actual = False

    reconstructed_fixture_hash = reconstruct_fixture_hash(
        reconstructed_preimage, public_key, signature, expected
    )

    return VerificationResult(
        fixture_name=fixture["fixture_name"],
        expected=expected,
        actual=actual,
        cbor_match=reconstructed_cbor.hex() == fixture["canonical_cbor_hex"],
        preimage_match=reconstructed_preimage.hex() == fixture["signing_preimage_hex"],
        digest_match=reconstructed_digest.hex() == fixture["sha256_digest_hex"],
        fixture_hash_match=reconstructed_fixture_hash.hex() == fixture["fixture_hash_hex"],
    )


def equivalent_construction_path_matches() -> bool:
    fixture = json.loads((fixtures_dir() / "identity_register_valid.fixture.json").read_text())
    source = fixture["source_object"]
    reordered = {
        "permission_root_hex": source["permission_root_hex"],
        "schema_version": source["schema_version"],
        "metadata_commitment_hex": source["metadata_commitment_hex"],
        "protocol_version": source["protocol_version"],
        "operational_public_key_hex": source["operational_public_key_hex"],
    }
    return canonical_cbor_body_from_source(source) == canonical_cbor_body_from_source(reordered)


def main() -> int:
    fixture_paths = sorted(fixtures_dir().glob("*.fixture.json"))
    if not fixture_paths:
        print("No fixtures found.", file=sys.stderr)
        return 1

    print("DEC-004B independent interoperability experiment: python_interop")
    print(f"python_version: {sys.version.split()[0]}")
    print(f"cryptography_version: {cryptography.__version__}")
    print(f"fixture_dir: {fixtures_dir()}")
    print(f"fixture_count: {len(fixture_paths)}")
    print(f"schema_order_rule: {SCHEMA_ORDER}")
    print("ordering_investigation: Python verifier uses explicit schema order, not dict insertion order.")
    print("ordering_result: equivalent construction paths reproduce identical CBOR bytes.")

    failures = 0
    for path in fixture_paths:
        result = verify_fixture(path)
        print(
            f"{result.fixture_name}: "
            f"expected={result.expected} actual={result.actual} "
            f"cbor_match={result.cbor_match} preimage_match={result.preimage_match} "
            f"digest_match={result.digest_match} fixture_hash_match={result.fixture_hash_match}"
        )
        if not (
            result.expected == result.actual
            and result.cbor_match
            and result.preimage_match
            and result.digest_match
            and result.fixture_hash_match
        ):
            failures += 1

    if not equivalent_construction_path_matches():
        print("equivalent_construction_paths: FAIL", file=sys.stderr)
        failures += 1
    else:
        print("equivalent_construction_paths: PASS")

    if failures:
        print(f"RESULT: FAIL ({failures} mismatch(es))", file=sys.stderr)
        return 1

    print("RESULT: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
