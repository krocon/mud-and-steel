# Prompt 076 – Raketensalve: Abschuss, Flamme und Rauchspur

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, in
> `docs/ARCHITECTURE.md` die Abschnitte „Heavy strike“, „Indirect fire and smoke“ und
> „Effects (presentation only)“ (mit „Drawing“, „Budget and LOD“, „Fog of war“ und „Big
> blasts (prompt 054)“), dazu `docs/PERFORMANCE.md` und `docs/SOURCES.md`. Dann den
> Code: `godot/scenes/world/strike_view.gd` (darin `launchers()`), `artillery_view.gd`
> (zeichnet fliegende Granaten), `effects.gd`, `effect_library.gd`,
> `effect_textures.gd`, `blast_effects.gd`, `graphics_quality.gd` (Schlüssel `fx_*`),
> `godot/scenes/tools/blast_shots.gd`, `crates/bridge/src/artillery_funcs.rs` und
> `strike_funcs.rs`, `data/strike.toml`.

**Vorlagen:** `.temp/stalinorgel0.png`, `.temp/stalinorgel2.png`,
`.temp/stalinorgel3.png` (nicht im Repo, `.temp/` ist ignoriert). Drei aufeinander
folgende Bilder aus dem Werbevideo eines anderen Strategiespiels, je etwa 447 × 566
Pixel: drei Raketenwerfer auf Lkw feuern eine Salve. Liegen die Dateien nicht vor,
reicht die Beschreibung in Abschnitt 2.

## 1. Ziel

Eine Raketensalve sieht aus wie in den Vorlagen: am Werfer ein gleißender Feuerschein
bei jedem Abschuss, jede Rakete mit heller Flamme am Heck, dahinter eine weiche, helle
Rauchspur, die stehen bleibt, und viele Raketen zugleich in der Luft, deren Spuren
sich zu Bändern legen.

Heute fliegen die Raketen des schweren Feuerschlags (Prompt 054) als dunkle Kugeln wie
jede Granate, ohne Flamme und ohne Spur, und der Abschuss hat keinen eigenen Effekt.

Fertig ist der Prompt, wenn

- eine Salve im Spiel neben den drei Vorlagen liegt, bei ähnlichem Blickwinkel, Licht
  und Biom (Winter, Abend), und in Feuerschein, Flamme, Spur und Dichte dazu passt,
- Abschuss, Flug und Spur bei Tag, Abend und Nacht und in allen drei Biomen stimmen,
- die Bildrate während zweier gleichzeitiger Salven in jeder Grafikstufe über 60 FPS
  bleibt,
- `just strike-shots`, `just check` und `just selftest` grün sind und der goldene
  Hash unverändert ist.

## 2. Die Vorlagen

Die Bilder sind **Vorbild für Wirkung und Ablauf**, kein Material: keine
Bildausschnitte und keine Texturen daraus, nichts aus anderen Spielen
(`docs/SOURCES.md`). Texturen werden wie bisher beim Start aus Rauschen erzeugt. Die
Schrift in der Bildmitte und die Schneeflocken sind Zutaten des Videoschnitts und
gehören nicht dazu. Drei Standbilder zeigen keinen Zeitverlauf; Zeiten kommen aus den
Spielregeln und aus Quellen (Abschnitt 4).

Die Szene: verschneite Lichtung in der Abenddämmerung, drei Werfer nebeneinander
(Lkw mit Schienengestell), Schussrichtung schräg nach rechts oben, rechts ein
brennendes Haus.

| Bild | Zeigt |
|---|---|
| `stalinorgel0.png` | Die ersten Raketen haben die Schienen verlassen und sind 10 bis 15 m weit. An jedem Werfer ein Feuerball am Fuß des Gestells, so groß wie das Fahrzeug: weißgelber Kern, oranger Hof, stark überstrahlt. Der Schnee ringsum leuchtet warm. |
| `stalinorgel2.png` | Die ersten Raketen sind weiter, die nächsten lösen sich. Hinter jeder Rakete eine gerade, helle Spur zurück zum Werfer. Der Feuerschein am Werfer ist kleiner, am Boden hinter den Werfern quillt Rauch auf. |
| `stalinorgel3.png` | Etwa zehn Raketen zugleich in der Luft, gestaffelt in derselben Richtung. Die Spuren liegen als parallele Bänder im Bild und werden nach hinten breiter und blasser. An den Werfern wieder drei große, überstrahlte Feuerbälle; die ganze Lichtung liegt im Dunst. |

Was die Wirkung ausmacht:

- **Die Flamme.** Ein länglicher Tropfen am Heck jeder Rakete, einige Meter lang,
  Kern weißgelb, Rand orange, mit weichem Leuchthof. Die Rakete selbst ist kaum zu
  sehen; man sieht die Flamme.
- **Die Spur.** Hell, warm grau bis fast weiß, weich, halb durchsichtig, vom Feuer
  der eigenen und der nächsten Rakete angeleuchtet. Sie ist gerade, bleibt stehen,
  wird mit dem Alter breiter und blasser und reißt nicht ab.
- **Der Feuerschein am Werfer.** Ein überstrahlter Ball bei jedem Abschuss, der den
  Boden im Umkreis von 10 bis 15 m warm beleuchtet und dann zusammenfällt.
- **Der Rauch am Boden.** Hinter und unter den Werfern wird Rauch und Schnee
  aufgewirbelt; nach einigen Raketen steht die Stellung im Dunst.
- **Die Menge.** Die Wirkung entsteht aus der Staffelung: viele Flammen und viele
  Spuren zugleich, im festen Takt.

## 3. Ausgangslage

- Der schwere Feuerschlag feuert je Fraktion Raketen von einer Batteriestellung
  (`base_rocket_battery`): 24 Raketen im Abstand von 250 ms, Regeln in
  `data/strike.toml`. Für die Sowjetunion ist das die 132-mm-Rakete M-13 der BM-13,
  die Stalinorgel.
- In der Simulation ist jede Rakete eine gewöhnliche Granate (`Sim::launch_shell`).
  `artillery_view.gd` zeichnet sie als dunkle, überhöht große Kugel entlang ihrer
  Bahn. Die Brücke liefert je Granate Position, Geschwindigkeit, Ziel, Besitzer,
  Kennung, Restzeit und Kaliber, aber nicht, ob es eine Rakete ist.
- `StrikeEvent::Launched` meldet den Beginn der Salve, nicht jede einzelne Rakete.
- Die Stellung trägt drei schräge Startgestelle mit je sechs Schienen
  (`strike_view.launchers()`), auf dem geliehenen Modell des Munitionslagers. Ein
  eigenes Werfer-Fahrzeug gibt es noch nicht; das bringt Prompt 062 (Entwurf).
- Die Einschläge zeichnen `effects.gd` und `blast_effects.gd`. Sie sind nicht Teil
  dieses Prompts (Prompt 072 überarbeitet die Explosionen).

## 4. Was zu bauen ist

**Abschuss am Werfer, je Rakete:**

- Feuerschein am Gestell: heller Kern, oranger Hof, so hell, dass er überstrahlt
  (Werte über 1 in den Glüheffekt, wie Mündungsfeuer heute).
- Feuerstrahl und Rauch nach hinten unten aus dem Heck der Schiene.
- Aufgewirbelter Boden hinter dem Werfer in der Farbe des Bodens: Schnee im Winter,
  Staub in der Wüste, Erde und Gras im Wald (`tint`, wie bei den anderen Effekten).
- Licht auf Boden und Umgebung: ein warmes Licht je Stellung, das mit jedem Abschuss
  aufflackert, nicht ein Licht je Rakete. Nachts deutlich stärker, mit den Faktoren
  je Tageszeit aus `blast_effects.gd`. Innerhalb der Lichter-Budgets.
- Die Raketen verlassen die Schienen reihum, eine Schiene nach der anderen über alle
  Gestelle, nicht alle vom selben Punkt.
- Nach der Salve bleibt Rauch über der Stellung stehen und zieht mit dem Wind ab.

**Rakete im Flug:**

- Statt der Kugel ein schlanker Raketenkörper, in Flugrichtung gedreht.
- Die Flamme am Heck, solange der Motor brennt. Ein Raketenmotor dieser Art brennt
  nur am Anfang der Bahn; danach fliegt die Rakete ohne Flamme weiter. Schlag die
  Brenndauer der M-13 nach, notiere die Quelle und nimm sie als Anhalt; im Spiel
  gilt der Zeitfaktor 2 (`docs/SCALE.md`). Die Flamme darf länger brennen als echt,
  wenn die Salve sonst nicht wie in den Vorlagen wirkt; dann die Abweichung
  begründen.
- Die Rauchspur: entsteht am Heck, solange der Motor brennt, bleibt in der Welt
  stehen (nicht an der Rakete), wird breiter und blasser, driftet mit dem Wind des
  Bioms und vergeht nach einigen Sekunden. Sie reißt weder bei hoher Geschwindigkeit
  in einzelne Tupfen auf noch verschwindet sie beim Einschlag der Rakete.
- Die Spur wird vom Feuer angeleuchtet: nahe der Flamme warm und hell, weiter hinten
  grau. Bei Nacht ist sie nur dort hell, wo eine Flamme in der Nähe ist.
- Im Anflug auf das Ziel hat die Rakete keine Flamme mehr; das Pfeifen der
  anfliegenden Granate (`shell_incoming`) bleibt.

**Woran der Code eine Rakete erkennt.** Ohne Änderung am Zustand der Simulation.
Zwei Wege, nimm den einfacheren, der sicher trifft:

1. In der Darstellung: eine Granate, die während einer laufenden Salve an der
   Batteriestellung ihres Besitzers erscheint, ist eine Rakete.
2. Eine lesende Funktion in der Brücke, die je Granate den Schützen oder die Art
   liefert, wenn die Simulation das ohnehin weiß. Kein neues Feld im Zustand.

**Für alle drei Fraktionen.** Die Salve der Batteriestellung bekommt die Effekte bei
jeder Fraktion. Unterschiede je Fraktion nur über wenige Werte (Dichte der Spur,
Länge der Flamme) und nur mit Quelle; sonst gleich.

**Für Prompt 062 wiederverwendbar.** Abschuss und Flug sind so gebaut, dass ein
späterer fahrbarer Werfer sie mit einem Aufruf nutzen kann: „Rakete startet an diesem
Punkt in diese Richtung“ und „diese Granate ist eine Rakete“. Nichts hängt fest an
der Batteriestellung.

Die Werte stehen als benannte Konstanten mit Kommentar im Code, woher sie stammen
(Vorlage, Quelle oder Urteil).

## 5. Grenzen

- **Nur Darstellung.** Kein Eingriff in die Regeln, keine neuen Ereignisse, die in
  den Zustand eingehen. Der goldene Hash ändert sich nicht. Godot-Zufall ist hier
  erlaubt. Die Raketen fliegen und treffen, wie die Simulation es sagt; die
  Darstellung biegt keine Bahn.
- **Budget.** Aussender gepoolt wie in `effects.gd`, Werte je Grafikstufe in
  `graphics_quality.gd`. Der schlimmste Fall sind zwei Salven zugleich: 48 Raketen,
  48 Spuren, zwei Stellungen. „low“ darf kürzere, dünnere Spuren und keinen
  Bodenrauch haben. LOD wie bisher: was auf dem Bildschirm winzig ist, wird
  ausgelassen oder vereinfacht.
- **Nebel des Krieges.** Halte dich daran, wie Granaten und Mündungsfeuer heute
  behandelt werden: Die Stellung wird während des Feuerschlags allen gezeigt,
  Geschosse sind ungefiltert. Bodeneffekte starten nur an sichtbaren Punkten
  (`effects.visible_at`).
- **Lesbarkeit.** Der Feuerschein darf die Stellung und Einheiten daneben nicht
  sekundenlang weiß überdecken; zwischen zwei Abschüssen fällt er sichtbar zusammen.
- **Bekannte Falle.** Ein Mesh als Teilchen mit `StandardMaterial3D` hat in Godot
  4.7.2 beim Beenden seinen Shader verloren (Tabelle „Decisions“ bei „Effects“). Bei
  jedem neuen Material prüfen, dass das Spiel ohne Leck-Meldung endet. Godot und
  godot-rust werden nicht aktualisiert.
- **Nicht gleichzeitig mit Prompt 072** im selben Arbeitsverzeichnis: beide ändern
  `effects.gd` und `effect_library.gd`. Läuft 072 parallel, dann in eigenem
  Worktree, und wer später fertig wird, führt zusammen.

## 6. Prüfen

1. **Vergleichsbilder.** `blast_shots.gd` um eine Bildreihe der Salve erweitern
   (eigenes Rezept oder Teil von `just strike-shots`): Blick schräg von oben auf die
   Stellung wie in den Vorlagen, Karte im Winter, Abend, Bilder im festen Takt ab
   dem ersten Abschuss bis der letzte Rauch vergangen ist. Die drei Bilder, die den
   Vorlagen entsprechen, daneben stellen. Nach jeder Änderungsrunde neu.
2. **Weitere Ansichten.** Die Salve von der Seite über die ganze Flugbahn, der
   Anflug am Ziel, Tag, Abend, Nacht, alle drei Biome, alle drei Fraktionen.
3. **Bewegtbild.** Eine Aufnahme der ganzen Salve mit Ton ansehen: Takt, Staffelung,
   stehende Spuren, kein Abreißen, kein Aufspringen.
4. **Bildrate.** Zwei Salven zugleich in jeder Grafikstufe messen (in
   `effects_bench.gd` ergänzen), vorher und nachher, dazu `just effects-bench`
   unverändert über 60 FPS.
5. `just strike-shots` (prüft Rauch, Sirene, Countdown und Start), `just selftest`,
   `just check`.

Die Bilder dem Nutzer zeigen. Was nicht erreicht wurde, steht im Abschlussbericht mit
Grund, nicht als erledigt.

## 7. Abschluss

- `docs/ARCHITECTURE.md`: bei „Effects“ ein Unterabschnitt „Rocket salvo (prompt
  076)“ (Schichten, Erkennung der Raketen, Budget je Stufe, Messwerte, offene
  Punkte), „Heavy strike“ und die Auslöser-Tabelle anpassen. `docs/SOURCES.md` für
  die Brenndauer. `docs/PERFORMANCE.md`, falls sich Messwerte dort ändern. Auf
  Englisch, ohne die Bilder der Vorlagen.
- Prompt 062 (Entwurf) um einen Satz ergänzen: Die Effekte für Abschuss und Flug
  gibt es, und wie man sie aufruft.
- `prompts/README.md`: Status auf „erledigt“ mit einer Zeile Ergebnis, die Datei mit
  `git mv` nach `prompts/done/`.
- Commits auf Englisch, in sinnvollen Schritten (Erkennung und Raketenkörper, Flamme
  und Spur, Abschuss, Werkzeug). Kein `git push`.
- Offen notieren: Das Demo-Video (`prompts/periodic/070-youtube-demo-video.md`)
  müsste für eine Einstellung mit Feuerschlag neu aufgenommen werden.

## 8. Nicht Teil dieses Prompts

Als offene Punkte notieren, nicht umsetzen:

- **Der Werfer als Fahrzeug** (BM-13 auf Lkw, Nebelwerfer, Calliope): Prompt 062.
- **Einschläge und Explosionen:** Prompt 072.
- **Ton:** ein eigener Abschussklang für Raketen (Fauchen, Heulen). Prüfe nur, was
  `battle_sound.gd` heute beim Start spielt, und notiere, was fehlt.
- **Leuchtspuren und Rauchspuren anderer Waffen** (Panzerfaust, Bazooka,
  Flugzeugraketen), die dieselbe Technik nutzen könnten.
