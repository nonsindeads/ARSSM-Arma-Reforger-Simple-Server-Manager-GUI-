# Linux Installation

Linux support is planned. The current MVP targets Windows first.

## Development usage (Linux)

If you want to build and run the backend for development:

```bash
git clone <repo-url>
cd ARSSM-Arma-Reforger-Simple-Server-Manager-GUI-
cd backend
cargo build
cargo run
```

Open the UI at:
```
http://127.0.0.1:3000/
```

## Notes

- The server process integration is Windows-first.
- Config and settings paths will use Linux defaults later:
  `~/.config/arssm/`
# Linux (Ubuntu 22.04) Setup

ARSSM is designed to run as a local service with a small web UI.
On Linux, we install SteamCMD in the user home and keep all data under `~/.local/share/arssm`.

## First-time setup (recommended)

Run the setup script once:

```bash
bash scripts/setup-linux.sh
```

This will:
- install required system packages via `apt`
- download SteamCMD into `~/.local/share/arssm/steamcmd`
- write default ARSSM settings to `~/.config/arssm/settings.json`

## Install Arma Reforger Server

Use SteamCMD to install the dedicated server into:
`~/.local/share/arssm/arma-reforger-server`

Example (appid 1874880):

```bash
~/.local/share/arssm/steamcmd/steamcmd.sh \
  +force_install_dir ~/.local/share/arssm/arma-reforger-server \
  +login anonymous \
  +app_update 1874880 validate \
  +quit
```

## Defaults written by setup

- SteamCMD directory: `~/.local/share/arssm/steamcmd`
- Reforger server executable: `~/.local/share/arssm/arma-reforger-server/ArmaReforgerServer`
- Reforger server work dir: `~/.local/share/arssm/arma-reforger-server`
- Profile base directory: `~/.local/share/arssm/profiles`

You can override these in the ARSSM Settings page.
