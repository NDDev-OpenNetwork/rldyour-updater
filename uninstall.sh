#!/bin/sh
set -eu
BIN="$HOME/.local/bin/rldyour-updater"
CONFIG_HOME=${XDG_CONFIG_HOME:-"$HOME/.config"}
case "$(uname -s)" in
  Linux)
    systemctl --user disable --now rldyour-updater.timer 2>/dev/null || true
    rm -f "$CONFIG_HOME/systemd/user/rldyour-updater.service" "$CONFIG_HOME/systemd/user/rldyour-updater.timer"
    systemctl --user daemon-reload 2>/dev/null || true
    ;;
  Darwin)
    launchctl bootout "gui/$(id -u)/io.nddev.rldyour-updater" 2>/dev/null || true
    rm -f "$HOME/Library/LaunchAgents/io.nddev.rldyour-updater.plist"
    ;;
esac
rm -f "$BIN"
echo "removed updater code and schedule; policy and reports were retained"
