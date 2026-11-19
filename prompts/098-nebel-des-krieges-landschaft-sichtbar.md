# Prompt 098 – Nebel des Krieges: Landschaft immer sichtbar, Einheiten nur im Sichtbereich

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `prompts/done/018-nebel-des-krieges.md`, `godot/scenes/world/fog_of_war.gd` ganz,
> `crates/bridge/src/vision_funcs.rs`, `godot/scenes/hud/minimap.gd` (Nebel-Anteil im
> Shader), in `godot/scenes/main.gd` alle Stellen mit `fog_of_war`, `_fogged` und
> `state_at`, `godot/scenes/audio/battle_sound.gd` (`visible_at`),
> `godot/scenes/debug/debug_menu.gd`, `data/vision.toml` und die Abschnitte zu Sicht
> und Nebel in `docs/ARCHITECTURE.md`, `docs/GAME_DESIGN.md`, `docs/CONTROLS.md` und
> `docs/PERFORMANCE.md`. Sieh dir das Vorbild-Bild an:
> `docs/.vorlagen/nebel/nebel_vorbild.webp` (nicht im Repo, nur lesen).

## 1. Ausgangslage

Der Nebel des Krieges (Prompt 018) ist eine Bildschirm-Überlagerung in
`fog_of_war.gd`: ein Quad vor der Kamera liest den Tiefenpuffer, rechnet je Pixel die
Weltposition aus und dunkelt nach dem Zustand der Sichtzelle ab (8-m-Zellen,
`SimHost.vision_image`: 0 nie gesehen, 128 erkundet, 255 sichtbar). Heute:

- **nie gesehen:** fast schwarz (`unexplored_color`, Deckkraft 0,86), die Landschaft
  ist dort praktisch nicht zu erkennen;
- **erkundet:** dunkel (`explored_color`, Deckkraft 0,5);
- **sichtbar:** klar.

Gegnerische Einheiten außerhalb der Sicht stehen gar nicht im Snapshot; `main.gd`
blendet ihre Knoten aus. Die Minimap benutzt dieselbe Textur.

## 2. Ziel

Der Nutzer will den Nebel wie im Vorbild-Bild:

- Die **Landschaft ist auf der ganzen Karte von Anfang an zu sehen**: Gelände,
  Straßen, Wasser, Bäume, Felsen, Häuser. Über allem, was gerade nicht aufgeklärt
  ist, liegt nur eine **halbdurchsichtige Schicht** (dunkler, kühler, etwas
  entsättigt), durch die man alles gut erkennt und die Karte lesen und planen kann.
- **Klar und in vollen Farben** ist nur, was eigene oder verbündete Einheiten und
  Gebäude gerade sehen. Der Übergang ist weich, ohne sichtbares Zellraster.
- **Gegnerische Einheiten sieht man nur dort, wo gerade aufgeklärt wird.** Das ist
  heute schon so und bleibt so.

Der Nutzer hat ein **zweites Vorbild-Bild** gezeigt (Wiese am Fluss mit Gräben und
Bunkern). Es liegt nur dann als Datei vor, wenn er es nach
`docs/.vorlagen/nebel/` gelegt hat (alle Bilder dort ansehen); sonst gilt diese
Beschreibung:

- Der Sichtbereich ist eine **runde, zusammenhängende helle Fläche** mit sattem
  Grün; außerhalb ist dieselbe Wiese grau-oliv, deutlich entsättigt und auf etwa
  die halbe Helligkeit gedämpft, aber Bodenstruktur, Ufer, Wasser, Schilf und
  Wald bleiben gut erkennbar.
- Der **Rand ist eine klare, glatte Kurve** mit nur schmalem weichem Saum (im
  Verhältnis wenige Meter, deutlich schmaler als ein Graben lang ist), kein
  breiter Verlauf und keine Treppen. Mehrere Sichtkreise verschmelzen zu einer
  Fläche.
- Feste Stellungen (Gräben, Sandsackringe, Bunker mit Fahne) sind auch im
  gedämpften Teil zu sehen, nur eben gedämpft. Für uns heißt das: was zur **Karte**
  gehört, ist immer sichtbar; was der Gegner im Spiel **neu baut**, folgt
  Abschnitt 3.2.
- Linien und Marken (Zonenrahmen, gestrichelte Linien) liegen **über** der Schicht
  und leuchten im gedämpften Teil unverändert.

Die Bilder sind nur Vorbild für die **Wirkung** (Verhältnis hell/gedämpft, weicher
Rand). Nichts daraus nachbauen oder übernehmen (keine Farben abgreifen, keine
HUD-Elemente, keine blauen Kreise oder Zonenrahmen; die gehören dort zu anderen
Spielfunktionen).

## 3. Umsetzung

Reine Darstellung. An `crates/sim` ändert sich nichts: Sichtregeln, Zellzustände,
Snapshot-Filter und was die KI sieht, bleiben. Der Golden Hash darf sich nicht
ändern.

### 3.1 Schicht im Hauptbild

- Den Shader in `fog_of_war.gd` umbauen: Der schwarze Zustand „nie gesehen“
  entfällt in der Darstellung. Alles, was nicht 255 ist, bekommt dieselbe
  halbdurchsichtige Schicht. Der Zustand 128 bleibt in der Bridge und für
  `state_at`/`share_of` erhalten (Tests, Geräusche, Effekte nutzen ihn).
- Die Schicht soll nicht nur schwarz darüberlegen. Ziel: außerhalb der Sicht
  Helligkeit etwa auf 55–65 %, Sättigung deutlich herunter, leicht ins
  Kühle/Graue, sodass der aufgeklärte Bereich wie von der Sonne beschienen
  heraussticht. Entsättigen braucht die Bildfarbe (`hint_screen_texture`); vorher
  prüfen, was das mit Wasser, Rauch, Leuchtspur und anderen durchsichtigen Flächen
  macht, die vor der Schicht gezeichnet werden, und was es an Bildrate kostet. Ist
  es zu teuer oder fehlerhaft, bei einer gemischten Farbe bleiben (`blend_mix` mit
  graublauem Ton) und das im Ergebnis sagen.
- Stärke, Farbton und Entsättigung als Shader-Uniforms mit Konstanten oben im
  Skript, damit der Nutzer sie leicht nachstellen kann. Werte so wählen, dass die
  Schicht bei allen Kartenlooks (Wald, Wüste, Winter, Stadt) und bei Nacht,
  Dämmerung, Regen und Nebelwetter wirkt: Im Schnee darf der gedämpfte Bereich
  nicht heller als der sichtbare wirken, bei Nacht muss der Unterschied noch
  erkennbar sein, ohne dass der Rest schwarz absäuft.
- **Weicher Rand:** Der Übergang soll eine runde, glatte Kurve sein wie im zweiten
  Bild: schmaler Saum (Richtwert 3–6 m, höchstens eine Zelle), ohne Treppen und
  ohne erkennbare 8-m-Ecken. Dafür muss die Kante aus dem groben Zellraster
  geglättet werden (z. B. weichgezeichnete Textur plus enger `smoothstep` um den
  Schwellwert, damit die Kurve rund, der Saum aber schmal bleibt). Wenn die fünf Abtastungen dafür nicht reichen:
  die Textur auf der CPU oder in einem kleinen Zwischenschritt weichzeichnen oder
  mehr/breitere Abtastungen. Zusätzlich die Änderung über die Zeit glätten (z. B.
  alte und neue Textur über ~0,3 s überblenden), damit der Rand beim Fahren nicht
  in 8-m-Sprüngen hüpft. Einfachste Lösung nehmen, die ruhig aussieht.
- Ränder außerhalb des Sichtgitters (heute Zustand 0) genauso gedämpft zeichnen wie
  den Rest, nicht schwarz.

### 3.2 Was im gedämpften Bereich zu sehen ist

Vor der Umsetzung im Code nachsehen, wie die folgenden Dinge heute behandelt werden
(Snapshot-Filter, `_fogged`, `state_at`), und dann diese Regeln sicherstellen:

- **Karteninhalt** (Gelände, Vegetation, Kartenobjekte, Häuser, Brücken, Straßen):
  immer sichtbar, auch nie betretene Teile.
- **Gegnerische Einheiten** (Fahrzeuge, Infanterie, Geschütze, Flugzeuge nach den
  heutigen Regeln): nur im Sichtbereich. Beim Verlassen der Sicht sofort weg wie
  heute, kein „Geist“ an der letzten Position.
- **Was der Gegner verändert hat**, darf im gedämpften Bereich nichts verraten.
  Prüfen und, wo nötig, verbergen: vom Gegner errichtete Gebäude und Feldstellungen
  (Gräben, Sandsäcke, Minen, Sperren), besetzte Häuser (Flagge/Markierung),
  eroberte Punkte, Fahrspuren, Staub, Rauch, Mündungsfeuer, Leuchtspur,
  Einschläge, Brände, frische Wracks und Krater, zerstörte Häuser und Brücken,
  gefällte Bäume.
  - Kurzlebige Effekte (Staub, Mündungsfeuer, Leuchtspur, Einschläge) im
    gedämpften Bereich nicht zeichnen (nach heutiger Regel in `main.gd`; nur
    Lücken schließen).
  - Dauerhafte Veränderungen der Karte (Zerstörung, Krater, Wracks, Feldstellungen)
    so einfach wie möglich: Wenn der Snapshot sie heute ungefiltert liefert und ein
    „zuletzt gesehener Zustand“ je Objekt nötig wäre, das **nicht** in diesem Prompt
    bauen, sondern die Lücke im Ergebnis klar nennen und als Folge-Prompt
    vorschlagen. Keine Änderung am Snapshot-Filter in `crates/sim` ohne Rückfrage.
- **Eigene und verbündete Einheiten und Gebäude:** immer sichtbar (sie sehen ja
  selbst).
- **Geräusche** im gedämpften Bereich: wie heute gedämpft (`battle_sound.gd`),
  unverändert.

### 3.3 Minimap

- Derselbe Grundsatz: die ganze Karte ist von Anfang an gezeichnet, außerhalb der
  Sicht gedämpft, im Sichtbereich hell. Kein Schwarz für „nie gesehen“.
  Gegnermarken nur im Sichtbereich (wie heute, prüfen).
- Hauptbild und Minimap sollen dieselbe Stimmung haben (gleiche Konstanten oder
  daraus abgeleitet).

### 3.4 Schalter und andere Ansichten

- Debug-Schalter „Fog of war“ und `reveal_all` funktionieren weiter: aus = keine
  Schicht, alle Einheiten.
- Replay und Zuschauer (Sicht eines Spielers oder alles sichtbar,
  `replay_funcs.rs`), Karteneditor (kein Nebel), Trailer-Aufnahmen
  (`trailer_shots.gd`), Einheiten-Thumbnails: prüfen, dass nichts ungewollt
  gedämpft wird oder Aufnahmen sich ändern, die ohne Nebel gedacht waren.
- Zieleingaben in den gedämpften Bereich (Bewegen, Angriff auf Boden, Artillerie,
  Luftschlag, Bauen) bleiben wie heute möglich bzw. gesperrt; nur prüfen, dass
  Zielmarken, Bauvorschau und Reichweitenkreise **über** der Schicht gut lesbar
  sind und nicht mitgedämpft werden.

## 4. Regeln

- Reine Darstellung in `godot/` (und höchstens kleine Lesefunktionen in
  `crates/bridge`). Kein Godot-Zufall, keine Godot-Physik für Spielrelevantes.
- Keinen Test abschwächen. `just selftest` prüft Nebelanteile (`share_of`,
  `updates`); die Aussagen bleiben, nur Erwartungen an entfernte Farben/Uniforms
  anpassen.
- Batch-Sessions teilen sich das Arbeitsverzeichnis: vor Messungen und schweren
  Builds nachsehen, ob andere Prompt-Sessions laufen.

## 5. Vorgehen

1. Bestand lesen und die Liste aus 3.2 gegen den Code abgleichen (Tabelle: Ding,
   heute sichtbar im Nebel ja/nein, soll, Maßnahme).
2. Vorher-Bilder aufnehmen: dieselbe Kameraposition auf einer Wald-, einer Wüsten-
   und einer Winter- oder Stadtkarte bei Tag, einmal bei Nacht; je mit eigener
   Einheit am Bildrand des Sichtbereichs und einer gegnerischen Einheit knapp
   innerhalb und knapp außerhalb der Sicht; dazu die Minimap.
3. Shader und Minimap umbauen, weichen Rand und zeitliche Glättung einbauen.
4. Lücken aus Schritt 1 schließen, soweit ohne Sim-Änderung möglich.
5. Nachher-Bilder derselben Stellen; mit dem Vorbild-Bild vergleichen
   (Helligkeitsverhältnis sichtbar/gedämpft, Randbreite) und die Konstanten
   nachstellen, bis die Wirkung passt.
6. Selftest erweitern: zu Spielbeginn ist ein nie gesehener Kartenpunkt im Bild
   nicht schwarz (z. B. Helligkeit eines Bildpunkts dort über einer Schwelle und
   unter der eines sichtbaren Punkts); eine gegnerische Einheit außerhalb der
   Sicht hat keinen sichtbaren Knoten, innerhalb schon; mit ausgeschaltetem Nebel
   ist die Schicht weg.
7. `just check`, `just selftest`, `just lan-selftest`, `just export-selftest`;
   `just bench-view` vorher/nachher (die Schicht deckt jetzt fast das ganze Bild,
   bei Bildfarben-Zugriff kommt eine Kopie des Bildes dazu). Bildrate darf nicht
   merklich fallen; sonst die einfachere Variante nehmen.
8. Doku: `docs/ARCHITECTURE.md` (Nebel-Darstellung), `docs/GAME_DESIGN.md`
   (Entscheidung: Karte von Anfang an bekannt, nur Einheiten verborgen, mit Datum),
   `docs/CONTROLS.md` falls dort der Nebel beschrieben ist, `docs/PERFORMANCE.md`
   (Messwert).
9. Für den Nutzer: Vorher/Nachher-Bilder nebeneinander, die Konstanten zum
   Nachstellen in einem Satz, und die Liste der Dinge, die im gedämpften Bereich
   noch etwas über den Gegner verraten (Vorschlag für einen Folge-Prompt). Er
   probiert es danach mit `just run`.

## 6. Nicht Teil dieses Prompts

- Änderungen an Sichtweiten, Sichtlinien, Tarnung oder am Snapshot-Filter
  (`crates/sim`, `data/vision.toml`).
- „Zuletzt gesehener Zustand“ für Gebäude, Wracks und Feldstellungen des Gegners
  (nur als Befund melden).
- Eine Einstellung im Menü für die Nebelart (alt/neu); die neue Darstellung ersetzt
  die alte.
- Sichtkreise, Zonenrahmen oder HUD-Elemente aus dem Vorbild-Bild.
