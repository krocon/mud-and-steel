# Prompt 231 – Schnelleres Infanterietempo und Laufbewegung

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/SCALE.md` (insb. „Time factor 2“ und „Vehicle speed and range after a
> reference video (prompt 097)“), `docs/INFANTRY.md`, `docs/BALANCE.md`,
> `docs/SOURCES.md`, `prompts/done/097-panzertempo-schussweite-nach-video.md`
> sowie die Regeldateien `data/scale.toml`, `data/infantry.toml`,
> `data/tactics.toml` (`[rush]`, `[escort]`, `[march]`, `[stance]`) und
> `data/terrain.toml`.
> Im Code:
> `crates/sim/src/scale.rs`,
> `crates/sim/src/sim/infantry.rs`,
> `crates/sim/src/sim/tactics.rs`,
> `crates/sim/src/sim/movement.rs` (bzw. wo `game_speed` aufgerufen wird),
> `godot/scenes/world/infantry_view.gd` (`RUN_FROM`, Clip-Wahl und
> `speed_scale`),
> `godot/assets/models/infantry/animations.json`.
>
> **Stil-/Spielgefühl-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4` (insb. Sek. `00:11–00:14`,
> `01:08–01:13`, `02:02–02:06`, `02:35–02:40`, `03:36–03:41`).
> **Keine Bilder, Daten oder Werte direkt aus dem Video ins Repo.** Der Katalog
> (`data/catalog/*.toml`) behält seine realen historischen Werte; angepasst
> werden ausschließlich die Umrechnungsfaktoren in `data/scale.toml` und die
> davon abhängigen Spielregeln.
> **Hinweis zu Golden Hashes:** Gezielte Regeländerung in `crates/sim` – die
> Golden Hashes in `crates/sim/tests/determinism.rs` werden sich ändern und
> müssen nach Prüfung bewusst aktualisiert und im Commit benannt werden.

## 1. Problem

Der Nutzer hat beim Ansehen des Referenzvideos hervorgehoben, dass sich die
**höhere Laufgeschwindigkeit der Fußsoldaten** deutlich besser anfühlt als das
derzeitige Tempo in *Mud and Steel*:

- **Heute bei uns:**
  - Im Katalog stehen für Infanterie reale Marschgeschwindigkeiten von
    `speed_offroad_kmh_x10 = 40` (4,0 km/h = 1,11 m/s) und
    `speed_road_kmh_x10 = 50` (5,0 km/h = 1,39 m/s).
  - Mit `[time] factor = 2/1` in `data/scale.toml` bewegt sich Infanterie
    im Gelände aktuell mit **2,22 m/s** (~1,23 Körperhöhen/s) und auf Straßen
    mit **2,78 m/s**; in geduckter Haltung (`kneel`, `60 %`) sogar nur mit
    **1,33 m/s**.
  - Weil `infantry_view.gd` erst ab `RUN_FROM = 2.6 m/s` vom `walk`-Clip in
    den `run`-Clip wechselt, **gehen** Soldaten im Gelände im Standard-Marsch
    gemächlich, statt zügig vorzurücken. Nur beim kurzen Sturmangriff/Sprung
    (`[rush]`, `300 %`, zeitlich begrenzt mit Erschöpfung) laufen sie schnell.
  - Nach Prompt 097 (`[speed] vehicle = 1/1`) fahren Panzer (z. B. Panzer IV
    4,44 m/s Gelände, Sherman 6,94 m/s) angenehm schwer, während Infanterie
    dagegen auf offenen Karten und im Häuserkampf träge wirkt.
- **Im Referenzvideo (Auswertung Sek. 02:02–02:04 bei ruhender Kamera):**
  - Fußsoldaten rücken bei normalen Bewegungs- und Deckungsbefehlen im
    **zügigen Gefechtslauf / Laufschritt** vor (~2,2–2,6 Körperhöhen/s,
    entspricht bei 1,80 m etwa **4,0–4,6 m/s** im Gelände).
  - Dadurch reagieren Trupps direkt auf Befehle, erreichen schnell Deckung,
    flankieren Häuser und halten im Gefecht spürbar besser mit Panzern Schritt.

## 2. Ziel

Infanterie bewegt sich im Spiel zügig im Gefechtslauf, ohne reale Katalogwerte
oder Nachlade-/Bau-/Haltungsdauern zu verfälschen:

1. **Eigener Bewegungsfaktor für Infanterie in `data/scale.toml` und `crates/sim/src/scale.rs`:**
   - Analog zu `[speed] vehicle` aus Prompt 097 ein eigenes Feld
     `[speed] infantry` einführen (wirkt ausschließlich auf die
     Bewegungsgeschwindigkeit von Infanterie-Kategorien, **nicht** auf
     Nachladezeiten, Stellungsbau, Haltungswechsel oder Unterdrückungsabbau –
     dafür bleibt `[time] factor = 2/1`).
   - Prüfen, ob schwere Waffenteams (`inf_hmg`, `inf_mortar_light`,
     `inf_mortar_medium`) und von der Bedienung geschobene Geschütze
     (`Group::Gun`) bewusst langsamer bleiben sollen als leichte Fußtruppen
     (z. B. `infantry = { num = 7, den = 2 }` → **3,89 m/s Gelände / 4,86 m/s
     Straße**, oder `{ num = 4, den = 1 }` → **4,44 / 5,56 m/s**; für schwere
     Waffenteams optional `infantry_heavy = { num = 5, den = 2 }` → 2,78 /
     3,47 m/s, geschobene Geschütze bleiben bei `[time]` = `2/1`).
   - Zwei Kandidaten (`7/2` und `4/1`) im Spielgefühl und im KI-Turnier
     vergleichen und den stimmigeren festlegen (Ziel: deutlich schneller als
     heute, spürbar langsamer als Lkw/Panzer auf Straßen, glaubwürdig neben
     einem 4,4-m/s-Panzer IV im Gelände).
2. **Abhängige Infanterie-Regeln sauber nachziehen (`data/tactics.toml`, `data/infantry.toml`):**
   - **`[rush]` (`data/tactics.toml`):** Bisher `speed_pct = 300` (war auf
     2,22 m/s ausgelegt → 6,67 m/s Sprint). Bei ~3,9–4,4 m/s Grundtempo würde
     `300 %` zu unrealistischen 12–13 m/s führen! `speed_pct` proportional
     anpassen (z. B. `145–160 %` → ~5,8–6,4 m/s Sprint im Sturmangriff bzw.
     Sprung), Erschöpfung (`tired_speed_pct`) prüfen.
   - **`[stance]` (`data/infantry.toml`):** Prüfen, ob `kneel` (`60 %` → dann
     ~2,3–2,7 m/s zügig geduckt) und `prone` (`20 %` → ~0,8 m/s Robben, ggf.
     auf `15 %` = ~0,6 m/s halten, damit Robben nicht zu schnell wirkt)
     stimmig bleiben.
   - **`[escort]` (`data/tactics.toml`):** Prüfen, wie Infanterie hinter
     Panzern jetzt Schritt hält (ein Panzer IV mit 4,4 m/s muss kaum noch
     abbremsen; `[march]`-Gruppentempo profitiert direkt).
   - **Überrollen (`data/combat.toml` `[overrun]`):** Prüfen, ob Infanterie
     weiterhin realistisch vor anrollenden Panzern ausweichen kann und Panzer
     sie trotzdem überrollen können.
3. **Animation ohne Fußrutschen abstimmen (`godot/scenes/world/infantry_view.gd`):**
   - `animations.json` hat `walk` (authored `1.4 m/s`), `crouch_walk`
     (`1.0 m/s`), `run` (`3.6 m/s`) und `crawl` (`0.35 m/s`).
   - Mit ~3,9–4,4 m/s passt der `run`-Clip (`3.6 m/s`) nahezu 1:1 (nur ~1,1×
     Playback-Speed, kein unnatürliches Zappeln und kein Rutschen!).
   - `RUN_FROM` (aktuell `2.6`) und den Übergang zwischen `walk`,
     `crouch_walk` (bei `stance == 1` geduckt!) und `run` prüfen, sodass:
     - aufrechte Trupps bei normaler Bewegung im Gelände sauber `run` laufen,
     - geduckte Trupps (`stance == 1`) in Bewegung `crouch_walk` (oder bei
       höherem Tempo sauber skaliert) zeigen,
     - erschöpfte oder sehr langsame Trupps sauber auf `walk` zurückfallen.
4. **KI & Balance prüfen:**
   - `crates/ai` prüfen, ob Annahmen über Marschzeiten, Flanken- oder
     Transporter-Nutzung angepasst werden müssen (Lkw/Halbketten sollen auf
     mittleren/großen Karten weiterhin klar schneller und sinnvoll bleiben!).
   - `just ai-tournament` über alle Fraktionen fahren und Fraktions-Balance
     sowie Gefechtsdauer in `docs/BALANCE.md` und `docs/SCALE.md` dokumentieren.

## 3. Prüfen und zeigen

- **Direkter Vergleich im Spiel** (kurzer Clip oder Messreihe Vorher/Nachher):
  - Schützengruppe über 100 m offenes Feld, Straße, geduckt, im Sturmangriff
    und neben einem fahrenden Panzer IV / Sherman (Zeiten in Sekunden +
    visuelle Prüfung der Laufanimation auf Fußrutschen).
  - Schweres MG / Mörsertrupp und geschobene PaK im Vergleich zu
    Schützengruppen.
- **Tests & Balance:**
  - Unit-Tests in `crates/sim/src/scale.rs` erweitern (`speed_factor_for` für
    Infanterie vs. Fahrzeug vs. Flugzeug vs. Zeitdauern).
  - `just ai-tournament` (Fraktions-Gewinnraten und Gefechtsdauern prüfen).
  - `just stuck-audit` (prüfen, dass schneller laufende Trupps nicht häufiger
    steckenbleiben).
  - Golden Hashes in `crates/sim/tests/determinism.rs` bewusst aktualisieren
    und in `docs/SCALE.md`, `docs/INFANTRY.md`, `docs/BALANCE.md`
    dokumentieren.
  - `just check`, `just selftest`, `just infantry-selftest` grün.

## 4. Nicht Teil dieses Prompts

- Änderungen an historischen Geschwindigkeitswerten in `data/catalog/*.toml`.
- Änderungen am Fahrzeugtempo (`[speed] vehicle = 1/1`) oder an
  Kampfreichweiten (`[range]`).
- Handgranaten (Prompt 232), Deckungsbefehl (Prompt 233) oder Rückzugsbefehl
  (Prompt 234).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/231-infanterie-schneller-laufen.md prompts/done/`
verschieben, Ergebnisabschnitt (gewählte Faktoren, gemessene m/s-Werte,
Turnier-Balance, aktualisierte Golden Hashes) ergänzen und den Status in
`prompts/README.md` auf „erledigt“ setzen.

## Ergebnis (Nachtlauf 26.11.2026)

- **Gewählte Faktoren (vorläufig, Nachtlauf):**
  - `data/scale.toml` `[speed]`: `infantry = { num = 7, den = 2 }` (**3,89 m/s** Gelände, **4,86 m/s** Straße; passt fast 1:1 zum 3,6-m/s-`run`-Clip bei `1,08×`) und `infantry_heavy = { num = 5, den = 2 }` (**2,78 m/s** Gelände, **3,47 m/s** Straße für `InfHmg`, `InfMortarLight`, `InfMortarMedium`). Geschobene Geschütze (`Group::Gun`) und alle Zeitdauern bleiben bei `[time] factor = 2/1`.
  - `data/tactics.toml` `[rush] speed_pct = 155` (~6,03 m/s Sturm/Sprung, vorher 300 %), `data/infantry.toml` `[stance.prone] speed_pct = 15` (~0,58 m/s Kriechen, vorher 20 %).
  - `godot/scenes/world/infantry_view.gd` & `soldier_motion.gd`: `RUN_FROM = 3.0` (frische leichte Trupps laufen im `run`-Clip, erschöpfte Trupps 2,72 m/s und schwere Trupps 2,78 m/s fallen sichtbar auf `walk` zurück) und `CROUCH_RUN_FROM = 3.2` (kniende Trupps bleiben in `crouch_walk`).
- **Nebenfund & Fixes:**
  - `crates/sim/src/sim/overrun.rs`: Auch langsam kriechende eigene Panzer (`step > 0` unterhalb `min_speed_mm_per_s`) schicken verbündete Soldaten aus der Fahrspur (behebt echten Wannen-Überlappungsfall in `escort::nobody_gets_under_a_hull_there_and_back`).
  - Missions-Autopiloten in `crates/ai/tests/missions.rs` und `godot/scenes/menu/mission_self_test.gd` auf das schnellere Infanterietempo nachgeschärft (gestrandete Boote nicht in Land-Formationsmärsche einreihen, gezogene Pak in Halte-Missionen deployed lassen, hängende Infanterie auf steilen Dünen gegen Infanterieziele nachziehen).
- **Messwerte & Balance:**
  - Variantenvergleich `7/2` vs `4/1` (90 Partien, 3 Karten): `7/2` (`de 53 % / su 35 % / us 61 %`, `ai-tournament` Hash `0xaf74b85d3d6a5eee`, Hard vs. Normal `47/52 %`, Ø 62 s) gegen `4/1` (`de 43 % / su 40 % / us 66 %`, zieht mit Panzer IV offroad gleich).
  - Vollständiger 4-Fraktionen-Balance-Check (300 Partien, 5 Karten): **`de 45 % / su 43 % / uk 44 % / us 60 %`**.
  - `just stuck-audit`: 3 545 Befehle, 11 344 Einheiten auf 48 Karten, 11 307 angekommen (**0,4 %** gesamt, **0,0 %** auf allen Originalkarten).
- **Golden Hashes (gewollt aktualisiert durch neue `[speed]`/`[rush]`/`[stance.prone]`-Regeln):**
  - `crates/sim/tests/determinism.rs`: `0x187e_58b0_2c2c_39fe` (`demo_battle_is_decided` Tick `625`, Sieger Spieler 0).
  - `crates/sim/tests/economy.rs`: `0x8a9a_8e7b_7c21_427c`.
- **Checks:** `just check`, `just selftest` (inkl. aller 5 Tutorials, 10 Missionen, `riders`, `gun crews`, `boats`) und `just infantry-selftest` grün.
- **Dokumentation:** `docs/SCALE.md`, `docs/INFANTRY.md`, `docs/BALANCE.md` aktualisiert.
