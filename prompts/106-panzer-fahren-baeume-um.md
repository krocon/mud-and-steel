# Prompt 106 – Schwere Fahrzeuge fahren Bäume um

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/DETERMINISM.md`, `docs/SOURCES.md`, `prompts/done/020-*.md` (Zerstörung,
> gefällte Bäume mit Stumpf), `prompts/done/085-*.md` (Bäume unter schwerem Feuer),
> `prompts/done/093-*.md` ganz (Mauern niederwalzen: Massengrenze, Zusatzkosten in
> der Wegfindung, Lücke, Staub), `data/destruction.toml` (Baumklasse mit
> `remains="fallen"`, bei Mauern `crush_min_mass_kg`), `data/objects.toml`,
> `data/terrain.toml` (Wald), `crates/sim/src/destruction.rs`, `objects.rs`,
> `path.rs`, `vision.rs` (Bäume als Sichtblocker), `data/infantry.toml` (`[cover]`,
> Bäume als Deckung), `godot/scenes/world/map_objects.gd` und
> `destruction_view.gd`. Wenn im Arbeitsverzeichnis fremde, nicht eingecheckte
> Änderungen an `crates/sim` liegen, erst den Nutzer fragen.

## 1. Ausgangslage

Im Referenzvideo `docs/.vorlagen/vorlage.mp4` (nicht im Repo, nur Vorbild für die
**Wirkung**) legen Panzer Bäume einfach um und ziehen eine Schneise durch den
Bewuchs. Der Tester zählt das zu den Dingen, die das Spiel wuchtig wirken lassen.

Bei uns heute fallen Bäume nur durch Beschuss (Baumklasse mit 400 Trefferpunkten,
bleibt als gefällter Baum mit Stumpf liegen). Die Baumklasse hat kein
`crush_min_mass_kg`; Fahrzeuge fahren um Bäume herum oder bleiben draußen.
Niederwalzen gibt es seit Prompt 093 für Mauern, Zäune und Draht.

## 2. Ziel

- **Einzelne Bäume, Baumreihen, Obstgärten, lichter Wald:** Fahrzeuge ab einer
  Massengrenze fahren Bäume um. Der Baum kippt in Fahrtrichtung, bleibt liegen
  (vorhandener Zustand „gefällt“), das Fahrzeug wird dabei langsamer.
- **Dichter Wald** bleibt für Fahrzeuge Sperre oder sehr teuer, damit Wald weiter
  ein Infanterie-Gelände ist und Karten ihre Engstellen behalten. Die Grenze an
  den vorhandenen Geländearten bzw. der Baumdichte festmachen und begründen.
- **Grenzen nach Stammstärke:** dünne Bäume ab mittlerer Masse, dicke nur für
  schwere Kettenfahrzeuge, sehr dicke gar nicht. Räderfahrzeuge und Lkw nie.
  Schwellen aus echten Angaben begründen (`docs/SOURCES.md`: keine Werte aus
  anderen Spielen), sonst als vorläufig kennzeichnen.
- **Wegfindung:** wie bei Mauern mit Zusatzkosten, sodass ein Panzer den Weg
  außen herum nimmt, wenn er kaum länger ist, und nur dann durchbricht, wenn es
  sich lohnt oder der Spieler es ausdrücklich befiehlt (prüfen, wie 093 das
  gelöst hat, und genauso machen).
- **Folgen, die schon geregelt sind, müssen stimmen:** Ein umgefahrener Baum
  blockt keine Sicht mehr und gibt die Deckung eines gefällten Baums (prüfen, was
  heute nach Beschuss gilt). Die Schneise ist danach für alle befahrbar bzw. so,
  wie gefällte Bäume heute gelten.
- **KI:** nutzt die Möglichkeit nicht gezielt, kommt aber damit zurecht (keine
  Staus vor Baumreihen, kein endloses Umpflügen von Wäldern). KI-Turnier vorher
  und nachher.
- **Darstellung:** Kippen mit Schwung in Fahrtrichtung, Blätter und Staub, Knacken
  (vorhandene Klänge prüfen), bei Gegnern außerhalb der Sicht nichts davon. Ob ein
  außerhalb der Sicht umgefahrener Baum im gedämpften Bereich sichtbar wird, ist
  dieselbe Lücke wie in Prompt 098 Abschnitt 3.2; hier nur nennen.

## 3. Regeln

- Regeländerung in `crates/sim`: feste Kommazahlen, kein Zufall außer `sim::Rng`,
  keine Iteration über `HashMap`. Der Golden Hash ändert sich **beabsichtigt**;
  das in der Commit-Nachricht sagen.
- Werte in `data/destruction.toml` bzw. der passenden Regeldatei, nicht im Code.
- Karten prüfen: `just map-reach` für alle Karten vorher und nachher. Wenn eine
  Karte dadurch eine gewollte Sperre verliert (Baumreihe als Grenze), als Befund
  melden und nicht stillschweigend die Karte umbauen.
- Missionen: prüfen, ob eine Mission auf unpassierbare Baumreihen baut.
- Keinen Test abschwächen.

## 4. Vorgehen

1. Bestand lesen, Lösung aus 093 nachvollziehen. Tabelle: Baumart/Dichte →
   Massengrenze, Tempoverlust, Wegkosten.
2. `just map-reach` und `just ai-tournament` als Vorher-Stand.
3. Regel, Wegfindung, Sicht und Deckung.
4. Darstellung und Ton.
5. Tests in `crates/sim`: schwerer Panzer bricht durch die Baumreihe, Lkw nicht;
   danach ist die Sichtlinie frei; dichter Wald bleibt gesperrt; Determinismus.
6. `just check`, `just selftest`, `just lan-selftest`, `just map-reach` je Karte,
   `just ai-tournament`, `just bench`.
7. Doku: `docs/GAME_DESIGN.md` (Entscheidung mit Datum, als vorläufig bis zur
   Bestätigung durch den Nutzer), `docs/MAPS.md`, `docs/BALANCE.md`,
   `docs/SOURCES.md`.
8. Für den Nutzer: Clip „Panzer durch den Obstgarten“, Tabelle der Schwellen,
   Befunde zu Karten und Missionen.

## 5. Nicht Teil dieses Prompts

- Häuser durchfahren.
- Umgefahrene Bäume als Hindernis für Räderfahrzeuge über die heutige Regel für
  gefällte Bäume hinaus.
