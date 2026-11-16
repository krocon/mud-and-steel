# Prompt 055 – Fahrzeuge und Luftlande Stufe 3: Brücken, Pioniertechnik, Fallschirmjäger

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/UNIT_CATALOG.md`, `docs/ARCHITECTURE.md` (Brücken, Zerstörung, Transport, Luft), `docs/SOURCES.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Flüsse und Brücken sollen das Schlachtfeld prägen: Brücken brechen auseinander und fallen
in den Fluss, Männer können ertrinken, Brückenlegepanzer schlagen neue Übergänge. Dazu
Luftlandungen und Fahrzeugtarnung aus der Epoche.

## 2. Umfang

1. **Brücken zerstören:** Teile stürzen sichtbar ins Wasser (Darstellung), Übergang
   gesperrt; Einheiten auf der Brücke fallen ins Wasser; Infanterie im tiefen Wasser
   ertrinkt nach Zeit, wenn sie nicht ans Ufer kommt (Sim-Regel).
2. **Brückenleger:** Brückenlegepanzer je Fraktion, soweit historisch belegt (z. B.
   Brückenleger IV, Valentine Bridgelayer, Churchill ARK in US-Nutzung prüfen), legt eine
   Brücke über schmale Gewässer und Gräben und nimmt sie wieder auf. Katalog mit Quelle,
   Modell aus dem Baukasten.
3. **Fahrzeugtarnung:** Zweige und Tarnnetze an Panzern als Material-/Anbauvariante
   (verringert Entdeckungsweite im Stand); grüne Tarnung als vierte Tarnvariante prüfen.
4. **Luftlandung:** Fallschirmjäger oder Lastensegler je Fraktion als Abruf (033), Absprung
   mit Streuung; Bombenteppich per Ziehen zielen (Richtung und Länge).
5. **KI** nutzt Brückenleger an Flüssen.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für Brückeneinsturz, Ertrinken, Brücke legen;
   Golden-Hash begründet.
2. `just gallery-check`, `just selftest`, `just lan-selftest` grün.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code (nach 014, 020, 033, 034, 035, 038, 040): Brücken sind Kartenobjekte
mit Fahrbahn (`Map::bridges`, Tragfähigkeit, Wegzellen entlang der Achse, Lastklassen im
Wegfinder). Pioniere sprengen (`DemolishBridge`) und reparieren (`RepairBridge`) sie über
`Sim::set_bridge_intact`; Einheiten auf der Fahrbahn blieben dabei einfach stehen (auf dem
Flussgrund), Godot blendet die Brücke nur aus. Tiefes Wasser ist unpassierbares Gelände;
kein Trupp kann hinein. Luftunterstützung (033) kennt Aufklärung, Jäger, Schlachtflieger
und Bomber mit festem Anflug von der eigenen Startseite und einem Bombenstock um den
Zielpunkt. Sicht (018) ist zellbasiert (Sichtfelder), es gibt keine Tarnung von
Fahrzeugen; 052 bringt Tarnnetze als *platzierte Objekte über Stellungen*. Spielstände
(038) speichern den ganzen `Sim` per serde und Kartenänderungen als `MapDelta`. Der
Katalog hat keine Brückenleger und keine Transportflugzeuge.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Eigene Sim-Schicht **„Crossings“** (`crate::crossings` mit Regeln in neuem `data/crossings.toml`, Zustand in `crate::sim::crossings`) für Einsturz, Schwimmen/Ertrinken, Brückenleger und Fahrzeugtarnung; Fallschirmjäger hängen an der Luftunterstützung (033). Zustand und Regeln gehen erst in den State-Hash, wenn eine der Regeln gegriffen hat. | Wie 034/035: kein neues Config-/Replay-Format, im Spielstand; Partien ohne diese Regeln behalten ihren Hash. |
| 2 | **Brückeneinsturz:** gesprengt wird weiter nur durch Pioniere. Beim Einsturz fallen alle Einheiten, deren Mittelpunkt auf der Fahrbahn steht (±1,5 m Höhe), mit: Fahrzeuge und Geschütze sind verloren (Wrack im Fluss, Insassen nach den Regeln aus 034), Trupps fallen ins Wasser, je Soldat 10 % Sofortverlust. Godot zerlegt die Fahrbahn in Stücke, die sich drehend ins Wasser fallen und versinken, mit Spritzern. **[vorläufig]** | „Teile stürzen sichtbar ins Wasser, Einheiten fallen ins Wasser“; ein Fahrzeug, das 5–10 m tief in einen Fluss stürzt, ist für das Spiel verloren. Beschuss und Bomben zerstören Brücken weiter nicht (offen, wie bisher). |
| 3 | **Schwimmen und Ertrinken (Sim-Regel):** ein Trupp, dessen Mittelpunkt in tiefem Wasser (unpassierbares Wassergelände) steht, schwimmt mit 0,7 m/s zur nächsten Wegzelle, auf der er stehen kann (Suche bis 80 m); Befehle erreichen ihn nicht. Jeder Soldat hat beim Hineinfallen eine Luft von 20–60 s (Zufall aus dem Sim-RNG); ist er danach noch im tiefen Wasser, ertrinkt er. **[vorläufig]** | „Infanterie im tiefen Wasser ertrinkt nach Zeit, wenn sie nicht ans Ufer kommt“; mit Gepäck und Waffe hält ein Soldat nicht lange durch, schmale Flüsse überlebt man meist, breite nicht. Allgemein formuliert gilt sie auch für Fallschirmjäger, die im Wasser landen. |
| 4 | **Brückenleger nur, wo belegt:** neue Katalogkategorie `bridgelayer` mit Feldern `bridge_length_mm`, `bridge_load_kg`. **de:** Brückenleger IV b (20 Stück 1940, Pz IV C/D, 11-m-Brücke, 28 t). **su:** Valentine Bridgelayer aus Lend-Lease (34-ft-Scherenbrücke, Class 30 = 30 480 kg; 25 an die UdSSR). **us: keiner** – die US Army hatte 1943–45 keinen Brückenlegepanzer im Einsatz (Brücken kamen von Pionieren: Treadway, Bailey; Churchill ARK und Valentine-/Churchill-Brückenleger waren britisch). Werte aus dem Gedächtnis der genannten Quellen, nicht online geprüft, als `uncertain` markiert. **[vorläufig, bitte prüfen]** | „Je Fraktion, soweit historisch belegt“; der Brückenleger IV liegt vor dem Hauptzeitraum 1943–45, ist aber der einzige deutsche. |
| 5 | **Brücke legen:** Befehl `LayBridge { units, target }`: der Brückenleger sucht die Lücke nächst dem Klick (bis 30 m): eine gerade Linie entlang einer Wegraster-Achse über Boden, auf dem er nicht stehen kann (tiefes Wasser, steile Gräben), höchstens Brückenlänge − 2 × 1 m Auflager breit, mit erreichbaren Zellen an beiden Enden, Höhenunterschied ≤ 20 %. Er fährt bis 10 m vor das nähere Ufer und legt die Brücke in 60 s; `PickUpBridge` nimmt sie in 90 s wieder auf, wenn niemand darauf steht. Die gelegte Brücke ist eine Brücke der Karte ohne Kartenobjekt (`Bridge::laid_by`), mit der Tragfähigkeit aus dem Katalog (die IS-2 kommt nicht über die Valentine-Brücke); neue Lastklassen beschriften die Komponenten des Wegfinders neu. Ein Brückenleger hat genau einen Platz in der Brückenliste und legt dort immer wieder. **[vorläufig: Zeiten, Auflager]** | Wegraster-Achse, weil der Wegfinder Fahrbahnen nur entlang des 4-m-Rasters kennt (wie die Kartenbrücken). Die Fahrbahn reicht bis auf die Uferzellen (Rampe), die Breitenprüfung gilt nur dem Wasser. Feste Plätze halten die Indizes stabil (`DemolishBridge` bezieht sich auf sie). |
| 6 | Gelegte Brücken stehen im **Spielstand** als Liste `MapDelta::laid` (Index und Lage); Pioniere können sie sprengen (sie stürzen ein wie andere Brücken, der Brückenleger hat seine Brücke dann verloren), aber nicht „reparieren“. | 038 speichert Kartenänderungen als Delta; die Brücke gehört zur Karte, damit Bewegung und Wegfinder nichts Neues lernen müssen. |
| 7 | **Fahrzeugtarnung** als Zustand am Fahrzeug, nicht als Objekt: `SetCamouflage { units, on }` für Fahrzeuge und Geschütze. Anbringen dauert 30 s im Stand; ein getarntes Fahrzeug, das 3 s still steht und nicht schießt, sieht der Gegner nur aus 40 % der Sichtweite seiner Einheit (und in deren Sichtfeld), auch aus der Luft. Fahren lässt die Zweige dran, verbirgt aber nichts. **[vorläufig]** | „Verringert Entdeckungsweite im Stand“; getrennt von 052 (Tarnnetze als platzierte Objekte über Stellungen). |
| 8 | **Darstellung der Tarnung:** Zweige (Busch-Modell aus 007, verkleinert) auf dem Fahrzeug in Wald und Winter, im Winter dazu ein weißes Netz, in der Wüste ein sandfarbenes Netz; beim Anbringen erst wenige Zweige. **Grüne Tarnung als vierte Tarnvariante: nicht eingeführt** – die Palette „forest“ ist schon grün (Olivgrün, 4BO, Olive Drab), die Zweige liefern das Grün. **[vorläufig, bitte prüfen]** | „Zweige und Tarnnetze als Material-/Anbauvariante“ ohne neue Paletten. |
| 9 | **Luftlandung:** neue Luftmission `airborne` (`AirMission::Airborne`, Kategorie `aircraft_transport`): Ju 52/3m (de), Li-2 (su), C-47 (us), Sitzplätze = Katalog-`passengers`. Drei Flugzeuge setzen je einen Trupp ab: den ersten Schützentrupp der Fraktion im Match (keine eigenen Fallschirmjäger-Trupps). Absprung aus 150 m, Sinkrate 5 m/s, Streuung ±40 m je Achse (Dreiecksverteilung); wo der Trupp auf einem Gebäude landen würde, sammelt er sich daneben; im Wasser greift Regel 3. Abgeschossene Flugzeuge verlieren die Truppen, die noch nicht gesprungen sind. Kosten 100/40/120, Abklingzeit 300 s. **Lastensegler: offen.** **[vorläufig]** | „Fallschirmjäger oder Lastensegler“: Fallschirmjäger sind bei allen drei Fraktionen belegt (Kreta, Dnepr 1943, Normandie), mit vorhandenen Infanteriemodellen und ohne neues Landefahrzeug. |
| 10 | **Bombenteppich per Ziehen:** neuer Befehl `CallAirLine { mission, from, to }`: die Flugzeuge fliegen entlang der gezogenen Linie über ihre Mitte; Bomber verteilen ihre Bomben gleichmäßig über die Linie (höchstens `bombs.carpet_max_m` = 300 m), Transporter setzen ihre Trupps entlang der Linie ab, andere Missionen nehmen nur die Richtung. In der Luftleiste: ziehen statt klicken (ab 15 m). | „Richtung und Länge“ mit einem Befehl für alle Missionen. |
| 11 | **Golden-Hash ändert sich** (Luftregeln mit neuer Mission, Trefferpunkten der Transporter, Teppichlänge und Fallschirmregeln; Transporter unter den Flugzeugtypen; fünfte Abklingzeit je Spieler). Die Demo-Schlacht bleibt gleich. Auch der Economy-Golden-Hash (Brückenleger in der Fahrzeugfabrik). | Begründet im Commit. |
| 12 | **KI** (Normal, Hard): liegt auf der Linie zum Gegner tiefes Wasser und steht dort keine Brücke (mehr), baut sie einen Brückenleger ihrer Fraktion (höchstens zwei je Partie) und versucht `LayBridge` an Punkten entlang des Flusses, bis einer passt. Ihre Armee nutzt die Brücke über den Wegfinder. Fallschirmjäger ruft die KI nicht. **[vorläufig]** | „KI nutzt Brückenleger an Flüssen“, nur über Befehle und den `PlayerView` (neu: eigene Brückenleger, welche Brücken stehen). |

## Ergebnis

Umgesetzt wie oben; Details in `docs/ARCHITECTURE.md` „Bridges, bridge-laying tanks and
camouflage“, `docs/CONTROLS.md` (Brückenleger, Tarnung, Luftleiste), `docs/AI.md`
„Bridge-laying tanks“, `docs/ASSETS.md` (Baugruppe `bridgelayer`, Objekt `Bridge`),
`data/catalog/SCHEMA.md` (neue Kategorien und Felder).

- **Sim:** `data/crossings.toml`, `crates/sim/src/crossings.rs`,
  `crates/sim/src/sim/crossings.rs`; Befehle `LayBridge`, `PickUpBridge`,
  `SetCamouflage`, `CallAirLine` am Enum-Ende; `Sim::collapse_bridge` (aus der
  Pioniersprengung), Schwimmen/Ertrinken, gelegte Brücken als Kartenbrücken ohne Objekt
  (`Map::put_laid_bridge`, `PathGrid::sync_bridge` mit neuen Lastklassen),
  `MapDelta::laid` im Spielstand; Fahrzeugtarnung über die Sicht-Maske; Luftmission
  `airborne` mit Fallschirm-Sticks, Bombenteppich per Linie. Katalog: Kategorien
  `bridgelayer`, `aircraft_transport`, Felder `bridge_length_mm`, `bridge_load_kg`;
  `de_brueckenleger4`, `su_valentine_bridgelayer`, `de_ju52_3m`, `su_li2`, `us_c47`;
  Brückenleger in der Fahrzeugfabrik (de, su).
- **KI:** `crates/ai/src/bridgelayer.rs`; `PlayerView::layers`, `bridges_intact`.
- **Godot:** `crossing_funcs.rs`, `world/crossing_view.gd` (einstürzende Fahrbahn,
  Spritzer, ausklappende/ausgeblendete Trägerbrücke, Treadway-Fahrbahn, Zweige/Netze,
  Fallschirme, Schwimmer an der Wasseroberfläche), `selection/crossing_orders.gd`, drei
  Schaltflächen in der Befehlsleiste, Luftleiste mit Fallschirmjägern und Ziehen.
- **Modelle** aus dem Baukasten (headless Blender, neue Baugruppe `bridgelayer`):
  Brückenleger IV b, Valentine-Brückenleger, Ju 52/3m, Li-2, C-47; Vorschaubilder der
  beiden Brückenleger.
- **Tests:** `crates/sim/tests/crossings.rs` (Einsturz mit Lkw und Trupp, Trupp
  schwimmt ans Ufer; Fallschirmjäger im See ertrinken; Landung entlang der gezogenen
  Linie; Brücke legen, leichter Panzer fährt drüber, IS-2 zu schwer, aufnehmen und neu
  legen; Tarnung; Bombenteppich; gleicher Hash), `crates/net/tests/snapshot_crossings.rs`,
  `crates/ai/tests/bridgelayer.rs`, Selbsttest-Teil `tools/crossing_self_test.gd`.
- **Golden-Hash:** `0x2022_32ed_b313_f95a` nach dem Zusammenführen mit main (bis 050) (055 allein
  `0xef27_60b4_5add_ed7b`), Economy `0xd6cd_83ac_0539_44dd`; Grund: Luftregeln (siehe 11).

Offen: vorläufige Werte (`data/crossings.toml`, `[airborne]`), Katalogwerte aus dem
Gedächtnis (nicht online geprüft), keine eigenen Fallschirmjäger-Trupps, keine
Lastensegler, Brücken nur durch Pioniere zerstörbar, Fallschirmjäger in der Luft nicht
beschießbar, Schwimmer schießen noch, gelegte Brücke als Treadway statt ausgeklapptem
Kit-Modell, nur entlang der Rasterachsen.
