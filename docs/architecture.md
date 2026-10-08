# Architecture

The Rust oneshot separates policy, release trust, orchestration, providers,
process ownership and private storage. No shared system-health subsystem exists.

- config: strict schema-2 TOML, platform/identity boundaries, no executable
  command templates. Root policy must be root-owned and nonwritable by others.
- catalog: typed domain-separated Ed25519 verification; source commit, complete
  SHA256/length table, issued/expiry times and monotonic sequence. Unknown fields,
  altered signatures, oversized metadata, expired/future releases, lower sequence,
  same-sequence equivocation and backwards clocks are refused.
- release/transport: bounded credential-free HTTPS download, exact raw source
  commit, verified bytes before any Python runs. Executed sources come from a
  private temporary snapshot, never a developer checkout. Acceptance is persisted
  before execution; equal sequence/digest remains retryable after failure.
- providers/gds: fixed isolated Python argv (-I -B) for reviewed bootstrap-owned
  installation and launcher operations. Structured receipts bind all nine CLI
  identities or the eight desktop classifications. No model calls or user profile
  mutations. Missing or truncated receipts are failures.
- providers/deb: root-only reviewed packages, installed-only selection, native
  version comparison, exact downloaded hash/length and Debian metadata. APT holds
  package locks, waits at most 30 seconds, and refuses removals. No downgrades.
- providers/native: package owners retain their own policy and update semantics.
  Homebrew excludes GDS-managed harnesses/Herdr, keeps pins, requires cask hashes,
  and does not quit apps. APT is observation-only in this Rust process.
- process/os: bounded streaming output and a common execution/drain deadline.
  POSIX owns process groups; Windows starts suspended in a kill-on-close Job Object
  and resumes only after assignment. Exit codes and truncation are retained.
- storage/lockfile/report: nonredirected bounded regular files, no-follow opens
  on POSIX, private exclusive native lock and atomic durable publication. One
  last report and one acceptance record; no archive backups.
- platforms: Linux user timer and separate root package timer, macOS launchd.
  Windows process code is compiled/tested, but no Windows bootstrap or scheduler
  support is claimed by this release.

The offline signer remains outside the clients. Changing trust requires explicit
local reprovisioning. Network interruption can deny updates, but cannot authorize
unsigned or stale source. Deleting local acceptance state removes rollback memory;
this protocol does not defend against a malicious local administrator.
