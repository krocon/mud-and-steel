# Prompt 186 – Feinere Gebäude-Oberflächen: Putz, Mauerwerk und Dachziegel pro Gebäude steuerbar

> In einer frischen Session im Repo-Ordner ausführen. Empfohlen nach Prompt
> 185 (Vorschau-Blatt), damit jede Änderung mit `just building-preview`
> gezeigt werden kann. Lies zuerst `CLAUDE.md`, dann `docs/ASSETS.md`
> (Abschnitte „Material slots“, „Materials and shaders“, „Roof patina“
> aus Prompt 075, Wüstenputz aus Prompt 089, „Town and winter buildings“),
> `docs/PERFORMANCE.md` (`just bench-view`). Im Code
> `godot/assets/shaders/env.gdshader` (Slots `Stone` 12, `Plaster`, `Roof`
> 15, Mörtelfugen „running bond“), `env_roof.gdshaderinc` (`roof_frame()`,
> `roof_clay`, `roof_slate`, …), `env_common.gdshaderinc`,
> `godot/scenes/world/env_look.gd` (`EnvLook.apply()`, Instanz-Uniforms),
> `tools/blender/kit/bake.py` (Eckfarbe `Color`, A = Art) und
> `tools/blender/kit/buildings.py`. Shader-APIs für die festgelegte
> Godot-Version nachsehen (`docs/VERSIONS.md`), nicht raten.
>
> **Vorlage** (nicht im Repo, nur lesen):
> `docs/.vorlagen/arkadenhaus/referenz.webp`: Referenzbild des Nutzers vom
> 2026-11-21. Zeigt die gewünschte Feinheit: rötlicher Putz, großflächig
> abgeplatzt mit sichtbarem Ziegelmauerwerk, Werkstein an Sockel,
> Fensterrahmen und Gesimsen, Tonziegeldach mit erkennbaren Reihen, einzelnen
> fehlenden Ziegeln und Verwitterung.

## 1. Problem

Der Nutzer legt Wert auf Wände (Muster, Oberfläche) und die Feinheit der
Dachziegel. Heute sind alle Oberflächen prozedural, aber pro Slot gibt es nur
**vier Arten**. Die Art steckt im Alpha-Kanal der Eckfarbe
(`A = 1 - kind / 4`). Alles Weitere kommt aus Zufallswerten pro Gebäude
(`v_seed`). Pro Haus lässt sich so kaum festlegen:
- Putzfarbe und -zustand, also wie viel Putz abgeplatzt ist und wo (unten mehr, an Kanten mehr);
- Ziegelformat und Verband unter dem Putz und an Sichtmauerwerk;
- Werkstein an Ecken, Rahmen und Gesimsen als eigene Fläche;
- Art der Dachziegel (Mönch-Nonne, Biberschwanz, Hohlpfanne/S-Ziegel,
  Schiefer), Reihenhöhe, fehlende und verschobene Ziegel, Moos und Flechten.

## 2. Ziel

- **Parameter pro Gebäude** im Blueprint, die im Shader ankommen, zum Beispiel
  ein Abschnitt `surface = { plaster = "ochre_red", spall = 0.4, brick =
  "red", bond = "stretcher", stone_trim = true, roof = "mission",
  roof_row = 0.33, roof_missing = 0.05, moss = 0.2 }`. Namen und Umfang
  entscheidet die Session; alles hat einen Standardwert, der das heutige
  Aussehen ergibt.
- **Transport zum Shader:** die billigste Lösung wählen und begründen.
  Kandidaten sind Instanz-Uniforms, gesetzt beim Platzieren über Metadaten des
  Modells (wie `EnvLook.apply()`); ein zweiter UV-Kanal bzw. `CUSTOM0` für
  Werte pro Fläche (zum Beispiel „hier Werkstein“, „hier Putz über Ziegel“);
  mehr Bits in der Eckfarbe. Instanz-Uniforms sind pro Gebäude, nicht pro
  Fläche. Beides wird vermutlich gebraucht. Prüfen, wie sich das mit
  `MultiMesh`-Instanzen und der Platzierung auf Karten verträgt.
- **Mehr Arten pro Slot**, wo vier nicht reichen (`Plaster`, `Stone`, `Roof`),
  ohne die bestehenden Modelle neu bauen zu müssen, oder mit einem
  kontrollierten Neubau aller Umwelt-Modelle.
- **Feine Struktur über Normalen im Shader**, nicht über Geometrie: Reihen und
  Wölbung der Dachziegel, Fugen, Abplatzkanten mit leichtem Relief und
  Schatten. Echte Ziegel als Dreiecke würden das Budget von 3.000 Dreiecken
  pro Haus sprengen. Höchstens die Firstziegel und die Traufkante als
  Geometrie.
- **Detailstufe nach Abstand:** Feinheiten blenden bei großem Kameraabstand
  aus (Abstand bzw. Bildschirmgröße, wie andere Shader im Projekt), damit es
  aus Spielhöhe nicht flimmert (Moiré) und billig bleibt.
- **Bestehende Häuser bleiben so, wie sie sind** (Standardwerte). Höchstens
  eine bewusste, gezeigte Verbesserung für alle Häuser, zum Beispiel feinere
  Dachziegel, und dann mit Vorher-nachher-Bildern.

## 3. Prüfen und zeigen

- Vorschau-Blätter (`just building-preview`) vorher/nachher für
  `env_house_town`, `env_town_hall`, `env_house_farm_brick` und ein
  Wüstenhaus, jeweils mit Nahansicht von Wand und Dach und der Ansicht aus
  Spielhöhe. Dazu ein Testhaus mit „Putz stark abgeplatzt, Ziegel darunter,
  Mönch-Nonne-Dach“ neben dem Referenzbild.
- Biome: Wald, Wüste, Winter (Schnee auf dem Dach muss weiter zu den neuen
  Dachziegeln passen, `env_snow.gdshaderinc`), verkohlt (`env_char`).
- **Bildrate:** `just bench-view` auf der größten Stadtkarte vorher/nachher
  (`docs/PERFORMANCE.md`); sie darf nicht messbar schlechter werden. Kosten
  im Shader nennen.
- Einheiten-Webseite: Wenn sie die Gebäude mit eigenem Shader zeigt, prüfen,
  ob die neuen Parameter dort ankommen oder bewusst nicht; Thumbnails der
  geänderten Häuser neu rendern, `just units-web-sync`.
- Doku: `docs/ASSETS.md` (neue Parameter, Transport, Arten pro Slot,
  Detailstufe).
- Keine Änderung an der Simulation; Golden Hash bleibt.
- `just check`, `just selftest`.

## 4. Nicht Teil dieses Prompts

- Textur-Dateien oder Fotos als Oberflächen (bleibt prozedural).
- Neue Gebäudeformen (Prompt 184) und das Vorschau-Werkzeug (Prompt 185).

## Entscheidungen

| Frage | Entscheidung | Grund |
|---|---|---|
| Feinheit über Geometrie oder Shader | Shader (Normalen, Muster) | Dreieck-Budget, viele Häuser pro Karte |
| Bestehende Häuser | Standardwerte ergeben das heutige Aussehen | keine ungewollten Änderungen auf allen Karten |

## Ergebnis

(von der Session auszufüllen)
