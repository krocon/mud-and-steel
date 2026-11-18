# Prompt 086 – Bäume bewegen sich sichtbar im Wind

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann in
> `docs/ASSETS.md` die Abschnitte zu Umgebungsobjekten (Vertexfarben, „Wind“, „Distance:
> far meshes instead of impostors“, Budgets) und in `docs/ARCHITECTURE.md` „Map objects
> and decoration (presentation only)“, den Biom-Ablauf (`atmosphere.gd` → Wind an die
> Env-Materialien), „Weather“ (Prompt 040, Wind je Wetter) sowie die Abschnitte zu Büscheln
> und hohem Gras (Böen, Flimmermaße). Code: `godot/assets/shaders/env_common.gdshaderinc`
> (`wind_offset`), `env_foliage.gdshader`, `env.gdshader` (Rinde, Slot 10),
> `tall_grass_common.gdshaderinc` (Böenwelle `v_gust`), `meadow_tuft.gdshader`,
> `godot/scenes/world/atmosphere.gd` (`WIND_MATERIALS`), `weather_look.gd`,
> `map_objects.gd`, `env_look.gd`, `godot/assets/biomes/*.tres`,
> `tools/blender/kit/env.py` und `kit/vegetation.py` (Windgewicht und Phase je Fläche).

## 1. Ausgangslage

- Bäume haben bereits einen Wind im Shader: `wind_offset()` verschiebt Laub- und
  Rindenvertices nach den eingebackenen Vertexfarben (G = Gewicht, B = Phase), Stärke,
  Richtung und Tempo kommen aus dem Biom (`forest.tres`: 0,25 / (1, 0,3) / 1,2) und werden
  vom Wetter skaliert.
- Im Spiel **sieht man davon fast nichts**: Der Ausschlag ist an der Kronenspitze höchstens
  rund 20 cm (0,25 × (0,85 · Gewicht² + 0,12 · Gewicht)), aus der Spielkamera weniger als
  ein Pixel bis wenige Pixel. Die Bewegung ist ein reiner Sinus, alle Bäume schwingen
  gleichförmig mit nur leicht versetzter Phase; es gibt keine Böen. Die Rinde bekommt die
  Hälfte (`* 0.5`), Stamm und Krone können dadurch auseinanderlaufen.
- Büschel und hohes Gras wogen dagegen deutlich, das hohe Gras mit einer Böenwelle, die über
  die Wiese läuft. Wald und Einzelbäume wirken daneben wie eingefroren.
- Zuerst prüfen, ob das stimmt: ob der Wind im Spiel wirklich an den Baum-Materialien
  ankommt (MultiMesh-Instanzen, nahe und ferne Meshes, Schatten-Kopien, Qualitätsstufen)
  oder irgendwo auf null steht. Ergebnis im Abschlussbericht nennen.

## 2. Ziel

Auf Karten im Wald-und-Wiesen-Biom (`forest`) bewegen sich die Bäume sichtbar und
glaubwürdig im Wind, im Wald wie als Einzelbaum auf der Wiese: Kronen wiegen sich, Äste
schwingen nach, Laub zittert, und Böen laufen als Welle durch den Wald – passend zum Gras
darunter. Die anderen Biome bekommen dieselbe Technik mit eigenen Werten.

Fertig ist der Prompt, wenn:

- die Bewegung **nur Darstellung** ist: nichts in `crates/sim`, kein Einfluss auf Sicht,
  Deckung, Treffer oder Hash. Der Wind der Simulation (Rauchdrift, Feuer, `weather_info()`)
  bleibt, wie er ist; die Bäume **lesen** ihn nur (Richtung und Stärke wie heute über
  Biom und `weather_look.gd`).
- die Bewegung aus drei Anteilen besteht, alle im Vertexshader, ohne Texturen und ohne
  Skelett:
  1. **Wiegen des ganzen Baums**: Stamm und Krone biegen sich gemeinsam in Windrichtung,
     wachsend mit der Höhe über dem Boden (Fuß bleibt fest), mit langsamer Schwingung um
     eine leicht geneigte Ruhelage. Rinde und Laub benutzen dieselbe Funktion, damit die
     Krone nicht vom Stamm rutscht.
  2. **Äste und Kronenteile**: je Phase (Vertexfarbe B) ein eigenes, schnelleres Nachschwingen,
     auch quer zum Wind und leicht auf und ab.
  3. **Laubzittern**: kleines, schnelles Flattern der Karten wie heute, bei starkem Wind
     kräftiger.
- **Böen** als wandernde Welle über die Karte laufen (Weltposition entlang der Windrichtung,
  langsames Rauschen statt reinem Sinus), sodass man im Wald eine Welle von Krone zu Krone
  sieht. Dieselbe Welle wie im hohen Gras verwenden (`tall_grass_common.gdshaderinc`) oder
  beide auf eine gemeinsame Funktion stellen, damit Gras und Bäume derselben Böe folgen.
- Bäume **nicht im Gleichschritt** schwingen: Phase, Tempo und Ausschlag je Baum leicht
  verschieden (aus der Instanzposition), große Bäume träger als kleine; Nadelbäume steifer
  (Spitze schwingt, Stamm kaum), Birken weich, Palmen mit schwingenden Wedeln, Büsche und
  Hecken nur wenig.
- die Stärke aus der Spielkamera **erkennbar** ist: bei normalem Wetter im Biom `forest`
  auf mittlerer Zoomstufe ohne Suchen sichtbar, aber ruhig (Richtwert: Kronenspitze eines
  20-m-Baums 0,4–0,8 m, in Böen bis etwa 1,2 m); bei Regen/Sturm deutlich stärker, bei Nebel
  fast still. Die Werte festlegen, begründen und in den Biom-Presets bzw. `weather_look.gd`
  halten, nicht als Zahlen im Shader verstreuen. `desert` und `winter` bekommen eigene
  Werte (kahle Winterbäume: Äste schwingen, kein Laubzittern).
- **Schatten** der Bäume mitgehen (Schatten-Kopien des fernen Meshes, alle Kaskaden) und
  **ferne Meshes** (`_Far`, ab 180/220 m) dieselbe Bewegung zeigen, damit beim Umschalten
  nichts springt. Prüfen, ob die fernen Karten Gewicht und Phase eingebacken haben; wenn
  die Modelle neu exportiert werden müssen (`kit/vegetation.py`, `kit/env.py`): alle
  betroffenen Modelle neu erzeugen, Thumbnails mit `just thumbnails --only=...` neu rendern
  und `just units-web-sync` laufen lassen. Wenn möglich ohne Modelländerung auskommen
  (Höhe aus `VERTEX.y`, Gewicht aus G).
- **umgestürzte, verkohlte und brennende** Bäume sich richtig verhalten: liegende Bäume
  bleiben still (`env_fallen`), der Übergang beim Umstürzen (`map_objects.gd`, `_falling`)
  springt nicht; Stümpfe bewegen sich nicht. Falls Prompt 085 (kahle Bäume) schon umgesetzt
  ist: kahle Bäume schwingen mit den Ästen, ohne Laubzittern.
- die Bewegung **nicht flimmert**: Kronenränder und Schatten auf der Wiese dürfen bei Wind
  nicht stärker flirren als heute. Mit den vorhandenen Messwerkzeugen prüfen
  (`ground-motion` / `trailer_shots.gd --off=wind` als Vergleich) und die Maße vor und nach
  der Änderung nennen.
- die **Bildrate** hält: über dem dichtesten Wald (`just objects-bench`) und im
  Gesamtbenchmark (`just bench-view`) auf „high“ und „medium“ höchstens 2 % langsamer als
  vorher; auf „low“ darf der Wind auf das einfache Wiegen reduziert sein. Die Böe darf im
  Vertexshader nur wenige Rechenschritte kosten (kein `fbm3` je Vertex in vier
  Schattenkaskaden).
- Wind in den **Werkzeugen** abschaltbar bleibt: `trailer_shots.gd --off=wind` stellt auch
  die Bäume still (heute nur Büschel, hohes Gras und Boden), Thumbnails und
  `model_gallery.gd` rendern Bäume in Ruhelage, damit Bilder reproduzierbar sind.
- `just check`, `just selftest` und `just editor-selftest` grün sind. Der Golden Hash darf
  sich **nicht** ändern (reine Darstellung); ändert er sich doch, ist etwas falsch.

## 3. Vorgehen

1. Vorher festhalten: ein kurzes Video oder eine Bildfolge (5 s, 60 fps) von einem
   Waldrand mit Wiese und Einzelbäumen auf mittlerer und naher Zoomstufe, dazu ein
   Differenzbild zweier Frames im Abstand von 1 s (zeigt, wie viel sich bewegt). Ein
   kleines Werkzeug dafür anlegen (`just wind-shots`, nach dem Muster von `blast_shots.gd`
   / `snow-shots`), Ausgabe nach `build/wind_shots/before_*`. Karte: `ambush_meadow` oder
   eine Waldkarte mit hohem Gras.
2. Ursache klären (Abschnitt 1, letzter Punkt), dann die Windfunktion in
   `env_common.gdshaderinc` neu aufbauen; `env_foliage.gdshader` und `env.gdshader`
   rufen dieselbe Funktion auf.
3. Böenwelle mit dem hohen Gras zusammenführen; Parameter (Böenlänge, Tempo, Anteil) in
   die Biom-Presets, vom Wetter skaliert.
4. Je Baumart abstimmen (Laubbaum, Nadelbaum, Birke, Palme, kahle Eiche, Busch, Hecke) in
   allen drei Biomen und bei Wetter „klar“, „Regen“, „Nebel“, „Schnee“.
5. Schatten, ferne Meshes, Umschaltgrenze, umgestürzte Bäume prüfen.
6. Nachher-Aufnahmen mit denselben Kameras nach `build/wind_shots/after_*`, Differenzbilder
   nebeneinander zeigen; Bildrate und Flimmermaße messen.
7. Doku: `docs/ASSETS.md` („Wind“, Vertexfarben, falls geändert), `docs/ARCHITECTURE.md`
   (Map objects: Wind; Biom-Ablauf; Wetter-Tabelle), `docs/PERFORMANCE.md` mit den
   Messungen.

## 4. Nicht Teil dieses Prompts

- Wind als Spielregel (Einfluss auf Geschosse, Rauch, Feuer, Sicht): bleibt unverändert.
- Wechselnde Windrichtung oder -stärke im Lauf einer Partie; höchstens unter „Open“
  notieren.
- Bäume, die auf Explosionen, Druckwellen, vorbeifahrende Panzer oder Hubschrauber/Flugzeuge
  reagieren (eigener Prompt; die neue Funktion so bauen, dass sich ein lokaler Stoß später
  ergänzen lässt).
- Fallendes Laub im Wind, Äste, die abbrechen, Blätterrauschen als neuer Klang (der
  Ambiente-Klang `leaves` bleibt, wie er ist; nur prüfen, ob seine Lautstärke zur
  sichtbaren Windstärke passt, und Auffälliges notieren).
- Neue Baummodelle oder mehr Dreiecke.

## 5. Abstimmung mit laufender Arbeit

- Prompt 085 (Bäume unter schwerem Feuer) fasst dieselben Dateien an (`env_foliage.gdshader`,
  `map_objects.gd`, eventuell die Baummodelle). Nicht gleichzeitig ausführen; wer zweiter
  ist, baut auf dem Stand des ersten auf.
- Prompt 082 (Video „Hinterhalt am Waldrand“) nimmt Wald und Wiese auf. Stärkerer Wind
  ändert die Bilder, nicht die Szene. Wenn 082 schon aufgenommen hat, im Prompt 082
  vermerken, dass die Shots mit dem neuen Wind anders aussehen.
