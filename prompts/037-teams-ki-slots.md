# Prompt 037 – Teams, KI-Plätze in der Lobby und Spielmodi

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Spielablauf, Basisbau), `docs/AI.md`, `docs/LAN.md`,
> `docs/ONLINE.md`, `docs/GAME_DESIGN.md`.
>
> **Konkretisiert** (Abschnitt 4) in der umsetzenden Session; der Nutzer war
> nicht erreichbar, Entscheidungen zum Bestätigen sind mit **[vorläufig]**
> markiert.

## 1. Ziel

Bisher spielt jeder gegen jeden, die Lobby hat keine KI-Plätze und im LAN gibt es keine
Moduswahl („There are no teams yet“, „The lobby offers no AI slots yet“). Nach diesem
Prompt sind 2-gegen-2 und 3-gegen-3 mit Menschen und KI gemischt möglich, lokal, im LAN
und über Steam.

## 2. Umfang

1. **Sim:** Teams in der Spielkonfiguration (im Hash), Sieg pro Team, Verbündete greifen
   sich nicht an, teilen Sicht (Option), können sich keine Einheiten steuern.
   Kapitulation eines Spielers übergibt nichts; Team verliert, wenn alle besiegt sind.
2. **Lobby (Einzelspieler, LAN, Steam):** Plätze mit Mensch/KI (Stufe)/geschlossen,
   Team- und Fraktionswahl pro Platz, Kartengröße passend zur Spielerzahl, Modus
   (Gefecht ohne Basis, Basisbau). KI-Spieler hostet der Host wie in 024 vorbereitet.
3. **Karten:** Startpositionen und Versorgungspunkte für 4 und 6 Spieler; Editor (028)
   kann Teams/Startplätze setzen; mindestens eine 2v2-Karte.
4. **KI:** kennt Verbündete, greift deren Gegner an, hilft bei Angriffen (einfach halten).
   Endspiel der KI in Basisbau-Partien (aus `GAME_DESIGN.md` „Open“).
5. **HUD:** Spielerfarben pro Team, Verbündete auf der Minikarte, Ping an Verbündete.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für Teamsieg, geteilte Sicht, kein Eigenbeschuss
   unter Verbündeten; Golden-Hash bewusst aktualisiert.
2. `just lan-selftest` mit zwei Menschen + zwei KIs im 2v2 grün; `just selftest` grün.
3. `just ai-tournament` unterstützt Teams.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## 4. Konkretisierung (vor der Umsetzung, gegen den Stand nach 031/038)

### Ausgangslage im Code

- `sim`: Gegner = „anderer Besitzer“ an fünf Stellen (Zielwahl direkt und
  indirekt, Feuer durch Kameraden, Beinahe-Treffer, `Attack`-Befehl),
  Versorgungspunkte „kein anderer Spieler da“; Sicht pro Spieler
  (`Vision`, Bitmaske je Einheit); Sieg: „höchstens ein Spieler übrig“
  (`outcome.rs`). `SimConfig` hat keine Teams.
- Szenarien kennen nur die Gruppen von Spieler 0 und 1; Karten haben zwei
  Startpositionen; der Editor setzt Starts ohne Team.
- `net::lobby`: Spieler = verbundene Menschen, `StartInfo.players` nur
  Menschen; die Sitzung kann schon Spieler hosten (024).
- Bridge: `SimHost.ai_opponent` = eine Stufe für alle anderen Spieler;
  `start_demo` startet im LAN/Steam immer die Schlacht ohne Wahl.
- 038: `Sim` wird komplett per serde gespeichert; neue Felder sind
  automatisch im Spielstand, `rules_hash` erkennt alte Stände.

### Entscheidungen

| Thema | Entscheidung | Grund |
|---|---|---|
| Teams im Sim | `SimConfig.teams: Teams { members: Vec<(PlayerId, u8)>, shared_vision }`, im `Sim` gespeichert (Spielstand) und im Zustands-Hash, aber nur wenn es Teams gibt | Alle Partien ohne Teams (Demo, Golden-Hash, LAN-Tests) bleiben bitgleich; kein unnötiger Golden-Hash-Wechsel, während parallel 033/034/041 am Sim arbeiten |
| Jeder in eigenem Team | `Teams::new` macht daraus „jeder gegen jeden“ (leer) | 1-gegen-1 aus der Lobby = die bisherige Partie, gleicher Hash |
| Verbündete | greifen sich nicht an (auch nicht per Befehl), schießen nicht durcheinander hindurch, unterdrücken sich nicht, sehen ihre Einheiten immer, bestreiten sich keine Versorgungspunkte; Bodenangriff und Splitter treffen weiter alle | wie eigene Einheiten; Splitter sind Physik, die KI meidet Beschuss nahe Verbündeter ohnehin (`friend_near`) |
| Geteilte Sicht | Option pro Partie, Standard an **[vorläufig]**; Nebel zeigt den besten Zustand des Teams | Teamspiel ist ohne gemeinsame Sicht zäh; als Option, weil manche Runden es ohne wollen |
| Sieg | Partie entschieden, wenn alle Unbesiegten verbündet sind; `Outcome.winner` = kleinster unbesiegter Spieler des Siegerteams, `Sim::winners()` = ganzes Team | `Outcome` bleibt unverändert (Bridge, Replays, Tests) |
| Kapitulation | übergibt nichts: Einheiten bleiben stehen und behalten ihren Feuermodus **[vorläufig]** | Prompt-Vorgabe; Einheiten zu entfernen wäre eine neue Regel |
| Steuern fremder Einheiten | unverändert verboten (`owned_indices`) | Prompt-Vorgabe |
| Ping | neuer `Command::Ping { target }` (am Ende des Enums), ohne Wirkung im Sim; die Bridge zeigt Pings von Verbündeten | Geht ohne eigenen Netzkanal durch Lockstep und Replay; Golden-Hash unberührt |
| Mehr Spieler im Szenario | `parse_scenario_for_slots`: Spieler ohne eigene Gruppen bekommen die Gruppen des Szenario-Spielers mit dem nächsten Start, auf ihren eigenen Start verschoben | Kein Szenario-Format pro Spielerzahl nötig; Demo und Basisbau funktionieren für 2–6 |
| Karten | `[[start]]` bekommt optional `team` (Lobby-Vorschlag, nicht im Hash); `cli gen-map --players 4/6`; neue 2v2-Karte `twin_meadow` (1 km, 4 Starts, 5 Versorgungspunkte); `test_large` bekommt 6 Starts und 3 weitere Versorgungspunkte (Raster unverändert) | Starts sind nicht im Karten-Hash; `test_small` bleibt unverändert (Golden-Hash) |
| Editor | Team pro Startplatz setzbar (`EditMap::set_start_team`, Taste im Startwerkzeug), gespeichert in `map.toml` | Prompt-Vorgabe, klein |
| Lobby | Plätze = Spieler-Ids (Platz *i* ist Spieler *i*): offen / Mensch / KI (Stufe) / geschlossen, Team und Fraktion pro Platz; Modus (Schlacht/Basisbau) und geteilte Sicht vom Host; Clients wählen Team und Fraktion des eigenen Platzes; Host setzt alles. Anzahl Plätze = Starts der Karte | Ein Modell für Einzelspieler, LAN und Steam; Platz = Spieler-Id macht Startposition und Farbe stabil |
| KI im Netz | Host hostet alle KI-Plätze (`LockstepSession::host_player`, wie in 024 vorbereitet); `StartInfo.players` enthält alle Spieler, `StartInfo.computers` die KI-Plätze mit Stufe | Andere Peers müssen die Spieler kennen, deren Frames sie erwarten |
| Protokoll | `PROTOCOL_VERSION` 3, Replay-Format 7 | `SimConfig` und `Command` haben sich geändert |
| KI | kennt Verbündete (Ansicht trennt eigene, verbündete, feindliche Einheiten), zieht zum nächsten feindlichen Start; „Helfen“ einfach: mit geteilter Sicht landen Gegner bei Verbündeten in ihrem Gedächtnis und werden Ziel. Endspiel Basisbau: am Einheitenlimit greift sie ohne Wellen an, ohne Ziel jagt sie zu feindlich gehaltenen Versorgungspunkten und feindlichen Starts statt zu Zufallspunkten | einfach halten (Prompt) |
| HUD | Farben: eigene grün-grau wie bisher, Verbündete blau, Gegner rot **[vorläufig]**; Minikarte gleich; Ping: Taste `G` + Linksklick bzw. Alt+Linksklick auf Minikarte/Gelände **[vorläufig]** | Drei Farben reichen für zwei Teams; Teamfarben pro Spieler wären bei 6 Spielern unübersichtlich |
| Turnier | `just ai-tournament --teams 2v2` (oder `3v3`): A spielt alle Plätze eines Teams | Prompt-Vorgabe |
| Tests | Sim: Teamsieg, geteilte Sicht, kein Beschuss unter Verbündeten, 2v2-Aufstellung; Netz: Lobby mit KI-Plätzen, 2v2 mit zwei Menschen + zwei KI; `just lan-selftest` spielt 2v2 (zwei Menschen, zwei KI) auf `twin_meadow` | Prompt-Akzeptanzkriterien |

### Nicht in diesem Prompt

Team-Chat, Ressourcen an Verbündete geben, Einheiten übergeben, Wiederbeitritt,
Zuschauer, mehr als 6 Spieler, Team-Startplätze automatisch tauschen.
