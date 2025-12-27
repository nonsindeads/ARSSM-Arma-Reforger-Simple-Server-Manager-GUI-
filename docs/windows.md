# Windows Installation

## Requirements

- Windows 10 or newer
- Rust toolchain (stable)
- Git
- SteamCMD
- Arma Reforger Dedicated Server

## Install SteamCMD and the server

1) Download SteamCMD and extract it to a folder, e.g. `C:\steamcmd`.
2) Run SteamCMD and install the Arma Reforger Dedicated Server.
3) Note the server executable path, e.g.
   `C:\steamcmd\steamapps\common\Arma Reforger Server\ArmaReforgerServer.exe`.

## Build and run ARSSM

```powershell
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

## Configure ARSSM

1) Open `Settings` in the UI.
2) Set:
   - SteamCMD directory
   - Arma Reforger server executable path
   - Arma Reforger server working directory
   - Profile base directory (where logs and storage live)
3) Save settings.

## Data location

Settings and profiles are stored under:
```
%APPDATA%\arssm\
```
