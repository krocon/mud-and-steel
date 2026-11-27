# Prompt 236 – Kampffeld-Lesbarkeit: Beförderungs-Pop-up, Gebäude-Besatzung und Welt-Feedback

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/CONTROLS.md`, `docs/ARCHITECTURE.md` („HUD“, „Experience“, „Base
> building“), `docs/PERFORMANCE.md`.
> Im Code:
> `godot/scenes/world/veterancy_view.gd`,
> `godot/scenes/world/combat_view.gd` (`HealthBars`),
> `godot/scenes/hud/state_marks.gd`,
> `godot/scenes/world/base_hud.gd`,
> `godot/scenes/world/supply_view.gd`,
> `godot/scenes/selection/selection.gd`,
> `godot/scenes/hud/order_feedback.gd`,
> `godot/scenes/hud/relation_colors.gd`,
> `godot/scenes/hud/hud_theme.gd`,
> `godot/scenes/tools/trailer_shots.gd`.
>
> **Stil-/Spielgefühl-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4` (Sek. `00:24–00:29` und
> `02:50`/`03:10` „PROMOTED“-Einblendung direkt über beförderten Einheiten;
> Sek. `01:46` Ablehnungsgrund direkt am roten Bau-Geist; Sek. `03:04–03:56`
> klare Besatzungs-Anzeige direkt auf Hausdächern inkl. Rückmeldung bei bereits
> besetztem Gebäude; Sek. `04:09` sofort lesbare „SUPPRESSED“-Markierung über
> festgenagelten Trupps; Sek. `01:05` Vorrats-Anzeige an Nachschubpunkten).
> **Keine Bilder, Namen oder Assets aus dem Video ins Repo.**
> **Wichtig:** Reine Darstellung – **keine Änderungen in `crates/sim`**,
> Golden Hashes bleiben unverändert!

## 1. Problem

Viele wichtige Gefechts- und Bau-Informationen existieren bei uns bereits in
der Simulation, gehen aber im laufenden Gefecht leicht unter, weil sie nur im
Event-Feed oder in kleinen Symbolen erscheinen:

1. **Beförderungen sind kaum spürbar (`veterancy_view.gd`):** Steigt ein Panzer
   oder Trupp im Rang auf, erscheint lediglich eine Zeile im Event-Feed – auf
   dem Schlachtfeld selbst sieht man den Moment gar nicht.
2. **Besetzte Gebäude & volle Häuser (`combat_view.gd`, `selection.gd`):**
   Aus Spielhöhe ist nicht auf einen Blick klar, *wie stark* ein besetztes
   Gebäude bemannt ist (Fraktion, Soldaten-Balken, Kapazität), und beim
   Anklicken eines bereits vollen Gebäudes fehlt ein klares, unmittelbares
   Feedback am Haus/Cursor.
3. **Niedergehaltene Trupps (`state_marks.gd`):** Der Unterschied zwischen
   „unter Beschuss“ und „komplett niedergehalten / festgenagelt“ ist im Gefecht
   taktisch entscheidend (wann kann man flankieren oder stürmen?) – ein
   einzelnes 15-px-Icon über dem Balken geht in Rauch und Gefecht leicht unter.
4. **Ablehnungsgrund beim Basisbau (`base_hud.gd`):** Ist ein Bauplatz
   ungültig (roter Geist), steht der Grund aktuell nur oben in der Hinweiszeile
   (`_hint`), weit weg vom Mauszeiger.
5. **Nachschubpunkte:** Der Restvorrat oder Ertrag eines Nachschubpunkts ist
   erst sichtbar, wenn man genau sucht, statt dezent beim Überfahren (oder auf
   Wunsch) direkt über dem Depot/Punkt lesbar zu sein.

## 2. Ziel

Fünf kompakte, klar lesbare Welt-Feedback-Verbesserungen (reine Darstellung,
0 Regeländerungen in `crates/sim`, minimaler Draw-Aufwand):

1. **Welt-Pop-up bei Beförderung (`veterancy_view.gd` + HUD-Overlay):**
   - Wird eine eigene Einheit befördert, steigt direkt über ihrer Weltposition
     ein kurzer, gut lesbarer Text auf (z. B. `„BEFÖRDERT“` / `„PROMOTED“`
     samt neuem Rang-Symbol/Sternen, 1,8–2,2 s Lebensdauer, sanftes Aufsteigen
     und Ausblenden).
   - Im HUD-aus-Modus (`F1`), Fotomodus und bei `trailer_shots.gd`
     (`hud = false`) strikt ausgeblendet!
2. **Klare Gebäude-Besatzungs-Plakette & Voll-Rückmeldung (`combat_view.gd`, `selection.gd`):**
   - Über jedem besetzten Gebäude (eigene, verbündete und aufgeklärte
     feindliche Besatzungen, Nebel-des-Krieges-treu!) erscheint eine kompakte
     Plakette direkt über dem Dach:
     - Fraktionsfarbe aus `relation_colors.gd`,
     - kurzes Haus-Icon + Besatzungszahl (`6/12`) bzw. kompakter
       Lebensbalken der Besatzung (und ggf. Rang-Sterne).
   - Klickt der Spieler mit Infanterie auf ein bereits volles oder vom Gegner
     blockiertes Gebäude, erscheint eine knappe, sofort lesbare Rückmeldung
     (am Cursor/Gebäude + im Event-Feed, z. B. `„Gebäude bereits voll“` /
     `„Gebäude feindlich besetzt“`).
3. **Deutliche „Niedergehalten“-Kennzeichnung (`state_marks.gd`):**
   - Für wirklich **niedergehaltene** Trupps (eigene und aufgeklärte
     feindliche!) zusätzlich bzw. hervorgehoben eine kompakte, kontrastreiche
     Text-Kapsel (`„NIEDERGEHALTEN“` / `„PINNED“` in warmem Rot/Amber auf
     dunklem Hintergrund, nur im Nah-/Mittelbereich bis `FAR_M`, bei weiter
     Kamera nur das Icon) – damit Feuerüberlegenheit im Gefecht sofort erkennbar
     ist.
   - Respektiert die bestehende Einstellung `state_marks` (`all` / `selected` /
     `never`).
4. **Bau-Ablehnungsgrund direkt am Bau-Geist (`base_hud.gd`):**
   - Ist `placement_reason` nicht leer, wird der kurze, lokalisierte Grund
     (`Picker.blocked_text(placement_reason)`, z. B. „Abstand zu Gebäuden
     lassen“, „Zu steil“, „Zu wenig Ressourcen“) direkt als kompaktes
     2D-Overlay-Schild neben dem roten Bau-Geist am Mauszeiger angezeigt
     (zusätzlich zum bestehenden `_hint`, sodass der Blick auf der Karte
     bleiben kann).
5. **Dezent lesbarer Vorrats-/Status-Hinweis an Nachschubpunkten (`supply_view.gd`):**
   - Beim Überfahren eines Nachschubpunkts (oder bei gehaltener `Tab`-Taste
     „Alle zeigen“) zeigt ein kleines, klares Welt-Label den Status/Ertrag bzw.
     Vorrat des Punktes direkt über dem Marker an (keine dauerhafte
     Textflut auf der Karte).

## 3. Prüfen und zeigen

- **Sichtprüfung (Screenshots in `build/battlefield-feedback/`):**
  - Beförderungs-Pop-up direkt über einer Einheit im Gefecht.
  - Eigenes und feindliches besetztes Gebäude mit Dach-Plakette + Rückmeldung
    bei vollem Gebäude.
  - Niedergehaltener eigener und feindlicher Trupp (nah und fern).
  - Ungültiger Bauplatz mit rotem Geist + Ablehnungsgrund direkt am Cursor.
  - Alles auch mit `F1` (HUD aus) und im Farbsehschwächen-Modus prüfen.
- **Performance & Tests:**
  - `just bench-view` vorher/nachher messen (Budget: keine messbare
    Regression, Dirty-Guards aus Prompt 208 beibehalten).
  - Selbsttest in `hud_self_test.gd` / `self_test.gd` erweitern (Pop-up bei
    Beförderung, Plakette auf besetztem Gebäude, Cursor-Grund bei ungültigem
    Bauplatz).
  - `just check`, `just selftest` grün; Golden Hashes unverändert.
  - `godot/locale/` (DE + EN) und `docs/ARCHITECTURE.md` aktualisieren.

## 4. Nicht Teil dieses Prompts

- Änderungen an Kampf-, Deckungs-, Erfahrungs- oder Bauregeln in `crates/sim`.
- Deckungsschilde und Deckungsvorschau (Prompt 233).
- Kampagnen-Kapitelkarten und Missions-HUD (Prompt 237).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/236-kampffeld-lesbarkeit-und-weltfeedback.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md`
auf „erledigt“ setzen.

## Ergebnis (Nachtlauf 27.11.2026)

- **Reine Darstellung (`crates/sim` & Golden Hashes unverändert):**
  - Neues dirty-guarded HUD-Overlay `godot/scenes/hud/world_feedback.gd` im `Hud`-`CanvasLayer` (dadurch bei `F1`, Fotomodus und `trailer_shots` mit `hud = false` automatisch ausgeblendet):
    1. **Beförderungs-Pop-up (`veterancy_view.gd` + `world_feedback.gd`):** Steigender goldener Banner (`★ BEFÖRDERT` / `★ PROMOTED` + Rangname, `2,1 s`, `38 px` Aufstieg, weiches Ein-/Ausblenden) direkt über beförderten eigenen Einheiten.
    2. **Gebäude-Besatzungs-Plakette & Voll-/Feind-Rückmeldung (`world_feedback.gd`, `selection.gd`, `order_feedback.gd`):** Kompakte Dachplakette (Fraktionsfarbe aus `relation_colors.gd`, Haus-Icon, `Besatzung/Kapazität`, Veteranen-Sterne, Lebensbalken) über eigenen, verbündeten und aufgeklärten feindlichen Gebäuden (Nebel-des-Krieges-treu). Klick auf zerstörte, feindlich besetzte oder volle Häuser zeigt sofortige Rückmeldung am Cursor, über dem Dach und im Event-Feed.
    3. **Deutliche „Niedergehalten“-Kapsel (`state_marks.gd`):** Pinned-Trupps (`unit_suppressed >= 2`, eigene & aufgeklärte Feinde) erhalten im Nah-/Mittelbereich (`<= FAR_M`) eine kontrastreiche `NIEDERGEHALTEN`/`PINNED`-Kapsel über der Statuszeile (fern nur warm-rotes Symbol), respektiert `state_marks` und ist dirty-guarded.
    4. **Bau-Ablehnungsgrund direkt am Bau-Geist (`base_hud.gd`):** Zeichnet `Picker.blocked_text(placement_reason)` direkt als kompaktes rotes Schild neben dem Mauszeiger am ungültigen Bau-Geist (`ghost_reason_shown`).
    5. **Nachschubpunkt-Statuslabel (`world_feedback.gd`):** Dezent nur beim Überfahren eines Nachschubpunkts oder bei gehaltener `Tab`-Taste („Alle zeigen“), zeigt Name, Besitz-/Einnahmestatus und Ertrag.
  - DE/EN-Übersetzungen in `godot/locale/hud.csv` (`1473` Keys, `0` Probleme) und Doku in `docs/ARCHITECTURE.md` ergänzt.
- **Prüfung:**
  - Neuer `_world_feedback_test` in `godot/scenes/tools/hud_self_test.gd` (Pop-up, Welt-Hinweis, `F1`/HUD-aus-Prüfung).
  - `just check` und `just selftest` komplett grün (`self-test: hud ok (21 dots, 0.08 ms HUD per frame, max 2.55 ms)`), `crates/sim` & Golden Hashes unverändert.
- **Vorläufige Entscheidungen (Nachtlauf):** Pop-up-Dauer `2,1 s` / Aufstieg `38 px`, Dachplaketten-Höhe `6,8 m`, Nachschub-Label `8,2 m`. Offen: Fenster-Screenshots in `build/battlefield-feedback/` & `just bench-view` mit Grafikfenster.
