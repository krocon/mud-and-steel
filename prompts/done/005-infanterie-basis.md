# Prompt 005 – Infanterie-Basis: Körper, Skelett, Ausrüstung, Animationen

> In einer frischen Session im Repo-Ordner ausführen. Voraussetzung: Prompt 003
> (Katalog, `docs/ASSETS.md`). Kann parallel zu Prompt 004 und zum Spiel-Strang laufen
> (eigener Worktree/Branch).

## 1. Ziel

Ein gemeinsamer Soldat für alle drei Fraktionen: ein Körper mit Skelett und Animationen,
der über austauschbare Uniformen, Helme, Ausrüstung und Waffen zu Deutschen, Sowjets oder
Amerikanern wird. Darauf bauen alle Infanterie-Einheiten auf (Prompts 006 und 008).

Größe 1:1: Der Soldat ist 1,80 m groß, Waffen und Ausrüstung haben echte Maße.

## 2. Phase 0 – Entscheidungen (prüfen, berichten, auf mein Go warten)

1. **Körper-Basis:** Optionen vergleichen, z. B. MPFB2/MakeHuman (Blender-Add-on,
   Exporte frei nutzbar), CC0-Basismodelle (z. B. Quaternius) oder ein eigener
   Low-Poly-Körper per Skript. Kriterien: Lizenz, Qualität in der Draufsicht,
   Skelett-Kompatibilität, Aufwand. Keine Installation ohne mein Go.
2. **Animationen:** Quellen vergleichen, z. B. CC0-Animationsbibliotheken oder Mixamo.
   Mixamo-Lizenz genau prüfen (Nutzung im Spiel ja, Weitergabe der Rohdateien nein) und
   klären, ob das Repo öffentlich ist; danach entscheiden, was ins Repo darf.
3. **Skelett:** kompatibel mit Godots `SkeletonProfileHumanoid`, damit Animationen
   per Retargeting auf alle Varianten passen.

## 3. Umfang

- **Körper:** ein Basiskörper mit 2–3 Gesichts- und Statur-Varianten, Budget gemäß
  `docs/ASSETS.md` (Vorschlag ≤ 3 000 Dreiecke für LOD0).
- **Uniformen** als Textur- und Mesh-Varianten pro Fraktion, jeweils in den drei Biomen
  (Wald, Wüste, Winter mit Wintertarnung oder Mantel).
- **Helme und Kopfbedeckungen:** Stahlhelm M35/M40, Feldmütze; SSh-40, Pilotka; M1-Helm.
- **Ausrüstung** als Anbauteile an Knochen: Koppel, Tornister, Brotbeutel, Spaten,
  Gasmaskenbüchse, Funkgerät, Sanitätstasche (ohne Rotes Kreuz, siehe
  `docs/SOURCES.md`).
- **Waffen** mit eigenem `Socket_Muzzle`, per Baukasten-Stil-Skript oder Low-Poly von
  Hand: Kar98k, MP 40, MG 42, Panzerschreck; Mosin-Nagant, PPSh-41, DP-27, PTRD;
  M1 Garand, Thompson, BAR, M1919A4, Bazooka; leichte Mörser je Fraktion;
  Zielfernrohr-Varianten für Scharfschützen; Flammenwerfer.
- **Animationen** (mindestens): Stehen, Gehen, Rennen, Knien, Liegen, Kriechen, Zielen
  und Schießen (stehend, kniend, liegend), Nachladen, Granate werfen, Getroffen,
  Sterben (2 Varianten), MG tragen und aufbauen, Mörser bedienen, Sanitäter versorgt.
  Übergänge sauber, Fuß-Rutschen minimal.
- **Zusammenbau per Skript:** `tools/blender/build_soldier.py <faction> <rolle> <biom>`
  setzt Körper, Uniform, Helm, Ausrüstung und Waffe zusammen und exportiert
  `godot/assets/models/<faction>/infantry/<rolle>.glb` (Biom über das Material-System,
  nicht als eigene Datei, soweit möglich).
- **Galerie:** Die Modell-Galerie aus 004 (falls vorhanden, sonst eine eigene einfache
  Szene) zeigt Soldaten mit abspielbaren Animationen.
- **Ausblick Performance:** dokumentieren, wie später Hunderte Soldaten flüssig laufen
  (z. B. Animations-Texturen/Vertex-Animation für die Ferne). Umsetzung erst in
  Prompt 016/022.

## 4. Akzeptanzkriterien

1. `just check` und `just selftest` grün; `just validate-assets` prüft auch Soldaten
   (Größe 1,80 m ± 2 %, Knochen-Namen, Sockets).
2. Pro Fraktion ein Schütze und ein MG-Schütze in allen drei Biomen, alle Animationen
   in der Galerie abspielbar.
3. Aufstellungs-Rendering mit Panzer und Referenzfigur (falls 004 fertig) im Scratchpad;
   Größenverhältnisse im Bericht bewertet.
4. Lizenz und Herkunft jeder Fremdquelle in `CREDITS.md`.

## 5. Arbeitsweise

- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe.
- Keine Software ohne mein Go installieren. Mein laufendes Blender nicht anfassen.
- Kein Test wird abgeschwächt. Pro Schritt ein Commit, Binärdateien über Git LFS.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` aktualisieren, committen.
