# Prompt 007 – Umwelt- und Zivilobjekte Stufe 1

> In einer frischen Session im Repo-Ordner ausführen, eigener Worktree/Branch
> `assets/neutral-tier1`. Voraussetzungen: Prompts 003 und 004. Läuft parallel zu den
> drei Sessions von Prompt 006. Bei Bedarf in zwei parallele Sessions teilen:
> „… nur Teil A (Vegetation)“ und „… nur Teil B (Bauten)“.

## 1. Ziel

Die Objekte, die das Gelände so dicht und glaubwürdig machen wie in den Vorlagen, als
geprüfte Modelle in den Biomen Wald/gemäßigt, Wüste und Winter. Größen 1:1 laut
`data/catalog/neutral.toml`. Gebirge und Flüsse sind Gelände (Prompt 010/014), keine
Modelle.

## 2. Umfang

### Teil A – Vegetation und Kleinkram
- Bäume: Laubbaum (2–3 Varianten), Nadelbaum (2), Birke, Palme (Wüste); Winter über
  Schnee-Auflage im Shader und kahle Laubbaum-Variante.
- Büsche (3), Grasbüschel und Blumen als Karten mit Alpha, Felsen (3 Größen),
  Baumstümpfe und umgestürzter Stamm.
- Kleinkram: Heuballen (rund und eckig), Kisten, Fässer, Holzstapel, Telegrafenmast.
- Wind-Bewegung als Shader-Parameter vorbereitet (Gewichte in Vertex-Farben).
- Impostor- oder Billboard-Variante für Bäume in großer Entfernung (oder belegen, dass
  Godots LOD reicht).

### Teil B – Bauten und Befestigungen
- Zivil: Bauernhaus (2), Scheune, Steinhaus, Lehmhaus mit Flachdach (Wüste), Dorfkirche.
  Modular mit Wänden, Dach und Innenboden, damit Schadensstufen in Prompt 020 möglich
  sind (intakt, beschädigt, Ruine schon jetzt als Varianten).
- Brücken: Holzbrücke und Steinbogenbrücke, Spannweite parametrisch.
- Begrenzungen: Holzzaun, Steinmauer, Hecke als Segmente mit End- und Eckstücken.
- Militärisch (neutral nutzbar, Farben über das Material-System):
  Stacheldraht-Segment, Panzersperre (Tschechenigel), Sandsack-Stellung, MG-Bunker,
  Schützengraben-Segmente (gerade, Ecke, T, Ende; sitzen in einer Gelände-Vertiefung,
  die Prompt 010/021 schneidet), Minen (Panzer- und Schützenmine sichtbar, für
  Pioniere/Debug) und Minenfeld-Warnschilder.

### Für beide Teile
- Bauen über den Baukasten aus 004 (neue Module `kit/parts_neutral.py`,
  `kit/buildings.py`, `kit/vegetation.py`), Baupläne in `assets-src/blueprints/neutral/`.
- Prüfung mit `just validate-assets`, Galerie-Kategorie „Umwelt“, Biom-Umschalter.
- Katalog-Feld `model` setzen.

## 3. Regeln für die parallele Arbeit

Wie in Prompt 006, Abschnitt 3: nur eigene Pfade (`…/neutral/`, eigene Kit-Module),
gemeinsame Dateien nur additiv, `prompts/README.md` nicht ändern. Zusammenführen
zusammen mit den 006-Branches oder in einer eigenen kurzen Session; danach diese Datei
nach `prompts/done/` verschieben.

## 4. Akzeptanzkriterien

1. `just check`, `just validate-assets`, `just selftest` grün.
2. Alle Objekte in der Galerie in allen drei Biomen.
3. Aufstellung mit Referenzfigur und einem Panzer im Scratchpad; Größenverhältnisse
   (Türhöhe, Zaunhöhe, Baumhöhen) im Bericht bewertet.
4. Dreiecks- und Texturbudgets eingehalten; Instanzierbarkeit (eine Mesh-Ressource pro
   Variante) dokumentiert.

## 5. Arbeitsweise

Mit mir Deutsch, im Repo Englisch. Kein Push, keine Installation ohne mein Go. Mein
laufendes Blender nicht anfassen. Kein Test wird abgeschwächt. Binärdateien über
Git LFS. Commits nur auf dem eigenen Branch.
