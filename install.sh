#!/bin/sh
set -eu

ROOT="$(unset CDPATH; cd -- "$(dirname -- "$0")" && pwd)"
BIN="$HOME/.local/bin/rldyour-updater"
CONFIG_HOME=${XDG_CONFIG_HOME:-"$HOME/.config"}
CONFIG_DIR="$CONFIG_HOME/rldyour-updater"
CONFIG="$CONFIG_DIR/config.toml"
STATE_HOME=${XDG_STATE_HOME:-"$HOME/.local/state"}

command -v cargo >/dev/null 2>&1 || { echo "cargo is required to build rldyour-updater" >&2; exit 1; }
cargo build --locked --release --manifest-path "$ROOT/Cargo.toml"
mkdir -p "$HOME/.local/bin" "$CONFIG_DIR" "$STATE_HOME/rldyour-updater"
install -m 0755 "$ROOT/target/release/rldyour-updater" "$BIN"

if [ ! -e "$CONFIG" ]; then
  estate=
  for candidate in \
    "$HOME/Developer/NDDev-it-com/github-device-sync-estate" \
    "$HOME/Developer/control-plane/github-device-sync-estate"
  do
    if [ -f "$candidate/modules/macos-ubuntu-bootstrap/scripts/managed_cli.py" ]; then estate=$candidate; break; fi
  done
  platform=macos
  [ "$(uname -s)" = Linux ] && platform=ubuntu
  python=/usr/bin/python3
  if [ "$platform" = macos ]; then
    python=$(command -v python3 || true)
    [ -n "$python" ] || python=/usr/bin/python3
  fi
  {
    printf 'schema = 1\nchannel = "signed-gds"\ncommand_timeout_seconds = 600\n\n'
    if [ -n "$estate" ]; then
      printf '[gds]\nenabled = true\nplatform = "%s"\nargv = ["%s", "%s/modules/macos-ubuntu-bootstrap/scripts/managed_cli.py", "install", "--platform", "%s"]\n\n' "$platform" "$python" "$estate" "$platform"
    else
      printf '[gds]\nenabled = false\nplatform = "%s"\nargv = []\n\n' "$platform"
    fi
    printf '[apt]\nobserve = true\napply = false\n\n[toolchains]\nobserve_only = true\n'
  } > "$CONFIG"
  chmod 0600 "$CONFIG"
fi

case "$(uname -s)" in
  Linux)
    command -v systemctl >/dev/null 2>&1 || { echo "systemctl is required on Linux" >&2; exit 1; }
    mkdir -p "$CONFIG_HOME/systemd/user"
    install -m 0644 "$ROOT/platforms/linux/systemd/rldyour-updater.service" "$CONFIG_HOME/systemd/user/"
    install -m 0644 "$ROOT/platforms/linux/systemd/rldyour-updater.timer" "$CONFIG_HOME/systemd/user/"
    systemctl --user daemon-reload
    systemctl --user enable --now rldyour-updater.timer
    ;;
  Darwin)
    mkdir -p "$HOME/Library/LaunchAgents" "$HOME/Library/Logs"
    sed -e "s|__BIN__|$BIN|g" -e "s|__HOME__|$HOME|g" "$ROOT/platforms/macos/launchd/io.nddev.rldyour-updater.plist" > "$HOME/Library/LaunchAgents/io.nddev.rldyour-updater.plist"
    launchctl bootout "gui/$(id -u)/io.nddev.rldyour-updater" 2>/dev/null || true
    launchctl bootstrap "gui/$(id -u)" "$HOME/Library/LaunchAgents/io.nddev.rldyour-updater.plist"
    ;;
  *) echo "unsupported platform: $(uname -s)" >&2; exit 1 ;;
esac

"$BIN" doctor
echo "installed $BIN; policy: $CONFIG"
