# Release builds and Steam

How Mud and Steel becomes a game players install: release export for macOS,
Windows and Linux, the CI workflow that builds it, signing, Steam upload,
logs, crash reports and licenses. Built in prompt 044.

Everything that needs accounts, certificates, money or a decision about the
store is done by **you** (the developer), never by CI or by an agent: the
Steam upload, the store page, prices, tax and bank data, Apple and Windows
certificates. This document says how, step by step.

## Overview

```
tools/release/build-lib.sh   cargo build --release (macOS universal) ─┐
                             + steam_api + THIRD_PARTY_LICENSES.txt   │ target/export/
tools/release/export.sh      Godot --export-release <preset>         <┘
                             + data/maps, data/scenarios, licenses   ─> build/export/<system>/
                             (macOS: tools/release/macos-sign.sh)        (SteamPipe content)
                             + archive                               ─> build/release/MudAndSteel-<version>-<system>.*
tools/release/smoke-test.sh  exported game --headless -- --self-test (the menu flow self-test)
tools/steam/upload.sh        SteamPipe scripts, steamcmd upload (manual only)
```

| Command | What it does | Needs |
|---------|--------------|-------|
| `just release-lib` | builds the bridge in the release profile and stages it in `target/export/` | Rust; for a universal macOS library `rustup target add x86_64-apple-darwin` |
| `just export pack` | exports only the `.pck` (all resources and scripts of the game) | Godot 4.7.2, no export templates |
| `just export-selftest pack` | plays the menu flow self-test from that `.pck` | as above |
| `just export macos` (`windows`, `linux`) | the real export: game, data, licenses, archive | Godot export templates 4.7.2; `release-lib` run on that system |
| `just export-selftest macos` | plays the self-test in the exported game | an export of the current system |
| `just steam-upload` | writes the SteamPipe scripts (dry run); `--upload` uploads | Steamworks app, steamcmd |

`just export pack` + `just export-selftest pack` is the quick check that an
export contains everything (missing resources, scripts that only work in the
repository): run it after adding scenes or resources that are loaded by path.

### Private sounds in release builds

The voice lines ("We are under attack!", "Enemy aircraft sighted!", "Fall back!") and
the other private sounds (machine gun, engine loops, rain, music) are not in this
repository (docs/ASSETS.md, "Private sounds"). A release takes them from the private
repository `mud-and-steel-sfx`:

- **On your machine:** with `../mud-and-steel-sfx` in place run `just audio-private`
  (only the private files; `just audio` and `just music` do the same and more) and
  `just import`. The export takes every `priv_*` file that is there.
- **The check:** `tools/release/export.sh` runs `tools/release/check_private_sounds.py`
  before every release export (not for `pack`) and stops if a voice line is missing
  for a faction, a new take of the private repository was not prepared, or a `priv_*`
  file was not imported. `ALLOW_PUBLIC_SOUNDS=1 just export macos` exports without
  the private sounds on purpose (the game then plays the public sounds and the alarm
  instead of the voices).
- **On GitHub (the Release workflow):** the export job checks out the private
  repository next to the sources, prepares its sounds and imports them. It reads the
  private repository with a deploy key, which **you** set up once:
  1. `ssh-keygen -t ed25519 -N "" -C "mud-and-steel release" -f sfx_deploy_key`
  2. Private repository on GitHub → Settings → Deploy keys → Add deploy key: the
     content of `sfx_deploy_key.pub`, read-only (no write access).
  3. This repository → Settings → Secrets and variables → Actions → New repository
     secret `SFX_DEPLOY_KEY`: the content of `sfx_deploy_key`. Then delete both files.
  4. Push the private repository (with Git LFS) so GitHub has every voice line.

  Without the secret the export job fails at the checkout; a release without voices
  is never built by accident. The workflow runs only on version tags and by hand, not
  for pull requests, so forks never see the secret. Each run downloads the private
  repository's LFS files (counts against its LFS bandwidth).

## What an exported game looks like

| System | Content of `build/export/<system>/` (= one Steam depot) |
|--------|---------------------------------------------------------|
| macOS | `Mud and Steel.app` (universal arm64 + x86_64): `Contents/MacOS/Mud and Steel`, `Contents/Frameworks/libbridge.dylib` + `libsteam_api.dylib`, `Contents/Resources/` with the `.pck`, `data/` and the license files |
| Windows | `MudAndSteel.exe` (the `.pck` embedded), `bridge.dll`, `steam_api64.dll`, `data/`, license files |
| Linux | `MudAndSteel.x86_64` (the `.pck` embedded), `libbridge.so`, `libsteam_api.so`, `data/`, license files |

`data/` holds `maps/` and `scenarios/`: the simulation reads maps from disk
(Rust, `std::fs`), so they cannot live in the `.pck`.
`godot/scenes/menu/game_paths.gd` (`GamePaths`) finds them next to the
executable in an exported game and in the repository otherwise; maps made in
the map editor go to `user://maps` in an exported game (the install folder
can be read-only, the macOS app is signed).

The rules (`data/*.toml`, the catalog, `test_small`) are compiled into the
bridge as before.

**Player data** (`user://`): settings, replays, own maps, logs, crash reports.

| System | Folder |
|--------|--------|
| macOS | `~/Library/Application Support/Godot/app_userdata/Mud and Steel/` |
| Windows | `%APPDATA%\Godot\app_userdata\Mud and Steel\` |
| Linux | `~/.local/share/godot/app_userdata/Mud and Steel/` |

## Version

One source: `version` in `[workspace.package]` of `Cargo.toml`.
`godot/project.godot` repeats it as `application/config/version` (Godot
shows it, the export presets use it); the test `crates/bridge/tests/release.rs`
fails when they differ. A release tag must be `v<version>`; the workflow
refuses anything else (`tools/release/version.sh`).

To release version 0.2.0:

1. Set `version = "0.2.0"` in `Cargo.toml` and `config/version="0.2.0"` in
   `godot/project.godot`; `just check` (also updates `Cargo.lock`).
2. Commit, then tag: `git tag v0.2.0`.
3. Push the commit and the tag (`git push && git push origin v0.2.0`): the
   **Release** workflow starts. Or start it without a tag under *Actions >
   Release > Run workflow* (then the version is only checked, not compared).

## CI: the Release workflow

`.github/workflows/release.yml`, separate from CI (which stays unchanged):

1. **version**: reads the version, compares it with the tag.
2. **lib** (macOS, Ubuntu 22.04, Windows): `tools/release/build-lib.sh`, the
   staged libraries as artifacts `lib-<system>`. macOS builds arm64 and x86_64
   and joins them with `lipo`.
3. **export** (macOS): downloads Godot 4.7.2 and its export templates from
   the Godot releases on GitHub (cached), imports the project, exports all
   three systems (signs and notarizes macOS when the secrets exist), plays the
   self-test in the exported macOS app and uploads three archives as
   artifacts `MudAndSteel-<version>-<system>`.
4. **smoke** (Ubuntu 22.04, Windows): unpacks the archive of its system and
   plays the self-test in the exported game.

The archives are workflow artifacts (kept 90 days by default): download them
from the run's page. The workflow creates no GitHub release and uploads nothing
to Steam.

## Step by step: what you do

### 1. First local export (macOS)

1. Install the export templates of Godot 4.7.2 (about 1 GB): in the Godot
   editor *Editor > Manage Export Templates > Download and Install*, or the
   file `Godot_v4.7.2-stable_export_templates.tpz` from
   <https://github.com/godotengine/godot/releases/tag/4.7.2-stable> with
   *Install from File*. They must be exactly 4.7.2.
2. Optional, for a universal app: `rustup target add x86_64-apple-darwin`.
   Without it the local app runs on Apple Silicon only (the script warns);
   CI always builds universal.
3. `just release-lib`, then `just export macos`.
4. `just export-selftest macos` (headless, exit code 0 = passed), then start
   `build/export/macos/Mud and Steel.app` by double click and play a match.
5. Windows and Linux builds come from CI (the bridge must be built on each
   system). To export them locally anyway, put the CI artifacts `lib-windows`
   and `lib-linux` into `target/export/`, then `just export windows` /
   `just export linux` (the export itself works on any system).

### 2. Release workflow

Push a tag (see [Version](#version)) or start it by hand. Check the run:
three green smoke tests, three artifacts. Download and try them on real
machines (the workflow cannot click through menus or check the picture).

### 3. macOS signing and notarization

Without it the app is signed ad-hoc: it runs where it was built; on other
Macs Gatekeeper blocks it after a download (players can allow it with
right-click > *Open*). Through Steam it starts, but notarization is still
recommended.

1. Join the Apple Developer Program (paid, yearly).
2. Create a *Developer ID Application* certificate (Xcode > Settings >
   Accounts > Manage Certificates, or developer.apple.com), export it from the
   Keychain as `.p12` with a password.
3. Create an app-specific password for your Apple ID (account.apple.com >
   Sign-In and Security > App-Specific Passwords) and note your Team ID.
4. In the GitHub repository *Settings > Secrets and variables > Actions*,
   add the secrets:

   | Secret | Value |
   |--------|-------|
   | `MACOS_SIGN_IDENTITY` | `Developer ID Application: <Name> (<TEAMID>)` |
   | `MACOS_CERT_P12_BASE64` | `base64 -i cert.p12` output |
   | `MACOS_CERT_PASSWORD` | the `.p12` password |
   | `APPLE_ID` | your Apple ID e-mail |
   | `APPLE_TEAM_ID` | the Team ID |
   | `APPLE_APP_PASSWORD` | the app-specific password |

   The secrets never go into the repository. Locally the same variables
   (in your shell, not in a file) sign and notarize `just export macos`;
   `MACOS_CERT_P12_BASE64` is not needed there when the certificate is in
   your login keychain.
5. `tools/release/macos-sign.sh` signs the libraries and the app with the
   hardened runtime and `tools/release/macos/entitlements.plist` (library
   validation off and DYLD variables allowed: both needed by the Steam
   overlay), notarizes with `notarytool` and staples the ticket.

### 4. Windows code signing (optional)

Steam does not need signed executables, and the Steam client is what players
start the game from. Without a signature, Windows SmartScreen warns when the
`.exe` is downloaded outside Steam (e.g. a demo on itch.io). If you want it:
buy an OV/EV code-signing certificate or use Azure Trusted Signing, then sign
`MudAndSteel.exe` with `signtool sign /fd sha256 /tr <timestamp-url> /td sha256 …`
after the export (on a Windows machine or a Windows CI job). Not automated:
the choice of certificate is yours.

### 5. Steam

You do these in the Steamworks partner site (partner.steamgames.com):

1. Pay the Steam Direct fee for the app, fill in the company, **tax and bank
   data** (only you can do this).
2. Note the **app id**. Create three **depots** (Windows, macOS, Linux) under
   *SteamPipe > Depots*, each limited to its operating system.
3. *Installation > General*: launch options, one per system:
   `MudAndSteel.exe` (Windows), `Mud and Steel.app` (macOS),
   `MudAndSteel.x86_64` (Linux). For Linux choose *Steam Linux Runtime 3.0
   (sniper)* under the Linux settings.
4. *SteamPipe > Builds*: create a branch `beta` with a password (private beta).
5. In the code: set `steam::APP_ID` (`crates/steam/src/lib.rs`) to the app id
   (one line; see [ONLINE.md](ONLINE.md#app-id)), tag and build.
6. Install `steamcmd` (Valve's command line tool; partner documentation
   *Uploading to Steam*). Put the three exports into `build/export/{windows,macos,linux}/`
   (unpack the CI archives there).
7. Upload:

   ```bash
   export STEAM_APP_ID=… STEAM_DEPOT_WINDOWS=… STEAM_DEPOT_MACOS=… STEAM_DEPOT_LINUX=…
   export STEAM_BRANCH=beta STEAM_USER=<your Steam login>
   just steam-upload            # dry run: writes build/steam/*.vdf, prints the command
   just steam-upload --upload   # runs steamcmd; asks for password and Steam Guard code
   ```

   With `STEAM_BRANCH=beta` the build is set live on the private beta branch;
   without it nothing is set live (do it under *SteamPipe > Builds*). Steam
   never lets a tool set the default (public) branch live.
8. Give testers the beta password (or keys under *Request Steam Product
   Keys*), check the overlay (Shift+Tab) and an online match between two
   accounts ([ONLINE.md](ONLINE.md#testing-with-two-steam-accounts)).
9. Store page, capsule images, trailer, price, release date, age ratings:
   all yours, in the partner site.

**Achievements** are not prepared yet: they need a design decision (which
ones) and the real app id. Steamworks-rs supports them (`UserStats`); a later
prompt can add them.

## Logs and crash reports

- **Log file:** Godot writes everything the game prints, warnings, errors and
  Rust panics (godot-rust reports them as errors) to `user://logs/godot.log`
  and keeps the last 10 sessions (`project.godot`, `debug/file_logging`).
- **Crash reports:** `godot/scenes/menu/crash_guard.gd` (autoload
  `CrashGuard`) writes `user://session.lock` at start and removes it on a clean
  exit. Found at the next start, it copies the log of the crashed session
  (with Godot's backtrace) to `user://crash-reports/crash-<date-time>.log`
  (the newest 10 are kept), and the main menu says where it is. **Nothing is
  sent**; a player can attach the file to a bug report. Active only in
  exported games (and with `-- --crash-guard`); in the repository the log is
  in the terminal.

## Licenses

- `LICENSE.txt`: the game's own license. **[provisional]** "All rights
  reserved" (proprietary), since the game is sold on Steam; replace it if you
  want an open license.
- `CREDITS.md`: origin of every asset (all own work so far, no CC0 assets yet).
- `THIRD_PARTY_LICENSES.txt`: made at build time by
  `tools/release/third_party_licenses.py` from `cargo metadata`: every crate
  the bridge links with version, license and the license texts the crates
  ship, plus the Steamworks API. godot-rust is MPL-2.0: unmodified, so
  pointing to its source (listed) is enough.
- Godot Engine's license and its components come from the engine itself
  (`Engine.get_license_text()`, `get_copyright_info()`).
- All of them are next to the executable and in the main menu under
  **Credits** (tabs Credits, License, Third party, Godot Engine).

## Known limits and open points

- **Not yet tried with export templates.** They were not installed while
  prompt 044 was built (downloading was not allowed). Checked: release
  libraries, the export of the `.pck` with all resources and the menu flow
  self-test from it, every preset valid except for the missing templates, the
  SteamPipe scripts (dry run). The first workflow run or `just export macos`
  after installing the templates is the real test.
- **Windows export from macOS:** `application/modify_resources` (icon and
  version info in the `.exe`) is on; Godot 4.7 does this itself. If the
  Windows export fails on it, set it to `false` in `godot/export_presets.cfg`.
- **Linux glibc:** the bridge is built on Ubuntu 22.04 (glibc 2.35); Steam
  Linux Runtime 3.0 (sniper) has an older glibc. If the game does not start in
  the runtime, build the Linux library in Valve's sniper SDK container
  (`registry.gitlab.steamos.cloud/steamrt/sniper/sdk`).
- **Steam overlay:** Steam is started with the online screen, not at launch
  ([ONLINE.md](ONLINE.md#choices)). The overlay may need `SteamAPI_Init`
  before the renderer starts; check it in the beta (step 5.8).
- **Own maps over the network:** LAN and online matches load maps from the
  shipped `data/maps` only; a map made in the map editor of an exported game
  plays in single player.
- **ETC2/ASTC textures:** a universal macOS export needs them, so
  `project.godot` imports textures in both VRAM formats (larger
  `.godot/imported`, longer first import).
- **Player data folder** is Godot's default (`Godot/app_userdata/Mud and
  Steel`); `application/config/use_custom_user_dir` could make it
  `Mud and Steel` directly, but would move the settings of existing players.
