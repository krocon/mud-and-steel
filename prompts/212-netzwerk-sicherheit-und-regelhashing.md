# Prompt 212 – Netzwerk-Härtung (Host-Relay, Queue-Grenzen), `rules_hash` ohne UI-Ruckler und Steam-Drain

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/DETERMINISM.md`, `docs/LAN.md` und `docs/ONLINE.md`. Im Code:
> `crates/net/src/lobby/node.rs`,
> `crates/net/src/protocol.rs`,
> `crates/net/src/session.rs`,
> `crates/net/src/snapshot.rs`,
> `crates/steam/src/link.rs`,
> `crates/steam/src/lobby.rs`,
> `crates/bridge/src/lib.rs`.
> **Keine Regeländerung in `crates/sim`** – alle Golden Hashes bleiben unverändert.

## 1. Problem

In `crates/net` und `crates/steam` gibt es vier konkrete Robustheits- und UX-Schwächen:

1. **Fehlende Absender-Prüfung beim Host-Relay (`crates/net/src/lobby/node.rs:346-369`):**
   In `LobbyNode::on_game_packet()` prüft der Host nur, ob `from` ein akzeptierter Peer ist,
   und leitet `data` danach unverändert an alle anderen Peers weiter – **ohne zu prüfen, ob
   `Message::Frame(f).player` bzw. `Message::Hash(h).player` zur zugewiesenen `PlayerId` von
   `from` passt**. In Partien mit 3+ Spielern kann ein fehlerhafter oder manipulierter Client
   fremde `PlayerId`s fälschen (fremde Einheiten kommandieren oder per `DuplicateFrame` die
   Partie abbrechen).
2. **Unbegrenzte zukünftige Ticks & Hash-Reports (`crates/net/src/session.rs:377-405`):**
   - `insert_remote_hash()` prüft weder `report.tick.is_multiple_of(self.hash_interval)` noch
     `report.tick >= self.verified_tick`; ungültige Hash-Berichte sammeln sich unbegrenzt in
     `remote_hashes` an.
   - `insert_remote_frame()` begrenzt `frame.tick` nach oben nicht relativ zu `self.next_tick`
     (ein Peer kann `tick = u64::MAX` senden und Speicher aufblähen).
3. **Synchroner 400-Tick-UI-Ruckler durch `rules_hash()` (`crates/net/src/snapshot.rs:137-145`):**
   Beim ersten Öffnen von „Spiel laden“ / „Replays“ oder beim ersten Speichern führt
   `rules_hash()` synchron auf dem Godot-Main-Thread `sim::scenario::run_demo(7, 400)` plus
   volle Postcard-Serialisierung aus, was einen spürbaren UI-Hänger verursacht.
4. **Steam-`pump()` bricht bei Bursts nach 64 Paketen ab + fehlende Unit-Tests (`crates/steam`):**
   - `crates/steam/src/link.rs:66-81` holt pro Kanal höchstens `BATCH = 64` Nachrichten pro
     `pump()`-Aufruf ohne weiterzulesen, wenn der Batch voll ist (verzögert Pakete bei Bursts
     um zusätzliche Frames).
   - `crates/steam` hat bei 852 LoC nur 2 Unit-Tests und 0 Integrationstests.

## 2. Ziel

1. **Absender-Authentifizierung im Host-Relay (`crates/net/src/lobby/node.rs`):**
   - Beim Empfang eines Spielpakets von `from` die `assigned_player`-ID aus `peers[&from]`
     mit der `PlayerId` der dekodierten `net::protocol::Message` (`Frame`/`Hash`) abgleichen.
   - Stimmt die `PlayerId` nicht (oder ist das Paket nicht dekodierbar), das Paket verwerfen
     (nicht weiterleiten!) und sauber loggen bzw. den Regelverstoß als Fehler/Disconnect behandeln.
   - **Hinweis:** Prüfen, ob ein Host ggf. auch KI-Spieler vertritt – diese laufen ohnehin
     lokal auf dem Host, nicht über fremde Peers; im Test (inkl. Koop/KI-Slots) sicherstellen,
     dass KI-Frames des Hosts weiter funktionieren!
2. **Grenzen in `LockstepSession` (`crates/net/src/session.rs`):**
   - `insert_remote_hash`: Reports verwerfen, wenn `self.hash_interval == 0`,
     `!report.tick.is_multiple_of(self.hash_interval)` oder `report.tick < self.verified_tick`
     (sowie zu weit in der Zukunft liegende Ticks).
   - `insert_remote_frame`: Frames ablehnen/verwerfen, deren `tick` weiter voraus liegt als
     `self.next_tick + MAX_FUTURE_FRAME_TICKS` (z. B. `MAX_INPUT_DELAY + 60`).
   - Beide Schutzregeln mit dedizierten Unit-/Integrationstests in `crates/net/tests/` absichern
     (inkl. 3-Peer-Spoofing-Test!).
3. **`rules_hash()` ruckelfrei machen:**
   - Sauberste Lösung wählen und begründen:
     - **Option A:** `rules_hash()` beim Start der Godot-Bridge in einem Hintergrund-Thread
       vorwärmen (`std::thread::spawn` in `crates/bridge`, nicht in `crates/sim`!), sodass der
       `OnceLock` fertig ist, lange bevor der Spieler ein Menü öffnet (0 Protokoll-/Hash-Änderung!).
     - **Option B:** Struktureller Hash über die eingebetteten `BUILTIN_*`-Regelstrings +
       `FORMAT_VERSION` (sofort O(ms), ergänzt durch einen Test, der sicherstellt, dass
       Schema-Änderungen erkannt werden).
   - Option A ändert nicht einmal den aktuellen `rules_hash()`-Wert (alte Replays/Saves bleiben
     kompatibel!) – falls Option B gewählt wird, Kompatibilität in `docs/DETERMINISM.md` beachten.
4. **`crates/steam`:**
   - In `crates/steam/src/link.rs:66-81` `pump()` in einer Schleife ziehen, solange
     `batch.len() == BATCH` (mit Sicherheits-Obergrenze, z. B. max. 16 Batches pro Aufruf,
     damit ein Flood-Peer nicht endlos blockiert).
   - Reine Zustandsautomaten-/Parsing-Logik in `crates/steam` mit Unit-Tests abdecken (ohne
     laufenden Steam-Client nötig zu machen).

## 3. Prüfen und zeigen

- Neue Tests in `crates/net/tests/`:
  - 3-Peer-Lobby-Test: gefälschte `PlayerId` eines Clients wird vom Host geblockt und nicht
    an den dritten Peer weitergeleitet.
  - Out-of-interval `HashReport` und `tick = u64::MAX` `CommandFrame` wachsen `remote_hashes` /
    `remote_frames` nicht an.
- `just check` (inkl. `clippy -p bridge --no-default-features` und `clippy -p cli --features steam`),
  `just lan-selftest`, `just selftest` grün; Golden Hash unverändert.

## 4. Nicht Teil dieses Prompts

- Änderungen an Spielregeln in `crates/sim`.
- Verschlüsselung / Anti-Cheat über das Relay-Spoofing hinaus.

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/212-netzwerk-sicherheit-und-regelhashing.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.
