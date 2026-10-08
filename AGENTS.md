# rldyour-updater — agent instructions

Rust 2024 coordinator for workstation updates. It is a public OpenNetwork
repository and must contain no host, account, estate or credential data.

## Ownership model

- GDS owns the eight vendor CLIs. The updater may invoke only the local
  estate's reviewed managed_cli.py install --platform ... argv.
- APT owns Debian/Ubuntu packages and its own locks. The updater observes the
  plan by default; root mutation is an explicit system-policy decision and is
  never enabled by the user installer.
- Snap, Flatpak, Homebrew, vendor self-updaters, CUDA, vLLM and model stores
  remain their own owners or observation-only until a signed provider contract
  exists. The updater never downloads arbitrary release assets.
- Vendor configuration, credentials, sessions and project trees are outside
  the updater boundary.

## Invariants

- plan is read-only.
- apply serializes one run with a private lock, executes only argv vectors,
  bounds output and time, and atomically replaces one private report.
- A malformed or incomplete policy fails closed.
- channel is signed-gds; a GDS action must contain managed_cli.py, install,
  and --platform.
- No shell interpolation, curl | sh, arbitrary manifest command, recursive
  file deletion, backup archive or automatic reboot.
- An update report distinguishes observation, mutation and failure. A
  successful schedule is not evidence that a component was updated.

## Verification

    cargo fmt --check
    cargo test --locked
    cargo clippy --locked --all-targets -- -D warnings
    shellcheck install.sh uninstall.sh

Use synthetic command-runner fixtures for mutation tests. Do not inspect real
clipboard, credentials, model sessions or private estate data.
