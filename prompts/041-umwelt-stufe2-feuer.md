# Prompt 041 – Umwelt-Objekte Stufe 2, Feuer und Brände

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ASSETS.md`, `docs/SOURCES.md`, `docs/MAPS.md`, `docs/ARCHITECTURE.md`
> (Kartenobjekte, Zerstörung, Effekte, Vegetation).
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Die Karten haben Bauernhäuser, Scheunen, eine Kirche, Mauern und Hecken (007). Für
abwechslungsreiche Schlachtfelder fehlen Dörfer und Kleinstädte, Industrie, Eisenbahn
und Befestigungen, und für Wüste und Winter passende Bauten. Gebäude stürzen beim
Zerstören ein, brennen aber nie. Nach diesem Prompt gibt es eine zweite Welle
Umwelt-Objekte aus dem Blender-Baukasten und Feuer als Spielregel.

## 2. Umfang

1. **Neue Objekte** (je mit Intact/Damaged/Ruin, Budgets aus `docs/ASSETS.md`, echte
   Maße mit Quelle): Stadthäuser in Reihe, Marktplatz, Rathaus, Fabrikhalle mit Schlot,
   Lagerhaus, Bahnhof, Gleise, Bahnübergang, Güterwaggons, Wasserturm, Windmühle,
   Tankstelle, Bunker und MG-Nest, Panzergraben, Sandsackstellung; Wüste: Lehmhäuser,
   Oase, Brunnen; Winter: Blockhäuser, verschneite Varianten. Telegrafenmasten, Zäune,
   Wegweiser (ohne echte Ortsnamen mit politischer Bedeutung). Viele Gebäude können
   parallel in mehreren Sessions gebaut werden (wie 008).
2. **Katalog und Regeln:** `data/objects.toml` und `data/destruction.toml` erweitern,
   besetzbare Gebäude für 022 (Fensterplätze), Kartenobjekt-Gruppen für den Editor.
3. **Feuer in der Sim:** Gebäude, Wald und Wracks können brennen (Auslöser: Treffer mit
   Brandwirkung, Explosionen), Feuer schadet über Zeit, breitet sich deterministisch auf
   Nachbarn aus (Wald, Holzgebäude; Stein kaum), erlischt; Munitions- und
   Treibstofflager explodieren (Flächenschaden). Rauch aus 032 blockiert Sicht, falls
   vorhanden. Im Hash.
4. **Darstellung:** Flammen, Glut, schwarzer Rauch, verkohlte Varianten, Licht bei Nacht.
5. **Karten:** mindestens eine Dorf-/Kleinstadt-Karte und eine Karte mit Bahnlinie.

## 3. Akzeptanzkriterien

1. `just kit-test`, `just validate-assets`, `just gallery-check` grün.
2. `just check` grün; Sim-Tests: Feuer breitet sich im Wald aus, nicht über eine
   Straße, Lager explodiert, gleicher Hash; Golden-Hash bewusst aktualisiert.
3. `just selftest`, `just editor-selftest` grün; Benchmark auf der Stadtkarte.
4. Quellen in `docs/SOURCES.md`; Doku aktualisiert.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code (nach Welle 10): Kartenobjekte haben Lebenspunkte und Stufen
(`data/destruction.toml`, 020), Ruinen-Varianten, Besetzen (022), Rauchwolken als
Sim-Objekte mit Wind-Drift, die die Sicht blockieren (032), Spielstände als
serde-Snapshot des ganzen `Sim` (038). Brennende Wracks gibt es nur als Darstellung
(019, `effects.gd`). Flammenwerfer stehen im Katalog, haben aber keine Waffe mit
Kaliber und schießen in der Sim nicht. Der Gebäude-Generator `house` des Baukastens
(007) ist parametrisch (Geschosse, Wandmaterial, Dach, Fenster, Tore, Schaden) und
baut neue Häuser allein aus einem Bauplan. Die Umwelt-Shader haben Instanz-Uniforms
für das Biom (Schnee, Staub, Trockenheit).

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | **Priorität:** Feuer als Sim-Regel zuerst, dann die wichtigsten neuen Objekte (Stadthaus in Reihe, Rathaus, Lagerhaus, Blockhaus, Munitions- und Treibstofflager), dann eine Kleinstadt-Karte. Der Rest der Objektliste bleibt offen (Ergebnis). | Auftrag: Feuer, Lager, Stadtkarte und die wichtigsten Gebäude; die lange Liste passt nicht in eine Session. |
| 2 | Regeln in eigener Datei **`data/fire.toml`**, von `Sim::new` geladen (wie 021/032), nicht in `SimConfig`. Feuer-Klassen pro Katalog-Id: Zündchance bei Treffer (`hit_pct`), Ansteckchance (`catch_pct`), Reichweite der Flammen (`reach_mm`, Lücke zwischen den Umrissen), Brenndauer, Anteil der Lebenspunkte, den das Feuer zerstört (`burn_hp_pct`: Holz brennt ab, Stein behält die Mauern), Rauchradius, Explosion (`blast_caliber_mm`, `fuse_ms`). **[vorläufig]** | Gemeinsame Dateien bleiben klein; der Katalog hat keine Brennbarkeit, als Regel bleibt es balancierbar. |
| 3 | Feuer schadet **über die Lebenspunkte von 020** (neue Ursache `ObjectCause::Fire`): ein abgebranntes Haus wird zur Ruine, ein Baum fällt, eine Hecke verschwindet – Wegegitter und Sicht folgen über `apply_destruction` im selben Tick. Brennen kann nur, was eine Zerstörungsklasse mit Lebenspunkten hat; Heu, Holzstapel und Lager bekommen deshalb Klassen (Heu, Holz: `gone`; Lager: verkohlte Reste `Burnt`, blockieren nichts). | Keine zweite Schadenslogik; Ruinen, Sicht und Wege sind schon gelöst. |
| 4 | **Zündung:** Direkttreffer ab 20 mm auf ein brennbares Objekt (`hit_pct`); Sprenggranaten ab 50 mm, die am Boden detonieren, zünden brennbare Objekte, deren Umriss höchstens 4 m entfernt ist (Artillerie in den Wald trifft selten einen Stamm); zerstörte Fahrzeuge brennen als **Wrack** 120 s real; explodierende Lager. Flammenwerfer zünden noch nicht (sie haben keine Sim-Waffe, offen). **[vorläufig]** | Nutzt die vorhandenen `Hit`/`Destroyed`-Ereignisse; kein Eingriff in die Flugbahn-Logik. |
| 5 | **Ausbreitung:** alle 2 s real würfelt jedes Feuer einmal pro brennbarem Nachbarn in Reichweite (Index-Reihenfolge, Sim-`Rng`); nicht über Straßen und Wasser: drei Punkte auf der Lücke zwischen den Umrissen werden gegen `firebreak = ["road", "shallow_water", "deep_water"]` geprüft. Neu entzündete Feuer breiten sich erst im nächsten Schritt aus. Ausgebrannte Objekte brennen nie wieder. Kein Wind-Einfluss auf die Ausbreitung (Wind kommt mit 040). | Deterministisch und billig (Objekt-Buckets aus 020); Straße als Brandschneise ist Akzeptanzkriterium. |
| 6 | **Lager explodieren** nach der Zündschnur (Munition 6 s, Treibstoff 2 s real) wie eine Sprenggranate (15 cm bzw. 12 cm) über die Splitterregeln von 032 (`Sim::detonate`), sind danach zerstört, brennen aber weiter; Nachbarn im halben Splitterradius fangen mit `catch_pct` Feuer (Kettenreaktion). Neue Katalog-Objekte `env_dump_ammo`, `env_dump_fuel` (Kategorie `env_prop`). **[vorläufig]** | Kein neues Schadensmodell; Fahrzeuge, Infanterie und Krater reagieren wie bei Artillerie. |
| 7 | **Besatzungen** brennender Gebäude verlieren 3 HP pro Soldat und Sekunde (real), bis sie das Haus verlassen. Brennende Objekte geben alle 20 s real eine **dunkle Rauchwolke** (Wolken von 032 mit Flag `dark`, Drift, Sichtblockade), höchstens 24 Feuerwolken gleichzeitig. **[vorläufig]** | Rauch aus 032 ist vorhanden und blockiert die Sicht; eine Obergrenze schützt Leistung und Rauchgranaten. |
| 8 | Feuerzustand (brennende Objekte und Wracks, ausgebrannte Objekte, Feuerwolken) liegt in `Sim` und ist damit **im Spielstand-Snapshot von 038** (serde) und im Hash. Test: Snapshot mitten im Waldbrand mit laufender Zündschnur, beide brennen gleich weiter. | Vorgabe; 038 speichert jedes Feld von `Sim`. |
| 9 | **Neue Objekte** aus dem Generator `house` (nur Baupläne): `env_house_town` (Stadthaus, drei Geschosse, Traufe zur Straße, Giebel bündig für Reihen), `env_town_hall` (Rathaus, Werkstein), `env_warehouse` (Lagerhaus, Backstein, Ladetore), `env_house_log` (Blockhaus für Winterkarten). Lager über neues Modul `parts_dumps.py` (einfache Kisten/Fässer, Budget 1500). Alle mit Intact/Damaged/Ruin bzw. Intact/Burnt, Fensterplätze (022) und Sicht (018). „Verschneite Varianten“ liefert der Biom-Shader (Schnee auf Oberseiten), kein eigenes Modell. | Baukasten wiederverwenden; Maße mit Quelle, als `uncertain` markiert. |
| 10 | **Darstellung:** `fire_view.gd` zeichnet Flammen (Partikel „fire“ aus 019, Anzahl nach Objektgröße), dunkle Rauchsäule, flackerndes Licht (Budget der Grafikstufe, wirkt nachts) und lässt Flammen gegen Ende abklingen; ausgebrannte Objekte werden über ein neues Instanz-Uniform `env_char` verkohlt gezeichnet. Wolken mit `dark` zeichnet `artillery_view.gd` dunkel. Wracks zeichnet weiter `effects.gd`. | Godot zeichnet nur, was die Sim sagt. |
| 11 | **Karte:** `cli gen-town` erzeugt die Kleinstadt-Karte `town_small` (600 m): Marktplatz mit Rathaus und Kirche, Häuserzeilen an zwei Straßen, Lagerhäuser, Gärten mit Heu und Holz, Bauernhöfe, Wälder (einer von einer Straße geteilt) und je ein Lagerplatz pro Seite. Bahnlinie offen. Keine echten Ortsnamen. | Eigener Generator statt Handarbeit: reproduzierbar, Feuer-Testgelände. |
