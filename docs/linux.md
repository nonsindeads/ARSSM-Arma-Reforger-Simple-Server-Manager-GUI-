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
