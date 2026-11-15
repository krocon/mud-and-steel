# Prompt 029 – Sound-Effekte

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Kampf, Effekte, Menüs/Einstellungen, Infanterie, Basisbau)
> und `docs/PERFORMANCE.md`.

## 1. Ziel

Das Spiel ist bisher stumm: Es gibt keine einzige Audiodatei und keinen Audio-Player,
nur einen Lautstärkeregler in den Einstellungen. Nach diesem Prompt hört man das
Gefecht: Kanonen, MGs und Gewehre, Einschläge und Explosionen, Motoren und Ketten,
einstürzende Gebäude, fallende Bäume, dazu UI-Klicks und knappe Rückmeldungen auf
Befehle. Sound ist reine Darstellung in Godot und liest nur Ereignisse und Zustand,
die die Brücke schon liefert.

## 2. Ausgangslage

- Kampf-Ereignisse kommen als Signal `combat_view.combat_event` (Schuss, Einschlag,
  Explosion, Trümmer); `effects.gd` reagiert schon darauf. `ARCHITECTURE.md` nennt
  Sound ausdrücklich als nächsten Abnehmer dieser Ereignisse.
- Zerstörung (020), Krater (021), Infanterie (022) und Basisbau (023) haben eigene
  Views mit eigenen Ereignissen.
- `menu/settings.gd` speichert eine Gesamtlautstärke, die noch nichts steuert.
- Nebel des Krieges: `fog_of_war.state_at(point)` sagt, ob ein Punkt sichtbar ist;
  Effekte sind schon daran gekoppelt.

## 3. Entscheidungen (vor dem Umsetzen prüfen und begründen)

| Frage | Vorschlag | Warum |
|---|---|---|
| Herkunft der Sounds | Selbst erzeugt (Synthese per Skript, z. B. Python/numpy oder SoX, versioniert unter `tools/audio/`) plus CC0-Aufnahmen mit Quellennachweis in `docs/SOURCES.md`. Der Nutzer kann zusätzlich KI-generierte Sounds (z. B. aus einem Text-zu-Sound-Werkzeug) unter `assets-src/audio/` ablegen; sie brauchen Werkzeug, Prompt und Lizenz (kommerzielle Nutzung erlaubt) in `docs/SOURCES.md` | Keine Sounds aus anderen Spielen; Synthese ist reproduzierbar wie die Blender-Modelle. Die Pipeline muss Dateien jeder Herkunft gleich behandeln (Normalisieren, Schneiden, Varianten). Downloads vorher mit mir abstimmen. |
| Format | OGG Vorbis für lange Loops, WAV für kurze Einzelschüsse; Git LFS | Godot-Standard, kleine Dateien. |
| Busse | `Master` → `Effects`, `UI`, `Music` (für 030), `Ambience` (für 030); Lautstärke je Bus in den Einstellungen | Der vorhandene Regler wird zum Master; Einzelregler kommen dazu. |
| Räumlicher Klang | `AudioStreamPlayer3D` mit Abschwächung nach Entfernung zur Kamera-Bodenmitte, nicht zur Kamera selbst | Bei hoher Draufsicht wäre sonst alles gleich leise. Zoom regelt die Hörweite. |
| Stimmenlimit | Pool fester Größe pro Klangklasse, ältester/leisester fliegt raus, Kanonen haben Vorrang vor Gewehren | Schlachten mit Hunderten Schüssen pro Sekunde dürfen weder übersteuern noch Performance kosten. |
| Nebel | Ereignisse im Nebel hört man gedämpft (Tiefpass, leiser), nicht stumm | Gefechtslärm ist im Krieg eine Aufklärungsquelle; verrät aber keine genaue Position. Entscheidung dokumentieren. |
| Fernwirkung | Schallgeschwindigkeit nicht simulieren (oder nur als kleine Verzögerung bei weit entfernten Explosionen, mit Zeitfaktor 2) | Einfach halten; entscheiden und begründen. |

## 4. Umfang

1. **Klangbibliothek** `godot/assets/audio/` mit Manifest (`audio.toml` oder `.tres`):
   Klangklasse → Dateien (mehrere Varianten pro Klasse gegen Wiederholung), Lautstärke,
   Tonhöhen-Streuung, Hörweite, Priorität.
2. **Zuordnung aus dem Katalog:** Waffenklang nach Kaliber/Waffenart (Gewehr, MG,
   Kanone klein/mittel/groß, Mörser/Haubitze vorbereitet für 032), Motorklang nach
   Fahrzeugart (Rad, Halbkette, Kette leicht/schwer). Keine Einzelzuordnung pro
   Einheit nötig.
3. **Gefecht:** Schuss, Einschlag (Boden/Panzerung/Abpraller), Explosion, Wrack brennt
   (Loop), Gebäude stürzt ein, Baum fällt, Krater.
4. **Bewegung:** Motor-Loops für fahrende Fahrzeuge (Lautstärke/Tonhöhe nach
   Geschwindigkeit), Kettengeräusch; nur die nächsten N Fahrzeuge zur Hörmitte bekommen
   einen eigenen Player.
5. **UI und Rückmeldungen:** Klicks, Auswahl, Befehl bestätigt, Bau fertig, Einheit
   produziert, Einheit unter Beschuss/verloren (Hinweiston mit Abklingzeit), Sieg und
   Niederlage. Keine Sprachausgabe (kann später kommen).
6. **Einstellungen:** Regler pro Bus, Stummschalten bei Fokusverlust (Option).
7. **Werkzeuge:** `just audio` erzeugt die synthetischen Sounds neu; eine Hörprobe-Szene
   (`--sound-gallery`) spielt jede Klangklasse einmal ab.
8. **Selbsttest:** headless prüfen, dass jede Klangklasse eine ladbare Datei hat, dass
   das Stimmenlimit hält (Schlacht-Benchmark ohne Fehler) und dass Busse aus den
   Einstellungen gesetzt werden.

## 5. Akzeptanzkriterien

1. `just check` grün, Golden-Hash unverändert (Sound fasst die Sim nicht an).
2. `just selftest`, `just lan-selftest`, `just editor-selftest` grün; neuer Audio-Check
   läuft in `just selftest` mit.
3. Benchmark aus 016 (Schlacht): FPS höchstens 5 % schlechter als ohne Sound.
4. Jede Datei hat eine Quelle in `docs/SOURCES.md` (Synthese-Skript oder CC0-Link).
5. Doku: Abschnitt „Audio“ in `docs/ARCHITECTURE.md`, Konventionen in `docs/ASSETS.md`.

## 6. Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Downloads oder Installationen (z. B.
SoX) fragen. Gemeinsame Dateien (`main.gd`, `self_test.gd`, `settings.gd`, `justfile`,
Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen. Am Ende nach
`prompts/done/` verschieben und `prompts/README.md` aktualisieren. Abschlussbericht:
was gebaut wurde, wie ich es höre und teste, Messwerte, was offen ist.
