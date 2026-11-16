# Prompt 043 – Lokalisierung, Barrierefreiheit und Einstellungen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Menüs, Einstellungen).
>
> **Konkretisiert** (Abschnitt 4). Ursprünglich Entwurf: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Alle UI-Texte sind englisch und fest im Code. Nach diesem Prompt ist das Spiel auf
Englisch und Deutsch spielbar (weitere Sprachen leicht ergänzbar), und es gibt die
üblichen Barrierefreiheits- und Komfort-Einstellungen.

## 2. Umfang

1. **Lokalisierung:** alle Texte über Godot-Übersetzungen (CSV oder PO) mit Schlüsseln;
   Einheiten- und Objektnamen aus dem Katalog übersetzbar; Sprache in den Einstellungen,
   Standard nach Systemsprache. Prüfskript findet fest kodierte Texte.
2. **Barrierefreiheit:** UI-Skalierung, Schriftgröße, farbenblind-taugliche
   Spielerfarben (Paletten wählbar), Untertitel/Text für akustische Warnungen (029),
   Bildschirmwackeln abschaltbar, Tastenbelegung für alle Befehle (nicht nur Kamera).
3. **Grafik-Einstellungen:** Auflösung, Vollbild/Fenster, VSync, FPS-Limit, Render-Skalierung
   (FSR) zusätzlich zur Qualitätsstufe.
4. **Selbsttest:** Sprache wechseln, jede Ansicht ohne fehlende Schlüssel; Einstellungen
   bleiben nach Neustart erhalten.

## 3. Akzeptanzkriterien

1. `just selftest` grün inklusive Sprachprüfung (kein Text ohne Übersetzung).
2. Doku: Übersetzungsablauf in `docs/ARCHITECTURE.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## 4. Konkretisierung (Ausgangslage und Entscheidungen)

Stand vor dem Umsetzen: rund 60 UI-Skripte bauen ihre Oberfläche im Code
(keine `.tscn`-Texte): Menüs, Einrichtung, Pause/Ergebnis, Spielstände und
Replays (027, 038), LAN-/Steam-Lobby und Teams (025, 026, 037), Einstellungen
mit Audio-Bussen und Tilt-Shift (027, 029, 030, 057), HUD mit Minikarte,
Einheitenanzeige, Befehlsleiste, Ereignissen (031), Bildkacheln (058),
Luftleiste (033), Transport/Pioniere/Nachschub/Stellungen/Schlag (034–036,
052, 054), Missions-HUD und Missionstexte in `data/maps/*/missions/*.toml`
(039), Karteneditor (028, 059, 060). Alles englisch und fest im Code.
Einheitennamen kommen aus dem Katalog (`name_de`/`name_en`, die Bridge gab
nur `name_en` aus), Gebäude- und Rangnamen aus `data/economy.toml` und
`data/experience.toml` (nur englisch). Spielerfarben waren als Konstanten
über sieben Skripte verteilt (eigen grün, verbündet blau, Feind rot).
Tastenbelegung: 30 Aktionen, Luftabrufe (Umschalt+1…5) und Raketenschlag
(Umschalt+6) fest; Bildschirmwackeln nur für Benchmarks abschaltbar;
Zurufe (053) ohne Text.

| Frage | Entscheidung | Begründung |
|---|---|---|
| Schlüssel oder englischer Text als Schlüssel? | Symbolische Schlüssel (`MENU_SKIRMISH`, Großbuchstaben, Bereichspräfix) | Fehlende Übersetzungen fallen als Schlüssel auf und der Selbsttest findet sie; englische Formulierungen ändern sich ohne Codeänderung. |
| Format | Godot-CSV (`keys,en,de`), eine Tabelle pro Bereich in `godot/locale/` (common, menu, settings, lobby, hud, editor, names) | Beide Sprachen nebeneinander, Godot importiert sie selbst; parallele Prompts (049, 056) ergänzen Zeilen in verschiedenen Dateien. Registrierung zur Laufzeit (`language.gd` liest `res://locale/`), keine Liste in `project.godot`. |
| Import | `.translation` nicht im Repo (war schon in `.gitignore`); `tools/ensure-imported.sh` importiert neue/geänderte Tabellen bei `just build` | Wie bei Sounds und Modellen. |
| Sprache | Einstellung `language`: `auto` (Systemsprache, wenn angeboten, sonst Englisch), `en`, `de`; `--language=` auf der Kommandozeile gewinnt; Selbsttests laufen englisch | Wie gefordert; Tests bleiben unabhängig von der Systemsprache. |
| Katalognamen | Bridge-Schalter `SimHost.set_language` wählt `name_de`/`name_en` für alle Namen, die die Bridge ausgibt; Gebäude/Ränge über Schlüssel `NAME_<ID>`/`RANK_<NAME>` (Bridge fragt Godots TranslationServer) | Kein Eingriff in Sim-Datenstrukturen (Hash), alle HUD-Stellen bekommen den Namen schon übersetzt. |
| Missionstexte | Englisch in der TOML, Übersetzung als `<id>.<lang>.po` daneben (msgid = englischer Text); `just mission-po` schreibt die Dateien, `sim::mission::mission_texts` liefert die Texte | Sim und Missionsformat bleiben unverändert; PO kennt jedes Übersetzungswerkzeug; Godot lädt PO direkt. |
| Prüfskript | `tools/l10n_check.py` (Tabellen vollständig, gleiche Platzhalter, Schlüssel im Code existieren, keine Prosa-Literale in Spielskripten; Ausnahme `# l10n-ignore`) + `cli mission-po --check`; `just l10n-check`, Abhängigkeit von `just selftest` | Python ist schon Projektwerkzeug (units-web); Werkzeuge, Debug und Selbsttests bleiben englisch. |
| Laufzeitprüfung | `menu/l10n_self_test.gd` im Ablauf-Selbsttest: alle Menübildschirme und Einstellungsreiter deutsch ohne fehlenden Schlüssel, HUD und Pausenmenü im Spiel, Katalogname und Missionstitel deutsch, alle neuen Einstellungen gespeichert und neu geladen | Akzeptanzkriterium 1 und 4. |
| Anrede | „du“ [vorläufig] | Üblich in aktuellen deutschen Spielen; kurze Texte. |
| UI-Skalierung | `content_scale_factor` des Hauptfensters (0,75–1,5), höchstens Fensterhöhe/720 [vorläufig] | Skaliert alle Menüs und das HUD ohne Umbau; Menüs sind für 1600×900 gebaut. |
| Schriftgröße | Faktor auf Theme-Schriftgröße und alle Schriftgrößen-Overrides (`MenuTheme.font`) | Unabhängig von der UI-Skalierung, z. B. größere Schrift bei gleicher Anordnung. |
| Farbpaletten | Standard, Rot-Grün-Schwäche, Blau-Gelb-Schwäche aus dem Okabe-Ito-Satz (`hud/relation_colors.gd`); Standard behält die bisherigen Farbtöne jeder Ansicht [vorläufig] | Bekannte farbenblind-taugliche Palette; keine sichtbare Änderung für Standard. |
| Untertitel | Aus per Standard [vorläufig]; eine Zeile für UI-Warnungen (angegriffen, Einheit verloren/bereit, Bau fertig, Sieg, Niederlage) und Zurufe (053); Zurufzeilen erscheinen auch ohne Sprachdateien | Zurufe haben noch keine Aufnahmen, die Einstellung soll trotzdem etwas zeigen. |
| Tastenbelegung | Alle Befehle der Befehlsleiste, Kamera, Luftabrufe und Raketenschlag belegbar, auch mit Umschalt/Strg/Alt; fest bleiben Gruppen 1…9, Bildkachel-Ziffern, Editor-Kürzel [vorläufig] | Ziffern als Gruppen sind RTS-Standard; Belegung = physischer Code mit Modifikator-Masken, alte Dateien bleiben gültig. |
| Grafik | Anzeigemodus (Fenster, randlos, exklusiv), Fenstergröße, VSync, FPS-Limit, Renderauflösung 0,5–1 mit Bilinear/FSR 1/FSR 2 | Wie gefordert; nur Geändertes wird angewandt (Benchmarks behalten ihr VSync). |
| Bildschirmwackeln | Schalter `screen_shake` | Wie gefordert. |
| Sichtprüfung | `just l10n-shots [sprache] [ordner]`: Bilder aller Menübildschirme, Einstellungsreiter, HUD, Baumenü, Pause und Ergebnis | Überläufe mit dem Auge prüfen; Bilder in `build/l10n/` (nicht im Repo). |

## 5. Ergebnis

Umgesetzt wie oben; Ablauf und Einstellungen in `docs/ARCHITECTURE.md`
(„Translations, accessibility and display“), Missionsübersetzungen in
`docs/MISSIONS.md`. 664 Schlüssel in 7 Tabellen, 4 Missionen übersetzt.
Gefunden über die Bilder: Einheitenanzeige überdeckte bei großer
UI-Skalierung die Befehlsleiste (rückt jetzt zur Seite bzw. über die
Minikarte), Beförderungsmeldung umformuliert.

Offen: Meldungen aus den Rust-Crates bleiben englisch (Editor-Validierung,
Lobby-/Netzwerkmeldungen, Dateifehler); Dezimalzahlen im deutschen Format
(„2.4 km“); Tastennamen kommen von Godot auf Englisch („Shift“); weitere
Sprachen (RTL/CJK bräuchten eine Schrift); 3D-Beschriftungen skalieren nicht
mit der UI-Skalierung; Feinschliff der deutschen Begriffe beim Spielen
(Liste der unsicheren Begriffe im Abschlussbericht).
