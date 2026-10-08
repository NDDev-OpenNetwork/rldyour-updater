use ed25519_dalek::{Signer, SigningKey};
use rldyour_updater::{
    catalog::{self, Envelope, Payload, SourceFile},
    config::Policy,
    model::unix_seconds,
    process::{CommandOutput, CommandRunner},
    providers, release,
};
use std::{cell::RefCell, fs, path::Path, time::Duration};
struct Fixture {
    feed: Vec<u8>,
    calls: RefCell<Vec<Vec<String>>>,
    corrupt: bool,
    receipt: bool,
}
impl CommandRunner for Fixture {
    fn run(&self, argv: &[String], _cwd: Option<&Path>, _timeout: Duration) -> CommandOutput {
        self.calls.borrow_mut().push(argv.to_vec());
        let mut stdout = String::new();
        if argv.iter().any(|a| a == "--url") {
            let url = &argv[argv.iter().position(|a| a == "--url").unwrap() + 1];
            let dest = &argv[argv.iter().position(|a| a == "--output").unwrap() + 1];
            let bytes = if url.ends_with("catalog.json") {
                self.feed.clone()
            } else if self.corrupt {
                b"evil".to_vec()
            } else {
                b"test".to_vec()
            };
            fs::write(dest, bytes).unwrap();
        } else if argv.iter().any(|a| a.ends_with("managed_cli.py")) && self.receipt {
            stdout=serde_json::json!({"schema":1,"components": (["antigravity","claude-code","codex","cursor","grok-build","opencode","pi","devin","gddy"].iter().map(|id|serde_json::json!({"id":id,"changed":false})).collect::<Vec<_>>()),"vendor_configuration_mutated":false}).to_string();
        }
        CommandOutput {
            ok: true,
            stdout,
            stderr: String::new(),
            truncated: false,
            exit_code: Some(0),
        }
    }
}
fn setup() -> (tempfile::TempDir, Policy, Fixture) {
    let parent = std::env::temp_dir().canonicalize().unwrap();
    let dir = tempfile::tempdir_in(parent).unwrap();
    let key = SigningKey::from_bytes(&[9; 32]);
    let now = unix_seconds();
    let value = Payload {
        schema: 1,
        debs: vec![],
        updater_binaries: vec![],
        sequence: 1,
        issued_at: now - 1,
        expires_at: now + 1000,
        bootstrap_commit: "a".repeat(40),
        files: catalog::FILES
            .iter()
            .map(|p| SourceFile {
                path: (*p).into(),
                bytes: 4,
                sha256: catalog::sha256(b"test"),
            })
            .collect(),
    };
    let signature = catalog::encode_hex(&key.sign(&value.signed_bytes().unwrap()).to_bytes());
    let feed = serde_json::to_vec(&Envelope {
        payload: value,
        signature,
    })
    .unwrap();
    let mut policy = Policy::defaults();
    policy.state_dir = Some(dir.path().join("state"));
    policy.gds.enabled = true;
    policy.release.url = "https://example.invalid/catalog.json".into();
    policy.release.public_key = catalog::encode_hex(key.verifying_key().as_bytes());
    policy.gds.python = "/usr/bin/python3".into();
    (
        dir,
        policy,
        Fixture {
            feed,
            calls: RefCell::new(vec![]),
            corrupt: false,
            receipt: true,
        },
    )
}
#[test]
fn corrupt_source_is_refused_before_any_installer_execution() {
    let (_dir, policy, mut runner) = setup();
    runner.corrupt = true;
    assert!(
        release::fetch(&policy, &runner)
            .unwrap_err()
            .contains("integrity mismatch")
    );
    assert!(
        runner
            .calls
            .borrow()
            .iter()
            .all(|call| call.iter().any(|a| a == "--url"))
    );
}
#[test]
fn repeated_receipt_reports_verified_without_claiming_mutation() {
    let (_dir, policy, runner) = setup();
    let source = release::fetch(&policy, &runner).unwrap();
    let report = providers::gds::run(&policy, &source, &runner);
    assert!(report.ok);
    assert_eq!(report.changed, Some(false));
    assert_eq!(report.state, "verified");
    assert!(
        runner
            .calls
            .borrow()
            .iter()
            .filter(|call| !call.iter().any(|a| a == "--url"))
            .all(|call| call.contains(&"-I".into()) && call.contains(&"-B".into()))
    );
}
#[test]
fn empty_installer_output_cannot_be_a_successful_update() {
    let (_dir, policy, mut runner) = setup();
    runner.receipt = false;
    let source = release::fetch(&policy, &runner).unwrap();
    let result = providers::gds::run(&policy, &source, &runner);
    assert!(!result.ok);
    assert_eq!(result.state, "failed");
}
#[test]
fn invalid_ledger_blocks_before_network() {
    let (_dir, policy, runner) = setup();
    fs::create_dir_all(policy.state_dir()).unwrap();
    fs::write(policy.state_dir().join("accepted-catalog.json"), b"broken").unwrap();
    assert!(release::fetch(&policy, &runner).is_err());
    assert!(runner.calls.borrow().is_empty());
}
