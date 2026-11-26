# Prompt 238 – Hochkant-Video-Pipeline (9:16-Shorts-Aufnahme, Schnitt & optionale Einblendungen)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` („Trailer shots“) sowie als Vorbilder
> `prompts/done/221-video-szene-bocage-artillerieschlag.md` und
> `prompts/periodic/082-video-hinterhalt-am-waldrand.md`.
> Im Code:
> `godot/scenes/tools/trailer_shots.gd`,
> `tools/trailer-shots.sh`,
> `tools/trailer-cut.sh`,
> `tools/trailer-scout.sh`,
> `godot/scenes/world/combat_view.gd` (`HealthBars`),
> `godot/scenes/camera/rts_camera.gd`,
> `justfile`.
>
> **Stil-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/This WWII RTS is Pure Chaos #panzerstrike [lskjKn-C9yo].webm`
> (Hochkant-Format `9:16`, 60 fps, 5 harte Schnitte aus derselben Schlacht,
> dezente Einheiten-Balken bei Bedarf, knappe konturierte Hook-Untertitel am
> Anfang).
> **Keine Bilder, Tonspuren, Texte oder Assets aus dem Video ins Repo.**
> **Wichtig:** Reine Aufnahme-/Schnitt-Werkzeuge und Darstellung – **keine
> Änderungen in `crates/sim`**, Spielregeln & Golden Hashes bleiben unverändert.
> Alle bestehenden 16:9-Shots müssen Bit-für-Bit unverändert weiterlaufen!

## 1. Problem

Unsere gesamte Trailer-Pipeline ist bisher fest auf **16:9-Querformat**
verdrahtet:

1. **Feste Fenstergröße in `trailer_shots.gd`:**
   - `const SIZE := Vector2i(1920, 1080)` wird in `_ready()` direkt auf
     `get_window().size = SIZE` gesetzt. Ein Hochkant-Shot (`1080 × 1920` oder
     `1440 × 2560`) lässt sich nicht konfigurieren.
   - **macOS-Display-Grenze beachten:** Auf Laptop-Retina-Displays (z. B.
     `3456 × 2234` physisch = `1728 × 1117` logische Pixel bei 2× Skalierung)
     klemmt das Betriebssystem ein 1920 px hohes Fenster auf die Bildschirmhöhe
     ab, wodurch Godot Movie Maker statt `1080 × 1920` ein abgeschnittenes oder
     gestauchtes Bild aufnimmt.
2. **Schnittskript `tools/trailer-cut.sh`:**
   - Prüft und übernimmt zwar Streams über `ffmpeg`, hat aber keine
     Hochkant-Profile, keine Absicherung gegen gemischte Auflösungen und keine
     optionale, reproduzierbare Möglichkeit für kurze **Shorts-Einblendungen**
     (fett konturierte Hook-Texte / Zeitmarker) oder eine optionale lokale
     **Kommentar-/Voiceover-Spur** mit automatischem Ducking des Spieltons.
3. **Selektive Einheiten-Balken im HUD-losen Shot:**
   - In `trailer_shots.gd` schaltet `hud = false` sämtliche UI-Elemente ab
     (inkl. `combat_view.health_bars`). Für klassische RTS-Shorts kann es
     optional gewünscht sein, ohne volle Menüleiste/Minikarte nur dezente
     Einheiten-Lebensbalken (`unit_bars = true`) sichtbar zu lassen.

## 2. Ziel

### Teil A – Hochkant-Aufnahmen (`9:16`) in `trailer_shots.gd` & `tools/trailer-shots.sh`

1. **Pro-Shot-Auflösung (`size` / `portrait`):**
   - Optionales Shot-Feld `"size": Vector2i(1080, 1920)` (oder Kurzform
     `"aspect": "9:16"`, Default `SIZE = Vector2i(1920, 1080)`).
   - **Saubere Auflösung unabhängig von der Monitorhöhe (macOS & Linux/Windows):**
     - Prüfen, wie Godot 4.7 im Movie-Maker-Modus (`--write-movie`) mit
       `--resolution 1080x1920` bzw. `Window`/`SubViewport`/HiDPI-Skalierung
       umgeht:
       - Wenn `--resolution WxH` (oder `--windowed` + `content_scale_size` /
         `SubViewport`-Renderziel + Fullscreen-TextureRect bzw. Retina-2×-Fenster
         `540 × 960` logisch → `1080 × 1920` physisch) echte `1080 × 1920` Pixel
         ins `.avi` und in `--stills`-PNGs schreibt, diesen Weg sauber in
         `tools/trailer-shots.sh` und `trailer_shots.gd` verankern!
       - Sicherstellen, dass `Camera3D` im Hochkant-Format (`keep_aspect = KEEP_HEIGHT`
         vs. `KEEP_WIDTH`) die gewünschte taktische Brennweite sauber hält (kein
         verzerrtes oder extrem abgeschnittenes FOV; dokumentieren, wie `fov` bei
         Hochkant wirkt).
     - `tools/trailer-shots.sh` liest bei Bedarf die Zielauflösung des Shots
       (oder ein `--size=WxH`-Argument) und übergibt Godot die nötigen Flags,
       damit sowohl `--write-movie` als auch `--stills` exakt `W × H` (z. B.
       `1080 × 1920`) ausgeben.
2. **Optionales `unit_bars` in `trailer_shots.gd`:**
   - Neues optionales Shot-Feld `"unit_bars": false` (Default `false`): wenn
     `hud = false` und `unit_bars = true`, bleiben Minikarte, Befehlsleiste und
     Fog of War aus, aber `combat_view.health_bars` (auf `"damaged"` oder
     `"selected"`) darf dezente Lebensbalken zeichnen. Bestehende Shots bleiben
     unverändert.

### Teil B – Shorts-tauglicher Schnitt & optionale Overlays in `tools/trailer-cut.sh`

1. **Auflösungs-treuer Schnitt:**
   - `tools/trailer-cut.sh` erkennt die Auflösung (`WxH`) der Eingangs-AVIs
     automatisch via `ffprobe`, prüft, dass alle Shots eines Schnitts dieselbe
     Auflösung haben (klare Fehlermeldung bei Mismatch), und behält Hochkant
     (`1080 × 1920` oder `1440 × 2560`, optional `SCALE=1440x2560` per lanczos
     falls YouTube-4K/1440p-Bitrate gewünscht) exakt ohne Balken/Verzerrung bei.
2. **Optionale Captions & Voiceover (standardmäßig komplett aus):**
   - Optionaler Parameter `CAPTIONS=<pfad.ass|srt>` (oder einfaches kleines
     Hilfsskript `tools/shorts_captions.py`, das aus einer kurzen TOML-Liste
     `[[caption]] start_s, end_s, text, y_pct, color` eine saubere ASS-Untertitel-
     Datei mit fetter schwarzer Kontur und sicherer Shorts-Margin erzeugt):
     - Wenn `CAPTIONS` gesetzt ist, brennt `ffmpeg` die Untertitel sauber in den
       Videofilter ein; ohne `CAPTIONS` bleibt das Video 100 % pur.
   - Optionaler Parameter `VOICEOVER=<pfad.wav|mp3>` (plus `VOICEOVER_START_S`,
     Default `0.0`):
     - Wenn gesetzt, mischt `ffmpeg` die lokale Sprecher-Spur mit sanftem
       Sidechain-Ducking des Gefechts-Spieltons (`-4` bis `-6 dB` während
       Sprache) und normalisiert die Summe sauber auf `-14 LUFS` / `-1,5 dB TP`.
   - `justfile` und `docs/ARCHITECTURE.md` („Trailer shots“) knapp um die
     Hochkant-Optionen ergänzen.

## 3. Prüfen und zeigen

- Einen kleinen **Test-Hochkant-Shot** (z. B. `survey_portrait_check`, `2,0 s`
  auf `valley_fields`, `1080 × 1920`) mit `just trailer-shots --only=survey_portrait_check --stills`
  und als kurzes Video aufnehmen und über `tools/trailer-cut.sh` schneiden:
  - `ffprobe` bestätigt exakt `1080 × 1920`, `60 fps`, `yuv420p`, `sar 1:1`,
    `-14 LUFS` ohne Clipping.
  - Standbild visuell prüfen: kein abgeschnittenes Fenster, kein gestauchtes
    Seitenverhältnis, scharfe Schatten und klare Einheiten.
  - Optional einen zweiten Durchlauf mit Test-Caption (`CAPTIONS=...` in
    `build/`) prüfen, dass Kontur, Position (oberes Drittel, frei von
    Shorts-UI-Rändern) und Timing sauber sitzen.
- Gegenprobe mit einem bestehenden 16:9-Shot (`--stills`), dass `1920 × 1080`
  unverändert bleibt.
- `just check` und `just selftest` grün; Golden Hashes unverändert.

## 4. Nicht Teil dieses Prompts

- Änderungen an Spielregeln in `crates/sim`.
- Bau neuer Karten, Umweltmodelle oder des eigentlichen Ostfront-Shorts
  (dafür folgen Prompts 239–242).
- Einchecken von Videodateien, Voiceover-Dateien oder Vorlagen-Texten ins Repo.

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/238-hochkant-video-und-shorts-schnitt.md prompts/done/`
verschieben, Ergebnisabschnitt (wie Godot Hochkant auf macOS rendert, `ffprobe`-Daten
des Test-Shots, optionale Flags) ergänzen und den Status in `prompts/README.md`
auf „erledigt“ setzen.


## Ergebnis (2026-11-26)

- **Pro-Shot-Hochkantauflösung (`godot/scenes/tools/trailer_shots.gd`, `tools/trailer-shots.sh`):**
  - `PORTRAIT_SIZE := Vector2i(1080, 1920)`, `_shot_size()` unterstützt `"size"`, `"aspect": "9:16"` und `"portrait": true`; optionales `"unit_bars": true` blendet in HUD-losen Shots selektiv nur `combat_view.health_bars` ein; optionales `"keep_aspect": "width"`.
  - **macOS-Retina-Lösung:** Godots `--write-movie` ignoriert `--resolution` und liest stattdessen `display/window/size/viewport_width.movie` / `viewport_height.movie`. `tools/trailer-shots.sh` erkennt die Shot-Auflösung automatisch (oder über `--size=WxH`), schreibt für Nicht-16:9-Aufnahmen temporär `godot/override.cfg` und entfernt sie via `EXIT`-Trap rückstandsfrei (`godot/project.godot` bleibt unberührt).
- **Shorts-Schnitt & optionale Overlays (`tools/trailer-cut.sh`, `tools/shorts_captions.py`):**
  - Automatische Auflösungsprüfung per `ffprobe` über alle Eingangs-AVIs, optionale Skalierung (`SCALE=WxH`), optionale Untertitel (`CAPTIONS=<datei.toml>` via ImageMagick-PNG-Overlays + `ffmpeg`-`overlay`, da Homebrew-`ffmpeg` ohne `libass`/`freetype` gebaut ist) und optionale Sprecherspur (`VOICEOVER=<datei>` mit Sidechain-Ducking + `-14 LUFS` Loudnorm). Standardmäßig 100 % pures Gameplay ohne Overlays.
- **Prüfung:** Test-Shot `survey_portrait_check` aufgenommen und geschnitten (`1080 × 1920`, `60 fps`, `yuv420p`, `sar 1:1`, `-14.0 LUFS`), auch mit Test-Caption und Gegenprobe 16:9 geprüft; `just check` und `just selftest` grün; keine Änderungen in `crates/sim`.
