# Prompt 071 – Bodentextur gleitet bei Kamerabewegung: beheben

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, in
> `docs/ARCHITECTURE.md` die Abschnitte „Terrain rendering“ (mit „Splatting shader“ und
> „Meadow (prompt 057)“) und „Trailer recordings“, dazu `docs/PERFORMANCE.md` und
> `godot/assets/materials/terrain.gdshader`. Die Analyse ist gemacht (Abschnitt 2);
> dieser Prompt behebt die gefundene Ursache und misst nach.

## 1. Ziel

Beim Schwenken, Zoomen und Drehen der Kamera liegen auf Wiese und Straßenbelag feine
Schlieren, die über den Boden wandern: lange, 1 Pixel breite, parallele Linien, die in
jedem Bild anders liegen. Im Demo-Video sieht man es ab 00:10 (Einstellung
`column_high`), im Spiel bei fast jeder Kamerabewegung in mittlerer Entfernung.

Nach diesem Prompt klebt die Bodentextur am Boden: sie bewegt sich bei jeder
Kamerabewegung genau mit dem Gelände, ohne Schlieren, in allen Biomen und auf allen
Bodenarten. Die Verbesserung ist gemessen, nicht nur angesehen.

## 2. Ausgangslage: die Ursache (belegt)

`godot/assets/materials/terrain.gdshader` zeichnet jede Bodenart mit zwei Abtastungen
gegen sichtbare Kachelung. Die zweite (`uv_b`) wird in der Ferne größer gezeichnet:

```glsl
float far = smoothstep(40.0, 250.0, length(world_pos - CAMERA_POSITION_WORLD));   // Zeile 189
float scale_b = 1.37 * mix(1.0, 4.0, far);                                        // Zeile 217
vec2 uv_b = rotate(q, angle_b) / (tile * scale_b) + vec2(0.31, 0.71) * float(i + 1);
```

Der Maßstab der Textur hängt damit **in jedem Pixel vom Abstand zur Kamera** ab. Die
Textur-Koordinate ist keine Funktion der Weltposition mehr. Das hat zwei Folgen:

1. **Gleiten.** Ändert sich der Abstand eines Bodenpunkts zur Kamera (jeder Schwenk,
   Zoom, jede Drehung), ändert sich `scale_b` und die Textur rutscht über den Boden.
   Die Verschiebung wächst mit dem Abstand des Punkts von der Kartenmitte (`q` zählt ab
   dem Ursprung in der Kartenmitte): etwa `|q| × 0,011` Meter je Meter Abstandsänderung
   bei 110 m Kameraabstand, also 3 m je Meter bei 300 m von der Mitte, 21 m je Meter
   am Rand einer 4-km-Karte. In der Kartenmitte ist nichts zu sehen, deshalb fiel es
   in den Testaufnahmen nicht auf.
2. **Schlieren.** `scale_b` ändert sich auch quer über den Bildschirm. Das schert und
   staucht die Textur (um das Drei- bis Sechsfache), und `textureGrad` bekommt
   Ableitungen ohne diesen Anteil, wählt also eine zu scharfe Mip-Stufe. Ergebnis:
   1 Pixel breite, 20–40 Pixel lange Linien und Unterabtastung.

Betroffen sind alle zehn Bodenarten, Farbe und Detail-Normale, überall dort, wo das
Rauschen `blend` die zweite Abtastung zeigt (daher fleckenweise), und nur zwischen 40
und 250 m Abstand (darunter und darüber ist `far` konstant). Auf der Wiese verstärkt
`meadow_contrast` das Muster, flaches Licht (Morgen, Abend) verstärkt die Normale.

**Beleg (A/B, 16.11.2026):** Einstellung `column_high` (Karte `farms`, Qualität „high“,
Kamera 120 → 95 m), einmal mit dem Shader wie er ist, einmal mit der einen Zeile
`float scale_b = 1.37 * 1.78;` (fester Wert, nur für den Versuch, wieder entfernt).
Gemessen mit dem Skript im Anhang, Bilder 120–149:

| Fläche (Fenster) | Maß | wie er ist | fester Maßstab |
|---|---|---|---|
| Wiese (320×200+1300+700) | Gleiten der feinen Textur gegen den Boden | 4,63 Pixel/Bild | 0,13 Pixel/Bild |
| | Übereinstimmung 4–8 Pixel | 0,55 | 0,90 |
| | Übereinstimmung 8–16 Pixel | 0,82 | 0,98 |
| Straße (128×128+940+620) | Gleiten | 2,31 Pixel/Bild | 0,19 Pixel/Bild |
| | Übereinstimmung 4–8 Pixel | 0,53 | 0,83 |
| | Übereinstimmung 8–16 Pixel | 0,90 | 0,97 |

Im Standbild verschwinden die Linien mit festem Maßstab vollständig. Ausgeschlossen
sind damit für diese Entfernung: Sonnenschatten (Linien gleich stark in Sonne und
Schatten), Grasbüschel (reichen nur 55 m weit), Schärfentiefe (aus ab 60 m),
Videokompression (Linien schon in der Rohaufnahme und im PNG-Standbild).

Der Rest im Band 2–4 Pixel (0,43 / 0,37) liegt an der Messgrenze der MJPEG-Aufnahme:
ein PNG-Standbild und dasselbe Bild aus dem Film stimmen dort nur zu 0,49 / 0,62
überein. Ob darunter noch echtes Pixelflimmern liegt, zeigt erst eine verlustfreie
Aufnahme (Schritt 4).

Bilder und Aufnahmen des Versuchs liegen, solange niemand `build/` leert, in
`build/ground-slide/` (`grass_before_variant_3x.png`, `road_before_variant_3x.png`,
Standbilder, `variant_column_high.avi`, `ground_flicker.py`). Der Prompt braucht sie
nicht; das Skript steht auch im Anhang.

## 3. Regel für den Boden-Shader

**Jede Textur-Koordinate des Bodens hängt nur von der Weltposition ab.** Größen, die
von der Kamera abhängen (`far`, Pixelgröße `px`, Abstand), steuern ausschließlich
Gewichte: überblenden, ein- und ausblenden. Diese Regel kommt als Kommentar in den
Kopf von `terrain.gdshader`.

## 4. Umfang

1. **Vorher messen.** `just trailer-shots --only=column_high` aufnehmen und mit dem
   Skript aus dem Anhang die Werte aus Abschnitt 2 nachstellen (Spalte „wie er ist“).
   Das Skript nach `tools/ground_flicker.py` übernehmen (Python mit numpy, auf dem
   Rechner vorhanden; fehlt numpy, fragen statt installieren).
2. **Zweite Messstelle weit von der Kartenmitte.** In `godot/scenes/tools/trailer_shots.gd`
   eine Einstellung `survey_ground` ergänzen (die `survey_*`-Einstellungen werden nur
   auf Namen aufgenommen): 4-km-Karte `test_large`, nahe einer Kartenecke über Wiese
   und Straße, 6 Sekunden, ohne Einheiten im Messfenster, mit Schwenk, Zoom (150 → 80 m)
   und Drehung zugleich. Vorher messen: dort muss das Gleiten deutlich größer sein als
   in `column_high`. Ein Rezept `just ground-motion` nimmt beide Einstellungen auf und
   druckt die Werte.
3. **Beheben.** Vorschlag (A): zwei feste Maßstäbe statt eines gleitenden, überblendet
   nach Abstand.
   - Nah-Abtastung mit `1.37`, Fern-Abtastung mit `1.37 * 4.0`, beide mit eigenen,
     jetzt korrekten Ableitungen für `textureGrad`; `mix(nah, fern, far)` für Farbe und
     Normale (die Normale wie bisher zurückdrehen).
   - Die Nah-Abtastung nur lesen, wo `far < 1`, die Fern-Abtastung nur, wo `far > 0`.
     Unter 40 m und über 250 m kostet der Shader dann so viel wie heute, dazwischen
     zwei Lesezugriffe mehr je sichtbarer Bodenart.
   - Hält das die Budgets nicht (`just terrain-bench`, `just bench-view`), in dieser
     Reihenfolge prüfen: Überblendband schmaler (zum Beispiel 120–250 m), dritte
     Abtastung nur für die Farbe, zuletzt Variante (B): ein einziger fester Maßstab
     ohne Vergrößerung in der Ferne. (B) nur, wenn die Übersicht (350 m, 600 m) in
     Wald, Wüste und Winter keine sichtbare Kachelung zeigt; sonst mir vorlegen.
4. **Nachmessen.** Beide Einstellungen neu aufnehmen und messen. Zusätzlich einmal
   verlustfrei (PNG-Folge über `--write-movie <dir>/f.png`, wie `just terrain-shots`),
   um das Band 2–4 Pixel ohne MJPEG-Rauschen zu beurteilen. Liegt dort die
   Übereinstimmung unter 0,6, die Quelle benennen (Kandidaten: Sprenkel in 0,3-m-Zellen
   bei 5–10 cm Pixelgröße, `meadow_contrast`, Detail-Normale bei flacher Sonne) und
   einen Vorschlag machen, nicht nebenbei umbauen.
5. **Gleiche Stelle im Shader absuchen.** Alle weiteren Textur-Koordinaten in
   `terrain.gdshader`, `terrain_meadow.gdshaderinc`, `meadow_palette.gdshaderinc`,
   `terrain_edge.gdshader` und `water.gdshader` gegen die Regel aus Abschnitt 3 prüfen
   (Stand der Analyse: `far` ist die einzige Stelle mit `CAMERA_POSITION_WORLD`).
6. **Nichts verschlimmern.** Standbilder vorher und nachher:
   `SHOT_SET=meadow just terrain-shots <dir>` und `just terrain-shots <dir>` in Wald,
   Wüste (`--biome=desert`) und Winter (`--biome=winter`). Die Wiese darf in Ruhe nicht
   flacher werden, die Übersicht keine Kachelung zeigen.
7. **Doku.** In `docs/ARCHITECTURE.md` den Punkt „Against tiling“ im Abschnitt
   „Splatting shader“ berichtigen (zwei feste Maßstäbe, Regel aus Abschnitt 3, wie man
   misst), `docs/PERFORMANCE.md` bei geänderten Messwerten, `CLAUDE.md` nur, wenn
   `just ground-motion` dort hingehört.

## 5. Nicht in diesem Prompt

- Kein neues Aussehen von Wiese oder Straße, keine neuen Texturen.
- Keine Änderung an `crates/sim`: reine Darstellung, der Golden-Hash bleibt.
- Keine andere Kantenglättung als Standard (MSAA 4× auf „high“ bleibt).
- Grasbüschel, Schatten und Schärfentiefe im Nahbereich (unter 60 m): das sind die
  übrigen Verdächtigen aus `prompts/068-wiese-artefakte-bei-kamerabewegung.md`. Hier
  nur einmal mit dem neuen Werkzeug nachmessen und berichten (siehe Abschnitt 7).
- Das Demo-Video neu aufnehmen und schneiden: eigener Lauf von
  `prompts/periodic/070-youtube-demo-video.md`, erst nach meiner Freigabe.

## 6. Akzeptanzkriterien

1. Keine Textur-Koordinate des Bodens hängt von der Kameraposition ab; die Regel steht
   im Kopf von `terrain.gdshader`.
2. `column_high`, Fenster und Bilder wie in Abschnitt 2: Gleiten unter 0,3 Pixel/Bild
   auf Wiese und Straße (vorher 4,63 / 2,31), Übereinstimmung 4–8 Pixel mindestens
   0,80, 8–16 Pixel mindestens 0,95.
3. `survey_ground` (Kartenecke, 4 km): dieselben Grenzen.
4. Standbilder bei 60, 110 und 250 m zeigen keine 1 Pixel breiten Linien; die
   Vergleichsbilder aus Schritt 6 sind nicht schlechter, die Übersicht kachelt nicht.
5. `just check` und `just selftest` grün; `just terrain-bench` und `just bench-view`
   im Budget aus `docs/PERFORMANCE.md`, Werte vorher/nachher im Bericht.
6. Abschlussbericht mit Messwerten vorher/nachher, Bildpaaren (Pfade unter `build/`),
   dem Ergebnis der verlustfreien Messung und dem, was offen ist.

## 7. Offene Fragen

| Frage | Vorschlag |
|---|---|
| Vergrößerung in der Ferne behalten (A) oder streichen (B)? | (A): sie verdeckt die Kachelung in der Übersicht, vor allem in Wüste und Winter ohne Wiesen-Schicht. (B) nur, wenn die Bildrate es verlangt und die Übersicht sauber bleibt. |
| Was wird aus Prompt 068? | Seine Hauptfrage ist beantwortet. Nach dem Fix einmal im Nahbereich messen (15, 35, 55 m mit Büscheln); bleibt dort nichts Auffälliges, 068 mit Vermerk „Ursache in 071 behoben“ nach `prompts/done/` verschieben, sonst mit dem Rest als Umfang offen lassen. Mir zur Entscheidung vorlegen. |
| Demo-Video neu aufnehmen? | Ja, alle Einstellungen zwischen 40 und 250 m Kameraabstand sind betroffen; als eigener Lauf von Prompt 070 nach Freigabe. |

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`terrain.gdshader`, `trailer_shots.gd`, `justfile`, Doku) nur klein
und gezielt ändern. Am Ende nach `prompts/done/` verschieben und `prompts/README.md`
aktualisieren. Abschlussbericht: was geändert wurde, wie ich es teste, Messwerte und
Bilder, was offen ist.

## Anhang: Messskript

Aufruf: `python3 tools/ground_flicker.py <Film oder PNG-Muster> <erstes Bild> <Anzahl> <x> <y> <b> <h>`.
Das Fenster darf in allen Bildern nur Boden zeigen (keine Einheiten, Bäume, Häuser).

```python
"""Does the ground texture stick to the ground while the camera moves?

usage: ground_flicker.py <video or png pattern> <first_frame> <count> <x> <y> <w> <h>

The window (pixels, 1920 x 1080 frames) must show only ground for all frames.
Per pair of consecutive frames: the ground motion from the coarse structure
(shadows, blotches of 24-120 px), then
- slide: how far the fine texture (wavelengths below 8 px) moved against the
  ground, in pixels per frame (0 = it sticks to the ground),
- ncc per band: how well frame n+1 matches frame n after undoing the ground
  motion (1 = stable, 0 = unrelated).
Needs ffmpeg and numpy.
"""
import subprocess
import sys

import numpy as np

video, first, count, x, y, w, h = sys.argv[1], *map(int, sys.argv[2:8])
W, H = 1920, 1080
raw = subprocess.run(
    ["ffmpeg", "-v", "error", "-i", video, "-vf",
     f"select='between(n\\,{first}\\,{first + count - 1})'", "-vsync", "0",
     "-f", "rawvideo", "-pix_fmt", "gray", "-"],
    check=True, capture_output=True).stdout
frames = np.frombuffer(raw, np.uint8).reshape(-1, H, W)[:, y:y + h, x:x + w].astype(np.float64)

fy = np.fft.fftfreq(h)[:, None]
fx = np.fft.fftfreq(w)[None, :]
fr = np.hypot(fx, fy)
win = np.hanning(h)[:, None] * np.hanning(w)[None, :]
coarse = np.exp(-(fr * 24.0) ** 2) * (fr > 1 / 120.0)
fine = fr > 1 / 8.0
bands = {"16-32 px": (fr > 1 / 32) & (fr <= 1 / 16), "8-16 px": (fr > 1 / 16) & (fr <= 1 / 8),
         "4-8 px": (fr > 1 / 8) & (fr <= 1 / 4), "2-4 px": fr > 1 / 4}


def spectrum(img, band):
    return np.fft.fft2((img - img.mean()) * win) * band


def shift_of(a, b, band):
    """Shift of b against a in pixels, from the phase correlation in `band`."""
    cross = spectrum(a, band) * np.conj(spectrum(b, band))
    corr = np.fft.ifft2(cross / (np.abs(cross) + 1e-9) * band).real
    iy, ix = np.unravel_index(np.argmax(corr), corr.shape)

    def refine(c, i, n):
        left, mid, right = c[(i - 1) % n], c[i], c[(i + 1) % n]
        d = left - 2 * mid + right
        i = i + (0.5 * (left - right) / d if d != 0 else 0.0)
        return i - n if i > n / 2 else i

    return refine(corr[iy, :], ix, w), refine(corr[:, ix], iy, h)


def ncc(a, b, band, sx, sy):
    pa = np.fft.ifft2(spectrum(a, band)).real
    pb = np.fft.ifft2(spectrum(b, band) * np.exp(-2j * np.pi * (fx * sx + fy * sy))).real
    return (pa * pb).sum() / np.sqrt((pa * pa).sum() * (pb * pb).sum())


ground, slide, match = [], [], {k: [] for k in bands}
for a, b in zip(frames[:-1], frames[1:]):
    gx, gy = shift_of(a, b, coarse)
    tx, ty = shift_of(a, b, fine)
    ground.append(np.hypot(gx, gy))
    slide.append(np.hypot(tx - gx, ty - gy))
    for k, band in bands.items():
        match[k].append(ncc(a, b, band, gx, gy))

print(f"{video}: frames {first}..{first + count - 1}, window {w}x{h}+{x}+{y}")
print(f"  ground motion  {np.mean(ground):.2f} px/frame")
print(f"  slide          {np.median(slide):.2f} px/frame (fine texture against the ground, median)")
for k, v in match.items():
    print(f"  ncc {k:8s}   {np.mean(v):.3f}")
```
