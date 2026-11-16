# Prompt 077 – Mündungsfeuer aus dem Panzerrohr

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, in
> `docs/ARCHITECTURE.md` den Abschnitt „Effects (presentation only)“ (Auslöser-Tabelle,
> „Drawing“, „Budget and LOD“, „Fog of war“, „Test scene and measurements“,
> „Decisions“, „Open“), in `docs/ASSETS.md` die Tabelle der Sockel (`Socket_Muzzle`)
> und die Hierarchie der Fahrzeugmodelle (`Turret`, `Gun`), dazu
> `docs/PERFORMANCE.md` und `docs/SOURCES.md`. Dann den Code:
> `godot/scenes/world/effects.gd` (`_shot`, `muzzle_flash`, `_muzzle_of`),
> `effect_library.gd` (die Arten `flash`, `powder`, `blast_dust`),
> `effect_textures.gd`, `graphics_quality.gd` (Schlüssel `fx_*`),
> `godot/scenes/world/combat_view.gd` (Leuchtspuren) und
> `godot/scenes/tools/effects_bench.gd`.

**Vorlage:** `.temp/blitz-aus-rohr.png` (nicht im Repo, `.temp/` ist ignoriert). Ein
Bildschirmfoto aus einem anderen Spiel, 433 × 248 Pixel: zwei Panzer auf einer Wiese
im Augenblick des Schusses. Liegt die Datei nicht vor, reicht die Beschreibung in
Abschnitt 2.

## 1. Ziel

Ein Schuss aus einer Panzerkanone hat Wucht: ein gleißender Feuerstoß aus der
Mündung, der in Schussrichtung weit vor das Rohr reicht, eine helle Gaswolke, die
nach vorn geworfen wird und zu Pulverrauch verblasst, aufgewirbelter Staub am Boden
und ein Lichtblitz auf Panzer und Umgebung. Aus der Spielkamera sieht man sofort,
wer geschossen hat und wohin.

Heute ist das Mündungsfeuer klein und kurz: fünf Leuchtteilchen für 0,08 s, etwas
weißer Rauch, ein Staubring.

Fertig ist der Prompt, wenn

- ein Schuss im Spiel neben der Vorlage liegt, bei ähnlichem Blickwinkel, und in
  Größe, Form, Helligkeit und Staub dazu passt,
- der Feuerstoß genau in Richtung des Rohrs zeigt, auch bei gehobenem Rohr,
- er nach Kaliber abgestuft ist und bei Tag, Abend und Nacht und in allen drei
  Biomen stimmt,
- die Bildrate bei 40 Schüssen in einer Sekunde in jeder Grafikstufe über 60 FPS
  bleibt,
- `just check` und `just selftest` grün sind und der goldene Hash unverändert ist.

## 2. Die Vorlage

Das Bild ist **Vorbild für Wirkung und Größe**, kein Material: keine Bildausschnitte
und keine Texturen daraus, nichts aus anderen Spielen (`docs/SOURCES.md`). Texturen
werden wie bisher beim Start aus Rauschen erzeugt. Das Bild ist sehr klein und zeigt
einen einzigen Augenblick; Zeiten kommen nicht aus ihm (Abschnitt 4).

Was es zeigt:

- **Linker Panzer, im Vordergrund.** Aus der Mündung steht eine helle Wolke schräg
  nach vorn oben, etwa eineinhalb bis zwei Panzerlängen lang und am Ende eine
  Panzerbreite dick: innen fast weiß, zur Mündung hin gelb bis orange. An der
  Mündung selbst ein oranger, glühender Ansatz. Aus der Wolke schießt ein dünner
  oranger Strich weiter in Schussrichtung, die Leuchtspur der Granate.
- **Staub.** Unter und hinter dem Panzer quillt eine flache, braune Staubwolke über
  die Wiese, etwa so groß wie das Fahrzeug.
- **Rechter Panzer, weiter hinten.** Ein kleinerer, sternförmiger, gelboranger
  Blitz an der Mündung mit hellem Kern und einem Strich in Schussrichtung; dahinter
  eine braune Staubfahne.
- **Helligkeit.** Beide Blitze sind die hellsten Stellen im Bild und überstrahlen
  ihre Ränder.

## 3. Ausgangslage

`effects.muzzle_flash(at, direction, caliber)` zeichnet für jedes `shot`-Ereignis ab
20 mm:

- `flash`: 5 leuchtende Teilchen, Lebenszeit 0,08 s, in einem Kegel von 6° nach
  vorn, Größe 1,5 m mal Maßstab,
- `powder`: 14 Teilchen weißer Pulverrauch, 3,5 s,
- ein Licht für 0,08 s, ab 37 mm einen Staubring 2,5 m vor der Mündung, eine leichte
  Erschütterung.

Der Maßstab ist Kaliber / 75 mm, begrenzt auf 0,4 bis 1,6. Die Richtung kommt nur aus
dem Drehwinkel des Turms (`_aim_yaws`), ohne Rohrerhöhung: der Blitz liegt immer
waagerecht. Die Leuchtspur (`combat_view.gd`) beginnt am Mündungspunkt der
Simulation, nicht am Sockel des Modells (steht bei „Open“). Einen Rohrrücklauf oder
ein Wippen der Wanne gibt es nicht.

Halte den Stand vor der ersten Änderung als Bildreihe fest (Abschnitt 6).

## 4. Was zu bauen ist

Der Feuerstoß besteht aus Schichten mit eigenem Zeitverlauf. Was genau, entscheidet
der Vergleich mit der Vorlage; das ist der erwartete Aufbau.

| Schicht | Wirkung | Dauer (Anhalt) |
|---|---|---|
| Kern | Gleißend heller Ball an der Mündung, überstrahlt | 1 bis 3 Bilder |
| Feuerzunge | Langer, schmaler Strahl in Rohrrichtung, mehrere Meter, weißgelb zu orange | unter 0,1 s |
| Glühende Gaswolke | Wird nach vorn geworfen, bläht sich auf, anfangs von innen hell, dann grau | 0,2 bis 0,5 s |
| Pulverrauch | Blassgrau, treibt mit dem Wind, vergeht | 2 bis 4 s |
| Seitliche Stöße | Bei Rohren mit Mündungsbremse zwei Strahlen quer zum Rohr | wie Feuerzunge |
| Bodenstaub | Unter der Mündung und um das Fahrzeug aufgewirbelt, in Bodenfarbe, in Schussrichtung gefächert | 2 bis 3 s |
| Licht | Warmer Blitz auf Boden, Fahrzeug und Umgebung, nachts stark | wie Kern |
| Leuchtspur | Der Strich der Granate beginnt an der Mündung des Modells | wie bisher |

- **Richtung.** Der Feuerstoß zeigt in die wirkliche Richtung des Rohrs: aus der
  Lage und Ausrichtung des Sockels `Socket_Muzzle` am Modell (vorn ist +Y), nicht
  nur aus dem Drehwinkel. Fehlt der Sockel, gilt die Richtung der Simulation.
- **Größe nach Waffe.** Die Abstufung nach Kaliber bleibt, aber mit mehr Spreizung
  als 0,4 bis 1,6 und in Klassen, die man unterscheidet: Maschinenkanonen 20 bis
  37 mm (kleine, schnelle Blitze, bei Feuerstößen jeder Schuss), Kanonen 50 bis
  76 mm, schwere Kanonen ab 88 mm, Haubitzen (mehr Rauch und Staub, weniger
  Feuerzunge). Prüfe, ob neben dem Kaliber die Mündungsgeschwindigkeit aus dem
  Katalog die Länge der Feuerzunge steuern soll. Die Größen bleiben 1:1 in Metern
  und glaubhaft: schlag für zwei oder drei Kanonen nach, wie groß ein Mündungsfeuer
  wirklich ist, und notiere die Quelle.
- **Mündungsbremse.** Stell fest, woran sich eine Mündungsbremse erkennen lässt
  (Bauplan, Knotenname, Katalog). Geht es ohne neuen Datenweg, bekommen diese Rohre
  die seitlichen Stöße; sonst offener Punkt.
- **Zeiten.** Echtes Mündungsfeuer dauert Hundertstelsekunden. Im Spiel muss es
  einige Bilder stehen, damit man es sieht, darf aber kein Feuerwerk werden. Die
  Dauern der Tabelle sind Anhaltswerte; nachschlagen, im Spiel ansehen, die
  Entscheidung begründen.
- **Bodenstaub.** Farbe und Menge nach dem Boden unter der Mündung, wie bei den
  Staubfahnen: viel auf Sand und Feldweg, wenig auf Gras und Straße, Schnee als
  Pulver, über Wasser Gischt. Bei hoch gehobenem Rohr (Haubitze) liegt er um das
  Geschütz, nicht davor.
- **Für alle Kanonen.** Panzer, Sturmgeschütze, Pak, Feldgeschütze und Haubitzen
  nutzen dieselbe Funktion und bekommen den neuen Feuerstoß. Handwaffen und
  Maschinengewehre unter 20 mm bleiben, wie sie sind.

**Rohrrücklauf und Wippen.** Zur Wucht gehört, dass das Rohr beim Schuss zurückfährt
und die Wanne kurz nachgibt. Bau beides als reine Darstellung, wenn es ohne Umbau der
Modellhierarchie geht: der Knoten `Gun` fährt entlang seiner Achse zurück und langsam
wieder vor, die Wanne nickt gedämpft. Beachte, dass die Zielrichtung des Turms, die
Abschussringe am Rohr (`veterancy_view.gd`) und die Sockel an denselben Knoten
hängen und weiter stimmen müssen. Wird das zu groß, bleibt es ein offener Punkt.

Die Werte stehen als benannte Konstanten mit Kommentar im Code, woher sie stammen
(Vorlage, Quelle oder Urteil).

## 5. Grenzen

- **Nur Darstellung.** Kein Rust-Code in `crates/sim`, nichts wird zurückgelesen,
  der goldene Hash ändert sich nicht. Godot-Zufall ist hier erlaubt. Rücklauf und
  Wippen ändern keine Position, die die Simulation oder die Auswahl mit der Maus
  kennt.
- **Budget.** Aussender gepoolt wie bisher, Werte je Grafikstufe in
  `graphics_quality.gd`. Der schlimmste Fall ist ein Gefecht, in dem 40 Kanonen
  innerhalb einer Sekunde schießen, dazu Maschinenkanonen im Dauerfeuer. „low“ darf
  weniger Schichten haben. LOD wie bisher, mit einer Ausnahme: der Kern bleibt auch
  aus großer Entfernung als heller Punkt sichtbar, weil er zeigt, wer schießt.
- **Nebel des Krieges.** Neue Schichten starten nur an sichtbaren Punkten
  (`effects.visible_at`), wie das Mündungsfeuer heute.
- **Lesbarkeit.** Der Feuerstoß verdeckt den eigenen Panzer höchstens für wenige
  Bilder. Der Pulverrauch ist dünn genug, dass man durch ihn Einheiten erkennt.
- **Bekannte Falle.** Ein Mesh als Teilchen mit `StandardMaterial3D` hat in Godot
  4.7.2 beim Beenden seinen Shader verloren (Tabelle „Decisions“). Bei jedem neuen
  Material prüfen, dass das Spiel ohne Leck-Meldung endet. Godot und godot-rust
  werden nicht aktualisiert.
- **Nicht gleichzeitig mit Prompt 072 oder 076** im selben Arbeitsverzeichnis: alle
  drei ändern `effects.gd` und `effect_library.gd`. Nacheinander ausführen oder in
  eigenen Worktrees, und wer später fertig wird, führt zusammen.

## 6. Prüfen

1. **Vergleichsbild.** Ein Schuss im Spiel bei ähnlichem Blickwinkel und Abstand
   neben der Vorlage, vorher und nachher. Nach jeder Änderungsrunde neu.
2. **Bildreihe.** `effects_bench.gd` um eine Reihe für den Schuss erweitern (eigenes
   Rezept oder Teil von `just effects-shots`): mit festem Bildtakt jedes Bild der
   ersten halben Sekunde, danach vier je Sekunde, bis der Rauch vergangen ist. Für
   20, 37, 75, 88 und 122 mm oder mehr, eine Haubitze mit gehobenem Rohr, ein Rohr
   mit Mündungsbremse; bei 15, 35, 110 und 350 m Kameraabstand; Tag, Abend, Nacht;
   alle drei Biome.
3. **Richtung.** Bilder von der Seite: der Feuerstoß liegt in der Verlängerung des
   Rohrs, die Leuchtspur beginnt an der Mündung.
4. **Bewegtbild.** Ein Gefecht mit mehreren Panzern aufnehmen und ansehen: Takt,
   Wucht, kein Aufspringen, Rücklauf und Wippen, Einheiten bleiben erkennbar.
5. **Bildrate.** 40 Schüsse in einer Sekunde in jeder Grafikstufe messen (in
   `effects_bench.gd` ergänzen), vorher und nachher, dazu `just effects-bench`
   unverändert über 60 FPS.
6. `just selftest` (prüft, dass der Gefechtstest Mündungseffekte erzeugt) und
   `just check`.

Die Bilder dem Nutzer zeigen. Was nicht erreicht wurde, steht im Abschlussbericht mit
Grund, nicht als erledigt.

## 7. Abschluss

- `docs/ARCHITECTURE.md`: bei „Effects“ die Zeile `shot` der Auslöser-Tabelle und
  ein Unterabschnitt „Muzzle blast (prompt 077)“ (Schichten, Klassen nach Waffe,
  Richtung aus dem Sockel, Rücklauf, Budget, Messwerte, offene Punkte); bei „Open“
  den Punkt zur Leuchtspur streichen, wenn er erledigt ist. `docs/SOURCES.md` für
  nachgeschlagene Werte. `docs/PERFORMANCE.md`, falls sich Messwerte dort ändern.
  Auf Englisch, ohne das Bild der Vorlage.
- `prompts/README.md`: Status auf „erledigt“ mit einer Zeile Ergebnis, die Datei mit
  `git mv` nach `prompts/done/`.
- Commits auf Englisch, in sinnvollen Schritten (Richtung und Leuchtspur,
  Feuerstoß, Staub und Licht, Rücklauf, Werkzeug). Kein `git push`.
- Offen notieren: Das Demo-Video (`prompts/periodic/070-youtube-demo-video.md`)
  zeigt danach das alte Mündungsfeuer.

## 8. Nicht Teil dieses Prompts

Als offene Punkte notieren, nicht umsetzen:

- **Handwaffen und Maschinengewehre** unter 20 mm.
- **Einschläge und Explosionen** (Prompt 072) und **Raketen** (Prompt 076).
- **Ton** des Abschusses (Prompt 069 ist erledigt).
- **Ausgeworfene Hülsen, Rauch aus dem Verschluss, Qualm im offenen Kampfraum.**
- **Gras, das unter dem Druck niedergedrückt wird** (hängt an Prompt 073).
