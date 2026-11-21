# Prompt 167 – Blaue und gelbe Balken für eigene Einheiten, dezenteres Hervorheben unter der Maus

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/CONTROLS.md` (Lebensbalken, „Own units“), `prompts/done/164-*.md`,
> `godot/scenes/world/unit_highlight.gd` ganz (`HOVER_SHARE`, `HOVER_MIN_M`,
> `HOVER_MAX_M`, `hover_radius`, `hovered`, `nearest`, `lit`, `RING_HOVER`,
> `LIGHT_UP`, `MARKER_BRIGHT`), `godot/scenes/world/combat_view.gd` (innere Klasse
> `HealthBars`, `_draw`, Einstellung `health_bars`), `godot/scenes/hud/relation_colors.gd`,
> `godot/scenes/menu/settings.gd` (`HEALTH_BAR_MODES`, `UNIT_MARKER_MODES`),
> `godot/scenes/tools/unit_marker_shots.gd` und den Abschnitt `_highlight_test` in
> `godot/scenes/self_test.gd`.

## 1. Ausgangslage

Der Nutzer hat ein Bild aus einem anderen Spiel gezeigt (nur Vorbild für die
**Wirkung**, nichts nachbauen): Über einer eigenen Einheit, die ausgewählt ist oder
über der die Maus steht, liegt ein kurzer, schmaler **blauer Balken** mit dunklem
Rand, der die Gesundheit zeigt. Eine andere Markierung braucht es dort nicht; man
erkennt sofort, welche Einheit gemeint ist und wie es um sie steht. Feinde tragen
einen roten Balken.

Ein zweites Bild desselben Spiels zeigt dieselben Balken **in Gelb** über einer
ausgewählten Truppe: über jedem Infanterietrupp mehrere kurze, schmale gelbe Balken,
treppenartig leicht versetzt übereinander (je Mann bzw. Teil des Trupps einer).
Auslegung (vom Nutzer so beschrieben, Farben im Ergebnis bestätigen lassen):
**gelb = ausgewählt, blau = unter der Maus** bzw. nahe bei ihr.

Bei uns heute (Prompt 031 und 164):

- `HealthBars` in `combat_view.gd` zeichnet Balken über beschädigten und über
  ausgewählten Einheiten (Einstellung `health_bars`), in der Beziehungsfarbe: eigene
  in der Standardpalette **grün**, Verbündete blau, Feinde rot. Die Maus spielt
  dabei keine Rolle.
- `unit_highlight.gd` hebt **alle** Einheiten im Umkreis des Mauszeigers hervor
  (20–80 m je nach Zoom): farbiger Ring, die nächste kräftiger, eigene Modelle
  aufgehellt, helle Bodenmarken unter ihren Soldaten. In einem Gefecht leuchten so
  schnell zehn und mehr Einheiten auf. Der Nutzer findet das zu viel: „Mache auch
  den Maus-Over-Effekt dezenter, viel weniger Einheiten.“

## 2. Ziel

1. **Blauer/gelber Balken:** Eine eigene Einheit zeigt einen Gesundheitsbalken,
   wenn die Maus über ihr bzw. nahe bei ihr steht (blau) **oder** sie ausgewählt
   ist (gelb). Infanterietrupps zeigen wie im zweiten Vorbild mehrere kurze Balken
   treppenartig versetzt (je Mann, bei großen Trupps zusammengefasst, Richtwert
   höchstens 6 Balken); Fahrzeuge und Geschütze einen Balken. Nach
   dem Wegfahren der Maus verschwindet er (kurz ausblenden, nicht flackern).
   Beschädigte eigene Einheiten zeigen ihn wie heute je nach Einstellung
   `health_bars` weiter.
2. **Dezenteres Hervorheben:** Unter der Maus wird nur noch **die eine** Einheit
   hervorgehoben, auf die der Zeiger zeigt (Treffer auf Modell oder sehr kleiner
   Umkreis), höchstens ein Trupp bzw. ein Fahrzeug. Kein Umkreis von 20–80 m mehr,
   der ganze Gruppen erfasst. Die Hervorhebung selbst wird schwächer: der Balken
   ist das Hauptzeichen, Ring und Aufhellen höchstens leise.
3. Die Taste „alle eigenen zeigen“ (Tab, Prompt 164) bleibt und zeigt dann für alle
   eigenen Einheiten im Bild den blauen Balken.

## 3. Umsetzung

Reine Darstellung, `crates/sim` und der Golden Hash bleiben.

### 3.1 Balken

- In `HealthBars._draw` die Bedingung erweitern: eigene Einheit **und**
  (ausgewählt **oder** unter der Maus laut `unit_highlight.gd` **oder** Tab
  gehalten) → Balken zeichnen. Den Zustand „unter der Maus“ aus `unit_highlight.gd`
  lesen, nicht ein zweites Mal ermitteln.
- Farbe für eigene Einheiten **blau** unter der Maus und **gelb** ausgewählt (gelb
  gewinnt, wenn beides zutrifft), wie vom Nutzer gewünscht. Achtung: In der
  Standardpalette sind Verbündete schon blau und eigene grün. Vor dem Umsetzen
  prüfen und im Ergebnis nennen, wie beides unterscheidbar bleibt (Vorschlag:
  eigene kräftiges Blau, Verbündete heller/türkis, oder den Verbündeten-Balken mit
  anderer Randfarbe); bei den Farbschwäche-Paletten (`relation_colors.gd`) deren
  eigene Farbe benutzen, nicht hart blau. Wenn das die Palette selbst ändern würde
  (Minimap, Ringe), nicht stillschweigend tun, sondern als Befund melden.
- Form wie im Vorbild: kurz (Richtwert 28–32 px), schmal (3–4 px), dunkler Rand,
  leerer Teil dunkelgrau statt durchsichtig, über dem Modell; bei Trupps die
  Gesundheit des Trupps (lebende Männer bzw. `unit_health`). Größe mit
  `HudTheme.scale` mitwachsen lassen, damit sie auf 4K lesbar ist.
- Kurzes Ausblenden (Richtwert 0,3 s) beim Verlassen der Maus.

### 3.2 Hervorheben unter der Maus

- `hover_radius()` ersetzen durch eine Auswahl der **einen** Einheit unter dem
  Zeiger: zuerst die, deren Modell bzw. Auswahlkreis auf dem Bildschirm den Zeiger
  enthält; sonst die nächste innerhalb eines kleinen Bildschirmabstands (Richtwert
  24 px, unabhängig vom Zoom). Dieselbe Regel, die ein Linksklick zum Auswählen
  benutzt, damit Hover und Klick dieselbe Einheit treffen; falls `selection.gd`
  schon eine solche Funktion hat, diese verwenden.
- `hovered`/`lit` enthalten dann höchstens eine Einheit (einen Trupp). Ring- und
  Aufhell-Stärke (`RING_HOVER`, `LIGHT_UP`) deutlich senken (Richtwert halbieren);
  helle Bodenmarken nur noch für diese eine Einheit.
- Feindliche und verbündete Einheiten unter der Maus: nur ihr Ring bzw. ihr
  vorhandener Balken, kein zusätzliches Aufleuchten.
- Einstellung `unit_markers` bleibt (`always` / `hover` / `off`); die Texte in den
  Einstellungen anpassen, falls „im Umkreis“ darin steht.

## 4. Regeln

- Keine Godot-Physik für die Auswahl unter der Maus, wenn der Klick sie heute auch
  nicht benutzt; Kamera-Projektion wie in `selection.gd`.
- Farben und Größen als Konstanten oben im Skript; lesbar auf Wiese, Schnee, Sand,
  Stadt und bei Nacht.
- Keinen Test abschwächen: der Selbsttest aus 164 prüft heute den Umkreis; ihn auf
  die neue Regel umstellen (eine Einheit), nicht lockern.
- Batch-Sessions teilen sich das Arbeitsverzeichnis: vor schweren Builds nachsehen,
  ob andere Prompt-Sessions laufen.

## 5. Vorgehen

1. Bestand lesen; im Spiel (oder mit `just unit-marker-shots`) vorher Bilder in
   einer Gefechtslage machen: Maus zwischen mehreren eigenen Trupps.
2. Hover-Auswahl auf eine Einheit umbauen, Stärken senken.
3. Balken-Bedingung und blaue Farbe; Unterscheidung zu Verbündeten klären.
4. Selftest: Maus über einem eigenen Trupp → genau diese Einheit `hovered` und mit
   Balken; Maus 30 px daneben zwischen zwei Einheiten → höchstens eine; Maus weg →
   kein Balken (nach dem Ausblenden); ausgewählte unbeschädigte Einheit hat einen
   gelben Balken (ein Trupp mehrere, je Mann); Tab → alle eigenen im Bild mit Balken; Feind unter der Maus → kein blauer
   Balken.
5. `just check`, `just selftest`; `just bench-view` vorher/nachher (HUD-Spalte).
6. Doku: `docs/CONTROLS.md` (Lebensbalken, „Own units“), `docs/ARCHITECTURE.md`.
7. Für den Nutzer: Bilder vorher/nachher an derselben Stelle (Wiese, Stadt, Nacht).

## 6. Nicht Teil dieses Prompts

- Umriss verdeckter Einheiten (Prompt 165).
- Routenlinien (Prompt 166), Reichweitenkreise (Prompt 099, erledigt).
- Neue Uniform- oder Modellfarben.

## 7. Ergänzung (Rückmeldung des Nutzers, 2026-11-21): Umriss statt Ring

Der Nutzer hat zwei weitere Bilder gezeigt. Das erste ist ein Bildschirmfoto aus
unserem Spiel: Maus zwischen Panzern und einem Trupp, mehrere hellgrüne Ringe
leuchten auf (Panzer und ein großer Ring um den ganzen Trupp). Sein Wunsch:

> „Ich möchte keine Ringe mehr haben wie im Bild beim Mausover. Nur noch eine
> dezente Markierung, wenn ich direkt drüber bin. Markierung soll aussehen wie
> Fahrzeug/Person selbst, nur etwas größer umrissen.“

Das zweite Bild (aus einem anderen Spiel, nur Vorbild für die **Wirkung**, nichts
nachbauen) zeigt ein Fahrzeug unter dem Mauszeiger mit einem dünnen, leicht
leuchtenden **blauen Umriss**, der genau der Silhouette des Modells folgt und ein
paar Pixel darüber hinausragt; darunter der kurze blaue Balken aus Ziel 1.

Das **ersetzt** Ziel 2 und Abschnitt 3.2 in folgenden Punkten (Balken aus 3.1 und
Tab aus Ziel 3 bleiben):

1. **Keine Ringe beim Hover.** Unter der Maus erscheint kein Ring mehr, auch kein
   leiser, und kein Aufhellen des Modells (`RING_HOVER`, `LIGHT_UP`, Hover-Zweig
   der Bodenmarken entfallen bzw. greifen beim Hover nicht mehr). Auswahlringe
   ausgewählter Einheiten sind nicht Teil dieser Änderung; bleiben wie sie sind,
   im Ergebnis kurz nennen, ob sie zum neuen Umriss noch passen.
2. **Nur direkt darüber.** Hervorgehoben wird nur, wenn der Zeiger **auf dem
   Modell** liegt (Fahrzeug, Geschütz bzw. einem Soldaten des Trupps), mit
   höchstens wenigen Pixeln Toleranz (Richtwert 4–6 px, mit `HudTheme.scale`),
   nicht mehr die nächste Einheit in 24 px Umkreis. Steht die Maus auf freiem
   Boden zwischen Einheiten, ist nichts hervorgehoben. Ohne Godot-Physik:
   projizierte Umrisse/Hüllkörper der Modelle wie beim Klick; Klick und Hover
   benutzen dieselbe Trefferfunktion (gegebenenfalls die Klickregel mit
   anpassen und das im Ergebnis sagen).
3. **Markierung = Umriss der Silhouette.** Die Einheit unter dem Zeiger bekommt
   einen dünnen Umriss, der der Form des Modells folgt (Richtwert 2–3 px auf dem
   Bildschirm, gleich dick bei jedem Zoom und mit `HudTheme.scale`), leicht
   weich/leuchtend, nicht grell. Bei einem Trupp bekommen **alle Soldaten
   dieses Trupps** den Umriss (sie gehören zusammen, ein Klick wählt den ganzen
   Trupp), bei einem Fahrzeug das Fahrzeug samt sichtbarer Besatzung. Farbe für
   eigene Einheiten dasselbe Blau wie der Hover-Balken (Farbschwäche-Paletten:
   deren eigene Farbe); Feinde unter der Maus bekommen den Umriss in ihrer
   Beziehungsfarbe (rot), damit man sieht, was ein Rechtsklick angreift;
   Verbündete in ihrer Farbe.
4. **Technik:** Prompt 165 (Umriss verdeckter Einheiten) baut gerade eine
   Silhouetten-Technik mit Stencil (`unit_xray.gdshader`, `STENCIL_VISIBLE` in
   `unit_camo.gdshader`/`soldier.gdshader`). Vorher nachsehen, ob 165 erledigt
   ist, und darauf aufbauen statt eine zweite Technik zu erfinden. Naheliegend:
   ein zusätzlicher `next_pass` nur für die gehoverte Einheit, der die Hülle in
   Bildschirmpixeln entlang der Normalen aufbläst und per Stencil nur außerhalb
   der eigenen Silhouette zeichnet (so entsteht ein Rand, keine Fläche). Das
   Material nur auf die eine Einheit setzen, nicht auf alle Modelle; Kosten mit
   `just bench-view` messen (Richtwert unter 0,2 ms pro Bild).
5. **Kurzes Ein-/Ausblenden** (Richtwert 0,15 s), damit beim Überstreichen
   vieler Einheiten nichts flackert.
6. Einstellung `unit_markers` (`always` / `hover` / `off`): die Bodenmarken unter
   Soldaten bleiben davon getrennt; im Modus `hover` zeigt die Maus nur noch den
   Umriss. Texte in den Einstellungen anpassen, falls dort „Ring“ oder „im
   Umkreis“ steht.

Selbsttest (ersetzt die Hover-Punkte in 5.4): Maus genau auf einem Panzer →
dieser `hovered`, Umriss-Material gesetzt, kein Ring sichtbar; Maus auf einem
Soldaten → alle Soldaten dieses Trupps mit Umriss, kein anderer Trupp; Maus auf
freiem Boden 30 px neben einer Einheit → nichts `hovered`, kein Umriss; Feind
unter der Maus → roter Umriss, kein blauer Balken. Bilder vorher/nachher an der
Stelle aus dem Bildschirmfoto des Nutzers (Panzerreihe mit zwei Trupps auf Wiese)
sowie in Stadt und bei Nacht.
