# ARSSM – Arma Reforger Simple Server Manager

## Config storage

The backend stores configuration in a single JSON file.

- Default path: `config/app_config.json` relative to the repository root.
- Override with `ARSSM_CONFIG_PATH` to point somewhere else.

The web UI directory can be overridden with `ARSSM_WEB_DIR`.

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
