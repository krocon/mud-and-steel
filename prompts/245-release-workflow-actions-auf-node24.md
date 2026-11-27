# Prompt 245 – Release-Workflow: GitHub-Actions auf Node-24-Versionen heben (Node-20-Warnung beseitigen)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`
> und `docs/RELEASE.md` (Abschnitte „CI: the Release workflow“ und „Private
> sounds in release builds“). Im Code: `.github/workflows/release.yml`
> (Jobs `lib`, `export`, `smoke`) und `.github/workflows/ci.yml`.
>
> **Reine CI-Pflege:** kein Rust-, Godot- oder Asset-Code ändert sich,
> `crates/sim` bleibt unberührt, die Golden Hashes ändern sich nicht.

## 1. Problem

Jeder Lauf des Release-Workflows zeigt diese Warnung, zuerst gesehen am Job
„GDExtension (linux)“:

```
Node.js 20 is deprecated. The following actions target Node.js 20 but are being
forced to run on Node.js 24: actions/upload-artifact@v4. For more information see:
https://github.blog/changelog/2025-09-19-deprecation-of-node-20-on-github-actions-runners/
```

Im Repo wird nirgends eine Node-Version gesetzt (kein `actions/setup-node`,
kein `package.json`). Die Laufzeit gehört zur jeweiligen Action: Jede
JavaScript-Action nennt sie in ihrer `action.yml` unter `runs.using`. Die
`v4`-Reihen von `upload-artifact`, `download-artifact` und `cache` nennen dort
`node20`; GitHub lässt sie inzwischen zwangsweise auf Node 24 laufen und
warnt. „Auf neues Node.js updaten“ heißt hier also: **die Major-Version der
betroffenen Actions anheben**, bis jede selbst `node24` nennt.

Die Warnung nennt nur die Action des jeweiligen Jobs. In den Jobs `export`
und `smoke` laufen weitere `v4`-Actions, die dieselbe Warnung auslösen.

## 2. Bestand (geprüft am 2026-11-27 mit `gh api repos/<repo>/releases/latest`)

| Action | im Repo | Stellen | neueste Version | Laufzeit dort |
|---|---|---|---|---|
| `actions/upload-artifact` | `@v4` | `release.yml`: Job `lib` (1×), Job `export` (3×) | `v7.0.2` | `node24` |
| `actions/download-artifact` | `@v4` | `release.yml`: Job `export` (1×, `pattern: lib-*` + `merge-multiple`), Job `smoke` (1×, `name:`) | `v8.0.2` | `node24` |
| `actions/cache` | `@v4` | `release.yml`: Job `export` („Cache Godot and its export templates“) | `v6.1.0` | `node24` |
| `actions/checkout` | `@v7` | beide Workflows | `v7.0.1` | `node24`, nichts zu tun |
| `Swatinem/rust-cache` | `@v2` | beide Workflows | `v2.9.3` | `node24` (der Tag `v2` wandert mit), nichts zu tun |
| `taiki-e/install-action` | `@nextest` | `ci.yml` | – | `composite`, kein Node, nichts zu tun |

`ci.yml` braucht nach diesem Stand keine Änderung. Die Tabelle ist eine
Momentaufnahme: Prüfe sie zu Beginn neu (Abschnitt 3.1) und arbeite mit dem,
was du dann findest.

## 3. Umsetzung

### 3.1 Stand neu erheben

- Alle Actions des Repos auflisten: `grep -rn "uses:" .github/`. Gibt es
  inzwischen weitere Workflows oder Actions, gehören sie in die Prüfung.
- Für jede Action die neueste Version und ihre Laufzeit nachsehen, nicht
  raten:
  `gh api repos/<owner>/<action>/releases/latest --jq .tag_name` und
  `gh api "repos/<owner>/<action>/contents/action.yml?ref=<tag>" --jq .content | base64 -d | grep using:`.
- Ziel je Action: die neueste Major-Version, deren `runs.using` `node24` (oder
  neuer) ist.

### 3.2 Versionssprünge prüfen, bevor du sie einträgst

Die Sprünge gehen über mehrere Major-Versionen (`upload-artifact` 4 → 7,
`download-artifact` 4 → 8, `cache` 4 → 6). Lies für **jede** übersprungene
Major-Version die Release-Notes
(`gh api repos/actions/<action>/releases --jq '.[] | select(.tag_name|test("^v[0-9]+\\.0\\.0$")) | .tag_name, .body'`)
und gleiche sie mit dem ab, was der Workflow nutzt. Das Verhalten muss gleich
bleiben:

- **`upload-artifact`**, genutzte Eingaben: `name`, `path`,
  `if-no-files-found: error`, `compression-level: 0`. Die drei Spielarchive
  (`MudAndSteel-*-macos.zip`, `-windows.zip`, `-linux.tar.gz`) werden weiter
  als normale, gepackte Artefakte hochgeladen. Führt eine neue Version einen
  Schalter für ungepackte Einzeldateien ein, bleibt er auf seinem Standard,
  solange der Standard das bisherige Verhalten ist.
- **`download-artifact`**, genutzte Eingaben: im Job `export`
  `pattern: lib-*`, `path: target/export`, `merge-multiple: true`; im Job
  `smoke` `name:` und `path: download`. Danach muss gelten wie heute:
  `target/export/` enthält die Bibliotheken aller drei Systeme **flach
  zusammengeführt** (so, wie `tools/release/build-lib.sh` sie gestaffelt
  hat), und `download/` enthält das Archiv des Systems, das der Schritt
  „Unpack“ auspackt. Ändert eine Version die Ordnerstruktur oder das
  Entpacken, passe die Eingaben an, nicht die Skripte dahinter.
- **`cache`**, genutzte Eingaben: `path: ~/godot-download`,
  `key: godot-<GODOT_VERSION>-stable`. Ein einmal verfehlter Cache nach dem
  Versionssprung ist in Ordnung (Godot wird dann neu geladen).
- Upload und Download müssen zueinander passen: Nimm die Versionen, die die
  Release-Notes als Paar vorsehen.
- Verlangt eine Version eine Mindestversion des Runners, ist das auf den
  gehosteten Runnern (`macos-latest`, `ubuntu-22.04`, `windows-latest`)
  erfüllt; halte es trotzdem im Abschlussbericht fest.

Findest du eine Änderung, die das Verhalten berührt und die du nicht sicher
einschätzen kannst, nimm für diese eine Action die höchste Version, bei der es
sicher ist (sie muss `node24` nennen), und nenne den Grund im Abschlussbericht.

### 3.3 Workflow ändern

- In `.github/workflows/release.yml` die Versionen an allen Stellen der
  Tabelle anheben. Schreibweise wie im Rest des Repos: Major-Tag (`@v7`),
  kein Commit-Hash, keine Minor-Version.
- Kommentare im Workflow, die eine Action-Version oder Node nennen, mit
  anpassen. `docs/RELEASE.md` nennt keine Action-Versionen; ändere es nur,
  wenn sich am beschriebenen Ablauf etwas ändert (das soll es nicht).
- Sonst nichts anfassen: keine Umbauten an Jobs, Matrix, Caches oder
  Skripten, kein Dependabot, kein neues Werkzeug.

## 4. Prüfung

1. **Lokal:** Die geänderte Datei muss gültiges YAML sein
   (`python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release.yml'))"`).
   Ist `actionlint` schon installiert, lass es laufen; installiere es nicht
   (CLAUDE.md: vor jeder Installation fragen). `just check` ist nicht nötig,
   es ändert sich kein Rust-Code.
2. **Auf GitHub:** Der Release-Workflow läuft nur bei einem Versions-Tag oder
   von Hand, und nur mit dem gepushten Stand. Deshalb:
   - Committen (Nachricht auf Englisch), dann den Nutzer **fragen**, ob
     gepusht werden darf (CLAUDE.md: nie ohne ausdrückliche Zustimmung) und ob
     du den Release-Workflow von Hand starten sollst. Der Lauf baut auf drei
     Systemen und exportiert auf macOS; er dauert entsprechend und kostet
     Runner-Minuten. Kein Tag setzen.
   - Nach dem Ja: `gh workflow run release.yml --ref main`, dann
     `gh run watch <id>`. Die Laufzeit liegt über zwei Minuten: im Hintergrund
     warten, nicht in einer Schleife abfragen.
   - Der Lauf muss grün sein, mit denselben Artefakten wie bisher:
     `lib-macos`, `lib-linux`, `lib-windows` und drei
     `MudAndSteel-<version>-<system>`; beide `smoke`-Jobs spielen den
     Selbsttest im entpackten Spiel.
   - Die Warnung muss weg sein, in **allen** Jobs, nicht nur in
     „GDExtension (linux)“: die Anmerkungen des Laufs lesen
     (`gh run view <id>` zeigt sie unter „ANNOTATIONS“) und nach „Node.js 20“
     und „deprecated“ suchen. Andere Warnungen, die dort stehen, im
     Abschlussbericht nennen, aber nicht in diesem Prompt beheben.
   - Der CI-Lauf, den der Push auf `main` auslöst, muss ebenfalls grün und
     ohne Node-Warnung sein.
3. Lehnt der Nutzer Push oder Lauf ab, ist der Prompt **nicht** erledigt:
   Sag klar, dass die Änderung nur lokal geprüft ist, und lass den Status in
   `prompts/README.md` auf „offen“.

## 5. Abschluss

- Abschlussbericht: alte → neue Version je Action, was die Release-Notes an
  Verhaltensänderungen nannten und warum sie uns nicht treffen (oder was du
  dafür angepasst hast), Link zum grünen Lauf.
- Erst nach dem grünen Lauf ohne Warnung: `git mv` dieser Datei nach
  `prompts/done/` und den Status in `prompts/README.md` auf „erledigt“ setzen
  (mit den neuen Versionen in Klammern).
