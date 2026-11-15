# Prompt 044 – Release-Builds und Steam-Veröffentlichung

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/VERSIONS.md`, `docs/ONLINE.md`, `.github/workflows/`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

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

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
