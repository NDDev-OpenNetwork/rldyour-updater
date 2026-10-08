//! Domain-separated Ed25519 catalogue, not a claim of full TUF compliance.
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey, pkcs8::DecodePrivateKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::Path};
pub const LIMIT: u64 = 256 * 1024;
pub const DOMAIN: &[u8] = b"rldyour-updater-catalog/v1\n";
pub const FILES: &[&str] = &[
    "scripts/managed_cli.py",
    "scripts/ai_launchers.py",
    "config/rldyour-contract.json",
    "config/ai-launchers.json",
    "scripts/ubuntu/harness-apps.py",
    "config/harness-apps.json",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub payload: Payload,
    pub signature: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Payload {
    pub schema: u8,
    pub sequence: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub bootstrap_commit: String,
    pub files: Vec<SourceFile>,
    #[serde(default)]
    pub debs: Vec<DebArtifact>,
    #[serde(default)]
    pub updater_binaries: Vec<UpdaterBinary>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdaterBinary {
    pub platform: String,
    pub version: String,
    pub url: String,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DebArtifact {
    pub package: String,
    pub version: String,
    pub url: String,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acceptance {
    pub sequence: u64,
    pub digest: String,
    pub observed_at: u64,
}
impl Payload {
    pub fn validate(&self, now: u64) -> Result<(), String> {
        if self.schema != 1 || self.sequence == 0 {
            return Err("invalid catalogue schema or sequence".into());
        }
        if self.issued_at > now + 300
            || self.expires_at <= now
            || self.expires_at <= self.issued_at
            || self.expires_at - self.issued_at > 31 * 86400
        {
            return Err("expired, future-dated or overlong catalogue".into());
        }
        if self.bootstrap_commit.len() != 40
            || !self.bootstrap_commit.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("bootstrap source must be an exact commit".into());
        }
        let paths: BTreeSet<_> = self.files.iter().map(|f| f.path.as_str()).collect();
        if paths != FILES.iter().copied().collect() || paths.len() != self.files.len() {
            return Err(
                "catalogue must bind exactly the reviewed bootstrap entrypoints and configs".into(),
            );
        }
        for file in &self.files {
            decode_hex::<32>(&file.sha256)?;
            if file.bytes == 0 || file.bytes > 4 * 1024 * 1024 {
                return Err("invalid source file length".into());
            }
        }
        if self.debs.len() > 64 {
            return Err("too many native package artifacts".into());
        }
        let mut names = BTreeSet::new();
        for deb in &self.debs {
            if !names.insert(&deb.package)
                || deb.package.is_empty()
                || deb.package.starts_with('-')
                || !deb
                    .package
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"+.-".contains(&b))
            {
                return Err("invalid or duplicate package identity".into());
            }
            if deb.version.is_empty()
                || deb.version.len() > 128
                || !deb
                    .version
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".:+~-".contains(&b))
            {
                return Err("invalid package version".into());
            }
            crate::transport::validate_https(&deb.url)?;
            decode_hex::<32>(&deb.sha256)?;
            if deb.bytes == 0 || deb.bytes > 1024 * 1024 * 1024 {
                return Err("invalid package artifact size".into());
            }
        }
        let mut platforms = BTreeSet::new();
        for artifact in &self.updater_binaries {
            if !platforms.insert(&artifact.platform)
                || !matches!(artifact.platform.as_str(), "linux/x86_64" | "macos/arm64")
            {
                return Err("unsupported or duplicate updater binary".into());
            }
            version_tuple(&artifact.version)?;
            if !artifact.url.starts_with(
                "https://github.com/NDDev-OpenNetwork/rldyour-updater/releases/download/",
            ) {
                return Err("self updates must come from an explicit updater release".into());
            }
            crate::transport::validate_https(&artifact.url)?;
            decode_hex::<32>(&artifact.sha256)?;
            if artifact.bytes == 0 || artifact.bytes > 64 * 1024 * 1024 {
                return Err("updater binary exceeds bound".into());
            }
        }
        Ok(())
    }
    pub fn signed_bytes(&self) -> Result<Vec<u8>, String> {
        let mut bytes = DOMAIN.to_vec();
        bytes.extend(serde_json::to_vec(self).map_err(|e| e.to_string())?);
        Ok(bytes)
    }
    pub fn digest(&self) -> Result<String, String> {
        Ok(sha256(&self.signed_bytes()?))
    }
}
pub fn verify(
    bytes: &[u8],
    key_hex: &str,
    acceptance: &Acceptance,
    now: u64,
) -> Result<Payload, String> {
    if bytes.len() as u64 > LIMIT {
        return Err("catalogue exceeds bound".into());
    }
    let envelope: Envelope =
        serde_json::from_slice(bytes).map_err(|e| format!("invalid catalogue: {e}"))?;
    let key = VerifyingKey::from_bytes(&decode_hex::<32>(key_hex)?).map_err(|e| e.to_string())?;
    let signature = Signature::from_bytes(&decode_hex::<64>(&envelope.signature)?);
    key.verify_strict(&envelope.payload.signed_bytes()?, &signature)
        .map_err(|_| "catalogue signature invalid")?;
    envelope.payload.validate(now)?;
    if acceptance.observed_at > now + 300 {
        return Err("clock rollback detected".into());
    }
    if envelope.payload.sequence < acceptance.sequence
        || (envelope.payload.sequence == acceptance.sequence
            && acceptance.digest != envelope.payload.digest()?)
    {
        return Err("catalogue rollback or same-sequence equivocation refused".into());
    }
    Ok(envelope.payload)
}
pub fn sign(payload: Payload, private_key: &Path) -> Result<(Envelope, String), String> {
    payload.validate(crate::model::unix_seconds())?;
    crate::storage::state_file(private_key, 16 * 1024)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if private_key
            .metadata()
            .map_err(|e| e.to_string())?
            .permissions()
            .mode()
            & 0o077
            != 0
        {
            return Err("signing key must be private".into());
        }
    }
    let key = SigningKey::read_pkcs8_pem_file(private_key)
        .map_err(|_| "cannot read Ed25519 PKCS#8 signing key")?;
    let public = encode_hex(key.verifying_key().as_bytes());
    let signature = encode_hex(&key.sign(&payload.signed_bytes()?).to_bytes());
    Ok((Envelope { payload, signature }, public))
}
pub fn sha256(bytes: &[u8]) -> String {
    encode_hex(&Sha256::digest(bytes))
}
pub fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn decode_hex<const N: usize>(value: &str) -> Result<[u8; N], String> {
    if value.len() != N * 2 || !value.is_ascii() {
        return Err("invalid hex length".into());
    }
    let mut result = [0; N];
    for (i, byte) in result.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16).map_err(|_| "invalid hex")?;
    }
    Ok(result)
}
pub fn version_tuple(value: &str) -> Result<(u64, u64, u64), String> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3 {
        return Err("stable three-part version required".into());
    }
    Ok((
        parts[0].parse().map_err(|_| "invalid version")?,
        parts[1].parse().map_err(|_| "invalid version")?,
        parts[2].parse().map_err(|_| "invalid version")?,
    ))
}
