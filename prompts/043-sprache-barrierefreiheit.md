# Prompt 043 – Lokalisierung, Barrierefreiheit und Einstellungen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Menüs, Einstellungen).
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Alle UI-Texte sind englisch und fest im Code. Nach diesem Prompt ist das Spiel auf
Englisch und Deutsch spielbar (weitere Sprachen leicht ergänzbar), und es gibt die
üblichen Barrierefreiheits- und Komfort-Einstellungen.

## 2. Umfang

1. **Lokalisierung:** alle Texte über Godot-Übersetzungen (CSV oder PO) mit Schlüsseln;
   Einheiten- und Objektnamen aus dem Katalog übersetzbar; Sprache in den Einstellungen,
   Standard nach Systemsprache. Prüfskript findet fest kodierte Texte.
2. **Barrierefreiheit:** UI-Skalierung, Schriftgröße, farbenblind-taugliche
   Spielerfarben (Paletten wählbar), Untertitel/Text für akustische Warnungen (029),
   Bildschirmwackeln abschaltbar, Tastenbelegung für alle Befehle (nicht nur Kamera).
3. **Grafik-Einstellungen:** Auflösung, Vollbild/Fenster, VSync, FPS-Limit, Render-Skalierung
   (FSR) zusätzlich zur Qualitätsstufe.
4. **Selbsttest:** Sprache wechseln, jede Ansicht ohne fehlende Schlüssel; Einstellungen
   bleiben nach Neustart erhalten.

## 3. Akzeptanzkriterien

1. `just selftest` grün inklusive Sprachprüfung (kein Text ohne Übersetzung).
2. Doku: Übersetzungsablauf in `docs/ARCHITECTURE.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
