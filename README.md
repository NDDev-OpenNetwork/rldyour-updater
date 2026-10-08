# rldyour-updater

Rust 2024 coordinator for signed workstation updates. Current version: **0.2.0**.
It runs as a short-lived scheduled process, with no model supervisor or general
system diagnostic aggregation.

## Update ownership

| Component | Owner and behavior |
| --- | --- |
| Eight vendor CLIs and GoDaddy CLI | Signed catalogue binds an exact bootstrap commit and SHA256/length of every installer/config file. Installer verifies the complete program payload and returns actual change receipts. |
| Full-auto launchers | Verified bootstrap companion; unchanged launchers are checked without rewriting. No vendor configuration or credentials are edited. |
| Ubuntu repositories, drivers and ESM | Native unattended-upgrades and stock APT timers. Updater only observes their package plan. Third-party repository eligibility must be explicitly configured. |
| Snap and Telegram | Their native update mechanisms; updater does not create a competing owner. |
| OpenCode Desktop, standalone ChatGPT package, Antigravity | Separate root-owned signed application transaction updates only already installed catalogue apps, rejects downgrades and disallows package removal. |
| Other reviewed Debian apps, including Happ | Signed package identity/version/size/SHA256 in the catalogue; validated before apt; absent packages are preserved. |
| macOS Homebrew apps/tools | Optional native owner provider, respecting pins, skipping GDS-managed harnesses and Herdr. Casks require hashes and are never forcibly quit. |
| CUDA/vLLM/Ollama/model stores | Existing native repository maintenance where applicable, otherwise explicit compatibility review. No blind Python environment rebuild, model download or running server restart. |

Automatic means **consuming new approved catalogue releases**. Vendor releases
are not silently promoted to approved releases. Catalogue publication is a
maintainer operation, with a maximum 31-day lifetime; clients fail visibly on
expiry rather than executing stale or unsigned content. This is a domain-separated
Ed25519 protocol with rollback/freeze checks, **not a full TUF implementation**.

## Installation

Review the source and signer public-key fingerprint before initial provisioning:

    RLDYOUR_UPDATER_PUBLIC_KEY=<reviewed-Ed25519-public-key-hex> ./install.sh

The trusted key is written to the local policy, never learned from an update
response. Source installs use Cargo.lock. Linux uses a persistent randomized user
timer; macOS uses a launchd calendar agent. The normal installer never uses sudo
or arms root package updates. To enable system applications, an administrator
installs the same binary in /usr/local/bin, provisions a root-owned policy in
/etc/rldyour-updater/config.toml with gds.applications=true and gds.enabled=false,
and enables the included rldyour-updater-system.timer.

Existing schema-1 policies require explicit migration to schema 2. Legacy argv
is never executed. A bad policy never enables the scheduler. User/root reports,
locks and acceptance state are separate. No backups of user files are made.

## Commands

    rldyour-updater plan --json     # policy-only, read-only, no network
    rldyour-updater apply --json    # fetch, authenticate, apply, record
    rldyour-updater status --json   # last actual outcome
    rldyour-updater doctor          # updater policy only

Reports distinguish verified, updated, native-owner, not-installed and failed.
changed=null means the native owner or failed partial transaction cannot establish
an exact mutation result. Success is never inferred from scheduler registration.
Root application status uses --config /etc/rldyour-updater/config.toml.

See [architecture](docs/architecture.md) and [publishing](docs/publishing.md).
