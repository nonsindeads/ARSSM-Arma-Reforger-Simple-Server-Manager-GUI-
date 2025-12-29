#!/usr/bin/env bash
set -euo pipefail

if [[ "${OSTYPE:-}" != linux* ]]; then
  echo "This setup script is for Linux only."
  exit 1
fi

if ! command -v apt-get >/dev/null 2>&1; then
  echo "apt-get not found. This script supports Ubuntu/Debian."
  exit 1
fi

ARSSM_CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/arssm"
ARSSM_DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/arssm"
STEAMCMD_DIR="$ARSSM_DATA_DIR/steamcmd"
SERVER_DIR="$ARSSM_DATA_DIR/arma-reforger-server"
PROFILE_DIR="$ARSSM_DATA_DIR/profiles"

echo "Installing system dependencies via apt..."
sudo apt-get update -y
sudo apt-get install -y curl tar lib32gcc-s1 lib32stdc++6 ca-certificates

echo "Preparing directories..."
mkdir -p "$STEAMCMD_DIR" "$SERVER_DIR" "$PROFILE_DIR" "$ARSSM_CONFIG_DIR"

if [[ ! -f "$STEAMCMD_DIR/steamcmd.sh" ]]; then
  echo "Downloading SteamCMD..."
  curl -fsSL "https://steamcdn-a.akamaihd.net/client/installer/steamcmd_linux.tar.gz" -o /tmp/steamcmd_linux.tar.gz
  tar -xzf /tmp/steamcmd_linux.tar.gz -C "$STEAMCMD_DIR"
  rm -f /tmp/steamcmd_linux.tar.gz
else
  echo "SteamCMD already present at $STEAMCMD_DIR"
fi

SETTINGS_PATH="$ARSSM_CONFIG_DIR/settings.json"
if [[ ! -s "$SETTINGS_PATH" ]]; then
  echo "Writing default settings to $SETTINGS_PATH"
  cat > "$SETTINGS_PATH" <<EOF
{
  "steamcmd_dir": "$STEAMCMD_DIR",
  "reforger_server_exe": "$SERVER_DIR/ArmaReforgerServer",
  "reforger_server_work_dir": "$SERVER_DIR",
  "profile_dir_base": "$PROFILE_DIR",
  "active_profile_id": null,
  "server_json_defaults": null,
  "server_json_enabled": {}
}
EOF
else
  echo "Settings already exist at $SETTINGS_PATH (not overwriting)."
fi

echo "Setup complete."
echo "Next steps:"
echo "1) Install the server via SteamCMD into $SERVER_DIR (appid 1874880)."
echo "2) Start ARSSM and review Settings."
