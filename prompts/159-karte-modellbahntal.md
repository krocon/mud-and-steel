# Prompt 159 – Karte nach Bildvorlage: Modellbahntal (kleines Alpenstädtchen mit Burg, Wasserfällen und Ringbahn)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt 148
> (Tunnel: `[[tunnel]]`, Bahnvariante, Portale) ist erledigt
> (`prompts/done/148-*`). **Empfohlen nach 139** (Fachwerkhäuser, Burgteile),
> **141** (intakte Bergfestung, Bogenviadukt), **149** (kleine Passkarte:
> Wasserfälle, Fußwege, Kamera im Gebirge) und **122** (farbiger Look je Karte);
> was davon erledigt ist, wird verwendet, was fehlt, ersetzt ein Stellvertreter.
> Lies zuerst `CLAUDE.md`, dann `prompts/done/148-*` und
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in `docs/MAPS.md` „Maps from
> picture sketches (prompt 088)“ ganz (besonders `[[river]]`, `[[lake]]`,
> `[[island]]`, `[[crossing]]` mit `stone`, `[[shoal]]`, `[[rail]]` (Gleisbett,
> für alle offen; Bahnbrücke ist eine Steinbrücke auf der Linie), `[[tunnel]]`,
> `[[hill]]`, `[[village]]`, `[[farm]]`, `[[field]]`, `[[wood]]`) und „Bridges“;
> die Karten `river_town` und `valley_fields` als Muster; in
> `docs/ARCHITECTURE.md` Sicht, Wegfindung an Hängen, Tunnel;
> `data/scale.toml` (`[range]`), `data/terrain.toml`,
> `crates/cli/src/gen_sketch.rs`, `docs/AI.md`, `docs/PERFORMANCE.md`.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/modellbahntal/modellbahntal.webp`. Genau ansehen. Ein mit KI
> erzeugtes Bild; der Nutzer dazu: **„Eine kleine romantische
> Modellbahnlandschaft mit Alpenstädtchen, Burg, Wasserfällen, Steinbrücken,
> Eisenbahntunneln und einem türkisblauen Fluss.“** Die Vorlage hat keine Lager
> und keine Ziele; sie werden hier gesetzt.
>
> **Dieser Prompt läuft in zwei Stufen.** Nach Stufe 1 Bilder aus dem Spiel neben
> der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine kleine, dichte Karte `railway_valley` (im Spiel „Modellbahntal“ / „Model
railway valley“), **1,2 km × 0,8 km**, für **1 gegen 1 und 2 gegen 2** (Team
West gegen Team Ost): ein Tal wie auf einer Modellbahnanlage, in dem alles nah
beieinander liegt. Ein Fachwerkstädtchen am Westufer, eine Burg auf einem
Felsen am Ostufer, dazwischen ein türkisblauer Fluss mit Wasserfällen, einer
Mühle und einem kleinen See, zwei Steinbrücken, und außen herum eine **Ringbahn
mit Viadukt, Bahnbrücke und zwei Tunneln**. Bewusst **unsymmetrisch**: Dorf
gegen Burg.

Die Vorlage zeigt:

- einen **türkisblauen Fluss** von oben links durch die Mitte in einen **See**
  unten, mit **Wasserfällen** (am Oberlauf, an der Mühle, am Seeauslauf) und
  einer kleinen Insel;
- ein **Städtchen** links der Mitte: Fachwerkhäuser mit roten Dächern um eine
  **Kirche** mit spitzem Turm, ein Platz mit Brunnen, Wirtshaustische, ein
  Bootssteg;
- eine **Burg** mit Türmen und Ringmauer rechts oben auf einem Felsen;
- eine **Wassermühle** mit Rad am Wasserfall, ein **Bauernhof** mit Viehweide
  und Zaun, ein **Weinberg**, Getreidefelder mit Garben;
- zwei **Steinbogenbrücken** für die Straße, ein **Bahnviadukt** mit hohen
  Bögen, eine **Bahnbrücke** am Seeauslauf, **Tunnelportale** in den Felsen
  links oben, links unten und rechts;
- Felsen und Nadelwald an den Rändern, blühende Bäume im Ort, geschwungene
  Feldwege.

Die Vorlage ist nur Vorbild für Anordnung und Wirkung.

## 2. Größe, Maßstab, Look

- **1,2 km × 0,8 km** (Seitenverhältnis der Vorlage), Biom `forest`, Wetter
  klar. Die Vorlage wirkt wie ein Spielzeug; die Karte ist es nicht: **Größen
  bleiben 1:1** (Häuser, Brücken, Fluss in echter Größe), nur die Landschaft ist
  eng gepackt. Was dann nicht mehr auf die Fläche passt, wird weniger (weniger
  Häuser, kleinerer See), nicht kleiner.
- **Look:** satte Farben, türkisfarbenes Wasser, helle Felsen, einzelne
  blühende Bäume (rosa und weiß) im Ort. Mit dem Look je Karte aus 122, falls
  erledigt und vom Nutzer angenommen; blühende Bäume als Farbvariante eines
  vorhandenen Baums, falls das Material es einfach hergibt, sonst offener Punkt.
- **Relief** (Höhen über dem See): Talboden 5–15 m, Burgfelsen 45 m, Felsränder
  im Westen, Süden und Osten 30–60 m (unpassierbar, wo Fels), Oberlauf des
  Flusses im Norden 25 m (er fällt in Stufen).

## 3. Aufbau

Richtwerte in Metern (x nach Osten, y nach Süden, Mitte 0/0), aus der Vorlage
abgegriffen. Die Session übernimmt die Werte und glättet sie nach den
Stufenbildern.

- **Fluss** (`[[river]]`, 20–30 m breit, schnell, für Fahrzeuge nur an Brücken
  und einer Furt, für Infanterie zusätzlich an zwei flachen Stellen): Nordrand
  (−350, −400) → **oberer Wasserfall** (−335, −330) → unter dem Viadukt
  (−330, −255) → unter der Nordbrücke (−250, −175) → (−120, −95) → (40, −50) →
  unter der Ostbrücke (120, 70) → in den **See**. **Mühlbach** von Nordosten
  (300, −140) über den **Mühlenfall** bei (190, −60) in den Fluss.
- **See** (`[[lake]]`) um (0, 210), etwa 400 m × 200 m, tief (für alle
  gesperrt), mit einer **Insel** bei (90, 190) (Bäume, Felsen; nur Kulisse) und
  einem **Bootssteg** am Dorfufer bei (−80, 110). **Auslauf** über den
  **Seefall** bei (160, 280) nach Südosten, unter der Bahnbrücke (280, 345) zum
  Südrand (300, 400).
- **Städtchen** am Westufer um den **Platz mit Brunnen** bei (−170, 40):
  **Kirche** bei (−210, −25), zwölf bis fünfzehn Fachwerkhäuser (139, sonst
  vorhandene Stadthäuser) in zwei Zeilen und um den Platz, ein Gasthof, eine
  Schmiede, Gärten mit Zäunen und Mauern; Gassen von 5–8 m, die Hauptgasse
  befahrbar. Ein **Bahnhäuschen** mit Bahnsteig an der Ringbahn bei (−470, −60).
- **Burg** am Ostufer auf einem **Felsen** um (295, −310): kleine Burg mit
  Ringmauer, Bergfried, Torturm und Wohnbau (Teile aus 141 bzw. 139, sonst
  ummauerte Höhe über `[[ring]]` mit Steinhaus und Kirchturm als Stellvertreter),
  Plateau etwa 80 m × 60 m, **Felswände nach Westen und Süden**. Zufahrten: ein
  **Fahrweg in zwei Kehren** von Osten durch das Tor (für alle Fahrzeuge, die
  durch das Tor passen; sonst bis vor das Tor) und ein **Steig für Infanterie**
  von der Mühle die Westwand hinauf.
- **Mühle** mit Wasserrad am Mühlenfall bei (145, −110): ein einfaches Modell
  `env_watermill` aus dem Baukasten (Steinhaus mit Rad, das sich dreht, nur
  Darstellung; zerstörbar wie ein Steinhaus, mit Vorschaubild und
  `just units-web-sync`), oder ein vorhandenes Steinhaus und das Modell als
  offener Punkt.
- **Ostufer sonst:** **Nordweiler** (drei Häuser) mit Getreidefeldern um
  (−60, −250); **Bauernhof** mit Stall, **Viehweide** im Zaunring und Brunnen um
  (440, 20); **Weinberg** in Zeilen am Hang um (480, −120) (als Feldart mit
  Rebzeilen, niedrige Deckung für Infanterie, für Fahrzeuge befahrbar; aus
  vorhandenen Bausteinen, sonst Hecken in Reihen).
- **Westufer sonst:** Getreidefelder mit Garben und zwei Höfe im Südwesten um
  (−400, 250); eine Wiese im Westen um (−500, −150); ein Bootshaus am See bei
  (−130, 300).
- **Zwei Straßenbrücken** (`[[crossing]]` `stone`): **Nordbrücke** bei
  (−250, −175) und **Ostbrücke** bei (120, 70) am Seeeinlauf; der Fluss läuft an
  jeder Brücke so, dass sie achsparallel liegt (Nordbrücke West-Ost, Ostbrücke
  Nord-Süd oder West-Ost, je nach Flussführung). Dazu eine **Furt**
  (`[[shoal]]`) für Fahrzeuge oberhalb der Mühle bei (−30, −75) und zwei flache
  Stellen nur für Infanterie.
- **Ringbahn** (`[[rail]]`, Gleisbett für alle offen) als geschlossene Schleife
  außen um das Tal, wie auf einer Modellbahn:
  - **Viadukt** über den Oberlauf im Nordwesten von (−420, −290) nach
    (−240, −300): hohe Steinbögen (141, sonst Steinbrücke auf einem Damm);
  - nach Osten am Nordrand entlang, hinter der Burg vorbei zur Nordostecke;
  - **Osttunnel** (`[[tunnel]]`, Bahn) unter dem Felsrand im Osten: Portal Nord
    (545, −200) ↔ Portal Süd (520, 175), etwa 380 m;
  - am Südostrand entlang über die **Bahnbrücke** am Seeauslauf (280, 345);
  - am Südrand nach Westen, dann der **Westtunnel** unter dem Felsrand im
    Südwesten: Portal Süd (−330, 335) ↔ Portal West (−480, 95), etwa 290 m;
  - am Westrand nach Norden am Bahnhäuschen vorbei zurück zum Viadukt.
  Die Ringbahn ist der **äußere Weg um das Tal**: Sie umgeht Dorf und Brücken,
  führt über zwei eigene Flussübergänge und durch zwei Tunnel. „Eine Ebene je
  Ort“ aus 148 beachten: Die Tunnel laufen unter gesperrtem Fels.
- **Wege:** eine gepflasterte Straße vom Westrand durch das Städtchen über die
  Ostbrücke zum Bauernhof und weiter zum Ostrand; eine zweite vom Städtchen über
  die Nordbrücke zum Nordweiler und zur Burg; Feldwege zu Mühle, Weinberg,
  Feldern, Bootshaus. Alles geschwungen.
- **Felsen und Wald:** Felsränder mit Nadelwald im Westen, Süden und Osten,
  Felsblöcke am Fluss, Baumgruppen im Tal.

## 4. Spiel: Dorf gegen Burg

- **Starts** (so viel ebene freie Fläche wie bei den vorhandenen kleinen
  Karten): Team West auf der Wiese (−500, −150) und auf dem Feld (−420, 260);
  Team Ost auf der Viehweide (450, 30) und am Weinberg (470, −200). Team 0 West
  (Spieler 0, 2), Team 1 Ost (1, 3). 1 gegen 1 nutzt Wiese gegen Viehweide.
- **Versorgungspunkte** (sieben): **Dorfplatz** (−170, 40) und **Burg**
  (295, −310); **Mühle** (145, −110), **Ostbrücke** (120, 70) und
  **Nordbrücke** (−250, −175), umkämpft; **Bahnhäuschen** (−470, −60);
  **Bahnbrücke am Seeauslauf** (280, 345).
- **Was jede Seite hat:** West das Städtchen (viele Häuser, Deckung bis ans
  Wasser, der See schützt die Südflanke) und den Bahnhof; Ost die Burg (Sicht
  über das ganze Tal, kleine starke Stellung), Mühle und Hof näher bei sich. Die
  Burg sieht ins Dorf, das Dorf nicht in die Burg; dafür ist die Burg mit einem
  Geschütz vom Dorfrand zu beschießen und hat nur einen Fahrweg.
- **Drei Wege hinüber:** Nordbrücke, Ostbrücke (und Furt), und außen die
  Ringbahn über Viadukt bzw. Bahnbrücke und durch die Tunnel. Mit beiden
  Straßenbrücken gesprengt bleiben Furt und Ringbahn.
- **Ausgleich prüfen:** Fahr- und Gehzeiten von jedem Start zu jedem Ziel als
  Tabelle in `docs/MAPS.md`; die Summen je Team höchstens 15 % auseinander,
  sonst Starts oder Ziele verschieben. **KI-Turnier mit vertauschten Seiten**,
  Ziel höchstens 60 % für eine Seite; Stellschrauben: Lage der Ziele, zweiter
  Fahrweg zur Burg, Deckung am Ostufer.
- **Reichweiten:** Auf dieser Größe reicht jedes Geschütz fast überall hin; die
  Reichweitentabelle (indirekt 1/8) trotzdem in `docs/MAPS.md`, und die
  Sichtweiten vom Burgturmplatz, vom Kirchplatz und vom Viadukt nennen: Hier
  entscheidet die Sicht.

## 5. Stufen

**Stufe 1 – Skizze:** `data/map_sketches/railway_valley.toml` nach Abschnitt 2
bis 4, erzeugt mit `just gen-sketch railway_valley`. Bestehende Skizzenkarten
müssen Bit für Bit gleich bleiben. Bilder aus dem Spiel neben der Vorlage, aus
ähnlichem Blickwinkel.

**Stufe 2 – Prüfen und Abschluss:**

- `just map-check railway_valley`, `just map-reach railway_valley` (mit offenen
  und verschütteten Tunneln): alle Starts und Ziele für Fahrzeuge erreichbar;
  Felsränder für alle zu.
- **Fahrprobe, als Test:** ein Panzer einmal um die ganze Ringbahn (Viadukt,
  Osttunnel, Bahnbrücke, Westtunnel), sofern er durch das Tunnelprofil passt;
  der schwerste Panzer über beide Straßenbrücken, durch die Furt und den
  Fahrweg zur Burg; Infanterie über den Steig. Zeiten in `docs/MAPS.md`.
- **Brückenprobe:** jede Brücke einzeln gesprengt, jeder Tunnel einzeln
  verschüttet: Jedes Lager bleibt mit jedem Ziel verbunden.
- **Sichtprüfung:** Sichtfeld-Bilder von der Burg, vom Kirchplatz, von der
  Mühle, vom Viadukt, vor jedem Tunnelportal.
- **Kamera und Darstellung:** enge Landschaft mit Felsen, Burg und Viadukt;
  Einheiten unter dem Viadukt, hinter dem Burgfelsen und im Tunnel bleiben
  sichtbar bzw. markiert; Bilder in jeder Zoomstufe.
- **Bildrate und Wegsuche** in jeder Ansicht (`docs/PERFORMANCE.md`).
- **KI-Turnier** 2 gegen 2 und 1 gegen 1, auch mit vertauschten Seiten,
  Ergebnis in `docs/AI.md`: Nimmt die KI die Ringbahn und die Tunnel, kämpft sie
  um Mühle und Brücken, wie lange dauert eine Partie? Probleme als offene
  Punkte; die KI nicht umbauen.
- Menüname und Übersetzung, Eintrag in der Tabelle der Skizzenkarten in
  `docs/MAPS.md` mit Fahrzeit- und Reichweitentabelle, Test wie bei den anderen
  Skizzenkarten (zweimal erzeugt, gleiche Dateien). `just check`,
  `just selftest`. Der Golden Hash ändert sich nur, wenn die Mühle einen neuen
  Eintrag in den Regeltabellen braucht; dann Ursache nennen, Hash neu setzen und
  im Commit sagen. Sonst bleibt er.

## 6. Nicht Teil dieses Prompts

- Fahrende Züge, ein Zug auf dem Viadukt (stehende Wagen aus Prompt 155 als
  Kulisse am Bahnhäuschen sind erlaubt, wenn 155 erledigt ist).
- Boote auf dem See (Prompt 119), Vieh auf der Weide, Figuren am Wirtshaus.
- Eine symmetrische Fassung; Winter- oder Herbstvariante (über `base` in der
  Skizze leicht nachzuholen).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
