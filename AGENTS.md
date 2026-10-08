# rldyour-updater — agent contract

Rust 2024 update coordinator; current release 0.2.0, policy schema 2. Public code
and synthetic fixtures only. Private topology, credentials and observations stay
outside this repository. Do not add a general system-health aggregator.

## Boundaries

- Signed catalogue, fixed reviewed bootstrap entrypoints and SHA256 source table
  precede execution. Developer checkouts and legacy policy argv never execute.
- GDS owns user CLIs/launchers; user updates must never run as root. Configuration,
  authentication and model sessions are preserved.
- Root application policy is separate and root-owned; only installed approved
  apps update. Refuse downgrades, removals, malformed identities and unknown state.
- APT/Snap retain native schedulers. Optional Homebrew respects pins and skips
  managed harnesses and Herdr; no forced app quit.
- Do not rebuild CUDA/vLLM/model environments blindly or restart healthy servers.
- No automatic reboot, arbitrary shell command, remote stream to shell, backups
  of user data, inferred deletion or silent trust reset.
- Output, deadlines, inherited descendant pipes, filesystem paths and state
  sizes are bounded. Lock/report destinations are validated before mutation.
- Reports distinguish verified/update/native-owner/failed; partial change is
  unknown, never an invented successful update.

## Verification

    cargo fmt --check
    cargo test --locked
    cargo clippy --locked --all-targets -- -D warnings
    cargo audit
    shellcheck install.sh uninstall.sh

CI runs native tests on Linux/macOS/Windows and the MSRV 1.88 check. Windows has
native contained processes but this release has no bootstrap/scheduler support
there. Synthetic negative tests must prove signature, expiry, sequence, path,
output/deadline and provider receipt refusal. Do not test against real user data.
