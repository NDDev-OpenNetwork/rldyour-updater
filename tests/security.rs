use ed25519_dalek::{Signer, SigningKey};
#[cfg(unix)]
use rldyour_updater::process::{CommandRunner, RealCommandRunner};
use rldyour_updater::{
    catalog::{self, Acceptance, Envelope, Payload, SourceFile},
    config::Policy,
    lockfile::RunLock,
};
#[cfg(unix)]
use std::{
    fs,
    time::{Duration, Instant},
};
fn payload(now: u64) -> Payload {
    Payload {
        schema: 1,
        debs: vec![],
        updater_binaries: vec![],
        sequence: 2,
        issued_at: now - 1,
        expires_at: now + 100,
        bootstrap_commit: "a".repeat(40),
        files: catalog::FILES
            .iter()
            .map(|path| SourceFile {
                path: (*path).into(),
                bytes: 4,
                sha256: catalog::sha256(b"test"),
            })
            .collect(),
    }
}
fn signed(value: Payload) -> (Vec<u8>, String) {
    let key = SigningKey::from_bytes(&[7; 32]);
    let signature = catalog::encode_hex(&key.sign(&value.signed_bytes().unwrap()).to_bytes());
    (
        serde_json::to_vec(&Envelope {
            payload: value,
            signature,
        })
        .unwrap(),
        catalog::encode_hex(key.verifying_key().as_bytes()),
    )
}
#[test]
fn authentic_payload_passes_but_tamper_or_other_key_is_refused() {
    let now = 1000;
    let (bytes, key) = signed(payload(now));
    assert!(catalog::verify(&bytes, &key, &Acceptance::default(), now).is_ok());
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["payload"]["bootstrap_commit"] = "b".repeat(40).into();
    assert!(
        catalog::verify(
            &serde_json::to_vec(&value).unwrap(),
            &key,
            &Acceptance::default(),
            now
        )
        .is_err()
    );
    let other = catalog::encode_hex(SigningKey::from_bytes(&[8; 32]).verifying_key().as_bytes());
    assert!(catalog::verify(&bytes, &other, &Acceptance::default(), now).is_err());
}
#[test]
fn expired_future_and_overlong_metadata_is_refused_even_if_signed() {
    for value in [
        Payload {
            expires_at: 999,
            ..payload(1000)
        },
        Payload {
            issued_at: 1500,
            expires_at: 1600,
            ..payload(1000)
        },
        Payload {
            expires_at: 1000 + 32 * 86400,
            ..payload(1000)
        },
    ] {
        let (bytes, key) = signed(value);
        assert!(catalog::verify(&bytes, &key, &Acceptance::default(), 1000).is_err());
    }
}
#[test]
fn ledger_refuses_rollback_equivocation_and_clock_rollback() {
    let value = payload(1000);
    let digest = value.digest().unwrap();
    let (bytes, key) = signed(value);
    assert!(
        catalog::verify(
            &bytes,
            &key,
            &Acceptance {
                sequence: 3,
                digest: digest.clone(),
                observed_at: 1000
            },
            1000
        )
        .is_err()
    );
    assert!(
        catalog::verify(
            &bytes,
            &key,
            &Acceptance {
                sequence: 2,
                digest: "wrong".into(),
                observed_at: 1000
            },
            1000
        )
        .is_err()
    );
    assert!(
        catalog::verify(
            &bytes,
            &key,
            &Acceptance {
                sequence: 2,
                digest: digest.clone(),
                observed_at: 1500
            },
            1000
        )
        .is_err()
    );
    assert!(
        catalog::verify(
            &bytes,
            &key,
            &Acceptance {
                sequence: 2,
                digest,
                observed_at: 1000
            },
            1000
        )
        .is_ok()
    );
}
#[test]
fn source_set_cannot_add_commands_redirects_or_omit_companion() {
    let mut value = payload(1000);
    value.files.pop();
    assert!(value.validate(1000).is_err());
    let mut value = payload(1000);
    value.files[0].path = "../../evil".into();
    assert!(value.validate(1000).is_err());
    let mut value = payload(1000);
    value.files.push(value.files[0].clone());
    assert!(value.validate(1000).is_err());
}
#[test]
fn malformed_policy_and_legacy_execution_are_not_allowed() {
    assert!(toml::from_str::<Policy>("schema=2\nchannel='signed-gds'\nunknown=true").is_err());
    let mut value = Policy::defaults();
    value.schema = 1;
    assert!(value.validate().is_err());
    let mut value = Policy::defaults();
    value.apt.apply = true;
    assert!(value.validate().is_err());
}
fn temp() -> tempfile::TempDir {
    let parent = std::env::temp_dir().canonicalize().unwrap();
    tempfile::tempdir_in(parent).unwrap()
}
#[test]
fn native_lock_survives_no_stale_file_problem_and_is_exclusive() {
    let dir = temp();
    let first = RunLock::acquire(dir.path()).unwrap();
    assert!(RunLock::acquire(dir.path()).is_err());
    drop(first);
    assert!(RunLock::acquire(dir.path()).is_ok());
}
#[cfg(unix)]
#[test]
fn redirected_state_and_special_files_are_refused_before_mutation() {
    use std::os::unix::fs::symlink;
    let dir = temp();
    let target = dir.path().join("target");
    fs::create_dir(&target).unwrap();
    let link = dir.path().join("redirect");
    symlink(&target, &link).unwrap();
    assert!(RunLock::acquire(&link).is_err());
    assert!(rldyour_updater::storage::replace(&link.join("receipt.json"), b"bad").is_err());
    assert!(!target.join("receipt.json").exists());
    assert!(rldyour_updater::storage::read_bounded(&target, 64).is_err());
}
#[cfg(unix)]
#[test]
fn reports_and_locks_are_private_and_temporary_publish_cannot_overwrite_redirect() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let dir = temp();
    let path = dir.path().join("report.json");
    rldyour_updater::storage::replace(&path, b"{}").unwrap();
    assert_eq!(path.metadata().unwrap().permissions().mode() & 0o777, 0o600);
    let other = dir.path().join("other");
    fs::write(&other, b"preserve").unwrap();
    let link = dir.path().join("last-run.json");
    symlink(&other, &link).unwrap();
    assert!(rldyour_updater::storage::replace(&link, b"{}").is_err());
    assert_eq!(fs::read(other).unwrap(), b"preserve");
}
#[cfg(unix)]
#[test]
fn runner_drains_large_output_and_deadline_contains_descendants() {
    let runner = RealCommandRunner;
    let output = runner.run(
        &[
            "/bin/sh".into(),
            "-c".into(),
            "yes output | head -c 262144".into(),
        ],
        None,
        Duration::from_secs(3),
    );
    assert!(output.ok);
    assert!(output.truncated);
    assert!(output.stdout.len() <= 131072);
    let start = Instant::now();
    let output = runner.run(
        &["/bin/sh".into(), "-c".into(), "sleep 30 & wait".into()],
        None,
        Duration::from_millis(150),
    );
    assert!(!output.ok);
    assert!(start.elapsed() < Duration::from_secs(3));
}
