# Changelog

## [0.2.0] - 2026-10-08

- Authenticate an expiring Ed25519 catalogue with exact source SHA256/lengths,
  monotonic release sequence and durable anti-rollback/equivocation state.
- Execute verified private source snapshots; stop trusting mutable checkouts or
  arbitrary legacy argv. Add offline catalogue build/sign/verify commands.
- Add separate root-owned installed desktop/DEB update transactions and optional
  native Homebrew/Flatpak providers with clear update ownership.
- Retain real exit codes, exact CLI change receipts and unknown partial outcomes.
- Bound storage/lock/report paths and sizes, private atomic publication, process
  groups and Windows Job Objects. Add synthetic security/transaction regression tests.

## [0.1.0] - 2026-10-08

- Initial coordinator, policy/plan/status commands, GDS bootstrap invocation,
  APT observation and daily Linux/macOS user scheduling.
