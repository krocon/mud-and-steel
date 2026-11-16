# Prompt 061 – Recherche: kostenlose Soundeffekte für echte Aufnahmen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/SOURCES.md` (Abschnitt Sounds), `docs/ASSETS.md` (Audio) und
> `godot/assets/audio/sounds.json`. Reine Recherche im Internet: **nichts
> herunterladen, nichts committen außer dem Ergebnis-Dokument.**

## 1. Ziel

029/030 liefern synthetische Sounds. Für 053 (Sound Stufe 2, „echte Aufnahmen“)
braucht es eine geprüfte Liste kostenloser Aufnahmen, die in einem **kommerziellen
PC-Spiel (Steam)** verwendet werden dürfen. Stil zwischen „cinematic“ (kraftvolle
Explosionen, Artillerie, dichte Schlachtfeld-Atmosphäre) und „klassisches RTS“
(authentische Waffen-, Fahrzeug- und Bewegungsgeräusche). Andere Spiele dienen nur als
Stil-Vorstellung; **nichts aus anderen Spielen, Mods oder Filmen** (`docs/SOURCES.md`).

Qualität vor Menge: lieber 30 geprüfte Sounds als 500 beliebige.

## 2. Quellen

1. Sonniss GDC Game Audio Bundles – https://sonniss.com/gameaudiogdc/
2. Freesound – https://freesound.org/
3. OpenGameArt – https://opengameart.org/
4. Wikimedia Commons – https://commons.wikimedia.org/
5. Library of Congress – https://www.loc.gov/audio/
6. Internet Archive – https://archive.org/details/audio
7. Weitere seriöse Gratis-Bibliotheken (z. B. Sounds von Behörden/Archiven in
   Public Domain, CC0-Sammlungen einzelner Sounddesigner).

## 3. Kategorien

1. **Panzer und Fahrzeuge:** Tiger I/II, Panther, Panzer IV, Sherman, T-34,
   Halbketten, LKW, Jeeps; Anlassen, Leerlauf, Beschleunigen, Bremsen, Ketten,
   Kurvenfahrt, Turmdrehung, Rohrbewegung.
2. **Infanteriewaffen:** MG42/MG34, MP40, Kar98k, Thompson, M1 Garand, BAR, Bren,
   M1 Carbine; Nachladen, Magazinwechsel, Hülsen.
3. **Artillerie und Explosionen:** Geschütze, Haubitzen, Mörser, Einschläge, Bomben,
   Minen, klein/mittel/groß, Gebäudeeinsturz, Druckwelle, Nachhall, Einschläge auf
   Metall/Erde/Beton.
4. **Flugzeuge:** Bf 109, Fw 190, Ju 87 (Sirene), Spitfire, P-51, B-17; Motoren,
   Überflüge, Sturzflug.
5. **Atmosphäre:** ferne Gefechte, fernes MG-Feuer, Artillerie im Hintergrund, Wind,
   Regen, Gewitter, Wald/Feld, Feuer, brennende Fahrzeuge, Funkverkehr.
6. **Gebäude und Spielaktionen:** Produktion, Reparatur, Zerstörung, Einstürze,
   Metall, UI (Auswahl, Befehl, Warnung).

## 4. Lizenzregeln

- Nur Lizenzen, die kommerzielle Nutzung in einem kostenpflichtigen Spiel ausdrücklich
  erlauben. Bevorzugt **CC0/Public Domain**; **CC BY** nur mit Namensnennung im
  Credits-Bildschirm (044); andere Gratislizenzen nur, wenn eindeutig.
- **Ausschließen:** CC BY-NC, CC BY-ND (wir schneiden/bearbeiten), unklare Lizenzen,
  „nur privat“, Lizenzen, die das Weitergeben der Rohdateien verbieten, wenn die Dateien
  im Repo liegen müssten (vermerken!).
- **Pro Sound** die tatsächliche Lizenz auf der Dateiseite prüfen, nicht nur die
  Plattform-Beschreibung (Freesound mischt CC0, CC BY, CC BY-NC).
- Historische Aufnahmen: prüfen, ob die Aufnahme selbst gemeinfrei ist (z. B. Werk
  einer US-Bundesbehörde) und keine Rechte Dritter (Rundfunk, Musik, Personen) bestehen.
- **Keine erfundenen Links, Lizenzen oder Angaben.** Was nicht geprüft werden konnte,
  wird als „ungeprüft“ markiert.

## 5. Angaben pro Fund

Name, direkter Link (Dateiseite/Download), Quelle/Autor, Lizenz mit Link,
kommerziell nutzbar (Ja/Nein/Unklar), Namensnennung, Format, Samplerate/Bit-Tiefe,
Größe (soweit angegeben), kurze Qualitätsbewertung, Eignung fürs Spiel und welche
Klasse aus `sounds.json` sie ersetzen könnte.

## 6. Ergebnis

`docs/AUDIO_LIBRARY.md` (Englisch, wie alle Docs):

1. Tabelle der besten 30–50 Sounds, sortiert nach Qualität/Authentizität, Eignung,
   Lizenzsicherheit, Verfügbarkeit.
2. Die zehn besten Gratis-Pakete mit möglichst breiter Abdeckung der Kategorien.
3. Lücken: Kategorien ohne guten freien Fund (dort bleibt Synthese oder eigene
   Aufnahme).
4. Empfohlene Ordnerstruktur für eine langfristige Bibliothek, abgeglichen mit den
   bestehenden Klassen in `sounds.json` und dem Ablauf `assets-src/audio/<class>/` aus
   `docs/SOURCES.md` (Vorschlag, keine Umbenennung im Code).
5. Ein Verweis darauf aus `docs/SOURCES.md` (Sounds).

## 7. Arbeitsweise

Recherche parallel in Teil-Agenten nach Kategorien, danach Stichproben-Prüfung der
Lizenzen der übernommenen Einträge. Deutsch mit mir, Englisch im Repo, kein Push ohne
Freigabe, Downloads erst nach Absprache.
