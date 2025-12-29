#!/usr/bin/env bash
set -euo pipefail

ARSSM_CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/arssm"
CREDENTIALS_PATH="$ARSSM_CONFIG_DIR/credentials.json"

mkdir -p "$ARSSM_CONFIG_DIR"

USERNAME="$(tr -dc 'a-zA-Z0-9' </dev/urandom | head -c 8)"
PASSWORD="$(tr -dc 'a-zA-Z0-9' </dev/urandom | head -c 20)"

cat > "$CREDENTIALS_PATH" <<EOF
{
  "username": "$USERNAME",
  "password": "$PASSWORD"
}
EOF

echo "New credentials:"
echo "  Username: $USERNAME"
echo "  Password: $PASSWORD"
