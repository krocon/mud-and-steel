# Prompt 025 – LAN-Mehrspieler

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Abschnitte Lockstep, Replays, Transport),
> `docs/DETERMINISM.md` und den Code von `crates/net` (`LockstepSession`, `Transport`,
> `Replay`) und `crates/bridge` (`SimHost`).

## 1. Ziel

Zwei (später mehr) Rechner im lokalen Netz spielen dasselbe Szenario im
deterministischen Lockstep. Ein Spieler hostet, die anderen finden das Spiel per
UDP-Broadcast, treten bei, melden sich bereit, und alle starten gemeinsam mit
demselben Seed und derselben Konfiguration. Latenz, Verbindungsabbruch und Desync
werden sauber behandelt. Der Transport sitzt hinter dem vorhandenen `Transport`-Trait,
sodass Steam (Prompt 026) später dieselbe Lockstep-Schicht nutzt.

## 2. Ausgangslage

- `net::LockstepSession` sammelt `CommandFrame`s, verzögert lokale Eingaben um
  `input_delay` Ticks, tauscht alle `hash_interval` Ticks `HashReport`s aus und meldet
  den ersten `Desync`. Die Frames tragen die Spieler-Id; die Session braucht nur
  zuverlässige Zustellung in Reihenfolge **pro Absender**.
- `net::Transport` sendet an „alle anderen Peers“ und empfängt Bytes, nicht blockierend.
  Es gibt `LoopbackTransport` (Tests) und `LocalTransport` (Einzelspieler).
- `net::Replay` speichert den aufgelösten `SimConfig` (Karte nur per Name und
  Inhalts-Hash), Seed, Spieler und alle Befehle.
- `SimHost` (Bridge) spielt nur Einzelspieler mit `LocalTransport`; Spieler 1 des
  Demo-Szenarios ist ohne Eingabe. Fehler beenden das Spiel (`game = None`), und ein
  Befehl, der während eines Wartens auf Frames eingeht, ginge verloren (bei einem
  Spieler unsichtbar, im Netz ein Fehler).

## 3. Entscheidungen

1. **ENet in Rust mit dem Crate `rusty_enet` (0.4, reines Rust, MIT), nicht Godot-ENet
   über die Bridge.** Abwägung:
   - *Determinismus:* Egal welches ENet, der Transport liefert nur Bytes; die
     Reihenfolge der Befehle legt `LockstepSession` fest. Kein Unterschied.
   - *Testbarkeit:* Mit einem Rust-Transport läuft der Akzeptanztest als normaler
     `cargo test` (zwei CLI-Prozesse über echtes UDP) auf allen drei CI-Plattformen,
     ohne Godot. Godot-ENet ließe sich nur in laufenden Godot-Instanzen testen, und
     `crates/net` bliebe ohne echten Transport.
   - *Plattformen:* `rusty_enet` ist aus dem C-Original (ENet 1.3.18) nach Rust
     übertragen und nutzt `std::net::UdpSocket`; kein C-Compiler, kein `bindgen`,
     keine Installation außerhalb von Cargo, gleiches Verhalten auf macOS, Linux und
     Windows. Die C-Bindings (`enet`/`enet-sys`) bräuchten `libclang` zum Bauen.
   - *Wartung:* Eine Abhängigkeit mit kleiner, stabiler API (ENet ändert sich kaum).
     Godot-ENet würde beim gekoppelten Update Godot ↔ godot-rust mitwandern und den
     Netzcode an Godot-Typen binden.
   - *Steam später:* Steam-Networking kommt über GodotSteam (GDExtension). Ein
     `SteamTransport` in der Bridge implementiert dasselbe `Transport`-Trait; die
     Lockstep-Schicht und das Lobby-Protokoll (Start-Nachricht, Bereitschaft) bleiben.
     Das gilt unabhängig davon, wo ENet lebt.
   - *Risiko:* `rusty_enet` hat eine niedrige Versionsnummer. Gegenmittel: Alles
     ENet-Spezifische steckt in einem Modul `net::lan`; ein Wechsel (C-ENet,
     Godot-ENet) berührt nur dieses Modul.
2. **Stern-Topologie:** Der Host ist ENet-Server, Clients verbinden sich nur mit ihm.
   Der Host leitet Spiel-Pakete jedes Clients unverändert an alle anderen Clients
   weiter. Die Reihenfolge pro Absender bleibt erhalten, mehr braucht die Session
   nicht. Kein NAT-Traversal (LAN).
3. **Zwei ENet-Kanäle:** Kanal 0 für das Lobby-Protokoll (`Hello`, `Welcome`,
   `Reject`, `Lobby`, `Ready`, `Start`, `PeerLeft`), Kanal 1 für die undurchsichtigen
   Lockstep-Pakete des `Transport`. Beide zuverlässig und geordnet.
4. **Spielsuche:** Clients fragen per UDP-Broadcast (`255.255.255.255`, die
   Broadcast-Adresse jeder IPv4-Schnittstelle über das Crate `if-addrs`, und
   `127.0.0.1`) auf dem Discovery-Port; ein hostendes Spiel antwortet per Unicast mit
   Name, Spielport, Spielerzahl und Protokollversion. Frage-Antwort statt
   Host-Ankündigung, damit mehrere suchende Clients auf einem Rechner laufen können.
   Standard-Ports: Spiel **47810**, Suche **47811** (UDP), beide einstellbar.
   Beitritt per IP:Port geht immer, auch ohne Broadcast.
5. **Start:** Der Host schickt `Start { seed, config: SimConfig, players, input_delay,
   hash_interval }`. Die Clients übernehmen den **aufgelösten** `SimConfig` des Hosts
   (wie der Replay), laden die Karte lokal nach Name und prüfen den Inhalts-Hash
   (über `Sim::new`). Unterschiedliche Kataloge können so nicht auseinanderlaufen;
   eine fehlende oder andere Karte bricht den Start mit klarer Meldung ab. Der Host
   ist immer Spieler 0, Clients bekommen die nächsten freien Ids; Spieler des
   Szenarios ohne Menschen bleiben wie bisher ohne Eingabe. Die Lobby nimmt höchstens
   so viele Spieler auf, wie das Szenario hat (Demo: 2).
6. **Versionsprüfung:** `Hello` trägt `net::PROTOCOL_VERSION`; bei Abweichung lehnt der
   Host mit Grund ab. Abweichende Spielregeln fängt die Hash-Prüfung ab.
7. **Eingabeverzögerung dynamisch:** Die anfängliche Verzögerung ist für alle gleich
   (aus `Start`; sonst kollidieren die vorab leeren Frames). Danach wählt jeder Peer
   seine eigene Verzögerung aus der gemessenen Rundlaufzeit
   (`Transport::round_trip_ms`, bei ENet die RTT des Peers):
   `ceil((rtt + rtt_varianz) / 2 / tick_ms) + 1`, begrenzt auf 1..=12. Erhöhen sofort,
   Senken erst nach mehreren Messungen in Folge. `LockstepSession::set_input_delay`
   ändert nur die eigene Planung: Beim Erhöhen werden die Lücken mit leeren Frames
   gefüllt, beim Senken pausiert die Planung, bis die Simulation aufgeholt hat. Frames
   tragen ihren Tick, deshalb bleibt das deterministisch, und es braucht keine
   Protokolländerung.
8. **Verbindungsabbruch:** Verliert ein Peer die Verbindung (ENet-Timeout oder
   Trennen), liefert der Transport `TransportError::Disconnected`; der Host meldet den
   anderen Clients `PeerLeft`. `SimHost` hält das Spiel an (statt es zu verwerfen),
   zeigt den Grund an, und der Replay bleibt speicherbar. Kein Wiederverbinden in
   diesem Prompt.
9. **Desync:** Die Session meldet ihn wie bisher; `SimHost` hält an, die Anzeige
   nennt Tick und Spieler, und der Replay wird sofort als
   `user://replays/desync-<datum>-p<spieler>.msreplay` gespeichert. Jeder Peer schreibt
   seinen eigenen, `just replay` vergleicht sie. Neu: `LockstepSession::verified_tick`
   (letzter Tick, dessen Hashes alle Spieler bestätigt haben) für Anzeige und Tests.
10. **Minimale Lobby-UI** als eigene Szene `godot/scenes/lan/lan_lobby.tscn`
    (`just lan`): Name, Spiel hosten, LAN durchsuchen mit Liste, Beitreten (auch per
    IP), Spielerliste mit Bereit-Status, Bereit-Schalter, Start (Host, wenn alle bereit).
    Beim Start wechselt sie zu `main.tscn`; die Lobby-Verbindung wird über eine
    statische Variable übergeben. Die schöne Menü-UI kommt mit Prompt 027.
11. **Kleine, additive Änderungen an gemeinsamen Dateien** (parallel laufen 013 und
    015): neue Dateien `crates/net/src/lan/*`, `crates/bridge/src/lan.rs`,
    `godot/scenes/lan/*`; in `SimHost` nur der Transport als `Box<dyn Transport>`, das
    Anhalten statt Verwerfen, `start_lan` und Status-Abfragen; in `main.gd` nur der
    LAN-Start und ein Status-Overlay.

## 4. Umfang

1. `crates/net`:
   - `Transport`: `round_trip_ms()` mit Standard `None`, Implementierung für
     `Box<dyn Transport>`.
   - `LockstepSession::set_input_delay`, `input_delay`, `verified_tick`;
     `recommended_input_delay(rtt_ms, rtt_variance_ms, tick_hz)`.
   - Modul `lan`: `LanNode` (Host oder Client: Lobby-Phase, danach `Transport`),
     Lobby-Nachrichten, `StartInfo`, `DiscoveryResponder`, `DiscoveryBrowser`,
     `GameInfo`, `PROTOCOL_VERSION`, Standard-Ports.
   - Tests: Lobby und Lockstep zweier und dreier Peers über echtes UDP auf localhost
     (Threads), Spielsuche über localhost, Ablehnung bei falscher Version, Abbruch
     wird erkannt, Loopback-Test mit wechselnder Eingabeverzögerung bleibt synchron.
2. `crates/cli`: `lan-host` und `lan-join` (per Adresse oder Suche) spielen das
   Demo-Szenario headless, jeder Peer nur sein eigenes Befehlsskript, so schnell wie
   der Lockstep erlaubt; Ausgabe von Endhash und bestätigtem Tick, Exit-Code 1 bei
   Desync, optionaler Replay-Export, verstecktes `--cheat-tick` zum Provozieren eines
   Desyncs.
3. **Akzeptanztest** `crates/cli/tests/lan.rs`: startet zwei echte `cli`-Prozesse auf
   localhost, der Client findet den Host über die Spielsuche, beide spielen 600 Ticks.
   Beide Endhashes sind gleich und gleich dem Einzelprozess-Lauf `run_demo`; kein
   Desync. Zweiter Fall: `--cheat-tick` führt bei beiden zu Exit-Code 1 und einem
   geschriebenen Replay.
4. Bridge: `LanLobby` (RefCounted) für die Lobby-UI, `SimHost.start_lan`,
   `net_status()`, dynamische Eingabeverzögerung, Anhalten bei Abbruch und Desync,
   ausstehende Befehle gehen beim Warten nicht verloren.
5. Godot: Lobby-Szene, Status-Overlay im Spiel (Warten auf Spieler, Verzögerung, RTT,
   Abbruch, Desync mit Replay-Pfad), `just lan`, `just lan-selftest` (zwei headless
   Godot-Instanzen über localhost mit automatischer Lobby und Befehlen, Exit-Code 0
   nur ohne Desync und mit bestätigten Hashes).
6. Doku: `docs/ARCHITECTURE.md` (Transport nicht mehr „geplant“), `docs/LAN.md` mit
   Protokoll, Ports, Firewall und **Test mit zwei Rechnern**, `docs/DETERMINISM.md`
   (Durchsetzung: LAN-Test), `docs/VERSIONS.md` falls dort Crates gelistet sind.

## 5. Akzeptanzkriterien

1. `just check` und `just selftest` sind grün; Einzelspieler, `-- --demo` und der
   Golden-Hash bleiben unverändert.
2. `crates/cli/tests/lan.rs`: Zwei Prozesse spielen über localhost (echtes UDP, Suche
   per Broadcast-Anfrage) 600 Ticks ohne Desync; Endhash = `run_demo`. Ein
   provozierter Desync wird von beiden erkannt, Replays werden geschrieben.
3. `just lan-selftest`: zwei headless Godot-Instanzen über die Lobby, gemeinsamer
   Start, Befehle beider Spieler, kein Desync, Exit-Code 0.
4. `docs/LAN.md` beschreibt den Test mit zwei Rechnern (macOS ↔ Windows/Linux).
5. Betroffene Doku in `docs/` ist aktualisiert.

## 6. Nicht in diesem Prompt

Wiederverbinden, Beobachter, mehr als die Spieler des Szenarios, NAT-Traversal und
Internet-Spiel (026), KI-Spieler (024), schöne Menüs und Szenario-Auswahl (027),
Verschlüsselung. Der Plattform-übergreifende Test mit zwei Rechnern ist beschrieben,
aber nur der Nutzer kann ihn ausführen.

## 7. Arbeitsweise

Deutsch mit dem Nutzer, Englisch im Repo, kein Push ohne Freigabe, keine Tests
abschwächen, Commits pro Schritt mit `just check` grün. Neue Cargo-Crates sind erlaubt,
Installationen außerhalb von Cargo nur nach Rückfrage. Am Ende diese Datei mit
`git mv` nach `prompts/done/` verschieben und `prompts/README.md` auf „erledigt“ setzen.
