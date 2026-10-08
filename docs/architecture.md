# Architecture

The updater is a short-lived Rust process started by the host scheduler. It
has no resident daemon and no network downloader.

1. config parses a deny-unknown-fields TOML policy and requires the signed-gds
   channel. A GDS action is valid only when its argv names the estate
   managed_cli.py, install, and --platform.
2. providers/gds invokes the pinned managed CLI and, from the same bootstrap
   checkout, the fixed ai_launchers.py install companion. It never constructs
   a shell command and never writes vendor configuration.
3. providers/apt observes an apt-get -s plan. APT mutation is root-only and
   opt-in; normal installation leaves it to Ubuntu unattended-upgrades.
4. providers/toolchain runs bounded version probes for vLLM, Ollama and CUDA.
   A failed probe is a review finding, not permission to repair or replace a
   compatibility-sensitive toolchain.
5. process owns process groups, output caps and deadlines. lockfile keeps one
   run per user. report writes one private JSON document atomically.
6. platforms contains only scheduler contracts: a persistent randomized
   systemd user timer for Linux and a calendar launchd agent for macOS.

The private estate remains the source of device paths, release pins and
credentials. The public repository carries only generic code and synthetic
policy examples.
