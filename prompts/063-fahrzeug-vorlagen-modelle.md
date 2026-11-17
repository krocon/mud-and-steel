# Prompt 063 – Fahrzeug-Vorlagen: fehlende Fahrzeuge aufnehmen, Modelle gegenprüfen

> **Dieser Prompt läuft in zwei Stufen.** Zuerst eine Vorab-Session auf `main`
> („Bitte führe `prompts/063-fahrzeug-vorlagen-modelle.md` aus, Vorab-Session“), danach
> neun Modell-Sessions parallel, jede in einem eigenen Git-Worktree/Branch
> `assets/refs-<gruppe>` („… für Gruppe `ger-pz4` aus“). Gruppen siehe Abschnitt 4.
>
> Lies zuerst `CLAUDE.md`, `docs/SOURCES.md`, `docs/ASSETS.md` (Abschnitte „Part kit“ und
> „Hierarchy and names“), `docs/SCALE.md` und `data/catalog/SCHEMA.md`.

## 1. Ziel

Der Ordner `docs/.vorlagen/fahrzeuge/` (nicht im Repo) enthält Referenzmaterial zu 141
Panzerfahrzeugen: 69 deutsche (`GER`), 48 sowjetische (`RU`), 24 amerikanische (`US`).
Nach diesem Prompt gilt:

1. Jedes Fahrzeug der Vorlagen ist entweder ein Katalog-Eintrag mit geprüftem Modell oder
   steht mit Begründung als „nicht aufgenommen“ in der Bestandsliste.
2. Die Modelle der Fahrzeuge, die es schon gibt, sind gegen Maße und Zeichnungen der
   Vorlagen geprüft und korrigiert.

## 2. Ausgangslage

- Aufbau der Vorlagen: `<NATION>/<Familie>/<Variante>/data.json`, dazu in den meisten
  Ordnern eine oder mehrere Zeichnungen (`*.jpg`, 117 Stück). 31 Ordner haben keine
  Zeichnung, darunter `Wespe`, `M5A1 Stuart`, `Hummel`, `Nashorn`, `SU-152`, `IS-1`
  und `IS-2 Model 1944`. Geladen wurde alles mit `just vehicle-refs`
  (`tools/onwar_download.mjs`) von onwar.com.
- `data.json` hat die Abschnitte `general_data`, `firepower`,
  `mobility_characteristics`, `armor_protection`, dazu `source_url` und teils `notes`.
  Alle Werte sind Text, die Schlüssel sind nicht einheitlich („Length /hull (m)“,
  „Length overall“, „Overall Length (m)“ …), Werte enthalten Zusätze wie
  „6.12 or 5.64*/5.64“ oder „3300 kg (7300 lbs)“. Panzerung steht als Dicke und Winkel
  je Seite für `Hull`, `Superstructure`, `Turret`, `Mantlet`, `Gun Shield` und
  Sonderfälle bei Mehrturmpanzern.
- Der Katalog hat je Fraktion neun Fahrzeuge. In den Vorlagen kommen davon vor:
  `de_pz2_l`, `de_pz4_h`, `de_tiger1_e`, `de_jgdpz38t`, `de_stug3_g`, `de_wespe`,
  `su_t70m`, `su_t34_76_1943`, `su_is2`, `su_su85`, `su_su76m`, `su_isu152`, `us_m5a1`,
  `us_m4a3_75`, `us_m26`, `us_m10`, `us_m4a3_105`. Die Zuordnung in der Vorab-Session
  prüfen (z. B. ob die Vorlagen den Luchs oder nur frühere Pz II führen).
  Halbketten, LKW und Spähwagen fehlen in den Vorlagen und bleiben, wie sie sind.
- Weitere Vorlagen außerhalb von `fahrzeuge/`: `docs/.vorlagen/hetzer/` und
  `docs/.vorlagen/Leichter-Panzerspähwagen-(2-cm)-Sd.Kfz.-222/` (Fotos und Modellbilder
  für `de_jgdpz38t` und `de_sdkfz222`). Für die Prüfung dieser beiden mitnutzen.
- Modelle entstehen aus Bauplänen (`assets-src/blueprints/<fraktion>/<id>.toml`) mit
  `just unit <fraktion> <id>`, geprüft mit `just validate-assets`, `just lineup` und
  `just gallery-check`. Picker-Bilder mit `just thumbnails --only=<id>,<id>`.

## 3. Regeln für die Vorlagen

- **Fakten ja, Bilder nein.** Maße, Massen, Panzerung, Geschwindigkeiten sind frei
  nutzbar und kommen mit Quelle in den Katalog. Zeichnungen und Fotos bleiben auf dem
  Rechner: nichts davon wird committet, nachgezeichnet als Textur verwendet oder in
  Vergleichsbildern ins Repo gelegt (`build/` ist dafür der Ort).
- **Quelle im Katalog:** `{ ref = "<source_url aus data.json>", covers = "…" }`. onwar.com
  ist eine Sekundärquelle (Rang 4 in `docs/SOURCES.md`). Spielrelevante Werte (Panzerung
  vorn, Masse, Geschwindigkeit, Kaliber, Mündungsgeschwindigkeit) gegen eine zweite
  Quelle prüfen; bei Widerspruch gewinnt die höherrangige Quelle, der Unterschied steht
  in `note`, das Feld in `uncertain`.
- Werte, die `data.json` als Bereich, mit „or“ oder mit Stern angibt, nicht still raten:
  gewählte Variante in `note` nennen, Feld in `uncertain`.
- `data.json` hat keine Mündungsgeschwindigkeit, keine Feuerrate und keine Reichweite.
  Diese Waffenwerte kommen aus den bevorzugten Quellen oder aus einem Katalog-Eintrag,
  der dieselbe Waffe schon führt (dann dessen Quelle übernehmen).
- Keine Hakenkreuze, keine SS-Runen, auch nicht, wenn eine Zeichnung sie zeigt.

## 4. Umfang

### Vorab-Session (einmal, auf `main`)

1. **Bestandsliste** `docs/VEHICLE_REFS.md` (Englisch): eine Zeile pro Vorlagen-Ordner
   mit Katalog-Id, Status (`existing`, `new`, `skipped`), Kategorie, Baugruppe und
   Fahrgestell-Verwandtschaft („shares chassis with …“), Zahl der Zeichnungen und Grund
   bei `skipped`. Dieselbe Zuordnung maschinenlesbar als `data/vehicles/index.json`
   (`"GER/Tiger/Tiger I": "de_tiger1_e"`), damit Prompt 064 sie nutzt.
   Ids nach dem Muster der vorhandenen (`de_pz4_h`, `su_t34_76_1943`, `us_m4a3_75`).
2. **Liste mir zeigen, bevor Modelle gebaut werden.** Die Vorab-Session endet mit der
   Bestandsliste und einem Vorschlag, was aufgenommen wird (siehe „Offene Fragen“).
   Die Modell-Sessions starten erst nach meiner Freigabe der Liste.
3. **Gemeinsamer Leser** `tools/vehicle_refs.py` (Python 3 ohne Abhängigkeiten, wie
   `tools/units_web.py`): liest alle `data.json`, vereinheitlicht die Schlüssel und
   wandelt Werte in Ganzzahlen mit Einheit im Namen (wie im Katalog). Was sich nicht
   sicher wandeln lässt, wird als Warnung gemeldet, nicht verworfen. Prompt 064 baut auf
   diesem Leser auf; die Aufteilung so halten, dass 064 nur einen Ausgabeweg ergänzt.
4. **Katalog-Einträge** für alle freigegebenen neuen Fahrzeuge in `de.toml`, `su.toml`,
   `us.toml`: Pflichtfelder nach `SCHEMA.md`, `model` leer, Quellen und `uncertain` wie
   in Abschnitt 3. Der Leser darf die Einträge vorschreiben; jeder Eintrag wird danach
   gelesen und geprüft, nicht blind übernommen. Prüfgrenzen in `catalog.rs` (z. B. Masse
   für die Maus) nur mit Begründung erweitern.
5. **Baukasten-Lücken** aus der Bestandsliste ableiten und schließen, bevor die
   Modell-Sessions starten: Mehrturm-Aufbauten (T-28, T-35, Neubaufahrzeug, M3 Lee mit
   Rumpfkanone), Schwimmpanzer-Rumpf (T-37, T-38, T-40), fehlende Laufwerke, offene
   Aufbauten der Beute-Selbstfahrlafetten. `just kit-test` grün.
6. **Vergleichswerkzeug** `tools/blender/ref_compare.py` mit `just ref-compare <id>`:
   rendert das gebaute Modell orthografisch von der Seite, von vorn und von oben in
   festem Maßstab mit Maßlinien (Länge Wanne, Breite, Höhe, Rohrüberstand,
   Kettenauflage, Bodenfreiheit) und legt die Zeichnungen des Vorlagen-Ordners daneben.
   Ausgabe nach `build/renders/compare/<id>.png` (nicht im Repo). Dazu eine Zahlentabelle
   Modell gegen `data.json` gegen Katalog mit Abweichung in Prozent.

### Modell-Sessions (neun, parallel)

| Gruppe | Vorlagen-Ordner | Fahrzeuge |
|---|---|---|
| `ger-light` | `GER/PzKpfw I`, `PzKpfw II`, `PzKpfw 35(t) and 38(t)` | 22 |
| `ger-pz3` | `GER/PzKpfw III` | 14 |
| `ger-pz4` | `GER/PzKpfw IV` | 15 |
| `ger-heavy` | `GER/Panther`, `Tiger`, `Maus`, `Neubaufahrzueg`, sechs Beute-Selbstfahrlafetten | 18 |
| `ru-light` | `RU/T-26`, `BT`, `T-27`, `T-37`, `T-38`, `T-40`, `T-50`, `T-60`, `T-70`, `SU-76` | 18 |
| `ru-medium` | `RU/T-34`, `T-24`, `T-28`, `T-35`, `SMK`, `TG` | 17 |
| `ru-heavy` | `RU/KV`, `IS` | 13 |
| `us-mixed` | `US/M3 Stuart`, `M3 Lee`, `M22 Locust`, `M24 Chaffee`, `M18 Hellcat`, `3 inch GMC M10`, `90mm GMC M36`, `M26 Pershing` | 14 |
| `us-sherman` | `US/M4 Sherman` | 10 |

Der Ordnername `Neubaufahrzueg` ist in den Vorlagen so geschrieben.

Jede Session, für jedes Fahrzeug ihrer Gruppe:

1. **Vorhandene Modelle zuerst prüfen** (Status `existing`): `just ref-compare <id>`,
   Bild ansehen, Prüfliste durchgehen, Bauplan korrigieren, neu bauen, erneut vergleichen.
2. **Neue Modelle** (Status `new`): Bauplan anlegen, Varianten einer Familie teilen
   Fahrgestell-Werte (erst das Grundfahrzeug sauber, dann die Ableitungen), bauen,
   vergleichen wie oben. `model` im Katalog setzen.
3. **Prüfliste** je Fahrzeug, Ergebnis als Zeile in `docs/VEHICLE_REFS.md`:
   - Länge Wanne, Breite, Höhe, Rohrüberstand: Abweichung zum Katalog innerhalb der
     Toleranz von `validate.py`.
   - Laufwerk: Zahl und Größe der Laufrollen, Stützrollen ja/nein und Zahl, Treibrad
     vorn oder hinten, Kettenbreite, Kettenauflage.
   - Proportionen: Lage und Grundriss des Turms oder Aufbaus auf der Wanne, Neigung der
     Bugplatte, Höhe der Wanne gegen Gesamthöhe, Bodenfreiheit.
   - Bewaffnung: Rohrlänge (Kaliberlängen), Mündungsbremse, Blende, Lage der Kanone
     (Versatz bei Kasematten), Zahl und Ort der MGs.
   - Erkennungsmerkmale der Variante gegen ihre Nachbarn (Schürzen, Kommandantenkuppel,
     Zusatzpanzerung, Turmform), soweit das Budget aus `docs/ASSETS.md` reicht.
   - Wo Zeichnung und `data.json` sich widersprechen oder die Zeichnung fehlt: melden,
     nicht raten.
4. **Bilder:** nach jeder Modelländerung `just thumbnails --only=<id>,<id>` (braucht ein
   Fenster). Geänderte Soldaten gibt es hier nicht; falls doch, alle betroffenen
   Trupp- und Besatzungsbilder neu.
5. `just validate-assets`, `just gallery-check` und `just catalog-check` grün,
   `just catalog-doc` neu erzeugt.

### Zusammenführen

Eine kurze eigene Session führt die neun Branches zusammen, löst Konflikte in den
Katalog-Dateien und in `docs/VEHICLE_REFS.md`, lässt alle Prüfungen laufen und rendert
`just lineup` als Gesamtbild.

## 5. Nicht in diesem Prompt

- Keine neuen Einträge in den Produktionslisten von `data/economy.toml`, keine
  KI-Änderung, keine neuen Regeln. Die neuen Fahrzeuge sind danach in Katalog, Galerie
  und Einheiten-Webseite zu sehen, aber nicht baubar (siehe „Offene Fragen“).
- Die JSON-Datenbank selbst (Prompt 064).

## 6. Akzeptanzkriterien

1. `docs/VEHICLE_REFS.md` und `data/vehicles/index.json` decken alle 141 Ordner ab;
   keine Zeile ohne Status.
2. Jedes Fahrzeug mit Status `existing` oder `new` hat ein Modell, eine ausgefüllte
   Prüfliste und ein Vergleichsbild, das ich mit `just ref-compare <id>` neu erzeugen
   kann.
3. `just check`, `just catalog-check`, `just validate-assets`, `just gallery-check`
   und `just selftest` grün.
4. Der Golden-Hash bleibt gleich. Ändert er sich durch die neuen Katalog-Einträge,
   erst die Ursache finden und mir melden.
5. Im Repo liegt keine Zeichnung und kein Foto aus den Vorlagen (`git status` und
   `git lfs ls-files` prüfen).
6. `just bench-view` hält das Budget mit den neuen Modellen in der Galerie.

## 7. Offene Fragen (Vorab-Session klärt sie mit mir)

| Frage | Vorschlag |
|---|---|
| Alle 141 aufnehmen oder auswählen? | Alle Serienfahrzeuge aufnehmen. Prototypen und Einzelstücke (Maus, Neubaufahrzeug, SMK, TG, T-24, T-43) als `skipped` vorschlagen, wenn weniger als etwa 20 gebaut wurden; ich entscheide je Zeile. |
| Ein Modell je Ausführung (Pz IV Ausf. A, C, D …)? | Ja, „ein Modell pro Einheit“ bleibt. Ausführungen, die sich im Modell-Budget nicht unterscheiden lassen, zu einem Eintrag zusammenfassen und das in der Liste begründen. |
| `tier` der neuen Einträge? | `tier = 3` („aus den Vorlagen, noch nicht baubar“) als neuer Wert in `SCHEMA.md` und `catalog.rs`. |
| Werden die neuen Fahrzeuge baubar? | Nicht hier. Das braucht erst eine Entscheidung zu Zeitabschnitten oder Stufen im Fahrzeugwerk, sonst hat der Picker 40 Kacheln pro Fraktion. Als Punkt für Prompt 045 notieren. |
| Fahrzeuge ohne Zeichnung (31 Ordner)? | Nach den Maßen, der Zeichnung der nächsten Verwandten in der Familie und frei zugänglichen Fotos bauen, in der Liste als „no drawing“ markieren. Gibt es auch keine Verwandte mit Zeichnung, als `skipped` vorschlagen. |
| Fehlende Vorlagen neu laden? | Nur nach Rückfrage (`just vehicle-refs` lädt von onwar.com). |

## Entscheidungen (autonom, Nachtlauf 17.11.2026)

Vorläufig: die Vorab-Session lief ohne Rückfrage; die Vorschläge aus Abschnitt 7
gelten als Entscheidung. Bitte prüfen und bei Bedarf in `data/vehicles/index.json`
ändern (`just vehicle-db` schreibt die Liste neu).

| Frage | Entscheidung |
|---|---|
| Alle 141 aufnehmen? | Serienfahrzeuge ja. `skipped`: Maus, Neubaufahrzeug, Panther F, SMK, TG, T-24, T-43, T-35 Modell 1938 (6), Pz 38(t) n.A. (15), 10.5 cm leFH auf FCM(f) (12), Sturmtiger (18). |
| Ausführungen zusammenfassen? | Nur StuG III Ausf. D in Ausf. B (außen gleich, nur Bordsprechanlage); alle anderen bleiben je ein Eintrag. Die Gruppen-Sessions dürfen weitere vorschlagen. |
| `tier` der neuen Einträge? | `tier = 3` in `SCHEMA.md` und `catalog.rs`; die Einheitenübersicht im Spiel zeigt `tier = 3` erst mit Modell (sonst kippt der Selbsttest „3/4 mit Modell“). |
| Baubar? | Nein (Punkt für Prompt 045). |
| Ohne Zeichnung und ohne Verwandte mit Zeichnung? | `skipped`: nur T-28 (Mehrturm-Baukasten kommt trotzdem, für T-35 und M3 Lee). |
| Fehlende Vorlagen neu laden? | Nein. |
| Zuordnung | Ordner „Brummbar“ = vorhandener `de_sturmpanzer4` (`existing`, Abweichungen zu onwar in `note`); Ordner „OT-133“ enthält Daten des OT-130 → Id `su_ot130`. |
| Zweitquelle | Spielrelevante Werte der neuen Einträge nur aus onwar.com (Rang 4); Zweitquellen-Prüfung erst, wenn die Fahrzeuge baubar werden. Waffenwerte aus gleichen Katalog-Waffen oder den Geschütz-Artikeln, `weapon` immer `uncertain`. |
| Jahre | `year_from`/`year_to` = Produktionszeitraum der Quelle. |
| Prüfgrenzen | Höhe ab 1,4 m (T-27A 1,45 m), Besatzung bis 12 (T-35: 11). |
| Modell-Sessions | Nacheinander auf `main` statt parallel in Worktrees (Vorgabe des Nachtlaufs). |

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (Katalog, `justfile`, Baukasten, Doku) nur klein und additiv ändern,
weil neun Sessions parallel laufen; Baukasten-Änderungen nach der Vorab-Session nur,
wenn sie rückwärtsverträglich sind. Am Ende (zusammenführende Session) nach
`prompts/done/` verschieben und `prompts/README.md` aktualisieren. Abschlussbericht:
was gebaut wurde, wie ich es teste, was an vorhandenen Modellen korrigiert wurde, was
offen ist.
