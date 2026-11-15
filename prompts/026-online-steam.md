# Prompt 026 – Online über Steam

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md`, `docs/LAN.md`, `docs/VERSIONS.md` und den Code von
> `crates/net` (`transport.rs`, `session.rs`, `lan/`), `crates/bridge/src/lan.rs`,
> `godot/scenes/lan/` und `godot/scenes/menu/`.

## 1. Ziel

Zwei (später mehr) Spieler mit Steam-Konten spielen eine Partie über das Internet:
Lobby erstellen, Freunde einladen oder eine offene Lobby aus einer Liste wählen,
bereit melden, gemeinsam starten. Steam übernimmt Vermittlung und NAT-Traversal
(Steam Datagram Relay). Der Lockstep-Kern bleibt unverändert: Steam ist nur ein
weiterer `net::Transport`.

Steam ist **optional**: Ohne laufenden Steam-Client (oder in einem Build ohne Steam)
funktionieren Einzelspieler, LAN, Editor und alle Selbsttests unverändert; der
Menüpunkt „Online (Steam)“ zeigt dann nur, warum Steam nicht verfügbar ist.

## 2. Entscheidungen

1. **steamworks-rs in Rust, nicht GodotSteam.** Wie bei LAN (ENet in Rust statt
   Godot-ENet) gewinnt die Rust-Seite:
   - *Testbar:* Lobby-Protokoll und Transport laufen in `cargo test` gegen eine
     Mock-Verbindung; mit GodotSteam läge die Netzwerklogik in GDScript und die
     Pakete müssten jeden Frame zwischen GDScript und `SimHost` hin- und hergereicht
     werden.
   - *Keine Binärdateien im Repo:* Das Crate `steamworks` 0.13.1 (MIT/Apache-2.0,
     Mai 2026, Steamworks SDK 1.64) bringt über `steamworks-sys` die
     Redistributables (`libsteam_api.dylib`/`.so`, `steam_api64.dll`, je ca.
     0,3–0,4 MB) aus crates.io mit, per `Cargo.lock` mit Prüfsumme festgelegt.
     GodotSteam (4.24, SDK 1.65) müsste als Addon mit Bibliotheken pro Plattform ins
     Repo oder per Skript geladen werden; sein GitHub-Repo ist archiviert (Umzug nach
     Codeberg).
   - *Ein GDExtension weniger:* kein zweites Paar, das mit Godot zusammen
     aktualisiert werden muss (`docs/VERSIONS.md`).
   - *Headless testbar:* CLI-Peers über Steam wie `just lan-host`/`lan-join`.
   - *Nachteil:* Die Bridge linkt `steam_api` dynamisch; die Bibliothek muss neben
     `libbridge` liegen. Deshalb Cargo-Feature und Kopierschritt (Punkt 4).
2. **Steam Networking Messages** (`ISteamNetworkingMessages`), nicht Sockets:
   verbindungslos an eine Steam-ID adressiert, zuverlässig und geordnet, mit
   Kanälen wie ENet, Relay und NAT-Traversal eingebaut. Kanal 0 Lobby-Nachrichten,
   Kanal 1 Lockstep-Pakete, wie im LAN.
3. **Gleiches Lobby-Protokoll wie im LAN.** Die Logik von `LanNode` (Hello, Welcome,
   Reject, Lobby, Ready, Start, PeerLeft, Stern mit Weiterleitung über den Host)
   wird generisch über eine schmale `Link`-Schnittstelle (Peers, Kanäle, Ereignisse
   Verbunden/Getrennt/Empfangen, Senden, Rundlaufzeit). `LanNode` wird zu
   `LobbyNode<EnetLink>` mit unveränderter öffentlicher API, Steam bekommt
   `LobbyNode<SteamLink>`, Tests `LobbyNode<MemoryLink>`. Die Steam-Lobby
   (`ISteamMatchmaking`) dient nur zum Finden, Einladen und als Mitgliederliste; der
   Host akzeptiert Sitzungen nur von Mitgliedern seiner Lobby. Der Host ist Spieler 0
   und Besitzer der Steam-Lobby.
4. **Crate `crates/steam`** kapselt alles Steam-Spezifische (`steamworks`-Abhängigkeit,
   `SteamLink`, Lobby-Verwaltung, Einladungen, Rich Presence). `net` bleibt ohne
   Steam. Die Bridge bindet es über das Cargo-Feature `steam` ein (Standard: an);
   `cargo build -p bridge --no-default-features` baut ohne Steam. `just build`
   kopiert die Steam-Bibliothek neben die Bridge; unter Linux bekommt die Bridge
   `rpath=$ORIGIN`. Die Godot-Klasse `SteamLobby` existiert immer und meldet ohne
   Feature „built without Steam“.
5. **App-ID an genau einer Stelle:** `steam::APP_ID` in `crates/steam/src/lib.rs`,
   vorerst 480 (Valves öffentliche Test-App „Spacewar“). Die Initialisierung setzt
   die App-ID selbst; es gibt keine `steam_appid.txt`. Weil auf 480 alle
   Steam-Entwickler Lobbys anlegen, filtert die Lobbyliste nach Lobby-Daten
   `game = "mud-and-steel"` und der Protokollversion.
6. **Steam wird erst beim Öffnen von „Online (Steam)“ initialisiert**, nicht beim
   Spielstart: Einzelspieler, LAN und Selbsttests zeigen den Nutzer nicht als „spielt
   Spacewar“ und hängen nicht vom Client ab. Folge: Das Steam-Overlay ist im
   Entwicklungsbetrieb nicht verfügbar (das hängt Steam ohnehin nur in von Steam
   gestartete Spiele ein). Einladen geht deshalb dreifach: Overlay-Dialog (wenn
   verfügbar), Lobby-Code zum Kopieren und „Spiel beitreten“ in der Steam-Freundesliste
   (Rich Presence `connect`), die das laufende Spiel per Callback erreicht. Die
   Startoption `+connect_lobby <id>` wird für den späteren Start über Steam
   ausgewertet.
7. **Rundlaufzeit:** Ping aus `GetSessionConnectionInfo`; Steam liefert keine
   Varianz, angenommen wird ein Viertel des Pings. Die dynamische Eingabeverzögerung
   aus dem LAN gilt unverändert.

## 3. Umfang

1. `crates/net`: generischer `LobbyNode<L: Link>` mit `LobbyState`; `LanNode` und
   `LanState` als Aliase, öffentliche API und Verhalten unverändert. `MemoryLink`
   (In-Process-Hub) für Tests.
2. `crates/steam`: Initialisierung (einmal pro Prozess, Fehlertext statt Absturz,
   erneuter Versuch möglich), Lobby erstellen (Freunde/öffentlich) und beitreten,
   Lobbyliste mit Filter, Einladungen (Overlay, Freundesliste, Lobby-Code), Rich
   Presence, `SteamLink` über Networking Messages, Callbacks pro Frame.
3. Bridge: Godot-Klasse `SteamLobby` (wie `LanLobby`), `SimHost.start_online`
   teilt sich den Start mit `start_lan`.
4. Godot: Hauptmenü-Punkt „Online (Steam)“, Szene `scenes/online/steam_lobby.tscn`
   im Menü-Look: Status, Name aus Steam, Karte (Host), Lobby erstellen, Liste
   aktualisieren und beitreten, Lobby-Code beitreten, Freunde einladen, Spielerliste,
   Bereit, Start, Verlassen, Zurück. Statuszeile im Spiel und Ende der Partie wie im
   LAN.
5. CLI (Feature `steam`): `just steam-host` und `just steam-join <lobby>` spielen die
   Demo headless über Steam und geben Hash und bestätigte Ticks aus.
6. CI: zusätzlicher Clippy-Lauf ohne Steam-Feature der Bridge, damit beide Varianten
   bauen. Der Steam-Client wird nirgends gebraucht.
7. Doku: neues `docs/ONLINE.md` (Entscheidungen, Protokoll, Testanleitung für zwei
   Konten), Verweise in `ARCHITECTURE.md`, `LAN.md`, `VERSIONS.md`.

Nicht im Umfang: Wiederverbinden nach Abbruch, Zuschauer, Ranglisten/Matchmaking
nach Spielstärke, KI-Plätze in der Lobby, Steam-Export-Pipeline.

## 4. Akzeptanzkriterien

1. `just check` grün auf macOS, Linux und Windows, ohne installierten Steam-Client.
   Golden-Hash unverändert.
2. Tests ohne Steam: LAN-Tests unverändert grün; neue Tests spielen über
   `MemoryLink` zwei und drei Peers durch die Lobby in eine Partie, deren End-Hash
   dem von `run_demo` entspricht; Ablehnung (voll, falsche Version, gestartet),
   verlorener Peer und Weiterleitung über den Host sind abgedeckt; die
   Start-Nachricht passt in eine Steam-Nachricht (512 KB).
3. `just selftest` (mit Flow-Test: der Online-Menüpunkt öffnet sich und „Zurück“
   funktioniert, ohne Steam zu initialisieren), `just lan-selftest`,
   `just editor-selftest`, `just gallery-check` und der Demo-Lauf sind grün.
4. Ohne laufenden Steam-Client zeigt „Online (Steam)“ einen verständlichen Hinweis;
   das Spiel stürzt nicht ab. Mit `--no-default-features` gebaut ebenso.
5. Die App-ID steht an genau einer Stelle (`steam::APP_ID = 480`).
6. Testanleitung für den Nutzer in `docs/ONLINE.md`: zwei Rechner mit zwei
   Steam-Konten (oder ein Rechner plus ein zweiter), erst headless (`just steam-host`
   / `steam-join`), dann im Spiel; Erwartung: gleicher Hash, keine Desyncs,
   Rundlaufzeit in der Statuszeile, Abbruch wird gemeldet. Plattformübergreifend
   (macOS/Linux/Windows) in derselben Lobby.

## 5. Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne
Freigabe, keine Tests abschwächen, Commits pro Schritt. Downloads (Crates mit
Binärdateien, SDKs) und Binärdateien im Repo vorher mit Quelle, Größe und Lizenz
erfragen. Parallel laufende Prompts (016, 021, 023): Änderungen an gemeinsamen
Dateien (`crates/bridge/src/lib.rs`, `main.gd`, `main_menu.gd`, `justfile`, CI)
klein und additiv halten. Am Ende den lokalen `main` übernehmen, diese Datei nach
`prompts/done/` verschieben und `prompts/README.md` aktualisieren.
