# Prompt 030 – Musik und Umgebungsklang

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Audio aus Prompt 029, Licht/Biome, Menüs).
>
> Konkretisiert am 2026-11-16 gegen den Stand nach 029 (Sound), 031 (HUD) und
> 038 (Spielstände/Replays): Ausgangslage und Entscheidungen unten.

## 1. Ziel

Das Spiel bekommt eine Klangkulisse: Wind, Vögel, Insekten, Regen (mit 040), Wasser an
Flüssen, fernes Grollen; dazu Musik in Menü, Ladebildschirm, Spiel und Ergebnis.

## 2. Ausgangslage (Stand main)

- 029 hat `godot/scenes/audio/` gebaut: `sound_buses.gd` legt die Busse `Music` und
  `Ambience` schon an, `menu/settings.gd` speichert `music_volume` und
  `ambience_volume`, der Audio-Reiter hat Regler dafür. Beide Busse sind still.
- `battle_sound.gd` (von `main.gd` erzeugt, fehlt mit `-- --no-sound`) hat einen
  `AudioListener3D` über der Bodenmitte der Kamera und `hearing_scale` nach Zoom,
  hört `combat_view.combat_event` und kennt `visible_at` (Nebel).
- `ui_sound.gd` lebt als Knoten unter der Baumwurzel über Szenenwechsel hinweg
  (`install()` aus den Einstellungen) — das gleiche Muster passt für Musik, die vom
  Menü über den Ladebildschirm ins Spiel und zum Ergebnis läuft.
- `tools/audio/synth.py` (`just audio`) synthetisiert alle Klänge mit numpy, mono
  WAV 44,1 kHz; Godot importiert WAV QOA-komprimiert (ganz im RAM).
- Kodierer auf der Maschine: `lame` und ffmpeg mit `libmp3lame`; kein libvorbis
  (ffmpegs eigener Vorbis-Kodierer ist experimentell). Keine Installation nötig.
- Wasser: `SimHost.terrain_at` (Ids 6/7 = Wasser). Wald: kein Geländetyp; die
  Brücke hat ein Walddichte-Raster (`objects.rs`, `DecoRasters::wood_at`) für die
  Dekoration, aber keine Funktion dafür.
- Biom: `main.atmosphere.preset.id` (`forest`, `desert`, `winter`), zur Laufzeit
  umschaltbar (`main.set_biome`). Ergebnis: `SimHost.match_result()`.
- Wetter/Regen (040) gibt es noch nicht.

## 3. Entscheidungen

| Frage | Entscheidung | Warum |
|---|---|---|
| Herkunft der Musik | Selbst erzeugt mit numpy: `tools/audio/music.py` (`just music`) komponiert und rendert alle Stücke aus Akkordfolgen, Motiven und additiven Instrumenten (Flächen, tiefe Streicher, Klavier-ähnliches Zupfen, Pauke), gesät pro Stück | Keine Downloads, keine Rechtefragen, reproduzierbar wie `just audio`. [vorläufig] Ohne Anhören gebaut: einfach und zurückhaltend gehalten; jedes Stück lässt sich durch eine Datei in `assets-src/music/` (CC0 oder vom Nutzer) ersetzen. |
| Format Musik | MP3 (lame, VBR ~130 kbit/s, Stereo), Godot `AudioStreamMP3` | Godot dekodiert MP3 beim Abspielen; im RAM liegt nur die komprimierte Datei (~1 MB/min statt ~10 MB als WAV). Kein Vorbis-Kodierer ohne Installation. Lücken am Dateiende stören nicht, weil Stücke überblendet werden statt zu loopen. |
| Format Ambiente | WAV-Loops (periodisch gebaut wie in 029, `smpl`-Chunk), Betten Stereo 32 kHz, Quellen (Wasser, Laub) mono; Vögel und Grollen als kurze One-Shots | Nahtlose Loops brauchen WAV; QOA hält sie klein im RAM. One-Shots statt Vogel-Loop: kein hörbares Muster alle 20 s. |
| Ablage | `godot/assets/audio/music/<stimmung>_NN.mp3`, `godot/assets/audio/ambience/<schicht>/…wav`; eigener Code in `godot/scenes/audio/music.gd`, `ambience.gd`, `match_music.gd` | `sounds.json` und `synth.py` bleiben unberührt (parallele Prompts fügen dort Klassen an). |
| Musik-Spieler | Ein Knoten unter der Baumwurzel (`music.gd`, wie `ui_sound.gd`), zwei Spieler zum Überblenden, Stimmungen `menu`, `calm`, `combat`, `victory`, `defeat` | Menümusik läuft über den Ladebildschirm weiter und blendet erst im Spiel um; kein Abriss bei Szenenwechseln. |
| Ladebildschirm | Kein eigenes Stück: die Menümusik läuft weiter | Laden dauert Sekunden; ein eigenes Stück würde nur anspielen. |
| Ruhe/Gefecht | `match_music.gd`: Intensität aus sichtbaren Kampfereignissen (Schuss 1, schwerer Treffer 2, Zerstörung 6), Abklingen mit 10 s; Gefecht ab 12, zurück unter 4 und frühestens 25 s nach Beginn; Überblenden 4 s | Hysterese und Mindestdauer verhindern Hin- und Herspringen. Nur Sichtbares zählt, Lärm im Nebel ist Ambiente. [vorläufig] Schwellen ohne Anhören gesetzt. |
| Ruhige Musik | Zwischen ruhigen Stücken 20–60 s Pause (nur Ambiente) | Dauerhafte Musik ermüdet; ohne Pathos heißt auch: nicht ständig. [vorläufig] |
| Ergebnis | `victory` / `defeat` beim entschiedenen Spiel, danach still; zurück im Menü wieder `menu` | Zum Ergebnisbildschirm passend; 029 spielt dazu schon den kurzen Hinweiston. |
| Ambiente-Schichten | Wind je Biom (Bett), Insekten (Wiese bzw. Wüste), Laub (Wald), Wasser, Regen (vorbereitet für 040, `rain` 0..1), Vögel je Biom (One-Shots), fernes Grollen (One-Shot alle 40–120 s) | Deckt die Liste des Prompts; Regen wartet auf das Wetter. |
| Hörmitte und Raster | Alle 0,5 s 7 × 7 Proben um die Hörmitte (Radius 40 m × Zoomfaktor): Wasseranteil aus `terrain_at`, Walddichte aus neuer Brückenfunktion `wood_density_at`; Wasser und Laub als je ein `AudioStreamPlayer3D` am Schwerpunkt der Treffer (ohne Entfernungsdämpfung, Pegel aus dem Anteil) | Kein Spieler pro Baum oder Fluss; der Schwerpunkt gibt die Richtung im Stereobild. Die Brückenfunktion liest nur ein vorhandenes Darstellungsraster, die Sim bleibt unberührt. |
| Kamerahöhe | Nah (≤ 60 m Kameraabstand): Details voll, Wind −8 dB; fern (≥ 450 m): Details −24 dB, Wind voll; logarithmisch dazwischen | Wie im Prompt: nah Vögel und Wasser, fern Wind. |
| Vögel nach Gefecht | Jedes Kampfereignis im Umkreis von 250 m × Zoomfaktor um die Hörmitte lässt Vögel 60 s schweigen, danach kommen sie über 20 s zurück | Glaubwürdig, billig. [vorläufig] |
| `--no-sound` | Lässt auch Ambiente und Spielmusik weg (beide hängen an `battle_sound`) | Ein Schalter für alles Hörbare im Spiel; `main.gd` bleibt unverändert. |

## 4. Umfang

1. **Ambiente** nach Biom (Wiese/Wald, Wüste, Winter) und nach Kamerahöhe gemischt
   (nah: Details wie Vögel und Wasser; fern: Wind). Wasserflächen und Wälder auf der
   Karte als Klangquellen, ohne einen Player pro Baum (Rasterabfrage an der Hörmitte).
   Vögel verstummen eine Weile nach Gefechtslärm in der Nähe.
2. **Musik:** wenige ruhige Stücke ohne Pathos, Wechsel zwischen „ruhig“ und „Gefecht“
   nach Kampfintensität mit Überblendung, eigene Stücke für Menü und Ergebnis
   (Sieg/Niederlage). Herkunft: selbst komponiert/erzeugt, CC0 oder vom Nutzer
   bereitgestellt (auch KI-generiert), jeweils mit Lizenz in `docs/SOURCES.md`.
   Keine Musik aus anderen Spielen oder Filmen, keine historischen Aufnahmen mit
   unklaren Rechten.
3. **Einstellungen:** Busse `Music` und `Ambience` mit eigenen Reglern (029 legt sie an).
4. **Selbsttest:** jede Biom-Kulisse und jedes Stück lädt; Wechsel ruhig/Gefecht
   passiert in der Demo-Schlacht.

## 5. Akzeptanzkriterien

1. `just check` grün, Golden-Hash unverändert.
2. `just selftest` grün, Musik und Ambiente laufen headless ohne Fehler.
3. Speicherbedarf der Musik gestreamt, nicht vollständig im RAM.
4. Doku in `docs/ARCHITECTURE.md` (Audio) und Quellen in `docs/SOURCES.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
