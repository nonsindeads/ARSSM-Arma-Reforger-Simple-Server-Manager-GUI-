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
CREDENTIALS_PATH="$ARSSM_CONFIG_DIR/credentials.json"

echo "Installing system dependencies via apt..."
sudo apt-get update -y
sudo apt-get install -y curl tar lib32gcc-s1 lib32stdc++6 ca-certificates openssl

if ! command -v cargo >/dev/null 2>&1; then
  echo "Installing Rust toolchain (cargo, rustc) via apt..."
  sudo apt-get install -y cargo rustc
else
  echo "Rust toolchain already installed."
fi

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

if [[ ! -s "$CREDENTIALS_PATH" ]]; then
  USERNAME="$(tr -dc 'a-zA-Z0-9' </dev/urandom | head -c 8 || true)"
  PASSWORD="$(tr -dc 'a-zA-Z0-9' </dev/urandom | head -c 20 || true)"
  cat > "$CREDENTIALS_PATH" <<EOF
{
  "username": "$USERNAME",
  "password": "$PASSWORD"
}
EOF
  echo "Generated credentials:"
  echo "  Username: $USERNAME"
  echo "  Password: $PASSWORD"
  echo "Stored at: $CREDENTIALS_PATH"
else
  echo "Credentials already exist at $CREDENTIALS_PATH (not overwriting)."
  echo "Show them with: cat \"$CREDENTIALS_PATH\""
fi

echo "Initializing SteamCMD (first run)..."
STEAMCMD_BIN="$STEAMCMD_DIR/steamcmd.sh"
if [[ ! -x "$STEAMCMD_BIN" ]]; then
  chmod +x "$STEAMCMD_BIN"
fi

echo "Running: $STEAMCMD_BIN +login anonymous +quit"
"$STEAMCMD_BIN" \
  +login anonymous \
  +quit

echo "Installing Arma Reforger server via SteamCMD (appid 1874900)..."
echo "Running: $STEAMCMD_BIN +force_install_dir \"$SERVER_DIR\" +login anonymous +app_update 1874900 validate +quit"
"$STEAMCMD_BIN" \
  +force_install_dir "$SERVER_DIR" \
  +login anonymous \
  +app_update 1874900 validate \
  +quit
echo "SteamCMD finished."

if [[ ! -f "$SERVER_DIR/ArmaReforgerServer" ]]; then
  echo "Warning: server binary not found at $SERVER_DIR/ArmaReforgerServer"
  echo "Check SteamCMD output above for errors."
fi

echo "Setup complete."
echo "Next steps:"
echo "1) Start ARSSM and review Settings."
