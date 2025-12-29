# Installation Guide

This guide covers the common steps for installing and running ARSSM locally.
Platform-specific details are in `docs/windows.md` and `docs/linux.md`.

## Requirements

- Rust toolchain (stable)
- Git
- Arma Reforger dedicated server + SteamCMD (Windows)

## Steps

1) Clone the repository.
2) Build the backend:
```bash
cd backend
cargo build
```
3) Run the backend:
```bash
cargo run
```
4) Open the UI in a browser:
```
http://127.0.0.1:3000/
```

## First-time configuration

1) Open `Settings` in the UI.
2) Fill in the required paths for SteamCMD and the Reforger server.
3) Save settings.
4) Create a profile and resolve a workshop URL.
5) Generate the `server.json` and start the server.
# Installation

ARSSM is a local web app with a Rust backend. Choose your platform below.

## Windows

See `docs/windows.md`.

## Linux (Ubuntu 22.04)

See `docs/linux.md`. Use `scripts/setup-linux.sh` for the first-time setup.

## Security

ARSSM uses HTTPS with a self-signed certificate and HTTP Basic authentication.
The Linux setup script generates credentials in `~/.config/arssm/credentials.json`.
Use `scripts/reset-credentials.sh` to generate new credentials.
