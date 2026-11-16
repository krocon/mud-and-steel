# Prompt 067 – Typische Kampfentfernungen statt Maximalreichweiten

> In einer frischen Session im Repo-Ordner ausführen, nach Prompt 064 (Fahrzeug-Datenbank).
> Lies zuerst `CLAUDE.md`, `docs/SCALE.md`, `docs/SOURCES.md`, `data/catalog/SCHEMA.md`,
> `docs/BALANCE.md` und `prompts/done/045-balancing-playtest.md`.

## 1. Ziel

Die Reichweiten im Katalog mischen bisher typische und maximale Kampfentfernungen. Ein
Beispiel aus 045: der Panzer IV trägt im Spiel 375 m, der T-34 nur 250 m (jeweils
Katalogwert × 1/4), weil die Quellen unterschiedliche Dinge angeben. **Entscheidung des
Nutzers (2026-11-16): Es gelten typische Kampfentfernungen.** Danach sind die Reichweiten
aller Waffen vergleichbar, ohne die Echtheit des Katalogs aufzugeben.

## 2. Umfang

1. **Feld und Regel:** Im Katalog eine typische Kampfentfernung je Waffe (z. B.
   `range_typical_m`, Bedeutung in `data/catalog/SCHEMA.md` genau festlegen: übliche
   Gefechtsentfernung, auf die die Waffe im Einsatz gegen ihr Hauptziel wirksam
   eingesetzt wurde). Die bisherige Reichweite bleibt als Maximal- oder Visierwert
   erhalten. Die Sim rechnet nur noch mit der typischen Entfernung (× Faktor aus
   `data/scale.toml`, Faktor neu prüfen).
2. **Werte mit Quelle:** für jede Waffe im Katalog eine typische Entfernung mit Quelle
   (Handbücher, Gefechtsberichte, Fachliteratur, Datenbank aus 064). Wo nur eine
   Waffengattung belegt ist (z. B. Kampfwertgrenzen für 7,5-cm-Kanonen), Gattungswert mit
   Begründung; als `uncertain` markieren, was geschätzt ist. Keine Werte aus anderen
   Spielen.
3. **Folgen:** Preise (Reichweite und Feuerrate im Preis, 045 und Nachtrag), KI-Abstände
   (051), Sichtweiten und Balance neu messen (`just balance`, `just duel`), Golden-Hash
   begründet aktualisieren, Modelle mit geänderten Katalogeinträgen neu bauen
   (`just validate-assets`), `just catalog-doc`.
4. **Doku:** `docs/SCALE.md`, `docs/BALANCE.md` mit Vorher/Nachher.

## 3. Akzeptanzkriterien

1. Jede Waffe mit Reichweite hat eine typische Entfernung mit Quelle; Katalogprüfung grün.
2. `just check`, `just selftest`, `just lan-selftest`, `just validate-assets` grün.
3. Balance-Bericht: Fraktionen im Gefecht näher an 50 % als vorher, Abweichungen begründet.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
