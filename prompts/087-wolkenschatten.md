# Prompt 087 – Schatten vorbeiziehender Wolken auf dem Boden

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `godot/scenes/world/atmosphere.gd` und `godot/scenes/world/weather_look.gd` ganz (vor
> allem `WeatherLook.adjust`, `_cloud_cover`, `WIND_MATERIALS` und wie der Wind aus
> `sim_host.weather_info()` gelesen wird), `godot/scenes/world/graphics_quality.gd`,
> `godot/assets/biomes/biome_preset.gd`, die Shader `godot/assets/materials/terrain.gdshader`
> (mit `terrain_meadow.gdshaderinc`, `terrain_tall_grass.gdshaderinc`), `water.gdshader`
> und `godot/assets/shaders/` (`env*.gdshader*`, `tall_grass*.gdshader*`,
> `meadow_tuft.gdshader`, `unit_camo.gdshader`, `soldier.gdshader`), in
> `docs/ARCHITECTURE.md` den Abschnitt „Weather, mud and time of day“ (Prompt 040) und die
> Abschnitte zu Atmosphäre (015, 057) und Wiese, in `docs/PERFORMANCE.md` die Messregeln.

## 1. Ausgangslage

Die Sonne ist ein `DirectionalLight3D` mit harten Schatten; das Wetter eines Gefechts
(`clear`, `overcast`, `rain`, `snow`, Nebel) dimmt über `_cloud_cover` nur die
Sonnenstärke und färbt den Himmel grau. Der Boden ist deshalb überall gleich hell: bei
klarem Wetter wirkt eine große Wiese flach und starr, obwohl Gras und Bäume sich im Wind
bewegen. Wolken selbst sieht man in der hohen Draufsicht nie, ihre Schatten wären das
Einzige, was vom Himmel im Bild ankommt.

Der Wind liegt schon vor: die Simulation hat eine Windrichtung und -stärke
(`sim_host.weather_info()["wind"]`, m/s in x/z), Rauch, Schnee und Regen folgen ihr.

## 2. Ziel

Große, weiche Schattenflecken ziehen langsam mit dem Wind über die Karte und dunkeln
alles ab, was in der Sonne liegt. Fertig ist der Prompt, wenn

- bei klarem Wetter am Tag einzelne Wolkenschatten über Gelände, Wiese und hohes Gras,
  Wasser, Bäume, Gebäude, Fahrzeuge und Soldaten wandern, und zwar **zusammenhängend**:
  ein Panzer, der in einen Wolkenschatten fährt, wird zusammen mit dem Boden unter ihm
  dunkler; kein Objekt bleibt hell auf dunklem Boden oder umgekehrt,
- die Schatten zur Welt passen: Flecken von grob 100–400 m mit weichen Rändern
  (Übergang über einige zehn Meter), dazwischen deutlich mehr Sonne als Schatten bei
  `clear`; sie ziehen in Windrichtung der Simulation, bei Zeitfaktor 2 mit sichtbarer,
  aber ruhiger Bewegung (Richtwert 10–20 m/s im Bild; entscheide selbst und begründe),
  und verändern beim Ziehen langsam ihre Form (zwei gegeneinander verschobene
  Rauschlagen), sodass sich kein Muster erkennbar wiederholt,
- nur das **direkte Sonnenlicht** abgedunkelt wird, nicht das Umgebungslicht: im
  Wolkenschatten bleibt alles lesbar, Einheiten und Teamfarben sind in jeder Zoomstufe
  klar zu erkennen (Richtwert: direkte Sonne im Kern auf 35–50 %), der Schlagschatten
  eines Baums verschwindet im Wolkenschatten weitgehend statt doppelt dunkel zu werden,
- das Wetter die Schatten steuert, alles an einer Stelle in `weather_look.gd` neben
  `_cloud_cover`: `clear` wenige klare Flecken; `overcast` fast geschlossene Decke mit
  wenigen, schwachen Sonnenlöchern (die Sonne ist dort schon gedimmt, also kaum
  Kontrast); Regen, Schnee, Nebel: keine erkennbaren Flecken; Morgen und Abend: Flecken
  wie am Tag, in Richtung der tiefen Sonne leicht gestreckt wäre schön, ist aber keine
  Pflicht; Nacht: höchstens ein Hauch im Mondlicht, eher aus,
- die Stärke je Biom einstellbar ist (`BiomePreset`, z. B. `cloud_shadow_strength`,
  `cloud_shadow_scale`; Wüste: wenige, harte Flecken, Winter: schwach),
- es einen Schalter in den Grafikeinstellungen gibt („Wolkenschatten“ an/aus, in allen
  vorhandenen Sprachen) und die Qualitätsstufe `low` sie nur dann zeigt, wenn die Messung
  (Abschnitt 4) keinen spürbaren Verlust ergibt,
- Nebel des Krieges, Wetter-Nässe, Schnee, Krater, Brandflecken und die Wiesenfarben
  unverändert darunter weiterarbeiten (Wolkenschatten ist eine Lichtänderung, keine
  Bodenfarbe),
- `just check`, `just selftest` und `just export-selftest` grün sind und der Golden Hash
  unverändert bleibt.

## 3. Regeln und Technik

- **Reine Darstellung.** Nichts in `crates/sim`; Wolkenschatten ändern weder Sicht noch
  Tarnung noch irgendeinen Wert. Die Position der Schatten darf deshalb auf jedem Rechner
  anders sein und aus der Bildzeit laufen (keine Godot-Zufallszahlen für Spielrelevantes
  wird dadurch nicht berührt). In Pause und im Zeitraffer sollen sie der Spielzeit folgen
  (bei Pause stehen), in Wiederholungen ebenso.
- **Ein gemeinsames Muster in Weltkoordinaten.** Alle Materialien müssen an derselben
  Weltposition denselben Wert bekommen. Naheliegend: eine kleine, kachelbare
  Rauschtextur (einmal erzeugt und eingecheckt oder beim Start aus `FastNoiseLite`
  gebacken) und globale Shader-Uniformen (Textur, Versatz, Maßstab, Stärke), gelesen über
  eine gemeinsame Include-Datei (z. B. `godot/assets/shaders/cloud_shadow.gdshaderinc`),
  die in jedem Welt-Shader an genau einer Stelle den **direkten** Lichtanteil dämpft
  (`light()`-Funktion bzw. dort, wo der Shader die Sonne verrechnet). `atmosphere.gd`
  setzt die Uniformen je Bild. Prüfe zuerst, ob Godot in der gepinnten Version
  (`docs/VERSIONS.md`) einen einfacheren Weg bietet (ein Projektor auf dem
  `DirectionalLight3D` wäre ideal; nach meinem Stand können das nur Omni- und
  Spot-Lichter – in der Dokumentation der gepinnten Version nachsehen, nicht raten), und
  wähle den Weg mit den wenigsten berührten Dateien.
- **Vollständigkeit prüfen statt annehmen.** Such alle Materialien, die in der Welt
  Sonnenlicht bekommen, auch `StandardMaterial3D`/importierte Materialien an Gebäuden,
  Wracks, Stellungen, Brücken und Effekten. Was nicht über einen eigenen Shader läuft,
  braucht eine Lösung (auf den vorhandenen Shader umstellen oder begründet auslassen:
  Rauch, Feuer, Mündungsblitze, unschattierte Marker und das HUD bleiben unberührt).
  Liste im Ergebnis auf, was abgedeckt ist und was bewusst nicht.
- **Keine neuen Lichter, keine zweite Schattenkarte, keine großen durchsichtigen
  Flächen** über der Karte (Überzeichnen, Sortierprobleme mit Rauch und Wasser).
- Thumbnails (`thumbnail_render.gd`), Modellgalerie, Einheitenkarten und die Webseite
  rendern **ohne** Wolkenschatten (Standardwert der Uniformen = kein Schatten), damit
  keine Bilder neu erzeugt werden müssen; prüfe das mit einem Thumbnail vor und nach der
  Änderung (bitgleich oder sichtbar gleich).
- Der Karteneditor zeigt sie nicht.
- Prompt 086 (Bäume im Wind) ändert dieselben Laub- und Gras-Shader und die
  Windparameter in `atmosphere.gd`. Ist er noch offen oder läuft er gerade, zuerst seinen
  Stand lesen, die Wolkenschatten in der Include-Datei halten und in den geteilten
  Shadern nur die eine Aufrufzeile ergänzen. Wolken und Bäume sollen denselben Wind
  zeigen (Richtung aus der Simulation).
- Trailer-Aufnahmen (`trailer_shots.gd`, Prompt 082 arbeitet dort gerade): Wolkenschatten
  dürfen an sein, müssen aber bei gleicher Szene und gleichem Seed gleich liegen (Versatz
  aus der Aufnahmezeit, nicht aus der Wanduhr). Nur das Nötigste dort ändern und
  Konflikte vermeiden.

## 4. Vorgehen

1. Vorher-Standbilder nach `build/cloud_shots/before_*.png`: eine offene Wiesenkarte
   (z. B. `ambush_meadow`) und eine Karte mit Wald und Gebäuden, je nahe, mittlere und
   weite Zoomstufe, Wetter `clear` am Tag; dazu `overcast` und Abend je einmal.
2. Entscheidung für den technischen Weg (Abschnitt 3) kurz begründen, dann umsetzen:
   Include-Datei, Uniformen, Steuerung in `atmosphere.gd`/`weather_look.gd`, Felder im
   `BiomePreset`, Einstellung im Menü.
3. Nachher-Standbilder mit denselben Kameras nach `build/cloud_shots/after_*.png`, dazu
   eine Bildfolge über 20–30 s Spielzeit an einer festen Kamera (z. B. alle 2 s ein Bild),
   an der man das Ziehen und die Formänderung sieht. Zeig Vorher und Nachher nebeneinander
   und mindestens ein Bild, in dem ein Fahrzeug und ein Trupp halb im Schatten stehen.
4. Messen nach `docs/PERFORMANCE.md` (`just bench-view`, vorher/nachher, alle
   Qualitätsstufen, mindestens die größte Karte): Bildzeit darf sich nicht messbar
   verschlechtern (Richtwert unter 2 %); sonst vereinfachen (eine Rauschlage statt zwei,
   Abtastung im Vertex-Shader bei Gras) oder auf `low` abschalten. Laufen gerade andere
   Sessions im selben Checkout, vor dem Messen pausieren lassen.
5. Self-Test ergänzen (`self_test.gd` oder ein kleines Werkzeug): Uniformen sind bei
   `clear` gesetzt und wandern mit der Spielzeit in Windrichtung, stehen in der Pause,
   sind bei Regen und im Thumbnail-Renderer aus, und der Schalter in den Einstellungen
   wirkt. Vorhandene Tests nicht abschwächen.
6. Doku: `docs/ARCHITECTURE.md` (Abschnitt Wetter: Wolkenschatten, Tabelle der
   Entscheidungen), `docs/PERFORMANCE.md` (Messwerte), `docs/ASSETS.md`, falls neue
   Materialien künftig die Include-Datei brauchen (eine Zeile „jeder Welt-Shader bindet
   `cloud_shadow.gdshaderinc` ein“), `docs/CONTROLS.md` nur, falls dort Grafikeinstellungen
   stehen.

## 5. Nicht Teil dieses Prompts

- Sichtbare Wolken am Himmel, Wolkenspiegelung im Wasser, volumetrische Wolken.
- Wetterwechsel während des Gefechts (bleibt: ein Wetter je Gefecht, Prompt 040).
- Jede Spielwirkung der Schatten (Sicht, Tarnung, Luftaufklärung).
- Änderungen an Sonnenstand, Schattenkaskaden oder Tiefenschärfe, außer ein Fehler dort
  fällt durch die Wolkenschatten erst auf (dann melden, nicht nebenbei umbauen).
