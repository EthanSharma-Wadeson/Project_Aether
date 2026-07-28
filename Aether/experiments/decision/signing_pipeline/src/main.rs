use ciborium::value::Value;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const DOMAIN_TAG: &str = "aether:v0:sign:v1";
const FIXTURE_DOMAIN_TAG: &str = "aether:v0:fixture:v1";
const MESSAGE_TYPE: &str = "identity.register";
const SIGNER_KEY_ID: &str = "operational:0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct IdentityRegisterV0 {
    protocol_version: u32,
    schema_version: u32,
    operational_public_key: Vec<u8>,
    permission_root: Vec<u8>,
    metadata_commitment: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourceObjectView {
    protocol_version: u32,
    schema_version: u32,
    operational_public_key_hex: String,
    permission_root_hex: String,
    metadata_commitment_hex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PreimageConstruction {
    algorithm: String,
    domain_tag_utf8: String,
    separator_hex: String,
    protocol_version_encoding: String,
    schema_version_encoding: String,
    message_type_length_encoding: String,
    body_length_encoding: String,
    body_encoding: String,
    layout: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VerificationContext {
    domain_tag_utf8: String,
    message_type: String,
    signer_key_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SigningFixture {
    fixture_name: String,
    description: String,
    expected_verification_result: bool,
    source_object: SourceObjectView,
    canonical_cbor_hex: String,
    signing_preimage_hex: String,
    preimage_construction: PreimageConstruction,
    sha256_digest_hex: String,
    public_key_hex: String,
    signature_hex: String,
    verification_context: VerificationContext,
    fixture_hash_construction: String,
    fixture_hash_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManifestEntry {
    fixture_name: String,
    path: String,
    expected_verification_result: bool,
    fixture_hash_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    schema: String,
    generated_at: String,
    fixtures: Vec<ManifestEntry>,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../schemas/v0/fixtures")
}

fn fixture_preimage_construction() -> PreimageConstruction {
    PreimageConstruction {
        algorithm: "sha256(domain-separated preimage over canonical CBOR body)".into(),
        domain_tag_utf8: DOMAIN_TAG.into(),
        separator_hex: "00".into(),
        protocol_version_encoding: "u32be".into(),
        schema_version_encoding: "u32be".into(),
        message_type_length_encoding: "u16be".into(),
        body_length_encoding: "u32be".into(),
        body_encoding: "schema-locked CBOR bytes".into(),
        layout: vec![
            "domain_tag_utf8".into(),
            "0x00 separator".into(),
            "protocol_version (u32be)".into(),
            "0x00 separator".into(),
            "schema_version (u32be)".into(),
            "0x00 separator".into(),
            "message_type_length (u16be)".into(),
            "message_type_utf8".into(),
            "0x00 separator".into(),
            "canonical_cbor_body_length (u32be)".into(),
            "canonical_cbor_body".into(),
        ],
    }
}

fn source_view(body: &IdentityRegisterV0) -> SourceObjectView {
    SourceObjectView {
        protocol_version: body.protocol_version,
        schema_version: body.schema_version,
        operational_public_key_hex: hex::encode(&body.operational_public_key),
        permission_root_hex: hex::encode(&body.permission_root),
        metadata_commitment_hex: body
            .metadata_commitment
            .as_ref()
            .map(hex::encode),
    }
}

fn body_value(body: &IdentityRegisterV0) -> Value {
    Value::Map(vec![
        (Value::Text("protocol_version".into()), Value::Integer(body.protocol_version.into())),
        (Value::Text("schema_version".into()), Value::Integer(body.schema_version.into())),
        (
            Value::Text("operational_public_key".into()),
            Value::Bytes(body.operational_public_key.clone()),
        ),
        (
            Value::Text("permission_root".into()),
            Value::Bytes(body.permission_root.clone()),
        ),
        (
            Value::Text("metadata_commitment".into()),
            match &body.metadata_commitment {
                Some(bytes) => Value::Bytes(bytes.clone()),
                None => Value::Null,
            },
        ),
    ])
}

fn encode_canonical_body(body: &IdentityRegisterV0) -> Vec<u8> {
    let mut bytes = Vec::new();
    ciborium::into_writer(&body_value(body), &mut bytes).expect("encode canonical body");
    bytes
}

#[cfg(test)]
fn decode_body(bytes: &[u8]) -> IdentityRegisterV0 {
    ciborium::from_reader(bytes).expect("decode canonical body")
}

fn signing_preimage(body: &IdentityRegisterV0, message_type: &str) -> Vec<u8> {
    let cbor = encode_canonical_body(body);
    signing_preimage_from_parts(
        DOMAIN_TAG,
        body.protocol_version,
        body.schema_version,
        message_type,
        &cbor,
    )
}

fn signing_preimage_from_parts(
    domain_tag: &str,
    protocol_version: u32,
    schema_version: u32,
    message_type: &str,
    cbor_body: &[u8],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(domain_tag.as_bytes());
    out.push(0x00);
    out.extend_from_slice(&protocol_version.to_be_bytes());
    out.push(0x00);
    out.extend_from_slice(&schema_version.to_be_bytes());
    out.push(0x00);
    let mt = message_type.as_bytes();
    out.extend_from_slice(&(mt.len() as u16).to_be_bytes());
    out.extend_from_slice(mt);
    out.push(0x00);
    out.extend_from_slice(&(cbor_body.len() as u32).to_be_bytes());
    out.extend_from_slice(cbor_body);
    out
}

fn digest(preimage: &[u8]) -> [u8; 32] {
    Sha256::digest(preimage).into()
}

fn fixture_hash(preimage: &[u8], public_key: &[u8], signature: &[u8], expected: bool) -> [u8; 32] {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(FIXTURE_DOMAIN_TAG.as_bytes());
    bytes.extend_from_slice(preimage);
    bytes.extend_from_slice(public_key);
    bytes.extend_from_slice(signature);
    bytes.push(if expected { 1 } else { 0 });
    Sha256::digest(bytes).into()
}

fn sign_digest(signing_key: &SigningKey, digest: &[u8; 32]) -> Signature {
    signing_key.sign(digest)
}

fn verify_fixture(fixture: &SigningFixture) -> bool {
    let public_key = VerifyingKey::from_bytes(
        &hex::decode(&fixture.public_key_hex)
            .expect("fixture public key hex")
            .try_into()
            .expect("public key len"),
    )
    .expect("public key");
    let signature = Signature::from_bytes(
        &hex::decode(&fixture.signature_hex)
            .expect("fixture signature hex")
            .try_into()
            .expect("signature len"),
    );
    let cbor_body = hex::decode(&fixture.canonical_cbor_hex).expect("fixture cbor hex");
    let preimage = signing_preimage_from_parts(
        &fixture.verification_context.domain_tag_utf8,
        fixture.source_object.protocol_version,
        fixture.source_object.schema_version,
        &fixture.verification_context.message_type,
        &cbor_body,
    );
    let digest = digest(&preimage);
    public_key.verify(&digest, &signature).is_ok()
}

fn baseline_body() -> IdentityRegisterV0 {
    IdentityRegisterV0 {
        protocol_version: 1,
        schema_version: 1,
        operational_public_key: vec![0xA1; 32],
        permission_root: vec![0xB2; 32],
        metadata_commitment: None,
    }
}

#[cfg(test)]
fn alternate_body_construction() -> IdentityRegisterV0 {
    let json = r#"{
        "permission_root": "b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2",
        "schema_version": 1,
        "metadata_commitment": null,
        "protocol_version": 1,
        "operational_public_key": "a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1"
    }"#;
    #[derive(Deserialize)]
    struct JsonBody {
        protocol_version: u32,
        schema_version: u32,
        operational_public_key: String,
        permission_root: String,
        metadata_commitment: Option<String>,
    }
    let parsed: JsonBody = serde_json::from_str(json).expect("alt body");
    IdentityRegisterV0 {
        protocol_version: parsed.protocol_version,
        schema_version: parsed.schema_version,
        operational_public_key: hex::decode(parsed.operational_public_key).unwrap(),
        permission_root: hex::decode(parsed.permission_root).unwrap(),
        metadata_commitment: parsed.metadata_commitment.map(|v| hex::decode(v).unwrap()),
    }
}

fn make_fixture(
    fixture_name: &str,
    description: &str,
    body: &IdentityRegisterV0,
    message_type: &str,
    domain_tag: &str,
    public_key: &VerifyingKey,
    signature: &Signature,
    expected: bool,
) -> SigningFixture {
    let cbor = encode_canonical_body(body);
    let preimage = signing_preimage_from_parts(
        domain_tag,
        body.protocol_version,
        body.schema_version,
        message_type,
        &cbor,
    );
    let digest_bytes = digest(&preimage);
    let fixture_hash_bytes = fixture_hash(
        &preimage,
        public_key.as_bytes(),
        &signature.to_bytes(),
        expected,
    );
    SigningFixture {
        fixture_name: fixture_name.into(),
        description: description.into(),
        expected_verification_result: expected,
        source_object: source_view(body),
        canonical_cbor_hex: hex::encode(&cbor),
        signing_preimage_hex: hex::encode(&preimage),
        preimage_construction: fixture_preimage_construction(),
        sha256_digest_hex: hex::encode(digest_bytes),
        public_key_hex: hex::encode(public_key.as_bytes()),
        signature_hex: hex::encode(signature.to_bytes()),
        verification_context: VerificationContext {
            domain_tag_utf8: domain_tag.into(),
            message_type: message_type.into(),
            signer_key_id: SIGNER_KEY_ID.into(),
        },
        fixture_hash_construction:
            "sha256(UTF8(\"aether:v0:fixture:v1\") || signing_preimage || public_key || signature || expected_result_byte)"
                .into(),
        fixture_hash_hex: hex::encode(fixture_hash_bytes),
    }
}

fn write_fixture(dir: &Path, fixture: &SigningFixture) {
    let path = dir.join(format!("{}.fixture.json", fixture.fixture_name));
    let json = serde_json::to_string_pretty(fixture).expect("fixture json");
    fs::write(path, json).expect("write fixture");
}

fn generate_fixtures() -> Vec<SigningFixture> {
    let base = baseline_body();
    let signing_key = SigningKey::from_bytes(&[0x11; 32]);
    let verifying_key = signing_key.verifying_key();
    let preimage = signing_preimage(&base, MESSAGE_TYPE);
    let digest_bytes = digest(&preimage);
    let baseline_signature = sign_digest(&signing_key, &digest_bytes);

    let mut modified_body = base.clone();
    modified_body.permission_root[0] ^= 0x01;

    let wrong_key = SigningKey::from_bytes(&[0x22; 32]).verifying_key();
    let mut bad_sig_bytes = baseline_signature.to_bytes();
    bad_sig_bytes[0] ^= 0x01;
    let bad_signature = Signature::from_bytes(&bad_sig_bytes);

    vec![
        make_fixture(
            "identity_register_valid",
            "Baseline valid identity.register fixture",
            &base,
            MESSAGE_TYPE,
            DOMAIN_TAG,
            &verifying_key,
            &baseline_signature,
            true,
        ),
        make_fixture(
            "identity_register_modified_message_data",
            "Body changed after signing; verification must fail",
            &modified_body,
            MESSAGE_TYPE,
            DOMAIN_TAG,
            &verifying_key,
            &baseline_signature,
            false,
        ),
        make_fixture(
            "identity_register_modified_domain",
            "Domain tag changed; verification must fail",
            &base,
            MESSAGE_TYPE,
            "aether:v0:sign:v2",
            &verifying_key,
            &baseline_signature,
            false,
        ),
        make_fixture(
            "identity_register_modified_message_type",
            "Message type changed; verification must fail",
            &base,
            "identity.rotate",
            DOMAIN_TAG,
            &verifying_key,
            &baseline_signature,
            false,
        ),
        make_fixture(
            "identity_register_modified_signature",
            "Signature bytes changed; verification must fail",
            &base,
            MESSAGE_TYPE,
            DOMAIN_TAG,
            &verifying_key,
            &bad_signature,
            false,
        ),
        make_fixture(
            "identity_register_wrong_public_key",
            "Wrong verifying key; verification must fail",
            &base,
            MESSAGE_TYPE,
            DOMAIN_TAG,
            &wrong_key,
            &baseline_signature,
            false,
        ),
    ]
}

fn write_manifest(dir: &Path, fixtures: &[SigningFixture]) {
    let manifest = Manifest {
        schema: "aether:v0:fixtures:manifest:1".into(),
        generated_at: "2026-07-28".into(),
        fixtures: fixtures
            .iter()
            .map(|fixture| ManifestEntry {
                fixture_name: fixture.fixture_name.clone(),
                path: format!("{}.fixture.json", fixture.fixture_name),
                expected_verification_result: fixture.expected_verification_result,
                fixture_hash_hex: fixture.fixture_hash_hex.clone(),
            })
            .collect(),
    };
    let json = serde_json::to_string_pretty(&manifest).expect("manifest json");
    fs::write(dir.join("manifest.json"), json).expect("write manifest");
}

fn ensure_fixtures() -> Vec<SigningFixture> {
    let dir = fixtures_dir();
    fs::create_dir_all(&dir).expect("create fixture dir");
    let fixtures = generate_fixtures();
    for fixture in &fixtures {
        write_fixture(&dir, fixture);
    }
    write_manifest(&dir, &fixtures);
    fixtures
}

#[cfg(test)]
fn load_fixture(path: &Path) -> SigningFixture {
    serde_json::from_slice(&fs::read(path).expect("read fixture")).expect("parse fixture")
}

#[cfg(test)]
fn load_all_fixture_paths() -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(fixtures_dir())
        .expect("read fixture dir")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(".fixture.json"))
                .unwrap_or(false)
        })
        .collect();
    paths.sort();
    paths
}

fn main() {
    let fixtures = ensure_fixtures();
    println!("DEC-004B experiment: signing_pipeline");
    println!("  fixture_dir: {}", fixtures_dir().display());
    println!("  domain_tag_utf8: {}", DOMAIN_TAG);
    println!("  message_type: {}", MESSAGE_TYPE);
    println!("  fixture_count: {}", fixtures.len());
    for fixture in fixtures {
        let actual = verify_fixture(&fixture);
        println!(
            "  {} => expected={} actual={} fixture_hash={}",
            fixture.fixture_name, fixture.expected_verification_result, actual, fixture.fixture_hash_hex
        );
    }
    println!("RESULT: fixtures generated and verification run completed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_objects_produce_identical_cbor() {
        let a = baseline_body();
        let b = baseline_body();
        assert_eq!(encode_canonical_body(&a), encode_canonical_body(&b));
    }

    #[test]
    fn equivalent_construction_paths_produce_identical_cbor() {
        let a = baseline_body();
        let b = alternate_body_construction();
        assert_eq!(a, b);
        assert_eq!(encode_canonical_body(&a), encode_canonical_body(&b));
    }

    #[test]
    fn encode_decode_encode_is_byte_identical() {
        let a = baseline_body();
        let bytes = encode_canonical_body(&a);
        let decoded = decode_body(&bytes);
        let reencoded = encode_canonical_body(&decoded);
        assert_eq!(bytes, reencoded);
    }

    #[test]
    fn fixtures_load_and_verify_as_expected() {
        ensure_fixtures();
        for path in load_all_fixture_paths() {
            let fixture = load_fixture(&path);
            let actual = verify_fixture(&fixture);
            assert_eq!(
                actual, fixture.expected_verification_result,
                "fixture {} mismatch", fixture.fixture_name
            );
        }
    }

    #[test]
    fn valid_signature_verifies() {
        let fixtures = ensure_fixtures();
        let valid = fixtures
            .into_iter()
            .find(|f| f.fixture_name == "identity_register_valid")
            .unwrap();
        assert!(verify_fixture(&valid));
    }

    #[test]
    fn modified_message_data_fails() {
        let fixtures = ensure_fixtures();
        let fixture = fixtures
            .into_iter()
            .find(|f| f.fixture_name == "identity_register_modified_message_data")
            .unwrap();
        assert!(!verify_fixture(&fixture));
    }

    #[test]
    fn modified_domain_fails() {
        let fixtures = ensure_fixtures();
        let fixture = fixtures
            .into_iter()
            .find(|f| f.fixture_name == "identity_register_modified_domain")
            .unwrap();
        assert!(!verify_fixture(&fixture));
    }

    #[test]
    fn modified_message_type_fails() {
        let fixtures = ensure_fixtures();
        let fixture = fixtures
            .into_iter()
            .find(|f| f.fixture_name == "identity_register_modified_message_type")
            .unwrap();
        assert!(!verify_fixture(&fixture));
    }

    #[test]
    fn modified_signature_fails() {
        let fixtures = ensure_fixtures();
        let fixture = fixtures
            .into_iter()
            .find(|f| f.fixture_name == "identity_register_modified_signature")
            .unwrap();
        assert!(!verify_fixture(&fixture));
    }

    #[test]
    fn wrong_public_key_fails() {
        let fixtures = ensure_fixtures();
        let fixture = fixtures
            .into_iter()
            .find(|f| f.fixture_name == "identity_register_wrong_public_key")
            .unwrap();
        assert!(!verify_fixture(&fixture));
    }
}
