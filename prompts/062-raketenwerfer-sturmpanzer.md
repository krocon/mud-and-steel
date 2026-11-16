# Prompt 062 – Raketenwerfer und Sturmpanzer

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` („Indirect fire and smoke“, „Heavy strike“), `docs/SOURCES.md`,
> `docs/ASSETS.md` und `data/catalog/SCHEMA.md`.
>
> Entwurf: Die ausführende Session konkretisiert zuerst (Abschnitt „Offene Fragen“),
> trägt die Entscheidungen hier als Tabelle ein und setzt dann um.

## 1. Ziel

Zwei neue Waffengattungen als fahrbare Einheiten, je eine pro Fraktion:

- **Raketenwerfer:** schießt eine Salve ungenauer Raketen auf eine Fläche und lädt dann
  lange nach. Stark gegen Infanterie, Geschütze und Stellungen, schwach gegen Panzer und
  wehrlos im Nahkampf.
- **Sturmpanzer:** schwer gepanzertes Fahrzeug mit kurzem, großkalibrigem Geschütz, das
  direkt auf kurze Entfernung Gebäude, Bunker und Stellungen zerschlägt.

Die Idee stammt aus den Stil-Vorlagen (`docs/.vorlagen/frontline/`, nicht im Repo). Von
dort wird nur die Einheitenart übernommen: alle Werte kommen aus realen Quellen, alle
Modelle aus dem eigenen Baukasten (siehe „Festgelegte Grundsätze“ in `README.md`).

## 2. Ausgangslage

- Indirektes Feuer (032) gibt es für Granatwerfer, Haubitzen und Selbstfahrlafetten
  (`crates/sim/src/indirect.rs`, `data/indirect.toml`): Kaliber, Feuerrate, Mündungs-
  geschwindigkeit und Reichweite kommen aus der ersten Waffe des Katalogeintrags.
- Der schwere Feuerschlag (054, `data/strike.toml`) feuert bereits Raketen als gewöhnliche
  Granaten (`Sim::launch_shell`) in den Kalibern 15 cm, 132 mm und 114 mm, mit Quellen.
  Er ist ein Abruf aus dem Basisgebäude `base_rocket_battery`, keine Einheit.
- Als Gegenstücke zum Sturmpanzer stehen `su_isu152` und `us_m4a3_105` schon im Katalog
  und haben Modelle; ein deutscher Sturmpanzer fehlt.
- Die Einheiten-Webseite (`just units-web`) zeigt neue Katalogeinträge ohne Änderung an.

## 3. Umfang

1. **Katalog** (`data/catalog/de.toml`, `su.toml`, `us.toml`), jeder Wert mit Quelle,
   unsichere Werte in `uncertain`:
   - Raketenwerfer, Vorschlag: 15 cm Panzerwerfer 42 auf Sd.Kfz. 4/1 (`de`), BM-13-16
     „Katjuscha“ (`su`), T34 Calliope auf M4 (`us`).
   - Sturmpanzer `de`, Vorschlag: Sturmpanzer IV (15 cm StuH 43). Der Sturmtiger
     (38 cm RW 61) ist die Alternative, siehe „Offene Fragen“.
   - Neue Kategorie für Raketenwerfer (Vorschlag `rocket_launcher`) in
     `crates/sim/src/catalog.rs` und `SCHEMA.md`; Pflichtfelder wie bei Fahrzeugen.
     Neues Waffenfeld für die Zahl der Rohre bzw. Schienen nur, wenn `count` nicht reicht.
2. **Sim-Regel Salve** in `data/indirect.toml` und `crates/sim/src/indirect.rs`:
   - Ein Feuerbefehl verschießt alle Rohre nacheinander (Abstand wie `ripple_ms` in
     `data/strike.toml`), dann Nachladen als Ganzes.
   - Streuung deutlich größer als bei Haubitzen, als eigener Wert pro Kategorie.
   - Mindestreichweite, Aufbauzeit und Erhöhungswinkel wie bei den anderen Kategorien.
   - Jede Rakete ist eine Granate ihres Kalibers: Splitter, Krater, Brand und Umwerfen
     kommen aus den vorhandenen Regeln, keine neue Schadensregel.
   - Munition und Nachschub über die Regeln aus 036.
   - Alles mit `sim::Fixed` und `sim::Rng`, keine Floats, keine neue Zufallsquelle.
3. **Sturmpanzer:** nach Möglichkeit ohne neue Regel, nur über Katalogwerte (Kaliber,
   Reichweite, Feuerrate, Panzerung, `gun_traverse_deg`) und die Kategorie `assault_gun`.
   Prüfen, ob die Regeln aus 020 großen Kalibern schon genug Wirkung gegen Gebäude und
   Bunker geben; wenn nicht, die Regel dort anpassen und begründen.
4. **Wirtschaft:** Einträge in den Produktionslisten von `data/economy.toml`
   (Fahrzeugwerk). Preise aus der Kostenformel; prüfen, ob die Formel eine Salve aus
   vielen Rohren sinnvoll bepreist.
5. **Modelle:** Baupläne in `assets-src/blueprints/<fraktion>/`, gebaut mit `just unit`,
   geprüft mit `just validate-assets`. Fehlende Bauteile (Werferrahmen, Schienen,
   Rohrbündel) im Baukasten ergänzen. Budgets und Namen nach `docs/ASSETS.md`.
6. **Darstellung:** Raketenschweif (Rauchspur) und Abschussfeuer über die Effekte aus
   019/054, Abschussklang `shot_rocket` aus 029. Thumbnails mit `just thumbnails`.
7. **KI:** Normal und Hard bauen die Einheiten und setzen Raketenwerfer wie Artillerie
   ein (hinter der Front, Ziel sind Gruppen und Stellungen, nie nahe eigener Einheiten).
   Die KI gibt nur `Command`s.
8. **Doku:** `docs/ARCHITECTURE.md`, `docs/GAME_DESIGN.md`, `docs/AI.md`,
   `docs/CONTROLS.md` (falls ein Befehl dazukommt), `just catalog-doc`.

## 4. Akzeptanzkriterien

1. `just check` grün. Neue Tests: Salve verschießt genau die Rohrzahl, alle Einschläge
   liegen im Streukreis, danach Nachladezeit; zwei gleiche Läufe ergeben denselben Hash.
2. `just catalog-check` grün, `docs/UNIT_CATALOG.md` neu erzeugt.
3. `just validate-assets` und `just gallery-check` grün für die neuen Modelle.
4. `just selftest` grün; `just bench` hält das Budget (eine Salve sind viele Granaten
   gleichzeitig).
5. `just ai-tournament` läuft durch; die KI baut und nutzt die neuen Einheiten.
6. Der Golden-Hash ändert sich nur durch die neuen Regeln und Produktionslisten; der
   Grund steht im Commit.
7. Kommt ein neuer `Command` dazu: am Ende des Enums anhängen und die Protokollversion
   erhöhen.

## 5. Offene Fragen (vor dem Umsetzen klären)

| Frage | Vorschlag |
|---|---|
| Sturmpanzer IV oder Sturmtiger? | Sturmpanzer IV: rund 300 gebaut, passt zu ISU-152 und M4 (105). Der Sturmtiger (18 gebaut) wäre eine Einzelwaffe ohne Gegenstück. |
| Flugbombe oder Fernrakete (V1/V2) als zweite Art des schweren Schlags? | Nicht in diesem Prompt. Es gibt sie nur auf deutscher Seite; das braucht erst eine Entscheidung zur Symmetrie der Fraktionen. |
| Bleibt `base_rocket_battery` neben den fahrbaren Werfern? | Ja: der Schlag ist ein spätes, teures Ereignis mit Vorwarnung, die Einheit eine normale Waffe mit kleiner Salve. |
| Gezogener Nebelwerfer 41 zusätzlich? | Nein, erst die fahrbaren Werfer; ein gezogener Werfer wäre ein weiteres Geschütz für 034. |
| Sowjetisches Fahrgestell: ZiS-6 oder Studebaker US6? | ZiS-6 (BM-13-16 von 1941), weil der ZiS-5 schon als Modell existiert und Bauteile teilt. |
| Calliope: eigene Einheit oder Aufsatz auf `us_m4a3_75`? | Eigene Einheit mit eigenem Modell; „ein Modell pro Einheit“ bleibt gültig. |

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Entscheidungen (autonom, Nachtlauf 17.11.2026)

Vorläufig: der Nutzer hat geschlafen; die Vorschläge aus Abschnitt 5 sind übernommen,
der Rest ist von der ausführenden Session konkretisiert.

| Frage | Entscheidung |
|---|---|
| Sturmpanzer IV oder Sturmtiger? | Sturmpanzer IV `de_sturmpanzer4` (Sd.Kfz. 166, 15 cm StuH 43 L/12), Kategorie `assault_gun`. |
| V1/V2 als zweite Art des schweren Schlags? | Nicht in diesem Prompt. |
| Bleibt `base_rocket_battery`? | Ja, unverändert. |
| Gezogener Nebelwerfer 41? | Nein. |
| Sowjetisches Fahrgestell | ZiS-6 (`su_bm13`, BM-13-16 von 1941). |
| Calliope | Eigene Einheit `us_t34_calliope` mit eigenem Modell (M4A3-Wanne, T34-Gestell). Ohne 75-mm-Kanone im Katalog: laut Quelle konnte sie mit montiertem Werfer nicht feuern; Bug- und Turm-MG bleiben. |
| Deutscher Werfer | `de_pzwerfer42`: 15 cm Panzerwerfer 42 auf Sd.Kfz. 4/1 „Maultier“ (10 Rohre). |
| Rohrzahl | Das vorhandene Feld `count` der ersten Waffe; kein neues Waffenfeld. Der Nachschub (036) zählt schon jetzt eine Salve als `count` Schuss. |
| Feuerrate im Katalog | `rate_rpm` des Werfers ist die Dauerfeuerrate (Salve plus Nachladen, Raketen pro Minute); daraus folgt die Nachladezeit nach der Salve. |
| Salvenabstand | `ripple_ms = 500` (reale ms, Zeitfaktor gilt) als neuer Kategoriewert in `data/indirect.toml`; 0 = keine Salve (alle bisherigen Kategorien). |
| Streuung | Eigener Kategoriewert `mrad_x10 = 80` (8 mrad statt 3 mrad global), 0 = globaler Wert. |
| Mindestreichweite, Aufbau, Erhöhung | `rocket_launcher`: 20 % der Reichweite, 30 s Aufbau (real), 45°. |
| Munition | Eine Salve kostet einen „salvo“ des Vorrats (Regel 036), abgebucht mit der ersten Rakete; die übrigen Raketen der Salve brauchen keinen Vorrat mehr. |
| Unterbrochene Salve | Bewegt sich der Werfer mitten in der Salve, verfällt der Rest und er lädt ganz nach. |
| Kosten | Formel unverändert: der indirekte Waffenterm zählt schon nur ein Rohr (Kaliber² ohne `count`), also bepreist sie eine Salve wie einen Schuss des Kalibers; das passt zur Dauerfeuerrate (siehe GAME_DESIGN.md). |
| Sturmpanzer-Regel | Keine neue Regel: 15 cm unter 300 m/s zählt als Hohlladung (240 mm Durchschlag) und macht 2250 Schaden je Treffer, genug gegen Bunker (2500) und Häuser. |
| Produktion | `de_pzwerfer42`, `de_sturmpanzer4`, `su_bm13`, `us_t34_calliope` und das schon vorhandene `us_m4a3_105` (Gegenstück zum Sturmpanzer) kommen ins Fahrzeugwerk. |
| KI | Raketenwerfer haben auf Normal und Hard die Rolle Artillerie (bisher nur Hard für Granatwerfer und Selbstfahrlafetten). |
| Darstellung | Abschussfeuer über das vorhandene Mündungsfeuer (Socket_Muzzle am Gestell), Klang `shot_rocket`, Rauchspur einfach über die vorhandenen Rauch-Bursts von 019; die eigentliche Raketendarstellung baut 076. |
