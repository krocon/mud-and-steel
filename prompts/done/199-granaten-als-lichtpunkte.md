# Prompt 199 – Granaten in der Luft als Lichtpunkte statt Kugeln

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> in `docs/ARCHITECTURE.md` die Teile zu indirektem Feuer (Prompt 032),
> Leuchtspur (Prompt 077) und Raketen (Prompt 076), `docs/PERFORMANCE.md`.
> Im Code `godot/scenes/world/artillery_view.gd` (`_shell_mesh` als `SphereMesh`,
> `SHELL_SIZE_PER_MM`, `SHELL_COLOR`), `godot/scenes/world/effects.gd`
> (`_note_tracer`, Leuchtspuren direkter Schüsse), `rocket_effects.gd` (Spur,
> Flamme), `effect_library.gd`, `effect_textures.gd`, `graphics_quality.gd`,
> `atmosphere.gd` (Tag/Nacht, Sonne).
>
> **Vorlage** (nur Stil-Referenz, nichts daraus übernehmen): Ankündigungs-Trailer
> eines anderen RTS-Spiels, lokal unter
> `.temp/trailer/Defilade - Official Announcement Trailer (1074p_30fps_H264-128kbit_AAC).mp4`
> (1920 × 1074, 76 s, nicht im Repo, `.temp/` ist ignoriert). Bilder daraus mit
> `ffmpeg -ss <s> -i <datei> -frames:v 1 -vf "crop=w:h:x:y" <scratchpad>/x.png`
> ziehen und ansehen; **keine Bilder aus dem Video ins Repo**. Keine Assets,
> Namen, Texturen oder Designs übernehmen; nur der Eindruck zählt.
>
> Was dort zu sehen ist (Auswertung vom 2026-11-22):
>
> - **0–9 s** (Wüste, Fahrzeuge schießen auf Hubschrauber): Feuer in der Luft
>   erscheint als **gestrichelte Linien aus winzigen hellen Punkten** (wenige
>   Pixel, weiß bis blassgelb), eng hintereinander, dazu an den Mündungen
>   kleine weiße Wölkchen. Kein Geschoss hat einen sichtbaren Körper.
> - **9–17 s** (Schneegraben): Gewehrfeuer als sehr dünne, kurze,
>   **leuchtende Striche** (hier bläulich-weiß), die nur Bruchteile einer Sekunde
>   stehen.
> - **16–17 s und 26 s** (Großangriff, Häuserkampf): Kanonen- und MG-Feuer als
>   **kurze gelb-orange Leuchtstriche** (einige Meter lang, scharf, hell, mit
>   etwas Glühen), keine Kugeln; Einschläge als Feuerball mit Rauch.
> - Insgesamt: Geschosse lesen sich als **Licht in Bewegung** (Punkt oder
>   kurzer Strich, je nach Tempo), nicht als Objekt. Die Länge des Strichs
>   wirkt wie Bewegungsunschärfe über ein Bild.

## 1. Problem

Der Nutzer (2026-11-22): Die Granaten in der Luft als Kügelchen sehen nicht gut
aus. Heute zeichnet `artillery_view.gd` jede indirekte Granate als dunkle,
übergroße Kugel (8 Segmente, `SHELL_SIZE_PER_MM` „übertrieben, damit man sie
sieht“) entlang ihrer Bahn. Echte Granaten sieht man im Flug praktisch nicht;
was man sieht, ist Licht (Leuchtspur, Reflex) und manchmal eine Spur in der Luft.

## 2. Ziel

Reine Darstellung, Bahn und Zeiten weiter aus `SimHost` (`shell_*`):

- **Kein Körper mehr** für Granaten (Mörser, Haubitzen, Sturmtiger-Rakete
  ausgenommen: Raketen behalten Körper, Flamme und Spur aus `rocket_effects.gd`,
  höchstens angeglichen).
- **Lichtpunkt bzw. Lichtstrich:** kleiner, heller, additiver Billboard-Punkt
  (warm-gelb bis orange), der sich mit dem Tempo am Bildschirm zu einem kurzen
  Strich in Flugrichtung streckt (Länge ≈ Strecke eines Bildes, wie in der
  Vorlage), Größe in Bildpunkten gedeckelt statt in Metern, damit er aus
  Spielhöhe sichtbar, nah aber nicht zur Kugel wird. Leichtes Flackern bzw.
  Aufblitzen, nicht dauerhaft gleich hell: z. B. kurz nach dem Abschuss und im
  letzten Abschnitt vor dem Einschlag deutlicher, dazwischen schwach oder
  zeitweise unsichtbar (deterministisch aus der Granaten-Id, damit Replays
  gleich aussehen).
- **Kondensspur (optional, nach Bild entscheiden):** sehr dünne, kurze, schnell
  verblassende Spur hinter dem Punkt (Ribbon oder wenige Partikel), heller
  Dunst, keine Rauchwurst. Bei Nebel, Regen, Nacht stimmig (nachts Punkt
  heller, Spur fast unsichtbar).
- **Direkte Schüsse** (Panzer, Pak): Leuchtspuren aus `_note_tracer` ansehen
  und auf denselben Stil bringen, falls sie ebenfalls als Kugeln oder zu dick
  erscheinen; MG-Leuchtspur bleibt, nur prüfen, dass sie zusammenpasst.
- Nebel des Krieges wie heute (keine Lichtpunkte, die verborgene Batterien
  verraten, über das hinaus, was heute sichtbar ist).
- Qualitätsstufen: Niedrig ohne Spur.
- Pfeifton (`shell_incoming`) und Einschlagmarken bleiben unverändert.

## 3. Nicht

- Keine Änderung an `crates/sim`, Golden Hash unverändert.
- Keine Godot-Zufälle, die Spielrelevantes berühren (hier nur Optik).

## 4. Abnahme

- Vorher/Nachher-Bilder und ein kurzer Clip (Tag, Abend, Nacht) aus Spielhöhe:
  Mörserbeschuss, Haubitzenbatterie, Panzerduell; zum Vergleich mit der
  Beschreibung des Nutzers vorlegen.
- Bildrate mit 30+ Granaten gleichzeitig in der Luft (`just bench-view` oder
  `blast_shots.gd`), höchstens im Rahmen von `docs/PERFORMANCE.md`.
- `just selftest`, `just check` grün.
