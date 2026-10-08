#!/bin/sh
set -eu
ROOT="$(unset CDPATH; cd -- "$(dirname -- "$0")" && pwd)"
BIN="$HOME/.local/bin/rldyour-updater"
CONFIG_HOME=${XDG_CONFIG_HOME:-"$HOME/.config"}
STATE_HOME=${XDG_STATE_HOME:-"$HOME/.local/state"}
CONFIG="$CONFIG_HOME/rldyour-updater/config.toml"
command -v cargo >/dev/null 2>&1 || { echo "cargo is required" >&2; exit 1; }
cargo build --release --locked --manifest-path "$ROOT/Cargo.toml"
mkdir -p "$HOME/.local/bin" "$CONFIG_HOME/rldyour-updater" "$STATE_HOME/rldyour-updater"
install -m 0755 "$ROOT/target/release/rldyour-updater" "$BIN"
if [ ! -e "$CONFIG" ] || ! "$BIN" --config "$CONFIG" doctor >/dev/null 2>&1; then
  # A policy migration requires an explicitly provisioned public trust anchor.
  # No remote response is ever allowed to choose this key.
  key=${RLDYOUR_UPDATER_PUBLIC_KEY:-}
  if [ -z "$key" ] && [ -f "$ROOT/catalog/public-key.hex" ]; then
    key=$(cat "$ROOT/catalog/public-key.hex")
  fi
  [ -n "$key" ] || { echo "provide RLDYOUR_UPDATER_PUBLIC_KEY before arming updates" >&2; exit 1; }
  feed=${RLDYOUR_UPDATER_FEED:-https://raw.githubusercontent.com/NDDev-OpenNetwork/rldyour-updater/main/catalog/stable.json}
  python=$(command -v python3)
  [ "$(uname -s)" = Linux ] && python=/usr/bin/python3
  "$BIN" --config "$CONFIG" configure --release-url "$feed" --public-key "$key" --python "$python" --replace-policy
fi
"$BIN" --config "$CONFIG" doctor
case "$(uname -s)" in
  Linux)
    mkdir -p "$CONFIG_HOME/systemd/user"
    install -m 0644 "$ROOT/platforms/linux/systemd/rldyour-updater.service" "$CONFIG_HOME/systemd/user/"
    install -m 0644 "$ROOT/platforms/linux/systemd/rldyour-updater.timer" "$CONFIG_HOME/systemd/user/"
    systemctl --user daemon-reload
    systemctl --user enable --now rldyour-updater.timer
    ;;
  Darwin)
    mkdir -p "$HOME/Library/LaunchAgents" "$HOME/Library/Logs"
    # Use the stdlib plist encoder so spaces/XML metacharacters remain valid.
    python=$(command -v python3)
    "$python" -I -B - "$ROOT" "$BIN" "$HOME" <<'PY'
import plistlib, sys
from pathlib import Path
root, binary, home = map(Path, sys.argv[1:])
data = plistlib.loads((root/'platforms/macos/launchd/io.nddev.rldyour-updater.plist').read_bytes())
data['ProgramArguments'] = [str(binary), 'apply']
data['StandardOutPath'] = str(home/'Library/Logs/rldyour-updater.log')
data['StandardErrorPath'] = data['StandardOutPath']
(home/'Library/LaunchAgents/io.nddev.rldyour-updater.plist').write_bytes(plistlib.dumps(data))
PY
    launchctl bootout "gui/$(id -u)/io.nddev.rldyour-updater" 2>/dev/null || true
    launchctl bootstrap "gui/$(id -u)" "$HOME/Library/LaunchAgents/io.nddev.rldyour-updater.plist"
    ;;
  *) echo "no scheduler installer for this platform" >&2; exit 1 ;;
esac
echo "installed updater; policy: $CONFIG"
