# Prompt 003 – Maßstab und Einheitenkatalog

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` und `docs/DETERMINISM.md`. Deren Regeln gelten unverändert.
> Kann parallel zu Prompt 002 laufen (andere Dateien). Am besten in einem eigenen
> Git-Worktree beziehungsweise Branch.

## 1. Ziel

Eine verbindliche Grundlage, damit alle Einheiten und Objekte zueinander passen: in der
Größe, in der Geschwindigkeit auf der Karte und in den Spielwerten. Sie ist die
Voraussetzung, um die Blender-Modelle früh und parallel zu bauen (Prompts 004–008).

Drei Fraktionen: **Deutsche (`de`)**, **Sowjets (`su`)**, **Amerikaner (`us`)**.
Zeitraum: Schwerpunkt 1943–1945. Drei Tarnungen: **Wald/gemäßigt**, **Wüste**,
**Winter**.

## 2. Feste Vorgaben (von mir entschieden)

- **Größen 1:1** in echten Metern, auch Infanterie (Referenzfigur 1,80 m). Keine
  Vergrößerung für die Lesbarkeit; die kommt später über Markierungen und Symbole.
- **Zeitfaktor 2:** Alles Zeitabhängige läuft doppelt so schnell wie real.
  Geschwindigkeiten × 2, Dauern (Nachladen, Bauen, Aufbauen) ÷ 2. Pro Kategorie
  überschreibbar, falls das Balancing es später verlangt.
- Echte Werte stehen in den Daten; die Umrechnung passiert an genau einer Stelle.

## 3. Umfang

### 3.1 `docs/SCALE.md` und `data/scale.toml`
- Begründung und Formeln: Größen 1:1, Zeitfaktor 2, gestauchte Reichweiten für
  Direktfeuer, Sicht und indirektes Feuer, Sonderregeln für Flugzeuge (z. B. eigener
  Geschwindigkeitsfaktor, Anflug vom Kartenrand). Für die noch offenen Faktoren
  begründete Startwerte vorschlagen; ich bestätige sie im Abschlussbericht.
- Tabelle „Was bedeutet das auf der Karte?“: Fahrzeit über 1 km und 4 km für typische
  Einheiten, Kampfentfernungen im Verhältnis zur Bildschirmbreite bei mittlerem Zoom
  (Kamera aus Prompt 002, falls schon vorhanden; sonst Annahme dokumentieren).
- `scale.toml` nur mit Ganzzahlen bzw. Brüchen (Zähler/Nenner), damit die Sim sie
  deterministisch nutzen kann.

### 3.2 Einheitenkatalog `data/catalog/`
- Eine TOML-Datei pro Fraktion (`de.toml`, `su.toml`, `us.toml`) plus `neutral.toml`
  für Umwelt und Zivilobjekte. Nur Ganzzahlen mit Einheit im Feldnamen
  (`length_mm`, `speed_road_kmh_x10`, `armor_front_mm` …).
- Felder je Eintrag (Vorschlag, in Phase 0 bestätigen): `id`, `faction`, `category`,
  `name_de`, `name_en`, `designation` (historische Bezeichnung), `years`,
  Abmessungen (Länge mit und ohne Rohr, Breite, Höhe), Masse, Besatzung bzw.
  Truppstärke, Geschwindigkeiten (Straße, Gelände, ggf. Wasser), Reichweite,
  Panzerung (Wanne/Turm vorn/seitlich/hinten), Bewaffnung (Liste mit Kaliber, Munition,
  Kadenz), `tier` (1 = Kern, 2 = Rest), `model` (Pfad, bis zum Modell leer),
  `sources` (Liste: URL oder Buch + was daraus stammt).
- **Umfang je Fraktion** (historisch passende Vertreter auswählen und begründen):
  - Infanterie: Schützentrupp, leichtes MG, schweres MG, leichter und mittlerer
    Mörser, Aufklärer, Scharfschütze, Sanitäter, Pionier, Panzerabwehr-Infanterie,
    Flammenwerfer, Offizier/Funker.
  - Fahrzeuge: leichter, mittlerer, schwerer Panzer, Panzerjäger/Jagdpanzer,
    Sturmgeschütz, Schützenpanzer/Halbkette, Lkw, Aufklärungsfahrzeug, Selbstfahrlafette
    (Artillerie oder Flak).
  - Geschütze: Panzerabwehrkanone, leichte und schwere Flak, Feldhaubitze, schwerer
    Mörser.
  - Befestigungen: MG-Bunker, Schützengraben-Abschnitt, Sandsack-Stellung,
    Panzersperre, Stacheldraht, Minenfeld (Panzer- und Schützenminen).
  - Flugzeuge: Jäger, Schlachtflugzeug/Jagdbomber, Bomber, Aufklärer.
- **Stufe 1 (Kern)**, für Prompt 006 festgelegt:
  - `de`: Schützengruppe, MG-42-Trupp, Panzer IV Ausf. H, Sd.Kfz. 251/1, 7,5 cm PaK 40
  - `su`: Schützentrupp, MG-Trupp (DP-27 oder Maxim), T-34/76 (1943), Lkw ZiS-5,
    76-mm-Divisionskanone ZiS-3
  - `us`: Rifle Squad, MG-Team (M1919A4), M4A3 Sherman (75 mm), M3 Halftrack,
    57-mm-Pak M1
- `neutral.toml`: Bäume (Laub, Nadel, Birke, Palme), Büsche, Gras, Felsen, Bauernhaus,
  Scheune, Steinhaus, Lehmhaus (Wüste), Kirche, Holz- und Steinbrücke, Holzzaun,
  Steinmauer, Telegrafenmast, Heuballen, Kisten/Fässer. Gebirge und Flüsse sind Gelände
  (Prompt 010/014), keine Modelle.
- `just catalog-doc` erzeugt aus den TOML-Dateien `docs/UNIT_CATALOG.md` (Tabelle pro
  Fraktion mit den wichtigsten Werten und Quellen). Die Doku wird nie von Hand gepflegt.
- Recherche darf parallel mit Subagenten laufen (z. B. einer pro Fraktion). Jeder Wert
  braucht eine Quelle; Unsicheres wird markiert statt geraten.

### 3.3 Quellen und Rechtliches (`docs/SOURCES.md`)
- Bevorzugt: US-Army Technical Manuals (TM 9-…, gemeinfrei), Museumsangaben,
  Fachliteratur. Fakten und Maße sind frei verwendbar; Zeichnungen und Fotos Dritter nur
  als Modellier-Referenz, nie ins Repo.
- Niemals Modelle, Texturen oder Daten aus anderen Spielen.
- Hoheitszeichen: Balkenkreuz für `de`, roter Stern für `su`, weißer Stern für `us`.
  Kein Hakenkreuz und keine SS-Runen (§ 86a StGB, Steam/USK). Taktische Zeichen und
  Nummern sind erlaubt.
- Sanitäter ohne Rotes Kreuz (geschütztes Zeichen nach den Genfer Abkommen);
  stattdessen z. B. weiße Armbinde und Sanitätstasche.

### 3.4 Asset-Konventionen (`docs/ASSETS.md`)
Grundlage für alle Blender-Prompts; aus dem Abschnitt „Asset pipeline“ in
`ARCHITECTURE.md` hierher verschieben und erweitern:
- Ordner: `assets-src/blueprints/<faction>/<id>.toml`,
  `assets-src/blender/<faction>/<id>.blend`, `godot/assets/models/<faction>/<id>.glb`.
- Maßstab 1 BU = 1 m, Z oben, Vorderseite +Y in Blender (−Z in Godot), Ursprung am
  Boden in der Mitte der Standfläche, Rotation und Skalierung angewendet.
- Hierarchie und Namen beweglicher Teile (`Hull`, `Turret`, `Gun`, `Wheel_L_01`,
  `Track_L`, `Propeller` …) und Anbaupunkte als Empties (`Socket_Muzzle`,
  `Socket_Exhaust`, `Socket_Hatch`, `Socket_Crew_01` …).
- Polygon- und Texturbudgets pro Kategorie, abgeleitet aus der RTS-Draufsicht
  (Vorschlag mit Begründung, z. B. Panzer LOD0 ≤ 8 000 Dreiecke).
- Material-Slots für das Tarn-System aus Prompt 004 (`Paint`, `Metal`, `Rubber`,
  `Wood`, `Canvas`, `Glass`, `Skin`, `Cloth`).
- Regeln für parallele Arbeit: Jede Session schreibt nur in ihre eigenen Dateien;
  gemeinsame Bausteine nur additiv ändern.

### 3.5 Simulation
- `sim` liest Einheitentypen aus dem Katalog statt aus `data/units.toml` (Datei
  entfernen) und wendet `scale.toml` an (Zeitfaktor als Bruch, exakt in `Fixed`).
- Das Demo-Szenario nutzt einen Katalog-Panzer statt `placeholder_tank`.
- Golden-Hash bewusst neu festlegen und den Grund im Commit nennen.

### 3.6 Referenzfigur
- `tools/blender/scale_reference.py` erzeugt eine 1,80-m-Referenzfigur und einen
  Maßstabsbalken als `.blend`, damit spätere Aufstellungs-Renderings sie nutzen können.

## 4. Akzeptanzkriterien

1. `just check` grün, neuer Golden-Hash.
2. Ein Test lädt alle Katalogdateien, prüft Pflichtfelder, eindeutige IDs, plausible
   Wertebereiche und dass jeder Stufe-1-Eintrag Quellen hat.
3. `docs/UNIT_CATALOG.md` ist generiert und stimmt mit den TOML-Dateien überein (ein
   Test vergleicht beides oder die CI erzeugt und vergleicht).
4. `docs/SCALE.md`, `docs/SOURCES.md`, `docs/ASSETS.md` existieren; `CLAUDE.md`
   verweist darauf.
5. Abschlussbericht mit den vorgeschlagenen Faktoren (Reichweiten, Flugzeuge) zur
   Bestätigung und einer Liste unsicherer Katalogwerte.

## 5. Arbeitsweise

- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe.
- **Phase 0:** Katalog-Felder, Einheitenauswahl pro Fraktion und Budget-Vorschläge
  zeigen und auf mein Go warten. Danach recherchieren und umsetzen.
- Kein Test wird abgeschwächt. Pro Schritt ein Commit.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` aktualisieren, committen.
