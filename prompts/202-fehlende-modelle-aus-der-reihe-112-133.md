# Prompt 202 – Fehlende Modelle aus der Reihe 112–133

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `docs/ASSETS.md` ganz (Konventionen, Budgets, Baukasten, Vorschaubilder),
> `docs/SOURCES.md`, `docs/SCALE.md`, den Baukasten unter `tools/blender/kit/`
> (besonders `buildings.py`, `med.py`) und die Baupläne unter `assets-src/blueprints/`.
> Für jede Gruppe unten zusätzlich den genannten Prompt in `prompts/done/`
> (Abschnitte „Offen“/„Nicht Teil dieses Prompts“ und Entscheidungen).
>
> **Dieser Prompt läuft in Gruppen.** Jede Gruppe ist für sich fertig (Modelle,
> Katalog, Vorschaubilder, Prüfungen, Commit), bevor die nächste beginnt.
> Nach jeder Gruppe kurz Bilder zeigen und ohne Halt weiterarbeiten.

## 1. Wunsch des Nutzers (2026-11-23)

Während der Prompts 112–133 haben die Sessions Modelle als „offen“ gemeldet, die
heute durch Platzhalter oder vorhandene Katalog-Modelle vertreten werden. Der Nutzer
möchte sie gesammelt in einem Prompt bauen lassen.

## 2. Weg

- Modelle entstehen **wie bisher aus Skripten** (Baukasten und Baupläne, Blender ohne
  Fenster), damit sie wiederholbar sind und `validate`/Budgets greifen. Das
  Blender-MCP darf zum Ansehen, Prüfen und für den Vergleich mit Bildvorlagen dienen,
  aber kein Modell darf nur als Handarbeit in einer `.blend` ohne Skript entstehen.
- Neue Katalogeinträge nur mit echten Werten und Quelle (`docs/SOURCES.md`; keine
  Daten aus anderen Spielen, keine Hakenkreuze/SS-Runen, kein Rotes Kreuz).
- Nach jeder Modelländerung: `just thumbnails --only=<id>,…`, `just units-web-sync`,
  bei Katalogänderung `just catalog-doc`; Fahrzeuge zusätzlich `just vehicle-db-check`.
- Ersetzt ein neues Modell einen Platzhalter auf einer Karte (Geodaten- oder
  Skizzenkarte), den Importer bzw. die Skizze umstellen, Karte neu erzeugen,
  `just gen-geo-check`, `just map-check`, `just map-reach` und im Commit sagen, dass
  sich die Karte beabsichtigt ändert. Golden Hash: neue Regeldaten sollen eine Partie
  nur dann berühren, wenn sie vorkommen (Muster `economy::keeps_faction`, Prompt 117,
  und die Boot-Regeln aus 119); ändert er sich doch, Grund finden und nennen.
- Bildrate auf den betroffenen Karten vorher/nachher messen (`just objects-bench`
  bzw. `just bench-view`), wenn Gebäude oder Pflanzen in großer Zahl ersetzt werden.

## 3. Gruppen

### A. Fahrzeuge und Flugzeuge

- **US-Jeep** (Willys MB): fehlt im US-Katalog; Prompt 121 musste die zweite Welle
  mit Halbkette fahren lassen. Katalog, Modell, Wirtschaft/Baulisten wie beim
  britischen Jeep aus 117; Mission `gulf_landing` danach prüfen.
- **Spitfire IX, Typhoon Ib, Mosquito FB VI** (117): heute aus US-Bauplänen skaliert,
  Silhouetten nur ungefähr. Nach Zeichnungen neu, Maße gegen Katalog.
- **Britische Vereinfachungen** (117): Universal Carrier hinten offen, 25-Pfünder
  mit Bettung (Feuerplattform), Churchill-Ketten über den Rumpf geführt.
- **Horsa-Lastensegler** (117): Modell. Die Landeregel (Luftlande-System kennt nur
  Absprung) nur, wenn sie klein bleibt; sonst Modell bauen und die Regel als offenen
  Punkt mit Vorschlag notieren.
- **LCT und Marinefährprahm** (119): Modelle nach dem Boots-Baukasten aus 119
  (Baugruppe `boat`, Bugklappe `Ramp`), Katalog mit Quellen; nur in Missionen.

### B. Befestigungen

- **Schwere Küstengeschütze, Kasematten, Flak am Kai** (113, 121): heute
  `fort_bunker_mg`, Gräben und Sandsäcke als Platzhalter; in 121 stehen leFH 18
  für 90- und 102-mm-Geschütze.
- **Forts, Batterien und Kasernen** der Küstenhöhen (113): heute normale
  Katalog-Häuser.

### C. Gebäude

- **Backsteinviertel** (112): Brownstone-Reihenhaus, Mietshaus mit Feuertreppen,
  gusseisernes Loft (SoHo), Lagerhaus am Kai; Wahrzeichen: Bogen am Washington Square,
  Kirchen. Importer `brick_quarter` danach auf die neuen Modelle umstellen.
- **Dünenküste** (114): Reetdachhaus und weitere Inselbauten.
- **`env_church`** (120): alter Fehler, eine Turmwand reicht bis in die Kirchenhalle;
  im neuen Campanile behoben, das alte Modell neu bauen und alle Karten mit Kirche
  ansehen.

### D. Pflanzen

- **Feigenkaktus, Agave, Weinstöcke** (120), mit Baum-/Buschregeln wie die anderen
  Mittelmeer-Pflanzen; Weinberge im Importer danach als Weinstock-Reihen statt
  `field_green`.
- **Pinienkrone** (120): wirkt aus der Nähe wie Laub; Schirmform nachschärfen.

### E. Weitere Funde aus 122–133

Die Sessions der Prompts 122–133 laufen noch. Was sie als fehlende Modelle melden,
wird hier nachgetragen, bevor dieser Prompt startet.

- **Größere Kirche** (122): Im größten Dorf von `archipelago` steht heute eine
  normale Kirche plus Rathaus, weil der Katalog keine größere Kirche hat.
- **Brückenturm / Torhaus** (123): An den Brückenenden von `river_front` stehen
  heute `env_house_stone` und `fort_bunker_mg` als Stellvertreter.
- **Lkw als Kulisse** (124): Es gibt kein neutrales Lkw-Objekt; die Lager von
  `trench_line` haben deshalb keine abgestellten Lkw.
- **Bohlenweg** (124): Holzbohlen über Schlamm und Grabengrund fehlen auf
  `trench_line`. Falls dafür eine Regel nötig ist (Wegkosten), klein halten oder als
  offenen Punkt notieren.
- **Zelt `env_tent`** (124): wirkt im Spiel sehr hell; Farbe/Material prüfen.

## 4. Nicht Teil dieses Prompts

- Britische Stimmen, Boots- und Paddelgeräusche (Ton, keine Modelle).
- Hochstraße und Güterbahn auf Stelzen (112), Tunnel (113), Unterführungen (115):
  brauchen Regeln für zwei Ebenen, nicht nur Modelle.
- Kanada, Polen, Land Mattress (117).

## 5. Abschluss

Je Gruppe: `just check`, betroffene Selbsttests, Vorschaubilder, ein Commit mit
„(prompt 202)“. Am Ende Tabelle aller gebauten Modelle (Dreiecke, Budget, ersetzt
welchen Platzhalter, betroffene Karten) und offene Punkte in der README-Zeile,
Prompt nach `prompts/done/` verschieben.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle eintragen (vorläufig), mit
Begründung.
