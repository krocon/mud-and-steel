# Prompt 225 – Durchgehende Artillerie-Rauchwölkchen-Bögen ab Abschuss

> In einer frischen Session im Repo-Ordner ausführen. Baut direkt auf Prompt 220
> (Artillerie-Einflugspuren & zweistufiger Explosionsrauch) auf.
> Lies zuerst `CLAUDE.md`, `docs/ARCHITECTURE.md` („Indirect fire and smoke“,
> „Effects“), `docs/PERFORMANCE.md` sowie das Ergebnis von
> `prompts/done/220-artillerie-einflugspuren-und-zweistufiger-explosionsrauch.md`.
> Im Code:
> `godot/scenes/world/artillery_view.gd`,
> `godot/scenes/world/effects.gd`,
> `godot/scenes/world/effect_library.gd`,
> `godot/scenes/world/rocket_effects.gd`.
>
> **Stil-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4`, Sekunden `00:20–00:30`
> (zwei US M7 Priest im rechten Vordergrund feuern im hohen Bogen über das
> gesamte Bild ins Dorf; eine dritte Spur steigt am rechten Bildrand auf).
> **Keine Bilder, Texturen oder Assets aus dem Video ins Repo.**
> Reine Darstellung – **keine Änderungen in `crates/sim`**, Golden Hashes
> unverändert.

## 1. Problem

In der Referenz (Sek. 00:20–00:30) sind die dramatischsten bewegten Elemente
die **langen, durchgehenden, perlenartigen Bögen aus runden weiß-grauen
Rauchwölkchen**, die direkt an der Rohrmündung der feuernden Selbstfahrlafetten
(`us_m7_priest`) im Vordergrund entstehen, steil in die Luft steigen und im
weiten Bogen bis zum Einschlag ins Dorf verfolgbar bleiben:

- Direkt über der Mündung sitzt ein **dichter, voller weiß-hellgrauer
  Abschussrauch-Ballen** (2–4 größere überlappende Puffs).
- Daran schließt sich **lückenlos vom Abschuss an** eine gebogene Kette aus
  weichen, runden Rauchwölkchen an, die anfangs etwas enger und kräftiger sitzen,
  entlang der Flugbahn leicht auseinandergezogen und etwas größer/weicher werden
  und erst nach wenigen Sekunden mit dem Wind verblassen.
- Bei uns zeichnet `artillery_view.gd` (Zeile 376–378, Prompt 220) die
  `IncomingPuffs` aktuell **ausschließlich im steilen Endanflug**:
  `velocity.y <= -PUFF_MIN_DESCENT_MPS and left_now <= PUFF_WINDOW_S` (letzte
  `2,4 s`). Feuert ein `us_m7_priest` oder eine `de_lefh18` im Vordergrund,
  sieht man daher nur den kurzen Mündungsstoß (`muzzle_puffs`) und dann
  *keine* aufsteigende Rauchwölkchen-Spur, bis das Geschoss weit hinten wieder
  fällt.

## 2. Ziel

Reine Darstellung (alle Flugbahnen, Abschusszeiten und Einschläge unverändert
aus `SimHost`):

1. **Aufsteigende & durchgehende Rauchwölkchen-Bögen für schwere indirekte Artillerie (`artillery_view.gd`):**
   - Für schwere indirekte Geschosse ab Kaliber `>= 100 mm` (z. B. 105-mm-Haubitzen
     wie `us_m7_priest`, `de_wespe`, `de_lefh18`, `us_m2a1_105`, schwere Mörser
     `>= 107 mm` und Raketen) werden die Rauchwölkchen im bestehenden gemeinsamen
     `IncomingPuffs`-`MultiMeshInstance3D` (weiterhin **1 Draw-Call!**) bereits
     **ab dem Abschuss** (`flown >= 0`, unabhängig vom Vorzeichen von
     `velocity.y`) über die gesamte sichtbare Flugbahn abgelegt und gehen
     nahtlos in den Endanflug über.
   - **Optik entlang des Bogens:**
     - **Abschuss-Zone (`flown < 0,6 s`):** dichterer, etwas größerer, heller
       Abschuss-Rauchfuß nahe der Mündung, der sauber an den Mündungsrauch
       (`effects.gd`) anschließt.
     - **Aufstieg & Flugbogen:** markante, perlenartig getrennte, weiche runde
       weiß-hellgraue Rauchwölkchen (`PUFF_SPACING_M`, Größe und Lebensdauer so
       abgestimmt, dass ein langer Bogen quer über den Bildschirm gut lesbar
       bleibt, die Wölkchen sanft wachsen und mit dem Wind verwehen, ohne das
       Bild bei großen Batterien zuzukleistern).
     - **Endanflug:** die warme Einschlag-Aufhellung nahe dem Ziel (`left_now < 0,45 s`)
       und die leuchtende Geschoss-Spitze (Prompt 199) bleiben erhalten.
   - **Leichte indirekte Geschosse (`75–99 mm`, z. B. 81-mm-Mörser):** behalten
     das bisherige Endanflug-Verhalten aus Prompt 220 (`velocity.y <= -4.0` und
     `left_now <= 2.4 s`), damit kleine Mörser das Schlachtfeld nicht überladen.
2. **Pool-Budget, Sichtbarkeit & Tageszeiten:**
   - `PUFF_MAX_INSTANCES` (aktuell `1024`) bei Bedarf auf `1536–2048` anheben
     und älteste/fernste Puffs sauber priorisieren, damit auch eine ganze
     Artillerie-Batterie vollständige Bögen zeichnet.
   - **Nebel des Krieges strikt wahren:** gegnerische Abschüsse und Bögen werden
     pro Wölkchen über `effects.visible_at.call(pos)` gefiltert (wie bisher in
     `_render_incoming_puffs`) – kein Verrat verdeckter feindlicher Batterien!
   - **Tageszeit & Qualität:** bei Nacht bleiben die Wölkchen dezent dunkel
     (`sky_lift = 0.0`, niedrige Alpha); bei Regen/Nebel gedämpft; auf Qualität
     `Low` (`fx_shell_trails = false`) komplett aus.
   - **Pre-Seed bei Fast-Forward/Standbildern:** sicherstellen, dass ein
     `trailer-shots`-Standbild oder ein schneller Zeitsprung einen sauberen
     Bogen zeigt und keine Lücke hinter dem Geschoss reißt.

## 3. Prüfen und zeigen

- **Vorher/Nachher-Bilder und kurzer Clip** (klassisch + Weitwinkel-Perspektive
  nach Prompt 217):
  - Zwei bis drei feuernde `us_m7_priest` im Vordergrund, die im hohen Bogen
    auf ein Dorf schießen (klare, durchgehende weiße Rauchwölkchen-Bögen direkt
    ab den Rohren bis ins Ziel!).
  - Tag, Herbst (falls Prompt 223 fertig), Abend und Nacht prüfen.
  - **Nebel-des-Krieges-Check:** gegnerische Haubitze im verdeckten Gebiet
    darf am Abschussort keine Rauchwölkchen zeigen, erst im sichtbaren Bereich.
- **Performance:**
  - `just effects-bench` (inkl. Artillerie-Salve) und `just bench-view --perf-kills`
    vorher/nachher messen und in `docs/PERFORMANCE.md` eintragen (Budget:
    keine messbare Regression bei 1 Draw-Call).
- `just check`, `just selftest` grün; Golden Hashes unverändert.

## 4. Nicht Teil dieses Prompts

- Änderungen an Ballistik, Flugzeit, Streuung oder Schaden in `crates/sim`.
- Herbstlandschaft (Prompt 223), Treppengiebel-Häuser (Prompt 224) und die
  Herbst-Video-Szene (Prompt 226).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/225-artillerie-rauchboegen-ab-abschuss.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.


## Ergebnis (2026-11-26)

- **Durchgehende Rauchbögen ab Abschuss:** `IncomingPuffs` in `godot/scenes/world/artillery_view.gd` (weiterhin ein einziger `MultiMeshInstance3D`-Draw-Call, `PUFF_MAX_INSTANCES = 2048`) erzeugt für schwere indirekte Geschosse (`≥ 100 mm`, z. B. `us_m7_priest`, `us_m2a1_105`, `us_m2_42in`) und Raketen bereits ab Abschuss (`flown >= 0`) eine durchgehende perlenartige weiße Rauchwölkchen-Kette über den gesamten sichtbaren Bogen inkl. dichterer Start-Puffs an der Mündung.
- **Abwärtskompatibilität & Qualität:** Kleinere Mörser (`< 100 mm`) behalten den steilen Ziel-Einflug aus Prompt 220; Nebel des Krieges, Nacht-Abdunklung und Abschaltung auf Qualitätsstufe `low` bleiben erhalten. Visuell in `autumn_barrage` über die volle Bogenbahn geprüft. Werte vorläufig.
