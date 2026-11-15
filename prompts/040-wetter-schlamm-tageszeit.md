# Prompt 040 – Wetter, Schlamm und Tageszeit

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Licht/Biome, Gelände, Wegfindung, Nebel), `docs/MAPS.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

„Mud and Steel“ hat noch keinen Schlamm. Nach diesem Prompt gibt es Wetter (klar,
bedeckt, Regen, Schnee, Nebel) mit spielerischer Wirkung: Regen weicht Böden auf,
Kettenfahrzeuge kommen im Schlamm langsamer voran, Radfahrzeuge bleiben stecken,
Straßen werden wichtig; Nebel und Dämmerung verkürzen die Sicht.

## 2. Umfang

1. **Sim:** Wetterzustand pro Partie (fest gewählt oder als deterministischer Ablauf),
   Bodenfeuchte pro Geländeart und Zelle, Geschwindigkeitsfaktoren nach Fahrwerk
   (Rad, Halbkette, Kette) und Boden; Wegfindungskosten passen sich an (Teil-Neubau wie
   bei Zerstörung). Sichtweite nach Wetter und Tageszeit. Alles im Hash.
2. **Tageszeit:** Morgen, Tag, Abend, Nacht als Partie-Option (Nacht: Mündungsfeuer
   und Brände sichtbar, Leuchtgeschosse optional).
3. **Darstellung:** Regen/Schnee-Partikel, nasser Boden, Pfützen, Fahrspuren im Schlamm
   (Decals), Licht und Himmel aus 015 an Wetter und Tageszeit gekoppelt; Ambiente
   (030) mit Regen und Wind.
4. **Setup und Editor:** Wetter und Tageszeit im Spiel-Setup und als Karten-Standard.
5. **KI:** berücksichtigt Bodenkosten über die Wegfindung automatisch; prüfen.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests: Schlamm bremst Rad stärker als Kette, Straße bleibt
   schnell, Sicht im Nebel kürzer, gleicher Hash; Golden-Hash bewusst aktualisiert
   oder Standardwetter „klar“ ändert ihn nicht (besser).
2. `just selftest` mit Regen- und Nachtpartie grün; Benchmark: Wetter kostet höchstens
   2 ms pro Frame.
3. Doku: `docs/MAPS.md`, `docs/ARCHITECTURE.md`, Regelwerte mit Begründung.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
