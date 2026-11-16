# Prompt 049 – Verwundung und Kameradenhilfe

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/INFANTRY.md`, `docs/ARCHITECTURE.md` (Infanterie, Kampf, Transport aus 034).
>
> Konkretisiert im Abschnitt „Konkretisierung“ (Ausgangslage, Entscheidungen).

## 1. Ziel

Ein Treffer ist bisher Lebenspunktabzug oder Tod. Nach diesem Prompt werden Soldaten
verwundet, versorgen sich, helfen einander und werden geborgen; das macht Infanterie
menschlicher und gibt Sanitätern eine Aufgabe.

## 2. Umfang

1. **Zustände (Sim):** leicht verwundet (humpelt, langsamer, bis er sich verbindet),
   schwer verwundet (liegt, kann nicht kämpfen, stirbt ohne Hilfe nach Zeit),
   versorgt (sichtbarer Verband, humpelt eine Weile). Regeln mit Begründung.
2. **Kameradenhilfe:** Nachbarn ziehen Verwundete aus der Schusslinie in Deckung (an der
   Schulter/am Kragen, Seil aus Deckung als Variante), helfen nach Explosionen auf.
3. **Sanitäter:** versorgen schwer Verwundete, tragen sie mit Trage aus Gräben/zum
   Sammelpunkt; Sanitäter mit weißer Armbinde und Tasche, **kein Rotes Kreuz**
   (`docs/SOURCES.md`).
4. **Ausgestiegene Besatzungen:** bei zerstörten Panzern steigen Überlebende aus (Anzahl
   nach Katalog-Besatzung und Zufall aus `sim::Rng`) und sind ein kleiner Trupp mit
   Pistolen/MPs; sie können ein Fahrzeug gleicher Art wieder besetzen (optional).
5. **Gefangene** [provisional]: umzingelte, niedergehaltene Soldaten ohne Ausweg ergeben
   sich; eigene Gefangene können befreit werden. Nur umsetzen, wenn es einfach bleibt.
6. **HUD:** Verwundete in der Einheitenanzeige, Ereignis „Mann verwundet“.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für alle Zustände, Bergung, Besatzung steigt aus,
   gleicher Hash; Golden-Hash begründet.
2. `just selftest`, `just lan-selftest`, `just ai-tournament` grün.

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code (nach Welle 14): Ein Trupp ist eine Sim-Einheit mit Soldaten, die
eigene Position, Höhe, Trefferpunkte (100) und Trefferbox haben (022); ein Gewehr- oder
MG-Treffer kostet 63 Punkte, der erste Treffer lässt also jeden Mann leben, der zweite
tötet. Treffer kommen aus vielen Stellen (direktes Feuer, Splitter, Flugzeuge, Brand,
Minen, Einsturz, Transport, Ertrinken). 048 wirft Soldaten um (`knocked`, Darstellung mit
Ragdolls und Aufsteh-Clips in `soldier_motion.gd`, Treffer zurückhaltend ohne Blut).
Es gibt die Clips `wounded` (auf dem Rücken, Hände am Bauch) und `medic_treat` (kniend,
beide Hände am Verwundeten). Sanitätstrupps stehen im Katalog (`*_inf_medic`: de 2 Mann
mit P 38, su 2, us 1 unbewaffneter Company Aid Man), haben in der Sim aber keine Aufgabe;
Modelle mit weißer Armbinde und Tasche, ohne Rotes Kreuz (`docs/SOURCES.md`). 050 zählt
Erfahrung über die Kampfereignisse, 036 hat Vorräte, 034 Mitfahrer und deren Verluste,
052 Gräben und Unterstände als Deckungspunkte, 031 einen Ereignis-Feed, 038 speichert
die ganze `Sim` per serde. Zerstörte Fahrzeuge bleiben als Wracks; ihre Besatzung
(Katalog `crew`) kam in der Sim bisher nicht vor.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | **Verwundung über Trefferpunkte:** `Sim::update_care` vergleicht nach allen Treffern eines Ticks die Trefferpunkte jedes Soldaten mit denen des letzten Ticks. Wer Punkte verloren hat und lebt, ist verwundet: schwer mit `heavy_pct` (40 %), sonst leicht; wer schon verwundet war (auch noch humpelnd), immer schwer. Der Wurf nutzt einen eigenen `Rng` aus dem Zustand des gemeinsamen Generators, Einheit und Tick, ohne ihn weiterzudrehen. Tödliche Treffer bleiben tödlich. **[provisional]** | Ein Haken an einer Stelle statt in den acht Schadensquellen: klein und additiv, verträgt sich mit 047/054, die Treffer ändern. Das bestehende Schadensmodell (63/100) bleibt; der Wurf entscheidet, ob der erste Treffer den Mann aus dem Gefecht nimmt. |
| 2 | **Leicht verwundet:** verbindet sich selbst (`bandage_ms` 20 s real, schießt dabei nicht), humpelt danach `limp_ms` (60 s). **Schwer verwundet:** liegt, bewegt sich nicht selbst, schießt nicht, wird wie ein Liegender getroffen, stirbt nach `bleed_ms` (180 s real = 90 s Spiel) ohne Sanitäter. **Versorgt:** sichtbarer Verband, humpelt `limp_ms`. Ein Trupp mit Humpelnden geht mit 70 %, einer, der einen Schwerverwundeten mitträgt, mit 50 % (ohne gesunden Mann: gar nicht). Werte in `data/care.toml`. **[provisional]** | So wie im Prompt; Zeiten kurz genug, dass man die Folgen im Gefecht sieht. Die Verlangsamung macht den Sanitäter wertvoll, ohne Verwundete zurückzulassen (ein Soldat bleibt immer bei seinem Trupp, sonst stimmen die räumlichen Abfragen nicht). |
| 3 | **Zielen:** Schützen zielen auf aufrechte Männer eines Trupps, solange einer steht; auf Liegende (umgeworfen oder schwer verwundet) zielen sie tief (Höhe der liegenden Trefferbox). | Sonst gingen Schüsse auf Liegende in Brusthöhe darüber; das senkte die Trefferquote auf stehende Trupps von 61 % auf 20 %. Mit der Regel bleiben die Tabellenwerte von 022 (stehend 65 %, liegend 25 %, Sandsäcke 10 %). |
| 4 | **Kameradenhilfe:** steht der Trupp, zieht ein Kamerad (nicht verwundet, nicht beschäftigt) den Schwerverwundeten zu dessen Deckungspunkt (bis 15 m, 40 % Gehgeschwindigkeit, rückwärts gehend vor ihm); bewegt sich der Trupp, trägt/zieht er ihn mit. Nach einer Explosion geht ein Kamerad (bis 6 m) zum Umgeworfenen und hilft ihm auf; die Liegezeit aus 048 bleibt (benommen). Wer hilft, schießt nicht. **Seil aus der Deckung: nicht umgesetzt.** **[provisional]** | Ziehen in Deckung hat eine echte Wirkung (Deckungsregeln gelten am Deckungspunkt). Aufhelfen verkürzt die Zeit nicht, damit die Regel und die Tests von 048 unverändert bleiben. Das Seil wäre eine zweite Ziehvariante ohne neue Spielwirkung. |
| 5 | **Sanitäter:** ein untätiger Sanitätstrupp (steht, keine Befehle) geht zum nächsten unversorgten Schwerverwundeten der eigenen Seite (auch Verbündete) im Umkreis von 60 m und stillt sofort die Blutung. Trupps ab 2 Mann (de, su) tragen ihn auf der Trage zu ihrem Posten (Sammelpunkt = wo sie standen, auch aus Gräben) und versorgen ihn dort (30 s real); der US-Sanitäter (1 Mann) versorgt ihn, wo er liegt. Danach steht der Mann auf, humpelt und geht zu seinem Trupp zurück. Ein Befehl des Spielers bricht den Auftrag ab. **[provisional]** | Gibt Sanitätern eine klare, automatische Aufgabe ohne neuen Befehl; die Trage nimmt dem Trupp den Verwundeten ab (er wird nicht langsamer) und holt ihn aus der Feuerlinie. Kein Rotes Kreuz: weiße Armbinde und Tasche wie bisher. |
| 6 | **Ausgestiegene Besatzungen:** Jedes Besatzungsmitglied (Katalog `crew`) eines zerstörten Fahrzeugs (Gruppe Fahrzeug: Panzer, Jagdpanzer, Sturmgeschütze, Halbketten, Lkw, Spähwagen) überlebt mit 50 % (Wurf aus eigenem `Rng`). Die Überlebenden steigen neben dem Wrack als Trupp des neuen Katalogtyps `<fraktion>_inf_tank_crew` aus (Kategorie `inf_crew`, 5 Plätze, MP/Pistolen: MP 40 + 4 P 38, PPSh-41 + 4 TT-33, M3 + 4 M1911A1), nicht vorhandene Männer sind „nicht da“ (unsichtbar). Szenarien nehmen die Besatzungstypen in die Einheitentypen auf. **Besatzungen zählen nicht für den Sieg** (ein Spieler nur mit Besatzungen ist besiegt), die KI greift sie nicht gezielt an und schickt eigene nicht ins Gefecht. **[provisional]** | Spec; eigener Katalogtyp, weil Trupps einen Typ brauchen (Waffen, Modell, Größe). Ohne die Siegregel würde ein Match an zwei versteckten Panzerfahrern hängen; in den KI-Tests liefen die Panzer sonst minutenlang hinter Besatzungen her. |
| 7 | **Wiederbesetzen eines Fahrzeugs gleicher Art: nicht umgesetzt** (optional im Prompt). | Es gibt keine intakten, unbesetzten Fahrzeuge in der Sim; dafür bräuchte es einen neuen Fahrzeugzustand. Offen. |
| 8 | **Gefangene: nicht umgesetzt.** | Nicht einfach: braucht Moral/Ausweglosigkeit (047 baut Niederhalten erst), Gefangenen-Einheiten und Befreien. Offen. |
| 9 | **Darstellung:** neue Clips `limp`, `bandage`, `drag`, `dragged`, `stretcher`, `help_up` (Blender headless, prozedural wie 042/048); Trage und Verband als einfache Godot-Primitiven (Trage: zwei Holme und Tuch zwischen den Trägern; Verband: weiße Binde am linken Oberschenkel, nicht am Arm, damit er nicht wie die Armbinde der Sanitäter aussieht). Liegende Schwerverwundete spielen `wounded`. Besatzungs-Trupps: neue Rollen `crew_commander` (MP/PPSh/Thompson) und `crewman` (Pistole) mit Feldmütze. **[provisional]**: Panzeruniformen (schwarze Panzerjacke, Kopfhaube, Tankerhelm) sind nicht modelliert. | Zurückhaltend wie 048 (kein Blut). Die Primitiven sparen Modelle im LFS; die Rollen nutzen vorhandene Uniformen und Waffen. |
| 10 | **HUD:** Einheitenanzeige zeigt leicht/schwer Verwundete eines Trupps; Ereignis-Feed meldet „Mann verwundet“ (eigene Trupps, gedrosselt) und „Besatzung ausgestiegen“. Abgefragt im Feed-Takt über `SimHost.unit_wounded_counts`, ohne neue Ereignis-Kette in der Bridge. | Klein und additiv; der Feed hat schon einen Abfragetakt (031). |
| 11 | **Speichern (038):** Wundzustand pro Soldat (`Soldier::wound`, serde default), Sanitäter-Aufträge und Typtabellen liegen in `Sim` und werden mitgespeichert; Test `crates/net/tests/snapshot_care.rs`. Der Zustand geht erst in den Hash, wenn etwas passiert ist (Verwundung, Umwerfen, Auftrag, Ausstieg). | Wie 055: Matches ohne Verwundete behalten ihren Hash. |

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
