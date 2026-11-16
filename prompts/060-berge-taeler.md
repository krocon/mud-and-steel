# Prompt 060 – Berge und Täler

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/MAPS.md` (Höhenfeld, Hangneigung, Generator, Editor), `docs/ARCHITECTURE.md`
> (Terrain rendering, Wegfindung, Nebel), `docs/PERFORMANCE.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Das Höhenfeld kann schon beliebige Höhen (i32 Millimeter, 2-m-Raster), Hänge über 60 %
sind unbefahrbar, die Sicht folgt dem Gelände (018), und der Editor hat Heben/Senken.
Der Generator baut aber nur sanfte Hügel (bis etwa 60 m). Nach diesem Prompt gibt es
Karten mit echten Bergen, Kämmen, Pässen und Tälern: Höhen beherrschen das Feld, Pässe
und Talstraßen werden zu Engstellen, Hänge geben Deckung vor Sicht und Feuer.

## 2. Umfang

1. **Generator:** Gelände-Profile (`--relief gentle|hilly|mountains`): Bergkämme mit
   Grat-Rauschen, Täler mit Fluss in der Sohle (014), Pässe als befahrbare Sättel,
   Serpentinen-Straßen mit Hangneigung unter der Grenze, Felsabbrüche als
   unpassierbare Zonen. Starts und Versorgungspunkte erreichbar (map-check prüft).
2. **Regeln** prüfen und ggf. ergänzen: Steigfähigkeit pro Fahrzeug (Katalog), Rad vs.
   Kette am Hang, Infanterie langsamer bergauf, Sichtvorteil von Höhen (größere
   Sichtweite ab Höhenunterschied), Hangdeckung; mit Begründung, im Hash.
3. **Darstellung:** Fels und Geröll nach Hangneigung im Splatting (011), Schnee über
   einer Höhengrenze im Winterbiom, Nadelwald an Hängen, Nebel in Tälern (015/040),
   Höhenlinien optional auf der Minikarte (031); Kamera bleibt über hohem Gelände
   bedienbar (Zoomgrenzen relativ zum Boden).
4. **Editor (028):** Pinsel Kamm/Tal/Glätten/Terrasse, Straße am Hang mit automatischer
   Begrenzung der Steigung.
5. **Karte:** eine Gebirgskarte (z. B. Talkessel mit Pass) für 2 und 4 Spieler.
6. **Höhenbereich:** `height_step_mm` so wählen, dass 300–500 m Relief möglich sind;
   Speicher- und Ladezeit messen (`docs/MAPS.md` „If 8 km maps ever come“).

## 3. Akzeptanzkriterien

1. `just check` grün; Tests für neue Hangregeln und Sichtvorteil; Golden-Hash nur
   geändert, wenn Regeln es verlangen, mit Begründung.
2. `just map-check` der Gebirgskarte, `just editor-selftest`, `just selftest` grün;
   Wegfindungs- und Terrain-Benchmark auf der Gebirgskarte im Budget von 016.
3. Doku: `docs/MAPS.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
