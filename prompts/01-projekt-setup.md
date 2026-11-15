# Prompt 01 – Projekt-Setup „Mud and Steel“

> Verwendung: Diesen Prompt in einer frischen Coding-Agent-Session (z. B. Claude Code)
> im Ordner `/Users/marckronberg/RustroverProjects/mud-and-steel` ausführen.

## 1. Kontext

**Mud and Steel** (Arbeitstitel) ist ein isometrisches Old-School-Echtzeit-Strategiespiel
im 2. Weltkrieg. Vorbilder: Sudden Strike, Blitzkrieg, Command & Conquer: Alarmstufe Rot 2,
Beyond All Reason.

- Plattformen: macOS, Linux, Windows (alle drei gleichwertig)
- Modi: Einzelspieler, Online-PvP, LAN-PvP, Online-Koop, LAN-Koop, plattformübergreifend
- Geplante Features: Basisbau, Ressourcenabbau, Nebel des Krieges, zerstörbare Gebäude
  und Vegetation, durch Explosionen verformbares Gelände
- Team: ein einzelner Entwickler (ich). Einfachheit schlägt Eleganz.
- Repo: `https://github.com/krocon/mud-and-steel.git`
- Lokal: `/Users/marckronberg/RustroverProjects/mud-and-steel`
  (bereits geklont, noch ohne Commits; `.idea/` von RustRover liegt schon dort)

## 2. Architektur (verbindlich)

Der Multiplayer läuft über **deterministisches Lockstep**: Alle Rechner simulieren
dasselbe Spiel und tauschen nur Befehle aus. Daraus folgen diese Regeln. Sie gelten
für das ganze Projekt, nicht nur für dieses Setup:

1. **Die Simulation ist die einzige Wahrheit.** Sie lebt in reinem Rust, ohne
   Godot-Abhängigkeit, und läuft mit festem Tick (Startwert: 20 Hz).
2. **Godot zeigt nur an.** Godot-Physik, `NavigationServer` und Godot-Zufall sind für
   alles Spielrelevante tabu, weil sie nicht deterministisch sind.
3. **Keine Gleitkommazahlen in der Simulation.** Windows-x86 und Apple Silicon müssen
   bitgleich rechnen. Also Festkomma-Arithmetik und ein eigener, geseedeter
   Zufallsgenerator. Umrechnung nach `f32` passiert ausschließlich in der Bridge.
4. **Keine versteckte Nichtdeterminismus-Quelle in der Simulation:** keine Systemzeit,
   keine Iteration über `HashMap`/`HashSet`, keine Threads mit unbestimmter Reihenfolge.
5. **KI ist ein normaler Spieler**, der Befehle in dieselbe Simulation einspeist.

Schichten:

```
Godot 4 (Darstellung: 3D-Szene, UI, Audio)
        ▲▼  Befehle rein, Snapshots raus
godot-rust Bridge (GDExtension, dünn)
        ▲▼
Rust-Workspace:  sim  |  net  |  cli
        ▲▼
Transport (später): Steam online, ENet im LAN
```

## 3. Ziel dieses Prompts

Setze das **Projektgerüst** auf und beweise mit einem minimalen Durchstich, dass alle
Schichten zusammenspielen:

- Eine Einheit steht auf einer Ebene. Rechtsklick setzt ein Ziel. Die Einheit fährt
  in gerader Linie dorthin. Die Bewegung wird in `sim` berechnet und in Godot nur
  dargestellt.
- Dieselbe Simulation läuft ohne Grafik in der `cli` und liefert einen
  reproduzierbaren State-Hash.

Mehr Gameplay gehört nicht in diesen Prompt (siehe Abschnitt 8).

## 4. Phase 0 – Bestandsaufnahme (erst prüfen, dann berichten, dann warten)

Bevor du etwas anlegst:

1. Prüfe, was installiert ist, jeweils mit Version und Pfad: `git`, `git lfs`,
   `rustc`/`cargo` (stable), Godot 4 (auch unter `/Applications` suchen), Blender.
2. Ermittle aus der **aktuellen Dokumentation** (nicht aus dem Gedächtnis):
   - die aktuelle Release-Version von godot-rust (Crate `godot` auf crates.io,
     Buch: https://godot-rust.github.io/book),
   - welche Godot-4-Versionen diese Version unterstützt,
   - ob GodotSteam (GDExtension) für diese Godot-Version verfügbar ist.
3. Wähle die neueste stabile Godot-Version, die godot-rust offiziell unterstützt.
   Weicht sie von der installierten Godot-Version ab, sag es mir.
4. Prüfe den Zustand des Repos (`git status`, Remote, vorhandene Dateien).
   Überschreibe nichts Vorhandenes. `prompts/` bleibt unverändert.

**Dann stoppen:** Zeig mir die Bestandsaufnahme, die gewählten Versionen und was
fehlt. Installiere keine Software ohne meine Zustimmung. Mach erst nach meinem „Go“
mit Phase 1 weiter.

## 5. Zielstruktur

```
mud-and-steel/
├─ Cargo.toml                 Workspace
├─ rust-toolchain.toml        stable, mit rustfmt und clippy
├─ crates/
│  ├─ sim/                    Spielregeln, deterministisch, ohne Godot
│  ├─ net/                    Lockstep-Protokoll, transportunabhängig
│  ├─ cli/                    Headless-Runner
│  └─ bridge/                 godot-rust, crate-type cdylib
├─ godot/                     Godot-Projekt (project.godot, *.gdextension,
│                             scenes/, ui/, assets/)
├─ assets-src/blender/        .blend-Quellen
├─ tools/                     Skripte, z. B. Blender-Export
├─ data/                      Einheitenwerte als TOML (vorerst ein Beispiel)
├─ docs/                      ARCHITECTURE.md, DETERMINISM.md, VERSIONS.md
├─ prompts/                   vorhanden, nicht anfassen
├─ .github/workflows/ci.yml
├─ .gitignore  .gitattributes  README.md  CLAUDE.md
```

## 6. Phasen 1–6

Arbeite die Phasen der Reihe nach ab. Nach jeder Phase: Tests laufen lassen, einen
Commit mit klarer Message anlegen, kurz berichten.

### Phase 1 – Repo-Grundlagen
- Cargo-Workspace, `rust-toolchain.toml`, `.gitignore` (u. a. `target/`, `.idea/`,
  `godot/.godot/`), `.gitattributes`.
- Git LFS für `*.blend`, `*.glb`, Texturen und Audio, sofern `git lfs` installiert ist.
- `README.md` (kurz: was, Stack, wie bauen und starten).
- Keine LICENSE-Datei anlegen. Frag mich, welche Lizenz ich will.

### Phase 2 – `crates/sim`
- Festkomma-Typ für Positionen und Strecken (Vorschlag: Crate `fixed`; wenn du etwas
  anderes wählst, begründe es in `docs/DETERMINISM.md`).
- Eigener kleiner, geseedeter Zufallsgenerator ohne OS-Entropie.
- Minimale API in dieser Art:
  - `Sim::new(config, seed)`
  - `sim.step(&[Command])` rechnet genau einen Tick
  - `sim.state_hash() -> u64` mit einem selbst implementierten, stabilen Hash
    (nicht `DefaultHasher`, dessen Ergebnis ist nicht garantiert stabil)
  - `sim.snapshot()` liefert die darstellbaren Daten (Einheiten-ID, Position, Ziel)
- `Command` als serialisierbares Enum, vorerst nur `MoveTo { unit, target }`.
- Lints im Crate: Gleitkomma-Arithmetik verbieten (`clippy::float_arithmetic`).
- Tests:
  - gleicher Seed + gleiche Befehle ⇒ gleicher Hash,
  - ein Golden-Hash-Test mit fest eingetragenem Erwartungswert. Dieser Test ist der
    plattformübergreifende Determinismus-Beweis in der CI.

### Phase 3 – `crates/net` und `crates/cli`
- `net`: Datentypen für Lockstep (Befehls-Frame pro Tick, Eingabeverzögerung,
  Hash-Austausch zur Desync-Erkennung) und ein `Transport`-Trait, das nur Bytes
  sendet und empfängt. Implementiere ausschließlich einen `LoopbackTransport` für
  Tests. Ein Test lässt zwei Sim-Instanzen über Loopback laufen und vergleicht die
  Hashes.
- `cli`: `cargo run -p cli -- run --ticks 1000 --seed 42` führt ein festes
  Befehlsskript aus und gibt den State-Hash aus.

### Phase 4 – `crates/bridge` und `godot/`
- `bridge` stellt Godot eine Node-Klasse bereit (z. B. `SimHost`), die
  - die Simulation mit Seed startet,
  - Befehle entgegennimmt,
  - pro Frame über einen Zeit-Akkumulator die fälligen festen Ticks rechnet,
  - den letzten und den vorletzten Snapshot zur Interpolation liefert.
- `godot/`: Projekt mit `.gdextension`-Datei für macOS, Linux und Windows
  (Debug und Release, Pfade auf `../target/...`).
- Hauptszene: orthografische `Camera3D` im Iso-Winkel, Bodenebene, eine
  Platzhalter-Einheit. Rechtsklick auf den Boden erzeugt einen `MoveTo`-Befehl.
  Die Einheit bewegt sich interpoliert zwischen den Sim-Ticks.
- UI-Logik in GDScript, kein C#.
- Prüfe den Start mit der Godot-Kommandozeile im Headless-Modus und werte die
  Ausgabe auf Fehler aus. Den Rechtsklick teste ich selbst. Beschreib mir, wie.

### Phase 5 – Blender-Pipeline
- Konvention dokumentieren: Quellen in `assets-src/blender/`, Export als `.glb`
  nach `godot/assets/models/`, 1 Einheit = 1 Meter, Transformationen angewendet.
- `tools/export_blender.py`: exportiert eine `.blend`-Datei headless
  (`blender -b … --python …`) nach glTF.
- Erzeuge per Skript ein einfaches Platzhalter-Modell aus Grundkörpern (Panzer:
  Wanne, Turm, Rohr), exportiere es und nutze es in der Hauptszene.
- Ist Blender nicht per Kommandozeile erreichbar, überspring den Export, nimm ein
  Godot-Primitive als Platzhalter und vermerke das im Abschlussbericht.

### Phase 6 – CI und Projektdoku
- `.github/workflows/ci.yml`: Matrix aus macOS (ARM), Ubuntu und Windows mit
  `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`,
  `cargo test --workspace`. Der Golden-Hash-Test muss auf allen drei Systemen
  denselben Wert bestätigen.
- `docs/ARCHITECTURE.md`: Schichten, Datenfluss, die Regeln aus Abschnitt 2 und der
  geplante Transport (Steam für Online über GodotSteam, ENet für LAN mit
  UDP-Broadcast zur Spielsuche). Nur beschreiben, nicht bauen.
- `docs/DETERMINISM.md`: was in `sim` erlaubt und verboten ist, und warum.
- `docs/VERSIONS.md`: festgelegte Versionen von Rust, Godot, godot-rust und Blender.
  Godot und godot-rust werden nur gemeinsam aktualisiert.
- `CLAUDE.md`: kurze Arbeitsanleitung für künftige Sessions mit den Regeln aus
  Abschnitt 2, den wichtigsten Befehlen und der Ordnerstruktur.

## 7. Akzeptanzkriterien

Das Setup ist fertig, wenn alles Folgende stimmt. Führ die Befehle wirklich aus und
zeig mir die Ausgabe:

1. `cargo fmt --check` und `cargo clippy --workspace -- -D warnings` sind sauber.
2. `cargo test --workspace` ist grün, inklusive Golden-Hash- und Loopback-Test.
3. `cargo run -p cli -- run --ticks 1000 --seed 42` liefert bei zwei Läufen
   denselben Hash.
4. `cargo build -p bridge` baut, und das Godot-Projekt startet ohne Fehler in
   der Ausgabe.
5. Die Dokumente aus Phase 6 existieren und stimmen mit dem Code überein.
6. `git status` ist sauber, pro Phase gibt es mindestens einen Commit.

## 8. Nicht-Ziele

Das kommt in späteren Prompts und wird hier weder gebaut noch vorbereitet:

- Basisbau, Ressourcen, Kampf, Nebel des Krieges, Wegfindung über gerade Linien hinaus
- Verformbares Gelände und Zerstörung
- Echter Netzwerk-Transport (ENet, Steam) und Lobbys
- Steamworks-Integration (braucht eine App-ID)
- Endgültige Grafik, Sound, Menüs, Karteneditor
- Export-Vorlagen, Signierung, Notarisierung

## 9. Arbeitsweise

- Sprich mit mir auf Deutsch. Code, Kommentare, Commit-Messages und Doku im Repo
  sind auf Englisch.
- Frag nach, bevor du Software installierst, etwas löschst oder eine Entscheidung
  triffst, die sich schwer zurücknehmen lässt.
- **Kein `git push` ohne meine ausdrückliche Freigabe.**
- Wenn etwas nicht funktioniert, sag es offen. Kein Test wird abgeschwächt oder
  deaktiviert, damit er grün wird.
- Halte dich an die gewählten Versionen aus Phase 0 und rate keine API-Namen.
  Schau in der Doku der festgelegten godot-rust-Version nach.

## 10. Abschlussbericht

Am Ende bitte kurz:

- was angelegt wurde (Struktur, Crates, Szenen),
- die festgelegten Versionen,
- wie ich baue, teste und das Spiel starte,
- die Ausgabe der Akzeptanzprüfungen,
- was offen ist oder übersprungen wurde,
- ein Vorschlag für den nächsten Prompt.
