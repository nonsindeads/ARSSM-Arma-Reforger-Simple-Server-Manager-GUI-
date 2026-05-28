# ARSSM – Arma Reforger Simple Server Manager

A local-first web-based manager for Arma Reforger dedicated servers.
Provide a workshop URL, pick a scenario — ARSSM resolves all mod dependencies, generates a valid `server.json`, and keeps it in sync.

**Stack:** Rust (Axum + Tokio) · HTMX · Bootstrap 5 · Server-Sent Events for live log streaming

---

## Web UI

ARSSM exposes a full browser-based management interface at `https://<host>:3000`.

| Section | What you can do |
|---|---|
| **Dashboard** | Server status, quick start/stop, system metrics (CPU/RAM) |
| **Profiles** | Create and manage server profiles per workshop mod |
| **Workshop** | Resolve mod dependencies and select scenarios from a URL |
| **Packages** | Group mods into reusable optional presets |
| **Run & Logs** | Start/stop the server, watch live output via SSE stream |
| **Settings** | Configure paths (SteamCMD, server exe, work dir) and global server.json defaults |

Authentication: HTTP Basic Auth. Credentials are auto-generated on first start.

---

## Quick Start

### Linux (Ubuntu 22.04)

```bash
# 1. Run setup once — installs deps, SteamCMD, Arma Reforger server, writes defaults
bash scripts/setup-linux.sh

# 2. Build and start ARSSM
cd backend && cargo build --release
./target/release/backend

# 3. Open the UI
# https://<host>:3000
# Credentials: ~/.config/arssm/credentials.json
```

### Windows

```
1. Build the backend: cd backend && cargo build --release
2. Run: target\release\backend.exe
3. Open: https://localhost:3000
4. Credentials: %APPDATA%\arssm\credentials.json
```

> **Note:** ARSSM uses a self-signed TLS certificate. Your browser will show a security warning — this is expected for a local-network tool.

### Reset credentials

```bash
# Linux
bash scripts/reset-credentials.sh

# Windows
# Delete %APPDATA%\arssm\credentials.json and restart
```

---

## How It Works

1. **Create a profile** — paste a workshop mod URL (e.g. from reforger.armaplatform.com)
2. **Resolve** — ARSSM fetches the mod page, recursively resolves all dependency mod IDs, and lists available scenarios
3. **Select a scenario** — pick the mission you want to run
4. **Generate config** — ARSSM writes a complete `server.json` under your server work directory
5. **Start** — click Start in the Run tab; live logs stream directly to your browser

Config is regenerated fresh on every server start so profile changes always take effect.

---

## Data Storage

All application data is stored locally. No cloud, no telemetry.

| Platform | Path |
|---|---|
| Linux | `~/.config/arssm/` |
| Windows | `%APPDATA%\arssm\` |

```
~/.config/arssm/
├── settings.json          # Paths and global server.json defaults
├── profiles/              # One JSON file per server profile
├── mods.json              # Custom mod registry
├── packages.json          # Mod package presets
├── credentials.json       # HTTP Basic Auth credentials
├── certs/                 # Auto-generated self-signed TLS cert
└── logs/                  # Per-profile server logs (timestamped)
```

Generated server configs are written to `configs/<profile_id>/server.json` inside your Reforger server work directory.

---

## API

The backend exposes a small JSON API alongside the web UI:

```
POST /api/workshop/resolve    Resolve a workshop URL → root ID, scenarios, dependency IDs
GET  /api/run/status          Current run state (running, pid, profile, uptime)
POST /api/run/start           Start the server (optionally specify a profile_id)
POST /api/run/stop            Stop the server
GET  /api/run/logs/tail       Last N log lines (?n=200)
GET  /api/run/logs/stream     SSE stream of live log output
GET  /health                  Health check
```

**Workshop resolve example:**
```json
POST /api/workshop/resolve
{
  "url": "https://reforger.armaplatform.com/workshop/595F2BF2F44836FB-RHS-StatusQuo",
  "max_depth": 5
}
```

---

## Building

Requires Rust 1.80+.

```bash
cd backend
cargo build --release
```

The binary embeds all templates and the baseline `server.sample.json` — no additional files needed at runtime.

---

## License

MIT License — Copyright (c) 2026 Simon Glashauser

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
