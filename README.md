# tko

Minimal org-mode ticket system.

## Install

Download the latest binary release from:

https://github.com/boring-org-name/tko/releases/latest

Linux x86_64:

```sh
mkdir -p "$HOME/.local/bin"
curl -L https://github.com/boring-org-name/tko/releases/latest/download/tko-x86_64-unknown-linux-musl.tar.gz |
  tar -xz -C "$HOME/.local/bin"
"$HOME/.local/bin/tko" help
```

Add `$HOME/.local/bin` to `PATH` if you want `tko` available in new shells.

Windows x86_64, from PowerShell:

```powershell
$InstallDir = "$env:LOCALAPPDATA\tko\bin"
New-Item -ItemType Directory -Force $InstallDir | Out-Null
$Zip = "$env:TEMP\tko.zip"
Invoke-WebRequest -Uri "https://github.com/boring-org-name/tko/releases/latest/download/tko-x86_64-pc-windows-msvc.zip" -OutFile $Zip
Expand-Archive -Path $Zip -DestinationPath $InstallDir -Force
& "$InstallDir\tko.exe" help
```

Add `$InstallDir` to `PATH` if you want `tko` available in new shells.

## Build From Source

```sh
cargo test --locked
cargo build --release --locked
```

## Release

Push a version tag:

```sh
git tag v0.1.0
git push origin v0.1.0
```

The release workflow builds Linux and Windows binaries, uploads them as workflow artifacts, creates a GitHub Release for the tag, and attaches both archives plus `SHA256SUMS`.
