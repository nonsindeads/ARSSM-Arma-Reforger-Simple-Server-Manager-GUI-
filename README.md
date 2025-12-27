# ARSSM – Arma Reforger Simple Server Manager

ARSSM (Arma Reforger Simple Server Manager) is a local-first management tool for Arma Reforger dedicated servers.
It allows you to create and manage server profiles by simply providing a workshop link and selecting a scenario.
ARSSM automatically resolves all required mod dependencies, generates valid `server.json` configurations, and keeps them in sync over time.

The application focuses on clarity, determinism, and automation:
- No cloud services
- No remote dependencies at runtime
- No heavyweight UI frameworks

ARSSM is built with a Rust backend and a minimal local web interface, designed to run on Windows hosts while remaining portable to Linux environments.

## Key features

- Create server profiles from Arma Reforger workshop URLs
- Automatic recursive mod dependency resolution
- Scenario discovery and selection
- Deterministic `server.json` generation
- Profile-based configuration storage
- Optional mod presets
- Dependency change detection on server start
- Local web UI (no Electron, no SPA)

## Project goals

- Reduce manual work when setting up Arma Reforger servers
- Prevent broken servers caused by missing or changed mod dependencies
- Keep server configuration transparent and reproducible
- Stay lightweight, inspectable, and easy to automate

## Non-goals

- Game server hosting service
- Cloud-based management platform
- Monolithic GUI application

## Config storage

The backend stores configuration in a single JSON file.

- Default path: `config/app_config.json` relative to the repository root.
- Override with `ARSSM_CONFIG_PATH` to point somewhere else.

The web UI directory can be overridden with `ARSSM_WEB_DIR`.

## Settings storage

Settings are stored under the per-user app data directory:
- Windows: `%APPDATA%\arssm\settings.json`
- Fallback: `~/.config/arssm/settings.json`

## Workshop resolver

`POST /api/workshop/resolve` resolves a workshop URL into the root ID, available scenarios,
and recursive dependency IDs.

Request:
```json
{
  "url": "https://reforger.armaplatform.com/workshop/595F2BF2F44836FB-RHS-StatusQuo",
  "max_depth": 5
}
```

`GET /health` returns plain `ok` for non-browser clients and provides a small HTML test UI
when accessed via a browser (Accept: `text/html`).

## Profiles

Profiles are stored as JSON files under the app data `profiles/` directory.

## Config generation

Baseline config: `backend/assets/server.sample.json`.
Generated configs are written to `profiles/<profile_id>/generated/server.json`.

## Run & Logs

The backend exposes basic run endpoints and an SSE log stream:
- `POST /api/run/start`
- `POST /api/run/stop`
- `GET /api/run/status`
- `GET /api/run/logs/stream`

## SteamCMD update (placeholder)

`POST /api/steamcmd/update` returns a placeholder response for now.

## Theme tokens

Badge-aligned palette lives in `web/css/theme.css`. Use the `--arssm-*` tokens for all new UI:
- `--arssm-bg`, `--arssm-surface`, `--arssm-border`
- `--arssm-text`, `--arssm-muted`
- `--arssm-accent` (primary actions, focus, warnings only)

## License

MIT License

Copyright (c) 2025 ARSSM

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
