# rldyour-updater

The fourth OpenNetwork workstation tool coordinates updates without becoming a
second package manager:

| Owner | What the updater does |
| --- | --- |
| Signed GDS release | Applies the pinned estate managed_cli.py install --platform ... transaction for the eight CLIs and refreshes no vendor settings. |
| Ubuntu APT / unattended-upgrades | Observes the upgrade plan; the stock root apt-daily-upgrade.timer remains the mutation owner. A separately reviewed root policy may opt in to apt-get update and apt-get --with-new-pkgs upgrade. |
| CUDA, vLLM, Ollama, model stores | Reports presence/version only. These need a compatibility-aware signed release before unattended mutation. |
| macOS Homebrew and launchd | The schedule is installed, but vendor package mutation is not guessed. Add a provider only with an ownership and pin contract. |

This makes automatic updates reproducible: a daily user timer runs the exact
signed GDS release already pinned by the private estate, while OS package
updates continue through their native scheduler. There is no curl | sh,
arbitrary download URL, vendor configuration edit, forced reboot, or
passwordless sudo grant.

## Commands

    rldyour-updater doctor
    rldyour-updater plan
    rldyour-updater apply
    rldyour-updater status --json

plan never launches a provider. apply uses a private lock and writes only
$XDG_STATE_HOME/rldyour-updater/last-run.json (normally
~/.local/state/rldyour-updater/last-run.json).

## Policy

The installer creates $XDG_CONFIG_HOME/rldyour-updater/config.toml. It binds
GDS only when it finds a local estate checkout and otherwise leaves GDS
disabled with a clear doctor error. A valid Ubuntu policy looks like:

    schema = 1
    channel = "signed-gds"
    command_timeout_seconds = 600

    [gds]
    enabled = true
    platform = "ubuntu"
    argv = [
      "/usr/bin/python3",
      "/home/you/Developer/NDDev-it-com/github-device-sync-estate/modules/macos-ubuntu-bootstrap/scripts/managed_cli.py",
      "install",
      "--platform",
      "ubuntu",
    ]

    [apt]
    observe = true
    apply = false

    [toolchains]
    observe_only = true

The GDS script itself verifies exact reviewed bytes and its signed/pinned
release contract. The Rust tool does not copy or expose any credentials.

## Install

    ./install.sh

The installer builds with the locked Cargo graph, installs the user binary and
arms a daily systemd user timer on Linux or a launchd user agent on macOS. It
does not use sudo. uninstall.sh removes only this binary and schedule; policy
and the last report are retained for review.
