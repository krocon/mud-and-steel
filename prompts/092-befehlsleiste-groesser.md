# Prompt 092 – Größere, lesbare Befehlsleiste unten links

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `godot/scenes/hud/command_bar.gd` und `godot/scenes/hud/air_bar.gd` ganz, in
> `godot/scenes/hud/hud.gd` die Anordnung (wo Befehlsleiste, Luftleiste,
> Bau-/Produktionsleiste von `base_hud.gd`, Einheitenpanel und Minimap liegen), in
> `godot/scenes/menu/settings.gd` `ui_scale`, `font_size`, `ui_scale_for()` und
> `_apply_display()`, `godot/scenes/menu/menu_theme.gd` (`font()`,
> `set_font_scale()`), den UI-Teil von `godot/scenes/menu/settings_panel.gd`,
> `godot/scenes/tools/hud_self_test.gd`, `godot/scenes/menu/l10n_self_test.gd`
> (prüft UI-Skala und Schriftgröße), `godot/locale/hud.csv`, `docs/CONTROLS.md` und
> die HUD-Abschnitte in `docs/ARCHITECTURE.md` (Prompts 031, 043).

## 1. Ausgangslage

Der Block unten links – Luftunterstützung („Air“: Recon, Fighters, Attack, Bombers,
Paratroopers mit ihren Kosten, darunter der Hinweis „Air support: left click the
target …“) und die Befehlsleiste („Formation“ mit Auswahl, Move, Attack, Ground,
Patrol, Advance, Secure, Stop, Hold fire, Occupy, Barrage, Smoke, Dismount,
Unlimber, Stand, Kneel, Prone, Auto) – ist auf dem Bildschirm des Nutzers so klein,
dass er die Schrift kaum lesen kann. Die Knöpfe sind fest 88 × 40 px groß, die Schrift
12–13 px (`MenuTheme.font(12)`), und ausgegraute Knöpfe haben zusätzlich wenig
Kontrast.

Es gibt schon eine globale UI-Skala (`ui_scale`, 0,75–1,5) und einen
Schriftgrößen-Faktor (`font_size`, 0,85–1,3). `ui_scale_for()` begrenzt die Skala
aber auf `Fensterhöhe / 720`; auf einem großen, hochauflösenden Bildschirm (Retina,
1440p, 4K) bleibt die Leiste dadurch in echten Pixeln winzig. Erst herausfinden,
warum es beim Nutzer so klein ist (Fenstergröße, `content_scale_factor`,
Bildschirm-Skalierung von macOS / `DisplayServer.screen_get_scale()`, Stretch-Modus
im `project.godot`), bevor etwas geändert wird.

## 2. Ziel

- Die Leiste ist bei Standardeinstellungen auf jedem üblichen Bildschirm (1080p,
  1440p, 4K, MacBook-Retina) gut lesbar: Knopfschrift mindestens so groß wie normaler
  Menütext, Kosten unter den Luftknöpfen ebenfalls lesbar.
- Sie ist skalierbar: entweder über die vorhandene UI-Skala (wenn die richtig
  arbeitet, reicht das) oder zusätzlich über eine eigene Einstellung „HUD-Größe“ in
  den Einstellungen (de/en), z. B. 0,8–1,6. Keine zweite Skala ohne Grund – erst
  prüfen, ob die vorhandene genügt.
- Auf- und zuklappbar: Weil die größere Leiste mehr vom Spielfeld verdeckt, kann der
  Spieler sie einklappen. Eingeklappt bleibt eine schmale Leiste (Titelzeile oder
  kleiner Griff mit Pfeil), aufgeklappt das volle Raster. Luft- und Befehlsleiste
  einzeln klappbar. Der Zustand bleibt über Partien erhalten (in den Einstellungen
  gespeichert), Standard: aufgeklappt.
- Eingeklappt funktionieren alle Tastenkürzel weiter (M, R, L, P, Z, I, H, F, U, G, T,
  J, O, Y, X, V, B, Komma für Formation); nur die Knöpfe sind verborgen. Ein
  Tastenkürzel zum Auf-/Zuklappen ist willkommen, wenn eine freie Taste da ist; dann
  in die Tastenbelegung der Einstellungen und `docs/CONTROLS.md`.
- Ausgegraute Knöpfe bleiben erkennbar ausgegraut, aber lesbar (Kontrast prüfen).

## 3. Regeln und Technik

- Feste Pixelgrößen (`custom_minimum_size = Vector2(88, 40)`, `MenuTheme.font(12)`)
  durch Werte ersetzen, die mit HUD-Skala und Schriftgröße wachsen; das Raster darf
  dabei nicht abschneiden: lange deutsche Texte („Sperrfeuer“, „Absitzen“,
  „Abprotzen“, „Bodenziel“) und die Tastenangabe in Klammern müssen passen. Lieber
  Spaltenzahl oder Knopfbreite anpassen als Text kürzen.
- Die übrigen HUD-Teile (Einheitenpanel unten Mitte, Minimap, Ereignisliste,
  Bau-/Produktionsleiste über der Befehlsleiste) dürfen sich nicht überlappen. Wenn die
  Befehlsleiste breiter wird, prüfen, dass das Einheitenpanel daneben noch Platz hat;
  bei schmalen Fenstern (1280 × 720) lieber automatisch kleiner skalieren als
  überlappen.
- Tooltips (`HUD_TIP_*`) und der Hinweistext unter der Luftleiste wachsen mit.
- Reine Darstellung: nichts in `crates/sim`, Golden Hash unverändert.
- Neue Texte in `godot/locale/hud.csv` bzw. `settings.csv`, deutsch und englisch.

## 4. Vorgehen

1. Ursache klären: im Spiel (`just run --skip-menu`) auf dem Rechner des Nutzers
   Fenstergröße, `content_scale_factor`, Bildschirm-Skalierung und tatsächliche
   Schriftgröße in Bildschirmpixeln ausgeben. Ergebnis kurz festhalten.
2. Vorher-Bilder der Leiste bei 1280 × 720, 1920 × 1080 und 2560 × 1440 (und auf dem
   Retina-Bildschirm des Nutzers) nach `build/hud-size/before/`, mit einem kleinen
   Shot oder vorhandenen Aufnahmewegen (`l10n_shots.gd`, `unit_card_shots.gd` als
   Vorbild).
3. Skalierung umsetzen (Standardgröße sinnvoll anheben, ggf. `ui_scale_for()` für
   hochauflösende Bildschirme korrigieren, ggf. eigene HUD-Größe), dann Auf-/Zuklappen.
4. Nachher-Bilder an denselben Auflösungen nach `build/hud-size/after/`, jeweils
   aufgeklappt, eingeklappt, auf Deutsch und Englisch, kleinste und größte Stufe.
5. Tests: `hud_self_test.gd` ergänzen – Knöpfe wachsen mit der Skala, nichts
   überlappt (Rechtecke von Befehlsleiste, Luftleiste, Einheitenpanel, Minimap),
   Einklappen verbirgt die Knöpfe, Tastenkürzel wirken eingeklappt weiter, Zustand
   wird gespeichert und wieder geladen. `l10n_self_test.gd` für neue Einstellungen
   (Standardwerte wiederherstellen) erweitern. Vorhandene Tests nicht abschwächen.
   `just selftest` und `just check` grün.
6. Doku: `docs/CONTROLS.md` (Einklappen, ggf. Taste, HUD-Größe) und
   `docs/ARCHITECTURE.md` (HUD-Skalierung).
7. Der Nutzer schaut es sich danach selbst mit `just run` an; lege ihm ein
   Vorher/Nachher-Paar in seiner Auflösung vor und nenne die Einstellung, mit der er
   die Größe ändert.

## 5. Nicht Teil dieses Prompts

- Neue Befehle, andere Befehlslogik oder andere Tastenbelegung (außer der Taste fürs
  Einklappen).
- Umbau anderer HUD-Teile (Einheitenpanel, Minimap, Ereignisliste) über das hinaus,
  was nötig ist, damit nichts überlappt.
- Neues Grafikdesign der Knöpfe (Icons statt Text o. Ä.).
