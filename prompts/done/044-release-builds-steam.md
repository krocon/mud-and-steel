# Prompt 044 – Release-Builds und Steam-Veröffentlichung

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/VERSIONS.md`, `docs/ONLINE.md`, `.github/workflows/`.
>
> Konkretisiert in Abschnitt 4 (Ausgangslage, Entscheidungen, Umfang, Ergebnis).

## 1. Ziel

Es gibt keine Export-Presets: Das Spiel läuft nur aus dem Repo über `just run`. Nach
diesem Prompt baut die CI spielbare Release-Builds für macOS, Windows und Linux, die
sich über Steam (zuerst als privater Beta-Zweig) verteilen lassen.

## 2. Umfang

1. **Export:** `godot/export_presets.cfg` für die drei Systeme, GDExtension im
   Release-Profil für jede Plattform (macOS universal arm64+x86_64), Steam-Bibliothek
   neben der Extension, Icon, Versionsnummer aus einer Quelle (Cargo/Git-Tag).
2. **CI:** Release-Workflow bei Git-Tag: bauen, Godot-Export, Artefakte hochladen;
   Smoke-Test des exportierten Builds headless (Menü startet, Selbsttest läuft).
3. **macOS:** Signierung und Notarisierung vorbereiten (Zertifikate richtet der Nutzer
   ein, Secrets nur in GitHub). Windows: Hinweis zu Code-Signing.
4. **Steam:** SteamPipe-Depot-Skripte (`tools/steam/`), Upload nur manuell durch den
   Nutzer oder nach ausdrücklicher Freigabe; Store-Seite, Preise, Steuer- und
   Bankdaten macht der Nutzer selbst. Steam-Errungenschaften optional vorbereiten.
5. **Absturzberichte und Logs:** Log-Datei in `user://`, Absturzbericht lokal
   speichern; kein Versand ohne Einwilligung.
6. **Lizenzen:** Lizenzdatei, Drittanbieter-Lizenzen (Godot, Rust-Crates, CC0-Assets)
   im Build und im Menü „Credits“.

## 3. Akzeptanzkriterien

1. Tag-Build erzeugt drei Archive; exportierter Build startet auf macOS lokal.
2. Bestehende CI unverändert grün.
3. Doku: `docs/RELEASE.md` mit Schritt-für-Schritt-Anleitung für den Nutzer.

## 4. Konkretisierung (Session p044, 2026-11-15)

### Ausgangslage im Code

- Keine Export-Presets (`godot/export_presets.cfg` stand sogar in `.gitignore`),
  kein Icon, keine Versionsnummer in `project.godot`, keine Lizenzdatei.
- `.gdextension` zeigte für Release auf `target/release/`, ohne Steam-Bibliothek;
  `docs/ONLINE.md` vermerkte „kein Export-Pipeline“.
- Karten und Szenarien liest die Simulation (Rust, `std::fs`) von der Platte über
  `res://../data/maps`. In einem exportierten Spiel löst Godot das relativ zum
  Arbeitsverzeichnis auf: Ohne Änderung fände ein exportiertes Spiel keine Karten.
- Godot 4.7.2 ist lokal installiert, die Export-Templates nicht; das Rust-Target
  `x86_64-apple-darwin` ist nicht installiert. Beides wurde nicht nachinstalliert
  (keine Downloads in dieser Session).

### Entscheidungen

| # | Entscheidung | Begründung |
|---|--------------|------------|
| 1 | Release-Bibliotheken werden in `target/export/` bereitgestellt (`tools/release/build-lib.sh`), `.gdextension` zeigt mit `[libraries]` und `[dependencies]` (steam_api) dorthin | Ein fester Ort für alle drei Systeme: CI legt die Artefakte der drei Runner zusammen, die Dateinamen kollidieren nicht. Die universelle macOS-Bibliothek (lipo) überschreibt nichts in `target/release/`. |
| 2 | Export aller drei Systeme auf einem macOS-Runner, Bibliotheken je System gebaut | Templates nur einmal laden (~1 GB, gecacht); Signieren und Notarisieren braucht ohnehin einen Mac. Rust lässt sich nicht sinnvoll für alle Systeme von einem Host bauen. |
| 3 | Smoke-Test = der bestehende Menü-Flow-Selbsttest (`-- --self-test`) im exportierten Spiel, auf jedem System, aus fremdem Arbeitsverzeichnis | Prüft Menü, Einstellungen, Credits, Lobbys, vier Matches und Ergebnis; kein neuer Test nötig. |
| 4 | `GamePaths` (`godot/scenes/menu/game_paths.gd`): `data/` neben der Exe (macOS: `Contents/Resources/data`), eigene Karten des Editors im Export nach `user://maps` | Siehe Ausgangslage; Installationsordner können schreibgeschützt sein, die macOS-App ist signiert. Im Repository bleibt alles wie bisher. |
| 5 | Version: eine Quelle `Cargo.toml` (`[workspace.package]`), `project.godot` wiederholt sie, ein Test (`crates/bridge/tests/release.rs`) hält beide gleich, der Tag muss `v<version>` sein | Godot kann die Version nicht aus Cargo lesen; `override.cfg` wirkt beim Export nicht (Editor ignoriert es). Ein Test ist einfacher als Dateien beim Build umzuschreiben. |
| 6 | macOS: Godot signiert ad-hoc, danach signiert `tools/release/macos-sign.sh` neu (ad-hoc oder Developer ID aus Umgebungsvariablen/GitHub-Secrets, Hardened Runtime, Notarisierung mit `notarytool`) | `data/` wird nach dem Godot-Export ins Bundle kopiert und bräche Godots Signatur; ein Skript macht den Ablauf lokal und in CI gleich. Entitlements für das Steam-Overlay. |
| 7 | `textures/vram_compression/import_etc2_astc=true` | Godot verweigert sonst den universellen/arm64-macOS-Export. Folge: Texturen werden zusätzlich als ASTC importiert (zwei `.import`-Dateien geändert). |
| 8 | Windows-Code-Signing nur dokumentiert, nicht automatisiert | Steam verlangt es nicht; Wahl und Kauf des Zertifikats liegen beim Nutzer. |
| 9 | SteamPipe: Vorlagen `tools/steam/*.vdf.in`, `tools/steam/upload.sh` (Standard: Trockenlauf; `--upload` nur von Hand), IDs aus Umgebungsvariablen, nie in CI | Kein Upload ohne den Nutzer; keine IDs oder Zugangsdaten im Repository. |
| 10 | Errungenschaften nicht vorbereitet | Brauchen eine Designentscheidung und die echte App-ID; in `docs/RELEASE.md` als offen vermerkt. |
| 11 | Logs: Godots Datei-Log (`user://logs`, 10 Sitzungen). Absturzbericht: Autoload `CrashGuard` kopiert das Log einer abgestürzten Sitzung nach `user://crash-reports/`, das Hauptmenü nennt den Pfad; nur im Export aktiv | Fängt auch native Abstürze (Godots Backtrace steht im Log), ohne Rust- oder Engine-Code; nichts wird versendet. |
| 12 | Lizenzen: `LICENSE.txt` **[provisional]** „All rights reserved“; `THIRD_PARTY_LICENSES.txt` per Python-Skript aus `cargo metadata` (nur Standardbibliothek, kein neues Werkzeug); Godot-Lizenzen aus der Engine (`Engine.get_license_text()` usw.); Credits-Screen im Hauptmenü mit vier Reitern | Kommerzieller Steam-Verkauf; `cargo-about` o. ä. hätte eine Installation gebraucht. Die Dateien liegen im Build neben der Exe und im `.pck` (`res://licenses/`, beim Export bereitgestellt). |
| 13 | Linux-Bibliothek auf `ubuntu-22.04` gebaut **[provisional]** | Ältester verfügbarer Runner; falls das Spiel in der Steam Linux Runtime (sniper) nicht startet: im sniper-SDK-Container bauen (dokumentiert). |
| 14 | Bundle-ID `com.mudandsteel.game`, Icon `godot/icon.svg` (eigene Zeichnung) **[provisional]** | Platzhalter, bis Name/Domain feststehen. |
| 15 | Keine GitHub-Releases, Archive als Workflow-Artefakte | Laut Auftrag; ein Release mit den Artefakten legt der Nutzer bei Bedarf selbst an. |

### Umfang (geschärft)

Umgesetzt: Presets, Release-Bibliothek, Export-/Sign-/Smoke-Skripte, Release-Workflow,
`GamePaths`, CrashGuard, Credits-Screen mit Lizenzen, SteamPipe-Skripte,
`docs/RELEASE.md` mit Schritt-für-Schritt-Anleitung. Nicht umgesetzt: Steam-Upload,
Store, Zertifikate (Nutzer), Errungenschaften (später).

### Ergebnis

- Release-Bibliothek gebaut (arm64; universal nur in CI, lokal fehlt das x86_64-Target).
- `just export pack` (ohne Templates) + `just export-selftest pack`: Flow-Selbsttest
  im exportierten `.pck` bestanden (inkl. Credits).
- Die drei Presets sind gültig; der Export scheitert lokal nur an den fehlenden
  Templates. Der erste echte Export (`just export macos` oder der Workflow) steht
  aus – siehe „Known limits“ in `docs/RELEASE.md`.
- CrashGuard mit simuliertem Absturz geprüft; SteamPipe-Skript im Trockenlauf geprüft.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
