# Prompt 048 – Soldaten-Animationen, Ragdolls und Treffer

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/INFANTRY.md`, `docs/ASSETS.md`, `docs/PERFORMANCE.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Soldaten sollen sich glaubwürdig bewegen und fallen: Deckungshaltungen an Mauern, Ecken
und Gräben, Drehen „Kopf zuerst“, Hinwerfen, Kriechen, Rollen, Hocken und geduckt gehen,
Fallen mit Physik und von Explosionen geworfen werden.

## 2. Leitregel

Ragdolls und Physik sind **nur Darstellung** (Regel 2 in `CLAUDE.md`): Die Sim
entscheidet, wer fällt, umgeworfen wird oder wieder aufsteht und wo er danach liegt;
Godot-Physik bewegt nur den sichtbaren Körper dahin und darf keine spielrelevante
Position zurückmelden.

## 3. Umfang

1. **Animationen** (Blender, gemeinsames Skelett aus 005, selbst erstellt; kein
   Motion-Capture-Material Dritter): an Mauer/Ecke/Grabenwand lehnen und herausschießen,
   Hinwerfen, Kriechen, seitlich Rollen, Hocken, geduckt gehen, Aufstehen, Drehen mit
   Kopf und Oberkörper zuerst.
2. **Ragdolls** in Godot (`PhysicalBone3D`/`PhysicalBoneSimulator3D`) für Tod und
   Umgeworfen-Werden; Explosionsimpuls aus dem Sim-Ereignis; nur im Nahbereich der
   Kamera und mit Budget (Anzahl gleichzeitiger Ragdolls).
3. **Umgeworfen (Sim):** Explosionen und Munitionsexplosionen von Panzern werfen
   Infanterie im Radius um (kurz handlungsunfähig), danach Waffe aufheben und weiter.
4. **Helm fliegt weg** als Darstellungs-Effekt bei Kopftreffer/Explosion (Zufall aus
   Darstellungs-RNG, nicht aus der Sim).
5. **Treffer-Darstellung:** zurückhaltend (Staub, Stoff, kurzer dunkler Puff) statt
   Blut-Effekten; [provisional], der Nutzer entscheidet über mehr Härte (Altersfreigabe).

## 4. Akzeptanzkriterien

1. `just kit-test`, `just validate-assets`, `just gallery-check` grün.
2. `just check` grün (Umgeworfen-Regel getestet), `just selftest` grün.
3. Infanterie-Benchmark: höchstens 10 % schlechter; Ragdoll-Budget dokumentiert.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
