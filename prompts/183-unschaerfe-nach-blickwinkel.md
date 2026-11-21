# Prompt 183 – Tiefenunschärfe nach Blickwinkel: eigene Einheiten im Vordergrund bleiben scharf

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> dann in `docs/ARCHITECTURE.md` die Zeile „Depth of field“ im Abschnitt
> „Light and atmosphere“ (Prompt 057), den Abschnitt zum Kippen der Kamera
> (Prompt 096: `view_stretch()`, `edge_depths()`, „Haze and depth of field“)
> und den Look-Abschnitt mit den Messungen ohne Tiefenunschärfe
> (`no_dof`); `docs/CONTROLS.md` (Kippen, Fotomodus F2). Im Code
> `godot/scenes/world/atmosphere.gd` (`DOF_MODES`, `_follow_camera()`,
> `_follow_dof()`), `godot/scenes/camera/rts_camera.gd` (`pitch()`,
> `base_pitch()`, `tilt_range()`, `edge_depths()`, `view_stretch()`),
> `godot/scenes/world/photo_mode.gd` und `godot/scenes/menu/settings.gd`
> (Einstellung `depth_of_field`). Für `CameraAttributesPractical` die API der
> festgelegten Godot-Version nachsehen (`docs/VERSIONS.md`), nicht raten.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/unschaerfe/flach_vorher.webp`: Bildschirmfoto des Nutzers
> vom 2026-11-21.

## 1. Problem

Der Nutzer hat die Kamera sehr flach gekippt, fast waagerecht, und zwei
**eigene, ausgewählte Panzer** stehen vorne in der Bildmitte (grüne
Auswahlringe). Sie sind **stark verschwommen**, ebenso die Straße und die Häuser
links und rechts davon. Scharf ist nur ein schmales Band in der Ferne (die
beiden Häuser am Waldrand). Der Nutzer:

> „Meine eigenen Panzer sollte ich scharf sehen. Wir schauen hier sehr flach,
> da sollte Unschärfe nur oben im Bild sein. Evtl. Unschärfe abhängig vom
> Betrachtungswinkel.“

Ursache (im Code geprüft, beim Bearbeiten bestätigen): `_follow_dof()` legt ein
scharfes Band **um den Fokus** (Kameraabstand `d`), verwischt alles **davor**
(`dof_blur_near_*`) und **dahinter** (`dof_blur_far_*`), und dehnt beide Ränder
mit `view_stretch()`. Bei flacher Kamera liegt der untere Bildrand aber viel
näher an der Kamera als der Fokus, also fällt die ganze untere Bildhälfte in
die **Nah-Unschärfe**. Dort stehen meist gerade die Einheiten, die der Spieler
steuert. Im Fotomodus (Kippen bis 4°, Sichtfeld bis 12°) wird das noch
stärker; im Spiel gilt es ab dem flachsten Winkel von 25°.

## 2. Ziel

- **Nah-Unschärfe hängt vom Blickwinkel ab:** Bei steilem Blick (Draufsicht,
  Miniatur-Look) bleibt sie wie heute. Je flacher die Kamera, desto weiter
  nimmt sie ab; **bei flachem Blick gibt es keine Nah-Unschärfe**, der ganze
  Vordergrund bis zum Fokus ist scharf. Übergang weich (zum Beispiel über
  `smoothstep` zwischen zwei Winkeln), kein Sprung beim Kippen. Vorschlag für
  den Startwert: voll ab 55°, keine unter 40°; die Session misst und wählt.
- **Fern-Unschärfe bleibt**, aber nur **oben im Bild**: Bei flachem Blick
  beginnt sie erst deutlich hinter dem Fokus, so dass die obere Bildhälfte
  weich wird und alles bis zur Bildmitte scharf bleibt. Die Dehnung über
  `view_stretch()` so anpassen, dass sich der scharfe Bereich am Bildschirm
  orientiert (zum Beispiel: scharf bis etwa 60 % der Bildhöhe von unten, dann
  weich), nicht an einem festen Vielfachen des Kameraabstands.
- **Eigene Einheiten sind nie unscharf**, solange sie im unteren und mittleren
  Bild stehen. Die Godot-Tiefenunschärfe wirkt auf Tiefe, nicht auf Objekte:
  Sie lässt sich nicht für einzelne Einheiten abschalten. Darum reicht die
  Winkelregel oben. Zusätzlich prüfen und entscheiden: Soll der Beginn der
  Fern-Unschärfe nach hinten rücken, wenn **ausgewählte** Einheiten weiter
  hinten im Bild stehen (Tiefe der fernsten ausgewählten Einheit plus Rand,
  sanft nachgeführt)? Nur, wenn das billig ist und nicht flackert; sonst als
  offenen Punkt notieren.
- Beide Modi (`subtle` und `miniature`) folgen der Winkelregel. Der
  **Miniatur-Look bei steilem Blick** und die **ruhige Draufsicht beim
  normalen Zoom** dürfen sich nicht sichtbar ändern (Bilder vorher/nachher bei
  der Neigung des Zooms).
- **Fotomodus:** gleiche Regel, damit flache Fotos die Szene vorne scharf
  zeigen. Wer im Fotomodus bewusst einen schmalen Schärfebereich will, wählt
  „Miniatur“ bei steilem Winkel. Ob der Fotomodus eine eigene Option „Schärfe
  vorne aus/an“ braucht, entscheidet die Session; Standard ist die neue Regel.

## 3. Umsetzung

- Die Winkelregel in `atmosphere.gd` (`_follow_dof()`); die Kamera liefert den
  Winkel (`pitch()`), die Abstände zu den Bildrändern liefert schon
  `edge_depths()`. Konstanten in `DOF_MODES` oder daneben, mit Kommentar und
  Begründung.
- Wenn die Nah-Unschärfe ganz aus ist, `dof_blur_near_enabled` abschalten
  statt nur die Menge auf null zu setzen (spart den Durchgang); prüfen, ob das
  in der festgelegten Godot-Version so wirkt.
- Dunst (`fog_depth_*`) bleibt wie er ist; nur wenn die neue Regel oben einen
  harten Übergang zwischen Dunst und Unschärfe zeigt, beides abstimmen.
- Keine Änderung an der Simulation; der Golden Hash bleibt.

## 4. Prüfen und zeigen

- **Bilder vorher/nachher** aus derselben Kameraposition, je Modus `subtle` und
  `miniature`, bei drei Neigungen: Neigung des Zooms (unverändert), 25° (im
  Spiel am flachsten) und 8° (Fotomodus). Die Szene des Nutzers nachstellen:
  zwei eigene Panzer vorne auf einer Straße zwischen Häusern, ausgewählt.
  Bilder in die Session zeigen und unter `docs/` ablegen, wo die anderen
  Look-Bilder liegen.
- Einen bestehenden Bildtest oder Selbsttest erweitern, der bei flacher
  Kamera prüft, dass `dof_blur_near` aus ist bzw. die Menge null hat, und bei
  steiler Kamera an ist (ohne Bildvergleich, über die Werte von
  `CameraAttributesPractical`). Bestehende Tests nicht lockern.
- Bildrate: Messung wie in `docs/PERFORMANCE.md` mit `subtle` bei flacher und
  steiler Kamera vorher/nachher; sie darf nicht schlechter werden.
- Doku: Zeile „Depth of field“ in `docs/ARCHITECTURE.md` (Winkelregel, Werte,
  Begründung, Bilder), Abschnitt zum Kippen (Prompt 096) und, falls sich der
  Fotomodus ändert, `docs/CONTROLS.md`.
- `just check`, `just selftest`.

## 5. Nicht Teil dieses Prompts

- Unschärfe pro Objekt oder eigener Nachbearbeitungs-Shader.
- Änderungen an Dunst, Licht oder Kamerawinkeln selbst.
- Neue Einstellungen im Hauptmenü (die drei Modi bleiben).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

| Frage | Entscheidung | Grund |
|---|---|---|
| Nah-Unschärfe | abhängig vom Blickwinkel, bei flachem Blick aus | Wunsch des Nutzers (2026-11-21): eigene Panzer vorne scharf |
| Fern-Unschärfe | bleibt, aber nur im oberen Bildteil | Wunsch des Nutzers: „Unschärfe nur oben im Bild“ |

## Ergebnis

(von der Session auszufüllen)
