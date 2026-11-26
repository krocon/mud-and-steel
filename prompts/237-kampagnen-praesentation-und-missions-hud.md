# Prompt 237 – Kampagnen-Kapitelkarten, Zeitleiste, einklappbares Missionsziel-Panel und Funk-Meldungen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/MISSIONS.md`, `docs/CONTROLS.md`, `docs/ARCHITECTURE.md` („Missions“,
> „HUD“), `data/campaigns/summer_front.toml` sowie das Ergebnis von
> `prompts/done/189-kampagne-und-neue-missionen.md`.
> Im Code:
> `godot/scenes/menu/campaign_screen.gd`,
> `godot/scenes/menu/missions.gd`,
> `godot/scenes/menu/menu_theme.gd`,
> `godot/scenes/hud/mission_hud.gd`,
> `godot/scenes/hud/subtitles.gd`,
> `godot/scenes/main.gd`.
>
> **Stil-/Spielgefühl-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4` (Sek. `00:07–00:10`, `00:15–00:19`,
> `00:32–00:35`, `00:48–00:52` Kapitel-Einblendungen mit Titel, Jahreszeit,
> kurzem Leitspruch und horizontaler Kampagnen-Zeitleiste; Sek. `01:04–01:25`
> einklappbare Missionsziele oben links `OBJECTIVES 0/3 · OPTIONAL 0/1 · HIDE/SHOW`
> sowie gut lesbare Funk-/Sprecher-Untertitel unten in der Mitte).
> **Keine Bilder, Videos, Texte oder Assets aus dem Video ins Repo.**
> **Wichtig:** Reine Darstellung & optionale, rückwärtskompatible Metadaten in
> Kampagnen-/Missionsdateien – **keine Regeländerungen in `crates/sim`**,
> Golden Hashes bleiben unverändert!

## 1. Problem

Kampagnen und Missionen funktionieren seit Prompt 189 technisch stabil, wirken
in der Präsentation aber noch sehr nüchtern und verschenken im Spiel Platz:

1. **Kampagnen-Bildschirm (`campaign_screen.gd`):** Zeigt Missionen nur in
   einer schlichten `ItemList` – es fehlt das Gefühl einer fortschreitenden
   Front/Reise über Jahreszeiten und Schauplätze hinweg.
2. **Start einer Kampagnen-Mission:** Die Mission springt ohne atmosphärischen
   Auftakt direkt ins Gefecht; der kurze Intro-Text steht nur im Menü davor.
3. **Dauerhaft offenes Missionsziel-Panel (`mission_hud.gd`):** Oben links
   belegt `MissionHud` permanent eine 380 px breite Box – besonders bei
   mehreren Zielen und einer langen Meldung verdeckt das Panel im Gefecht einen
   spürbaren Teil des Sichtfelds und lässt sich nicht einklappen.
4. **Missions-Meldungen gehen unter:** Wichtige taktische Hinweise oder
   Funkmeldungen einer Mission erscheinen aktuell klein im oberen linken Panel
   und werden während eines Gefechts leicht übersehen.

## 2. Ziel

Atmosphärischere Kampagnen- und Missions-Präsentation mit unseren eigenen
Mitteln (keine externen Videos oder Bilder nötig, vollständig lokalisiert und
rückwärtskompatibel):

1. **Kampagnen-Zeitleiste im Kampagnen-Bildschirm (`campaign_screen.gd`, `data/campaigns/*.toml`):**
   - Optionale, rückwärtskompatible Felder pro `[[mission]]` in
     Kampagnen-TOMLs (z. B. `chapter_label` / `date_label` wie `"Frühjahr"` /
     `"Sommer"` / `"Winter"`, `tagline` für 1 kurzen Leitspruch) – fehlen sie,
     greifen saubere Fallbacks (Missionsnummer / Titel).
   - Im `campaign_screen.gd` über oder neben der Missionsliste eine
     **horizontale Kampagnen-Zeitleiste** zeichnen: verbundene Knoten für jede
     Mission der Kampagne, mit Status (gesperrt, aktuell ausgewählt, gewonnen
     inkl. Sternen) und Datum/Jahreszeit-Beschriftung. Klick auf einen
     freigeschalteten Knoten wählt die Mission direkt aus.
2. **Atmosphärische Kapitel-Einblendung beim Missionsstart (`mission_hud.gd` / eigenes Overlay):**
   - Beim Start einer Kampagnen-Mission (nicht bei Replays, nicht bei
     `--skip-menu` / Headless-Selbsttests!) blendet sich für ca. 3,5–4,5 s ein
     ruhiges, vollflächiges **Kapitel-Overlay** über die bereits geladene
     3D-Karte (mit weicher dunkler Vignette – 100 % echte Spielgrafik als
     Hintergrund, keine externen Artworks!):
     - Kampagnen-Obertitel, Kapitelnummer & Missionstitel,
       Jahreszeit/Datum-Zeile, kurzer Intro-Satz und die kompakte
       Kampagnen-Zeitleiste unten mit hervorgehobenem aktuellem Kapitel.
     - Klick, `Leertaste` oder `Esc` überspringt die Einblendung sofort;
       während der Einblendung pausiert die Simulation optional (oder startet
       sanft beim Ausblenden), damit der Spieler keinen Anfangs-Beschuss
       verpasst.
3. **Einklappbares Missionsziel-Panel oben links (`mission_hud.gd`):**
   - Oben links eine kompakte Kopfzeile:
     `ZIELE 1/3 · OPTIONAL 0/1 · AUSBLENDEN ▲` (bzw. `ZEIGEN ▼`, lokalisiert).
   - Ein Klick auf die Kopfzeile (oder eine Taste) klappt die Zielliste sauber
     auf und zu; die schmale Kopfzeile mit dem Fortschrittszähler bleibt
     immer lesbar, ohne das Spielfeld zu blockieren.
   - Bei neuen oder abgeschlossenen Zielen blinkt/pulsiert die Kopfzeile
     dezent (oder klappt auf Wunsch kurz auf), damit nichts unbemerkt bleibt.
4. **Klare Funk-Meldungen / Missions-Untertitel (`mission_hud.gd`, `subtitles.gd`):**
   - Missions-Meldungen (`info["message"]`) können optional einen Sprecher
     enthalten (z. B. `speaker = "Hptm. Weber"` oder Präfix aus der
     Missionsdatei, rückwärtskompatibel!).
   - Neue Missions-Meldungen erscheinen für einige Sekunden **gut lesbar unten
     mittig über der Einheitenleiste** (wie Untertitel mit optionalem
     Sprechernamen und schlichtem stilisiertem Rang-/Funk-Emblem – **kein**
     Schauspieler-Video!), mit klarer Outline auf dunklem Streifen, und bleiben
     zusätzlich im Missions-Panel nachlesbar.
   - Im Gegensatz zu den allgemeinen Sound-Untertiteln (`subtitles` für
     Warntöne) sind Missions-Funksprüche standardmäßig aktiv (über eigene
     Einstellung oder zusammen mit Missions-HUD abschaltbar), und sie blenden
     bei `F1` (HUD aus), Fotomodus und `trailer_shots` sauber aus.
5. **Bestehende Kampagne `data/campaigns/summer_front.toml` anreichern:**
   - Optionale `date_label`- und `tagline`-Einträge für die 5 Missionen in
     `summer_front.toml` (plus deutsche Übersetzungen in
     `summer_front.de.po`) ergänzen, damit die Zeitleiste und die
     Kapitelkarten sofort im Spiel sichtbar sind.

## 3. Prüfen und zeigen

- **Sichtprüfung (Screenshots in `build/campaign-ui/`):**
  - `campaign_screen.gd` mit horizontaler Zeitleiste (gemischt: gewonnene,
    aktuelle und gesperrte Mission).
  - Kapitel-Intro-Overlay über der geladenen Karte (lesbare Typografie bei
    Tag, Winter und Wüste).
  - `mission_hud.gd` aufgeklappt und eingeklappt + Funk-Meldung unten mittig.
- **Tests & Kompatibilität:**
  - Alle bestehenden Kampagnen- und Missionsdateien ohne die neuen optionalen
    Felder laden fehlerfrei.
  - Headless-Tests (`just selftest`, `mission_self_test.gd`, `flow_self_test.gd`)
    hängen nicht an der Kapitel-Einblendung und prüfen Auf-/Zuklappen sowie
    Zeitleiste mit.
  - `just check`, `just selftest` grün; Golden Hashes unverändert.
  - `docs/MISSIONS.md`, `docs/CONTROLS.md`, `godot/locale/` (DE + EN)
    aktualisieren.

## 4. Nicht Teil dieses Prompts

- Realfilm- oder KI-generierte Schauspieler-Videos/Porträts.
- Sprachausgabe / Vertonung der Missions-Funksprüche (kann später auf den
  Sprecher-Feldern aufbauen).
- Änderungen an Missions-Skriptregeln oder KI in `crates/sim`.

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/237-kampagnen-praesentation-und-missions-hud.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md`
auf „erledigt“ setzen.
