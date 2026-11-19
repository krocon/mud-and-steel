# Prompt 091 – Hellblaueres Wasser, am Ufer ganz hell

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `godot/assets/materials/water.gdshader` ganz (vor allem `fragment()`: `transmit`,
> `deep`, `water`, `edge`/`foam`, und wie `ALBEDO` und `EMISSION` gemischt werden),
> `godot/scenes/world/water.gd` (welche Biom-Werte als Uniforms gesetzt werden und was
> `UV2` mit `v_shore`/`v_depth` enthält), den Abschnitt „Water“ in
> `godot/assets/biomes/biome_preset.gd` und die Wasserwerte in
> `godot/assets/biomes/{forest,desert,winter}.tres`, in
> `godot/assets/materials/terrain.gdshader` den Wasser-Teil (nasses Uferband,
> Flussbett unter Wasser), `godot/scenes/tools/water_self_test.gd`, in
> `docs/ARCHITECTURE.md` die Wasser-Abschnitte (Prompt 014) und
> `prompts/done/014-wasser-fluesse-bruecken.md`. Die Vorlage liegt in
> `docs/.vorlagen/_todo/maps/img_6.png` (nicht im Repo, nur auf dem Rechner).

## 1. Ausgangslage

Flüsse, Seen und Meer wirken im Spiel dunkel und grünlich-braun, fast wie
Moorwasser. Der Wald-Biom setzt gar keine Wasserfarbe und nimmt die Standardwerte aus
`biome_preset.gd` (`water_shallow` 0,24/0,26/0,14, `water_deep` 0,07/0,10/0,06); Wüste
und Winter sind kaum blauer. Dazu kommt, dass `absorb_tint` (1,0/0,9/0,55) Blau am
stärksten schluckt, sodass auch durchscheinender Grund gelblich wird.

Die Vorlage `img_6.png` zeigt das gewünschte Bild: Wasser in kräftigem, eher hellem
Blau (Flüsse und Buchten ein mittleres Türkisblau, offenes tiefes Meer ein dunkleres
Blau), und an jedem Ufer ein schmaler, deutlich hellerer, fast weiß-hellblauer Saum,
der nach innen weich ins Blau übergeht. Der Übergang folgt der Tiefe: flach = hell,
tief = dunkler; kein harter Strich.

Die Vorlage ist eine gezeichnete Draufsicht. Übernommen werden nur die Farbstimmung
und der helle Ufersaum, keine Zeichentrick-Optik: Spiegelung, Wellen, Lichtbrechung
und Schaum bleiben.

## 2. Ziel

- Wasser in allen drei Biomen sichtbar blauer und heller als jetzt, ähnlich der
  Vorlage: Flüsse/Seen hell- bis mittelblau, tiefes Wasser dunkleres Blau.
- Am Ufer ein heller, fast hellblau-weißer Saum, einige Meter breit (am Fluss eher
  2–4 m, an der Meeresküste breiter, weil das Wasser dort flacher ausläuft), weich
  nach innen auslaufend. Er kommt aus der Wassertiefe bzw. dem Uferabstand, nicht aus
  dem Schaum.
- Biome bleiben unterscheidbar: Wüste eher türkis/klar, Wald ein etwas satteres Blau,
  Winter kühler und etwas grauer, aber alle deutlich blau.
- Wetter und Tageszeit wirken weiter (Nacht, Regen, bedeckt dunkeln ab).
- Reine Darstellung: nichts in `crates/sim`, Golden Hash unverändert.

## 3. Regeln und Technik

- Farben gehören in die Biom-Werte (`water_shallow`, `water_deep`, `water_clarity`,
  ggf. neu `water_shore` und `water_absorb`), nicht fest in den Shader. Der Wald-Biom
  bekommt eigene Einträge in `forest.tres`; die Standardwerte in `biome_preset.gd`
  passen dazu.
- Ufersaum: eine eigene Farbe `shore_color`, gemischt über `v_depth` (z. B. bis etwa
  0,6–1,0 m Tiefe) und `v_shore`, zusätzlich zum vorhandenen Schaum. Er muss auch bei
  `thickness` aus der Bildschirmtiefe stimmen, ohne beim Zoomen zu pumpen.
- Prüfen, ob das Blau eher über die eigene Wasserfarbe (`ALBEDO`) oder über die
  Absorption (`absorb_tint`, durchscheinender Grund als `EMISSION`) kommen soll. Ziel
  ist, dass flaches Wasser den hellen Grund durchscheinen lässt, aber bläulich
  getönt, und tiefes Wasser seine eigene blaue Farbe zeigt. Den Flussbett-Ton im
  `terrain.gdshader` nur anpassen, wenn er das Blau sonst braun färbt.
- Das nasse Uferband an Land (dunkler, glänzender Boden) bleibt; der helle Saum liegt
  auf der Wasserseite.
- Minimap, Editor und Thumbnails: dort, wo Wasser eine eigene Farbe hat, an die neue
  Farbe angleichen, damit Karte und Spiel zusammenpassen.
- Nicht übertreiben: das Spiel bleibt realistisch, kein Neon-Türkis. Die Vorlage ist
  die Richtung, nicht der exakte Farbwert.

## 4. Vorgehen

1. Vorher-Bilder: dieselben Stellen auf mindestens `river_city` (Wald),
   `desert_lagoon` (Wüste, Meer und Lagune), `island_chain` und `winter_town`, je
   Spielkamera mittlerer Zoom, bei Tag. Dafür vorhandene Aufnahmewege nutzen
   (`just trailer-shots --stills` oder einen kleinen eigenen Shot); Ablage in
   `build/water-color/before/`.
2. Farben und Ufersaum umsetzen, Biom für Biom abstimmen. Neben die Vorlage legen und
   vergleichen.
3. Nachher-Bilder an denselben Stellen nach `build/water-color/after/`, zusätzlich je
   ein Bild bei Nacht und bei Regen, damit nichts leuchtet.
4. Bildrate nach `docs/PERFORMANCE.md` kurz prüfen (Wasser-lastige Karte, eine
   mittlere und die niedrigste Qualitätsstufe); erwartet: unverändert. Laufen andere
   Sessions im selben Checkout, vor dem Messen pausieren lassen.
5. `water_self_test.gd` um eine Prüfung ergänzen, dass jeder Biom eine blaue Wasser-
   und Uferfarbe hat (Blauanteil größer als Rot und Grün, Saum heller als flaches
   Wasser). Vorhandene Tests nicht abschwächen.
6. Doku: `docs/ARCHITECTURE.md` (Wasser: Farbmodell, Ufersaum, neue Biom-Werte).
7. Der Nutzer schaut es sich danach selbst mit `just run` an; nenne ihm die Karten und
   Stellen, an denen der Unterschied am deutlichsten ist, und lege ihm je ein
   Vorher/Nachher-Paar vor.

## 5. Nicht Teil dieses Prompts

- Andere Wellen, Spiegelung, Strömung, Schaum-Muster oder Wassergeometrie.
- Andere Ufer- oder Flussformen in den Karten, Änderungen an Karten selbst.
- Regeln für Wasser in der Simulation.
- Zeichentrick- oder Kartenoptik wie in der Vorlage (Umrisse, flache Flächen).
