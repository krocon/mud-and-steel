# Prompt 108 – Verstärkung in Missionen fährt sichtbar ein

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/DETERMINISM.md`, `docs/MISSIONS.md` ganz (Aktion `spawn`, Aktion `air`,
> Abschnitt „Open“), `prompts/done/039-*.md`, `prompts/done/055-*.md`
> (Fallschirmjäger), `crates/sim/src/mission.rs` und
> `crates/sim/src/sim/mission.rs` (um Zeile 348–372: Einheiten erscheinen in
> Formation am Punkt, Blick zur Kartenmitte), `crates/sim/src/sim/economy.rs`
> (`exit_spot`, Sammelpunkt), `crates/sim/src/path.rs`, die vorhandenen Missionen
> (`data/maps/<karte>/missions/*.toml` mit `mission_points.toml`) und
> `godot/scenes/hud/mission_hud.gd`. Wenn im
> Arbeitsverzeichnis fremde, nicht eingecheckte Änderungen an `crates/sim` liegen,
> erst den Nutzer fragen.

## 1. Ausgangslage

Im Referenzvideo `docs/.vorlagen/vorlage.mp4` (nicht im Repo, nur Vorbild für die
**Wirkung**) kommt Verstärkung nie aus dem Nichts: Sie rollt vom Kartenrand
herein, trifft mit dem Zug am Bahnhof ein, landet mit Booten oder springt mit dem
Fallschirm ab. Der Tester nennt das ausdrücklich „wunderschön anzusehen“; es macht
aus einer Zahl im Konto ein Ereignis auf der Karte.

Bei uns heute:

- Gebaute Einheiten erscheinen neben dem Gebäude und fahren zum Sammelpunkt. Das
  passt zum Basisbau und bleibt so.
- In Missionen lässt die Aktion `spawn` Einheiten **schlagartig** in Formation an
  einem Punkt erscheinen. Auch im Bild des Spielers.
- Fallschirmjäger gibt es über den Lufteinsatz `airborne`.

## 2. Ziel

- **Einfahren vom Kartenrand:** Die Missionsaktion `spawn` bekommt eine Art des
  Eintreffens. Neu ist „fährt ein“: Die Einheiten starten knapp **außerhalb** des
  sichtbaren Kartenbereichs bzw. am Rand, hintereinander in Kolonne auf der
  nächsten Straße oder dem angegebenen Weg, und fahren zum Zielpunkt, wo sie die
  heutige Formation einnehmen. Bisheriges Verhalten bleibt als „erscheint“
  erhalten und ist Standard, damit alte Missionen unverändert laufen.
- **Abstand in der Kolonne** nach Fahrzeuglänge; Infanterie marschiert in Reihe
  oder sitzt auf, wenn die Mission Transporter mitgibt.
- **Wehrlos ist die Kolonne nicht:** Sie verhält sich ab dem ersten Tick wie jede
  Einheit mit Bewegungsbefehl. Kein Schutz, keine Sonderregel.
- **Rand gesperrt oder vom Gegner besetzt:** festlegen und dokumentieren, was dann
  gilt (Vorschlag: sie fährt trotzdem ein; die Mission kann einen anderen
  Eintrittspunkt wählen).
- **Meldung:** Die Ereignisleiste meldet „Verstärkung trifft ein“ mit Sprung zum
  Eintrittspunkt, auf der Minimap ein Ping.
- **Bestehende Missionen:** dort, wo es passt, auf „fährt ein“ umstellen, und
  jede umgestellte Mission einmal durchspielen lassen (Selbsttest der Missionen),
  weil sich der Zeitpunkt ändert, an dem die Truppe am Ziel ist.
- **Gegnerische Verstärkung** nutzt dieselbe Aktion.

## 3. Regeln

- Änderung in `crates/sim` (Missionsaktion): feste Kommazahlen, kein versteckter
  Zufall. Der Golden Hash in `determinism.rs` ändert sich nur, wenn sein Szenario
  die Aktion nutzt; prüfen und in der Commit-Nachricht sagen, ob und warum.
- Alte Missionsdateien bleiben gültig (neues Feld mit Standardwert).
- Karten: Eintrittspunkte brauchen befahrbaren Rand; `just map-check` und
  `just map-reach` für die betroffenen Karten.
- Keinen Test abschwächen.

## 4. Vorgehen

1. Bestand lesen; Liste aller `spawn`-Stellen in den vorhandenen Missionen mit
   Vorschlag „erscheint / fährt ein“.
2. Aktion erweitern, Kolonne und Wegwahl, Tests in `crates/sim`.
3. Meldung und Ping.
4. Missionen umstellen, je Mission Selbsttest.
5. `just check`, `just selftest`, `just lan-selftest`, `just map-check` und
   `just map-reach` je betroffener Karte.
6. Doku: `docs/MISSIONS.md` (neues Feld, Beispiel), `docs/ARCHITECTURE.md`.
7. Für den Nutzer: Clip einer einfahrenden Kolonne und die Liste der umgestellten
   Missionen.

## 5. Nicht Teil dieses Prompts (Vorschlag für später)

- **Zug:** Verstärkung trifft per Zug am Bahnhof ein (braucht fahrende Züge auf
  den Gleisen als Kartenobjekt).
- **Landungsboote** an der Küste.
- Verstärkung im Gefecht gegen KI bzw. im Mehrspieler statt Basisbau (das wäre
  eine andere Wirtschaft; `docs/GAME_DESIGN.md` ist dort bestätigt).
