# Prompt 111 – Ton: mehr Wucht für schwere Waffen und Fahrzeuge (Entwurf)

> **Entwurf.** Vor dem Ausführen konkretisieren lassen („Bitte konkretisiere Prompt
> 111“). Grund: Am Ton wird gerade gearbeitet (nicht eingecheckte Änderungen an
> `godot/scenes/audio/ambience.gd`, `music.gd`, `tools/audio/music.py`,
> `tools/sounds_web.py`; private Aufnahmen aus `../mud-and-steel-sfx`), und der
> Auftrag braucht ein Hörurteil des Nutzers, das ein Agent nicht ersetzen kann.
>
> Zum Konkretisieren lesen: `CLAUDE.md`, `docs/AUDIO_LIBRARY.md` ganz,
> `docs/SOURCES.md`, `prompts/done/029-*.md`, `053-*.md`, `061-*.md`, `069-*.md`,
> `083-*.md` (Sound-Testseite, `just sounds-web`),
> `godot/scenes/audio/battle_sound.gd`, `sound_classes.gd` (`blast_tail_db`,
> `hit_ricochet`, `hit_penetration`), `acoustics.gd` (Laufzeit des Schalls,
> Überschallknall, Hall, Tiefpass beim Herauszoomen), `sound_buses.gd`,
> `battle_voices.gd` (Rufe für de/su/us, „None ship yet“),
> `godot/assets/audio/sounds.json` und `tools/audio/synth.py`.

## 1. Ausgangslage

Das Referenzvideo `docs/.vorlagen/vorlage.mp4` ist ein kommentierter Test; der
Spielton liegt unter der Sprecherstimme und lässt sich **nicht getrennt
beurteilen**. Verwertbar ist, was der Tester über den Ton sagt:

- Gelobt: Infanterie feuert in **kurzen Stößen**, Panzer schießen mit **dumpfem
  Grollen**, Artilleriegeschosse **pfeifen**, Luftkämpfe ergeben ein gewolltes
  Durcheinander. Einheiten **antworten in ihrer Landessprache**; das Briefing ist
  vertont.
- Kritik: Den Klängen fehlt **Substanz**. Wer mit schwerem Gerät eine Stellung
  zerlegt, will diese Macht in **Fahrgeräusch und Schuss** hören, bei Panzern,
  Artillerie und dem Panzerzug.

Bei uns heute (Stand der Bestandsaufnahme vom 2026-11-19):

- Aufnahmen (CC0/CC BY bzw. privat) für Kanonen, Haubitze, Mörser, Treffer,
  Explosionen, MG, Kettenlaufwerk; noch **synthetisch**: Gewehr, Knall, das
  Pfeifen anfliegender Granaten, die meisten Motoren.
- Entfernungsmodell mit Schalllaufzeit, Überschallknall, Hall je Umgebung, Tiefpass
  beim Herauszoomen. Eine eigene **Tiefton-Schicht** gibt es nur als
  `explosion_tail` ab 100 mm, nicht für Abschüsse und Motoren.
- **Stimmen fehlen ganz** (nur Untertitel), auch im Briefing.

## 2. Fragen an den Nutzer (vor dem Konkretisieren klären)

1. Was genau gefällt am Ton des Videos? (Bitte zwei, drei Stellen mit Zeit nennen
   und in Worten sagen, was dort gut klingt; daraus werden die Ziele.)
2. Was stört am eigenen Ton am meisten: zu dünn, zu leise, zu gleichförmig, zu
   viel auf einmal, zu wenig Raum?
3. Dürfen weitere private Aufnahmen aus `../mud-and-steel-sfx` dazukommen, oder
   nur CC0/CC BY und Synthese?
4. Stimmen: sollen Rufe und Bestätigungen der Einheiten jetzt kommen (Sprecher,
   erzeugte Stimmen, Lizenz)? Das ist ein eigener, großer Auftrag.

## 3. Mögliche Inhalte (nach den Antworten auswählen)

- **Tiefton-Schicht für Abschüsse** nach Kaliber: kurzer Druckstoß unter dem
  Mündungsknall, in der Nähe kräftig, in der Ferne nur noch das Grollen (das
  Entfernungsmodell hat die Laufzeit schon).
- **Motoren und Ketten nach Masse:** schwere Fahrzeuge tiefer und mit mehr Körper,
  Last hörbar (Anfahren, Steigung, Schlamm), Kettenrasseln getrennt vom Motor.
  Synthetische Motoren durch Aufnahmen ersetzen, wo es welche gibt.
- **Anflug der Granate** als Aufnahme oder bessere Synthese; Einschlag mit
  Nachhall je Umgebung.
- **Feuerstöße:** prüfen, ob MG und Maschinenpistolen in Stößen hörbar sind oder
  als Dauerton; Rhythmus aus der Simulation (Schusszeitpunkte), nicht geschätzt.
- **Mischung:** Pegel der Klassen gegeneinander, Vorrang bei vielen Stimmen, damit
  ein Panzerschuss in der Nähe nicht im Gewehrfeuer untergeht; Gesamtdynamik
  (Begrenzer auf dem Hauptbus prüfen).
- **Bildschirmwackeln und Ton** bei schweren Abschüssen in der Nähe abstimmen
  (`screen_shake` gibt es).
- Vergleich immer mit `just sounds-web` und kurzen Aufnahmen vorher/nachher, die
  der Nutzer anhört; kein Urteil nur nach Messwerten.

## 4. Regeln

- Nichts aus anderen Spielen übernehmen, auch keine Klänge. Jede neue Aufnahme
  mit Quelle und Lizenz in `docs/AUDIO_LIBRARY.md` und `CREDITS.md`.
- Reine Darstellung; `crates/sim` und der Golden Hash bleiben.
- Erst prüfen, ob die laufende Ton-Arbeit eingecheckt ist; sonst den Nutzer
  fragen.
