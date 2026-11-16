# Prompt 050 – Erfahrung und Ränge

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/GAME_DESIGN.md`, `docs/INFANTRY.md`, `docs/ARCHITECTURE.md` (Kampf).
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Einheiten, die überleben und kämpfen, werden besser: Erfahrung in Stufen mit moderaten
Boni und sichtbaren Zeichen, damit man erfahrene Männer schützen will.

## 2. Umfang

1. **Sim:** Erfahrungspunkte für Treffer, Abschüsse, überstandenes Feuer; Stufen
   (z. B. Rekrut, erfahren, Veteran, Elite) mit kleinen Boni (Zielgenauigkeit,
   Nachladen, Moral/Niederhalten). Regeln mit Begründung, Balance in 045.
2. **Darstellung** im Stil der Zeit: Rangabzeichen in der Einheitenanzeige, an Soldaten
   kleine Unterschiede (Ausrüstung, Tarnjacke, erbeutete Waffe), an Fahrzeugen
   Abschussringe am Rohr. Keine modernen Zusätze (Visiere, Laser, Schutzwesten).
3. **HUD:** Stufe und Fortschritt in der Einheitenanzeige (031).
4. **KI** zieht Veteranen bei Verlusten eher zurück.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für Aufstieg und Boni; Golden-Hash begründet.
2. `just selftest`, `just ai-tournament` grün; `GAME_DESIGN.md` ergänzt.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Konkretisierung (Ausgangslage und Entscheidungen)

Der Nutzer war nicht erreichbar; alle Entscheidungen wurden nach dem Prompt getroffen
und in den Docs als **[provisional]** markiert.

Ausgangslage im Code (nach 017, 022, 031, 036, 038, 042): Kampf über Geschosse mit
Streuung (`dispersion.mrad_x10`, Haltung, Fahrt) und Nachladezeit pro Waffe; Treffer-
und Zerstört-Ereignisse (`CombatEvent::Hit`/`Destroyed`) mit Schütze; Trupps werden
durch Nahtreffer niedergehalten (`suppressed`, gesetzt in `near_miss`, Granat- und
Luftangriffen). Einheitenanzeige (031) mit Porträt, Balken, Kacheln; Soldatenoptik
über `SoldierLook` mit Tarnkleidung pro Rolle (042); Fahrzeugmodelle haben einen
`Gun`-Knoten mit `Socket_Muzzle`, die nach dem Zusammenfassen (016) erhalten bleiben.
038 speichert den ganzen `Sim` per serde. KI zieht auf Hard beschädigte Einheiten
unter 30 % zurück. Es gibt keinen Ersatz für gefallene Soldaten.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Erfahrung gehört der Einheit (Fahrzeug, Geschütz, ganzer Trupp), nicht dem einzelnen Soldaten | Einheiten sind die Sim-Einheit; Soldaten haben keine eigene Identität für den Spieler |
| 2 | Punkte: 100 für den ganzen Schaden an einer feindlichen Einheit, nach Schaden aufgeteilt (nur echter Verlust, kein Überschuss); +25 für den zerstörenden Treffer; +2 für einen überlebten Schadens-Tick; +1 je Spielsekunde niedergehalten | Typunabhängig vergleichbar (Trupp gegen Trupp = Panzer gegen Panzer); deckt „Treffer, Abschüsse, überstandenes Feuer“ aus dem Prompt ab |
| 3 | Intern Hundertstelpunkte | MG-Treffer bringen <1 Punkt; Rundung würde sie verschlucken |
| 4 | Vier Stufen: Rekrut 0, Erfahren 100, Veteran 300, Elite 700 | Erster Panzerabschuss = Erfahren; Veteran nach etwa drei, Elite nach etwa sechs zerstörten Einheiten |
| 5 | Boni: Streuung 92/84/76 %, Nachladen 96/92/88 %, Niederhalten 85/70/55 % | „Moderat“: Panzerung, Durchschlag, Reichweite, Trefferpunkte bleiben gleich; Prüfung in `data/experience.toml` begrenzt Boni auf höchstens Halbierung |
| 6 | Auswertung einmal pro Tick aus den Kampfereignissen nach Wirtschaft/Nachschub | Ein Ort für alle Quellen (direkt, Granaten, Brände, Transporte); kleine additive Änderung in `sim.rs` |
| 7 | Minen und Flugzeuge geben keine Punkte; Splitterschaden nur über den Abschuss | Kein Schütze als Einheit; Splitterschaden steht nicht pro Ziel im Ereignis |
| 8 | Regeln in `data/experience.toml`, eingebaut und gehasht wie `logistics.toml` (nicht in `SimConfig`) | Gleiches Muster wie 034/036; keine Änderung des Konfigurationsformats |
| 9 | Golden-Hash ändert sich (Regeln + Punkte im Hash, Demo-Schlacht läuft anders) | Absichtliche Regeländerung, im Commit begründet |
| 10 | Rangabzeichen im Stil der Zeit, nur gezeichnet: deutsche Ärmelwinkel (Spitze unten, Elite mit Stern-Punkt), US-Winkel (Spitze oben), sowjetische Schulterstücke mit roten Querstreifen | Fraktionstypisch, ohne Bilddateien, ohne verbotene Symbole; Zuordnung zu Spielrängen, nicht zu echten Dienstgraden |
| 11 | Soldaten: ab Erfahren ausgeblichene Uniform; deutsche und sowjetische Veteranen tragen Tarnkleidung (Veteran jeder zweite, Elite alle, nur Waldbiom); US-Veteranen nur abgetragen | Erbeutete/organisierte Tarnjacken sind zeittypisch; US-Frogskin wurde 1944 in Europa zurückgezogen |
| 12 | Erbeutete Waffen nicht umgesetzt | Braucht Waffenmodelle pro Soldat und Sim-Werte; offen |
| 13 | Fahrzeuge: weißer Ring am Rohr je zerstörtem feindlichem Fahrzeug/Geschütz (max. 12) | Zeittypische Abschussmarkierung; Infanterie und Gebäude zählen nicht als Ring |
| 14 | HUD: Abzeichen vor dem Namen, Rang und Abschüsse im Titel, schmaler Erfahrungsbalken neben Munition/Treibstoff (Tooltip mit Punkten und Boni), Abzeichen in den Kacheln; Beförderungen im Ereignis-Feed | Panel behält seine Höhe |
| 15 | KI: Normal zieht Veteranen/Elite unter 40 % zurück (Rekruten nie), Hard unter 50 % (sonst 30 %) | „Zieht Veteranen bei Verlusten eher zurück“, ohne die übrige KI zu ändern |
| 16 | Keine modernen Zusätze (Vollpanzerung, Visiere, Lampen, Laser) aus der Wunschliste | Gegen die Zeitvorgabe des Projekts |
| 17 | Debug-Funktion `debug_set_experience` nur ohne Netzwerk und nach dem Replay-Selbsttest | Kein Befehl; würde Replays verändern |
