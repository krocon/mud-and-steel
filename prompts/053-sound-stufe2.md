# Prompt 053 – Sound Stufe 2: Akustik und Stimmen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Audio aus 029/030), `docs/SOURCES.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

029 liefert synthetische Sounds. Dieser Prompt macht das Gefecht akustisch glaubwürdig:
Geschosse knallen beim Vorbeifliegen, weit entfernte Schüsse kommen verzögert an,
Herauszoomen klingt fern, große Explosionen klingen größer, und Soldaten rufen.

## 2. Umfang

1. **Überschallknall:** Geschosse, die nahe an der Hörmitte vorbeifliegen, erzeugen ein
   „Krack“ vor dem Mündungsknall (aus den Geschossbahnen der Brücke).
2. **Schallgeschwindigkeit:** Verzögerung nach Entfernung (mit Zeitfaktor 2), Echo/Hall
   nach Umgebung (Wald, Dorf, offenes Feld).
3. **Entfernung und Zoom:** Tiefpass und Pegel nach Kamerahöhe; nah: Details, fern:
   dumpfes Grollen.
4. **Explosionen nach Größe** skaliert (Kaliber, Munitionsexplosion), mit Nachhall.
5. **Echte Aufnahmen:** Struktur für Waffenaufnahmen pro Waffentyp (CC0 oder vom
   Nutzer bereitgestellt, mit Quelle), synthetisch als Rückfall.
6. **Stimmen:** kurze Rufe beim Hineinzoomen (Feind gesichtet, Deckung, Nachladen,
   verwundet) je Fraktion in ihrer Sprache; Herkunft: vom Nutzer aufgenommen oder
   bereitgestellt; bis dahin Platzhalter-Struktur ohne Dateien. Keine Parolen,
   keine Ideologie.

## 3. Akzeptanzkriterien

1. `just selftest` grün; Golden-Hash unverändert.
2. Schlacht-Benchmark hält das Budget aus 029.
3. Quellen in `docs/SOURCES.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
