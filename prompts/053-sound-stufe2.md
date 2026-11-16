# Prompt 053 – Sound Stufe 2: Akustik und Stimmen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Audio aus 029/030), `docs/SOURCES.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

029 liefert synthetische Sounds. Dieser Prompt macht das Gefecht akustisch glaubwürdig:
Geschosse knallen beim Vorbeifliegen, weit entfernte Schüsse kommen verzögert an,
Herauszoomen klingt fern, große Explosionen klingen größer, und Soldaten rufen.

## 2. Umfang

1. **Überschallknall:** Geschosse, die nahe an der Hörmitte vorbeifliegen, erzeugen ein
   „Krack“ vor dem Mündungsknall (aus den Geschossbahnen der Brücke).
2. **Schallgeschwindigkeit:** Verzögerung nach Entfernung (mit Zeitfaktor 2), Echo/Hall
   nach Umgebung (Wald, Dorf, offenes Feld).
3. **Entfernung und Zoom:** Tiefpass und Pegel nach Kamerahöhe; nah: Details, fern:
   dumpfes Grollen.
4. **Explosionen nach Größe** skaliert (Kaliber, Munitionsexplosion), mit Nachhall.
5. **Echte Aufnahmen:** Struktur für Waffenaufnahmen pro Waffentyp (CC0 oder vom
   Nutzer bereitgestellt, mit Quelle), synthetisch als Rückfall.
6. **Stimmen:** kurze Rufe beim Hineinzoomen (Feind gesichtet, Deckung, Nachladen,
   verwundet) je Fraktion in ihrer Sprache; Herkunft: vom Nutzer aufgenommen oder
   bereitgestellt; bis dahin Platzhalter-Struktur ohne Dateien. Keine Parolen,
   keine Ideologie.

## 3. Akzeptanzkriterien

1. `just selftest` grün; Golden-Hash unverändert.
2. Schlacht-Benchmark hält das Budget aus 029.
3. Quellen in `docs/SOURCES.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code: 029 liefert `battle_sound.gd` (Hörer über dem Kamerafokus,
Stimmen-Pools pro Gruppe aus `sounds.json`, Motor-/Ketten-/Feuer-Schleifen für die
nächsten Fahrzeuge und brennenden Wracks), `sound_classes.gd` (Katalog → Klasse),
`sound_buses.gd` (Master → Effects → Muffled, UI, Music, Ambience), `tools/audio/synth.py`
(`just audio`, Übersteuerung je Klasse aus `assets-src/audio/<class>/`). 030 liefert
Ambiente mit Wald-Dichte-Raster (`SimHost.wood_density_at`). Lücken: Flugzeuge (033)
haben keinen Motorklang, Flak- und Bordwaffen sind stumm (die Luft-Ereignisse liest nur
`air_view.gd`); brennende Kartenobjekte (041) sind stumm; Explosionen skalieren nur
±6 dB; alle Klänge kommen sofort an; kein Hall je Umgebung; Zoom ändert nur die
Hörweite. Geschossbahnen gibt es pro Tick (`projectile_positions/velocities`),
Geschwindigkeiten in Spiel-m/s (real × Zeitfaktor 2).

| Thema | Entscheidung | Begründung |
|---|---|---|
| Schallgeschwindigkeit | 343 m/s real × Zeitfaktor 2 = **686 Spiel-m/s**; Verzögerung = Bodenabstand zum Kamerafokus / 686, höchstens **2,5 s** [provisional] | Gleiche Umrechnung wie alle Geschwindigkeiten (`docs/SCALE.md`); die Obergrenze hält die Übersicht (bis 6 km Hörweite) lesbar |
| Was verzögert wird | Schüsse, Treffer, Explosionen, Zerstörung, Objekt- und Kraterereignisse, Flak, Bordwaffen; **nicht**: Pfeifen anfliegender Granaten, Überschallknall, Meldungen | Pfeifen und Knall entstehen am Hörer bzw. sind Spielinformation |
| Warteschlange | sortiert nach Fälligkeit, höchstens 192 Einträge, bei Überlauf fällt der schwächste (Priorität); das Stimmenbudget pro Frame gilt beim Abspielen | Lastspitzen bleiben begrenzt, wichtige Klänge gehen nicht verloren |
| Überschallknall | Einmal pro Sim-Tick: jedes Geschoss schneller als 686 Spiel-m/s, dessen nächster Punkt zur Hörmitte (Fokus + 1,5 m) in diesem Tick-Schritt liegt und näher als 12 m (Kaliber < 20 mm) bzw. 25 m (Granaten) vorbeigeht, spielt `crack_bullet` / `crack_shell` am nächsten Punkt, sofort; nur bis 200 m Kameraabstand, leiser mit Zoom; eigene Gruppe `crack` (4 Stimmen, 2 pro Frame) [provisional] | Keine Projektil-IDs nötig (Abschnitt pro Tick wird genau einmal geprüft); der Knall kommt so von selbst vor dem verzögerten Mündungsknall |
| Hall nach Umgebung | Alle 0,5 s: 5 × 5 Raster Wald-Dichte und `SimHost.buildings_near` (neue Lesefunktion in `audio_funcs.rs`: Kasten-Objekte ab 3 m Höhe) um den Fokus → Anteile Wald / Dorf / offen; ein `AudioEffectReverb` auf dem Effects-Bus, Parameter gemischt und geglättet: offen fast trocken, Wald dichter dumpfer Hall, Dorf kurzer heller Hall mit frühen Reflexionen [provisional: Werte ohne Abhören] | Ein Bus-Effekt statt Hall pro Stimme kostet auf dem Hauptthread nichts |
| Zoom | Tiefpass auf dem Effects-Bus: 40 m Kameraabstand 20 kHz → 450 m 2,2 kHz (logarithmisch); Klassen der Priorität ≤ 1 (Gewehr, MG, Kugeleinschläge) bis −8 dB leiser in der Übersicht [provisional] | nah Details, fern dumpfes Grollen; Godots Distanz-Tiefpass pro Stimme bleibt |
| Explosionen nach Größe | Treffer ab 20 mm: Tonhöhe nach Kaliber (75 mm = 1,0, 150 mm ≈ 0,87, 37 mm ≈ 1,1); ab 100 mm und bei Fahrzeug-/Munitionsexplosionen zusätzlich `explosion_tail` (langes Nachgrollen, eigene Gruppe `tail`, 4 Stimmen), lauter mit Kaliber/Größe | Größere Ladung = tiefer und länger; die Schwanzschicht kostet nur bei großen Ereignissen eine Stimme |
| Echte Waffenaufnahmen | Ordner pro Waffe: `assets-src/audio/weapon/<slug>/` → `just audio` → `godot/assets/audio/weapon/<slug>/`; Slug = Katalog-Waffenname klein, Nicht-Buchstaben → `_` (`MG 42` → `mg_42`); hat der Ordner Dateien, spielt der Schuss sie mit den Pegeln seiner Klasse, sonst die synthetische Klasse. `python3 tools/audio/synth.py --weapons` listet alle Slugs. Herkunft in `docs/SOURCES.md` | Struktur ohne Dateien, Rückfall automatisch; `weapon/` (Einzahl) kollidiert nicht mit der Rohbibliothek `weapons/` aus 061 |
| Flugzeuge | Schleifen `engine_aircraft` (Einmot, Sternmotor-/Reihenmotor-Brummen) und `engine_bomber` (mehrmotorig, Schwebung), nach Spannweite ab 18 m; die 3 nächsten Flugzeuge, Doppler von Hand (Tonhöhe = c / (c − v_radial), 0,75–1,35); Flak schwer `flak_burst`, leicht `shot_cannon_small`, Bordwaffen `shot_mg`, Absturz `explosion_vehicle`; dazu ein Signal `air_event` in `air_view.gd` | Luft-Ereignisse werden von `air_view` verbraucht; ein Signal ist die kleinste Änderung |
| Brände (041) | brennende Kartenobjekte (`SimHost.fires`, Schlüssel ≥ 0) teilen sich die Feuer-Schleifen mit den Wracks (nächste zuerst, jetzt 6); ab 8 m Größe `fire_large` (tiefes Fauchen, Knacken, Balken), Pegel nach Größe und Restbrenndauer | Gleiche Mechanik, kein neuer Pool |
| Wetter/Nacht (040) | keine Änderung | Regen hat ein eigenes Ambiente-Bett; Nacht-Akustik wäre Feinschliff ohne Abhören |
| Stimmen | `battle_voices.gd`: Rufe `contact` (Feind gesichtet: erster Schuss eines Trupps nach 20 s Ruhe), `take_cover` (Granate ≥ 20 mm schlägt in ≤ 15 m bei einem Trupp ein), `reloading` (zufällig 1 von 12 Schüssen eines Trupps), `wounded` (Trupp getroffen); nur bis 80 m Kameraabstand, Trupp sichtbar und ≤ 50 m vom Fokus, Abklingzeit 4 s global, 12 s pro Trupp; Fraktion = Präfix der Typ-ID (`de`, `su`, `us`); Dateien `assets-src/audio/voice/<fraktion>/<ruf>/` → `godot/assets/audio/voice/…`. **Es werden keine Stimmdateien ausgeliefert**; ohne Dateien zählt der Ruf nur (`voices_missing`) [provisional: Auslöser und Wortlaut] | Aufnahmen kommen später vom Nutzer; keine Parolen, nur taktische Rufe (Vorschläge in `docs/ASSETS.md`) |
| Kosten | Überschallprüfung nur bei Tickwechsel, Umgebung alle 0,5 s, Flugzeuge mit den Schleifen alle 0,2 s; Ziel: `battle_sound` bleibt unter 0,3 ms pro Frame (029: 0,11–0,20 ms) | Budget aus 029 (≤ 5 % FPS) |
