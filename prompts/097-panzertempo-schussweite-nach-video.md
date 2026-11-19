# Prompt 097 – Panzertempo und Schussweite nach Referenzvideo prüfen und anpassen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/SCALE.md` ganz (Zeitfaktor, Reichweitenstauchung, „Typical combat distances“,
> Tabellen „Travel times“ und „Combat ranges“, „Changing the scale“), `data/scale.toml`,
> `data/terrain.toml` (Geschwindigkeit je Untergrund), `data/vision.toml`,
> `data/combat.toml`, `docs/BALANCE.md`, `docs/SOURCES.md` und in `crates/sim` die
> Stellen, die `Scale` benutzen (`direct_fire_range_m`, `sight_range_m`, Zeitfaktor,
> `[[time.override]]`). Prüfe, ob Prompt **093** (Panzer überrollen Infanterie, hängt
> vom Fahrtempo ab) schon eingecheckt ist; wenn im Arbeitsverzeichnis noch fremde,
> nicht eingecheckte Änderungen an `crates/sim` liegen, erst den Nutzer fragen.

## 1. Ausgangslage

Der Nutzer hat ein Referenzvideo genannt:
<https://youtube.com/shorts/rCVzZZq9Q5M> (YouTube-Short, Titel „Sudden Strike 5
Review“, Spielszenen eines WW2-Echtzeitstrategiespiels in Schrägansicht). Sein
Eindruck: **Bei uns fahren die Panzer zu schnell und schießen zu weit.** Im Video
wirken Fahrzeuge schwer und langsam, und Gefechte finden auf kurze Distanz statt,
alle Beteiligten sind gleichzeitig im Bild. So soll es sich bei uns auch anfühlen.

Heutige Werte (aus `docs/SCALE.md`, vor der Arbeit nachrechnen):

- Zeitfaktor 2: Panzer IV 21,1 m/s Straße / 8,9 m/s Gelände, T-34 30 / 20 m/s,
  M8 Greyhound 49 / 28 m/s. Bei 5,9 m Wannenlänge fährt der Panzer IV im Gelände
  1,5 und auf der Straße 3,6 eigene Längen pro Sekunde; ein Bild mittlerer
  Zoomstufe (~190 m breit) durchquert er auf der Straße in 9 s.
- Direktes Feuer × 1/4 der typischen Kampfentfernung: 75-mm-Kanonen 175 m
  (~30 Panzerlängen, ein ganzes Bild), 88 mm 250 m, Gewehre 75 m, MG 100–200 m.
  Sicht × 1/4 (500 m).

**Wichtig (docs/SOURCES.md):** Aus anderen Spielen werden keine Daten übernommen.
Das Video ist nur Vorbild für das **Spielgefühl** (Tempo und Entfernungen im
Verhältnis zur Fahrzeuggröße und zum Bild). Der Katalog mit echten Werten bleibt
unangetastet; geändert werden nur die Umrechnungsfaktoren in `data/scale.toml` (und
bei Bedarf Regeldateien). Nichts aus dem Video nachbauen, keine Namen, Werte oder
Gestaltung übernehmen.

## 2. Ziel

1. Das Video ausmessen (Verhältniszahlen, keine absoluten Spielwerte). Schon
   erledigt, Ergebnis in Abschnitt 3.
2. Dieselben Verhältniszahlen in unserem Spiel messen.
3. Daraus neue Faktoren für Fahrzeugtempo und Schussweite ableiten, sodass unser
   Spiel in diesen Verhältnissen nahe am Video liegt, und sie umsetzen.
4. Alles, was an Tempo und Reichweite hängt, mitziehen und die Balance neu prüfen.

## 3. Analyse des Videos (schon gemessen, 2026-11-19)

Das Video liegt lokal (außerhalb des Repos, nur lesen, nichts davon ins Repo kopieren):
`/Users/marckronberg/Filme.nosync/Sudden Strike 5 Review/Sudden Strike 5 Review (1920p_30fps_H264-128kbit_AAC).mp4`
(1080 × 1920 hochkant, 30 Bilder/s, 23,3 s, 696 Bilder). Die Messung unten ist
erledigt; nur wiederholen, wenn eine Zahl für die Entscheidung genauer sein muss.

**Wie gemessen wurde:** alle Bilder mit `ffmpeg` ausgeschrieben, auf 540 × 960
verkleinert. Die Kamera bewegt sich in jeder Szene, deshalb wurde die Bewegung des
Bodens neben dem Fahrzeug (Bildausschnitte per Kreuzkorrelation alle 6–10 Bilder
verfolgt) von der Bewegung des Fahrzeugs im Bild abgezogen. Tempo steht in
**eigenen Fahrzeuglängen pro Sekunde (L/s)**: Fahrstrecke und Fahrzeuglänge liegen
in derselben Bildrichtung und Tiefe, die Zahl braucht also keine Annahme über
Kamerawinkel. Meterwerte rechnen mit ~5,9 m Wannenlänge (Panzer) bzw. ~6 m (Lkw).

### 3.1 Die Szenen

Neun Szenen von je 2–4,5 s, dazu ein Untertitel mitten im Bild und überall
Kamerafahrten. Es sind Schaubilder mit naher Kamera, kein normales Spielbild mit HUD.

| Zeit (s) | Bilder | Szene | Brauchbar für |
|---|---|---|---|
| 0,0–2,0 | 0–60 | Trümmerstadt, Panzerkolonne, Flugzeugschatten; Kamera kreist | Tempo (unsicher) |
| 2,0–4,3 | 60–128 | Winterdorf, Einschlag zwischen Fahrzeugen | nichts Messbares |
| 4,3–9,0 | 129–270 | Weinberg, Einschläge, geparkte Lkw | nichts Messbares |
| 9,0–11,2 | 270–336 | Friedhof, Panzer stehen unter Bäumen | steht |
| 11,2–13,8 | 337–412 | Wüste, zwei Fahrzeuge nah; Kamera kreist | steht (Fahrzeuge und Boden ruhen gemeinsam) |
| 13,8–16,4 | 413–490 | Schlammfeld, vier helle Panzer in Fahrt, ein dunkler Panzer unten, Einschlag am Fels | Tempo, Entfernungen |
| 16,4–18,5 | 491–555 | Wüste, Lkw/Halbketten und Infanterie rücken vor | Tempo |
| 18,5–21,0 | 556–630 | Zug, Panzer wird getroffen | nichts Messbares |
| 21,0–23,2 | 630–695 | Winter, Artillerieeinschläge | nichts Messbares |

### 3.2 Fahrtempo

| Szene | Fahrzeug | Im Bild (px/s bei 540 Breite): Fahrzeug / Boden daneben | Relativ zum Boden | Länge im Bild | Tempo | ≈ echt | Güte |
|---|---|---|---|---|---|---|---|
| Schlammfeld, Bilder 420–480 | heller Panzer (Mitte rechts), Gelände, feuert in der Fahrt | (−9, −1) / (−56, +26) | 55 px/s | 85 px | **0,65 L/s** | 3,8 m/s, 14 km/h | gut |
| Schlammfeld | dunkler Panzer unten | (−105, +40) / (−98, +40) | ~0 | 110 px | steht | – | gut |
| Wüste, Bilder 495–555 | Lkw unten links, Gelände | (−8, +31) / (+62, +84) | 88 px/s | 80 px | **1,1 L/s** | 6,6 m/s, 24 km/h | gut |
| Wüste | Lkw rechts | (+7, +21) / (+60, +78) | 78 px/s | 80 px | **1,0 L/s** | 6 m/s, 22 km/h | gut |
| Trümmerstadt, Bilder 2–52 | zwei Panzer in Kolonne, Schutt | (+8…+19, −13…−16) / (~+40, ~−5) | ~30 px/s | 75 px | **~0,4 L/s** | ~2,4 m/s, ~9 km/h | unsicher (±50 %, Kamera kreist) |
| Trümmerstadt | kleine Panzer im Mittelgrund | – | ~24 px/s | ~40 px | ~0,6 L/s | ~3,5 m/s | unsicher |

- **Panzer im Gelände: 0,4–0,65 L/s** (9–14 km/h). Das liegt bei oder unter echter
  Geländegeschwindigkeit, also etwa Zeitfaktor 1 oder weniger.
- **Lkw/Halbketten im Gelände: 1,0–1,1 L/s** (22–24 km/h), auch etwa echtes Tempo.
- **Infanterie:** läuft in der Wüstenszene im Pulk um die Lkw mit, die Lkw ziehen
  langsam davon. Einzelne Soldaten ließen sich nicht sauber verfolgen; kein Zahlenwert.
- **Straßentempo, Wendekreis, Turmdrehung, Anfahren/Bremsen:** im Video nicht zu
  sehen bzw. nicht messbar.

Zum Vergleich wir heute (aus `docs/SCALE.md`, Zeitfaktor 2, Wanne ~5,9–6,1 m):

| Fahrzeug | Gelände | Straße |
|---|---|---|
| Panzer IV H | 8,9 m/s = **1,5 L/s** | 21,1 m/s = **3,6 L/s** |
| M4A3 Sherman | 13,9 m/s = **2,4 L/s** | 23,3 m/s = **3,9 L/s** |
| T-34/76 | 20,0 m/s = **3,3 L/s** | 30,0 m/s = **4,9 L/s** |
| Lkw (ZiS-5, 6 m) | 13,9 m/s = **2,3 L/s** | 33,3 m/s = **5,5 L/s** |

Unsere Panzer fahren im Gelände also **2,3- bis 5-mal** so viele eigene Längen pro
Sekunde wie im Video (auf der Straße noch mehr), die Lkw gut **2-mal** so viele.
Der Eindruck des Nutzers stimmt.

### 3.3 Bildausschnitt

- Schlammfeld: Bildbreite auf Höhe der Panzer ≈ 6,4 Panzerlängen ≈ **38–41 m**; die
  ganze sichtbare Tiefe von oben bis unten ≈ **110 m** (Maßstab aus den
  Panzerlängen: ~12 px/m oben bei y = 440, ~18 px/m unten bei y = 870; Kameraneigung
  auf 45–55° geschätzt, daher ±25 %).
- Wüste: Bildbreite auf Höhe der Lkw ≈ 6,8 Lkw-Längen ≈ **41 m**.
- Das Video ist hochkant; ob es aus einem Querformatbild ausgeschnitten ist, lässt
  sich nicht sagen. Sicher ist: Die Kamera ist **viel näher** als unsere mittlere
  Zoomstufe (~190 m Bildbreite). Folge: In Bildbreiten pro Sekunde ist das Video
  nicht langsamer als wir (Panzer dort ~10 s für die Bildbreite, unser Panzer IV im
  Gelände ~21 s für 190 m). Das „schwere, langsame“ Gefühl kommt aus den wenigen
  Fahrzeuglängen pro Sekunde, nicht aus der Zeit pro Bild. Beim Vergleich im
  eigenen Spiel deshalb auch eine nahe Zoomstufe (~40–60 m Bildbreite) ansehen.

### 3.4 Schussentfernung

Hier ist das Video dünn: In keiner Szene sind Schütze und Ziel eindeutig zuzuordnen,
und wegen der nahen Kamera sieht man nie eine Höchstreichweite. Belastbar ist nur:

- Schlammfeld, Bild 450: der dunkle Panzer (unten) und die hellen Panzer (Gegner,
  einer feuert im selben Moment) sind **~35 m ≈ 6 Panzerlängen** voneinander entfernt.
- Schlammfeld, Bild 480: der Einschlag am Fels liegt **~60–65 m ≈ 11 Panzerlängen**
  vom dunklen Panzer (dem einzigen plausiblen Schützen im Bild); das Rohr des hellen
  Panzers zeigt nicht dorthin.
- Alles, was man kämpfen sieht, spielt sich innerhalb von ~110 m Bildtiefe ab, meist
  innerhalb von 30–70 m.
- Infanterie gegen Infanterie, MG, Pak: im Video nicht zu sehen.

Bei uns reicht die 75-mm-Klasse 175 m ≈ **30 Panzerlängen**, die 88 mm 250 m ≈ 42.
Das Video zeigt Gefechte auf 6–11 Längen. Das ist eine **Untergrenze** für die
dortige Reichweite, keine Messung der Höchstreichweite; es stützt die Richtung
(„kürzer“), aber keinen genauen Faktor.

### 3.5 Was daraus folgt (Startwerte, im Spiel zu prüfen)

- **Fahrzeugtempo etwa halbieren** (Zeitfaktor für Bodenfahrzeuge 1/1 statt 2/1):
  Panzer IV 0,75 L/s, Sherman 1,2, T-34 1,7 im Gelände; Lkw 1,15 L/s, das trifft
  die Lkw im Video fast genau. Panzer lägen noch etwas über dem Video (0,4–0,65);
  wenn das im Spiel noch zu flott wirkt, zuerst den Geländeanteil in
  `data/terrain.toml` prüfen statt unter Faktor 1 zu gehen (langsamer als echt
  wäre schwer zu begründen und macht 4-km-Karten zäh).
- **Direktes Feuer:** Kandidaten 1/6 (75 mm: 117 m ≈ 20 Längen, Gewehr 50 m) und
  1/8 (87 m ≈ 15 Längen, Gewehr 37 m). 1/8 liegt näher am Video, macht aber kurze
  Waffen sehr kurz (Maschinenpistole 12 m, Panzerfaust 7 m). Beide ausprobieren.

## 4. Vergleichsmessung im eigenen Spiel

- Dieselben Größen bei uns messen, nicht nur aus der Tabelle rechnen: Szene mit
  Panzer IV, T-34, Sherman, einem Radfahrzeug und einem Schützentrupp auf Straße und
  Gelände, dazu ein Gefecht Panzer gegen Panzer und Panzer gegen Infanterie
  (`just run --skip-menu` bzw. die Aufnahmewerkzeuge aus `just trailer-shots`, eine
  kleine Hilfsszene oder ein Headless-Lauf mit Positionsprotokoll).
- Ergebnis: eine Vergleichstabelle Video / wir heute / Faktor dazwischen.

## 5. Anpassung

Hebel in dieser Reihenfolge prüfen, den einfachsten nehmen, der passt:

- **Fahrtempo:** `[[time.override]]` je Kategorie in `data/scale.toml` (dafür ist es
  gedacht). Vorher im Code nachsehen, was ein Override alles umfasst: wenn er auch
  Nachladen, Aufbau und andere Dauern der Kategorie ändert, einen eigenen
  Geschwindigkeitsfaktor für Bodenfahrzeuge einführen (z. B. `[speed] vehicle =
  { num, den }`), statt Feuerraten ungewollt mitzuändern. Ketten- und Radfahrzeuge
  ggf. getrennt (Radfahrzeuge auf der Straße sind heute mit bis zu 49 m/s besonders
  auffällig). Infanterietempo nur ändern, wenn die Messung es verlangt; das
  Verhältnis Panzer zu Infanterie soll danach zum Video passen und der Panzer nicht
  langsamer als ein laufender Soldat wirken.
- **Schussweite:** Faktor `[range] direct_fire` (heute 1/4) verkleinern. **Sicht**
  (`sight`) so mitziehen, dass weiter jede Einheit sieht, worauf sie schießen kann,
  und Aufklärer ihren Vorteil behalten. Indirektes Feuer nur anfassen, wenn es nach
  der Änderung unverhältnismäßig wirkt; dann begründen.
- Die Reihenfolge der Waffenklassen untereinander (Gewehr < MG < 75 mm < 88 mm)
  bleibt, weil nur der gemeinsame Faktor geändert wird. Für sehr kurze Waffen
  (Panzerfaust 15 m, Bazooka 25 m, Pistole 6 m) prüfen, ob sie nach der Stauchung
  noch spielbar sind; wenn nicht, eine Mindestreichweite als Regel einführen
  (steht in SCALE.md schon als offene Beobachtung) statt den Katalog zu ändern.

Folgen mitziehen und einzeln prüfen:

- Trefferwahrscheinlichkeit und Durchschlag rechnen mit der echten Entfernung
  (Spielentfernung × Kehrwert des Faktors). Sicherstellen, dass dort der Faktor aus
  `scale.toml` kommt und nirgends eine feste 4 steht.
- Flak-Reichweiten und Obergrenzen (`data/air.toml`) hängen am Faktor für direktes
  Feuer; entscheiden, ob sie mitschrumpfen oder einen eigenen Faktor brauchen
  (Flugzeuge werden nicht langsamer).
- Geschossgeschwindigkeit und Flugzeit bei kürzerer Entfernung; Zielauffassung,
  Turmdrehung, Anfahren/Bremsen, Wendegeschwindigkeit (wirkt ein langsamer Panzer
  mit schneller Drehung falsch?).
- Überrollen (093), Ausweichen der Infanterie, Transport, Schleppen, Treibstoff
  (`fuel`-Faktor: reicht ein Tank noch für dieselbe Strecke, das ist beabsichtigt),
  Nachschub, Verstärkungen vom Kartenrand.
- KI (`crates/ai`, docs/AI.md): feste Entfernungen, Zeitpläne und Annahmen über
  Reichweiten und Marschzeiten suchen und anpassen.
- Missionen und Karten: Zeitvorgaben in Missionsskripten (docs/MISSIONS.md),
  Abstände von Startpunkten; Spieldauer auf der größten Karte (4 km) neu rechnen.
  Wird das Vorrücken zu zäh, das im Ergebnis nennen (mögliche Gegenmittel:
  Straßenbonus, schnellere Spielgeschwindigkeit als Einstellung), aber nicht
  eigenmächtig einbauen.
- Darstellung: Kettenlauf-/Radanimation, Staub, Motorgeräusch-Tonhöhe, Reichweiten-
  und Sichtkreise im HUD, Trailer-Aufnahmen (`trailer_shots.gd`), die auf feste
  Zeiten geschnitten sind.

## 6. Regeln

- Katalog (`data/catalog/*.toml`) nicht ändern. Faktoren bleiben Brüche aus
  positiven ganzen Zahlen; in `crates/sim` keine Floats.
- **Golden Hash** in `crates/sim/tests/determinism.rs` ändert sich beabsichtigt
  (Regeländerung): erst nach der Umsetzung aktualisieren und in der Commit-Nachricht
  nennen. Tests mit festen Entfernungen oder Zeiten an die neuen Faktoren anpassen,
  aber keinen Test abschwächen oder abschalten; wo ein Test eine Regel prüft (z. B.
  „75 mm reicht weiter als Gewehr“), bleibt die Aussage erhalten.
- Batch-Sessions teilen sich das Arbeitsverzeichnis: vor Turnieren und Messungen
  nachsehen, ob andere Prompt-Sessions laufen.

## 7. Vorgehen

1. Messwerte aus Abschnitt 3 übernehmen, eigenes Spiel messen (Abschnitt 4,
   in L/s und auch bei naher Zoomstufe), Vergleichstabelle.
2. Neue Faktoren festlegen, ausgehend von den Startwerten in 3.5 (Fahrzeuge 1/1,
   direktes Feuer 1/6 oder 1/8). Runde Brüche wählen und kurz begründen, warum
   dieser Wert und nicht der nächste.
3. Umsetzen, Folgen aus Abschnitt 5 abarbeiten.
4. Balance neu prüfen wie bei Prompt 067: `just ai-tournament` (10 Karten × 3
   Paarungen, Schwer gegen Schwer), Siegquoten der Fraktionen vorher/nachher,
   mittlere Spieldauer. Kippt die Balance deutlich, den Faktor eine Stufe
   zurücknehmen und erneut messen.
5. `just check`, `just selftest`, `just lan-selftest`; `just bench` (kürzere
   Reichweiten und Sicht ändern die Last).
6. Doku: `docs/SCALE.md` (Entscheidung mit Datum, Tabellen „Travel times“ und
   „Combat ranges“ neu rechnen, Hinweis, dass das Video nur Gefühlsvorlage war),
   `docs/BALANCE.md` (Turnierergebnis), `docs/AI.md` falls geändert,
   `just catalog-doc` und `just units-web-sync`, falls die Einheitenseite
   Spielwerte zeigt.
7. Für den Nutzer: Vergleichstabelle, alte und neue Faktoren, je ein kurzer Clip
   oder eine Bildfolge „vorher/nachher“ derselben Szene (Panzer fährt durchs Bild,
   Panzergefecht auf Höchstreichweite), und in einem Satz, wie er die Faktoren in
   `data/scale.toml` selbst nachstellen kann. Er probiert es danach mit `just run`.

## 8. Nicht Teil dieses Prompts

- Änderungen an Katalogwerten, Durchschlag, Panzerung oder Feuerraten.
- Flugzeugtempo.
- Kamera, Zoomstufen oder HUD (nur Reichweitenkreise folgen automatisch).
- Optik oder Inhalte des Videos nachbauen.
