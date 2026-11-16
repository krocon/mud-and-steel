# Prompt 051 – KI-Kommandeur Stufe 2

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/AI.md`, `docs/GAME_DESIGN.md`, `docs/ARCHITECTURE.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Die KI (024) erinnert sich an Gesehenes, sammelt, bündelt Feuer und zieht Angeschlagene
zurück. Nach diesem Prompt führt sie wie ein Kommandeur: Sie klärt auf, sammelt ihre
Armee, greift gemeinsam an, zieht sich bei Unterlegenheit zurück und kommt von einer
anderen Seite wieder. Sie sieht weiterhin nur, was ihre Einheiten gesehen haben.

## 2. Umfang

1. **Aufklärung:** schnelle Einheiten oder Aufklärer (033) klären gezielt auf; Wissen
   veraltet mit der Zeit.
2. **Lagebild:** Gefahrenkarte aus Verlusten und gesehenen Waffen („hier haben wir
   Männer verloren“), Kräfteverhältnis pro Front; Rückzug bei klarer Unterlegenheit,
   neuer Angriff über eine andere Achse.
3. **Konter:** Bau- und Produktionswahl nach gesehener Armee (Panzer → Pak, Infanterie →
   MG/Mörser, Flugzeuge → Flak).
4. **Versorgung:** kämpft um Versorgungspunkte, baut näher an der Front (Depots),
   schützt eigenen Nachschub und überfällt den des Gegners (036).
5. **Nutzt neue Systeme:** Angriffsbewegung und Formationen (046), Feuer und Bewegung
   (047), Artillerie/Rauch (032), Luftunterstützung (033), Transport (034), Pioniere (035),
   Teams (037).
6. **Aufgeben:** kapituliert, wenn die Lage aussichtslos ist (Schwelle begründen).
7. Turnier misst den Fortschritt gegen die alte KI (`Hard` alt vs. neu).

## 3. Akzeptanzkriterien

1. `just check` grün; neue KI gewinnt im Turnier klar gegen die alte `Hard`.
2. `just selftest`, `just lan-selftest` grün; Rechenzeit der KI pro Tick gemessen.
3. `docs/AI.md` aktualisiert.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Konkretisierung (Session 051)

Ausgangslage im Code: `crates/ai` steuerte jede Rolle als Gruppe (024) mit Gedächtnis,
Fokusfeuer, Rückzug Angeschlagener und Flankieren bei Stillstand; dazu Luft (033),
Transport (034), Pioniere (035), Nachschub (036), Teams (037), Angriffsbewegung (046),
Veteranen (050), Stellungen (052), Feuerschlag (054), Brückenleger (055), Besatzungen
(049). Gemessen vor dieser Session (Schlacht, 20 Spiele, 12 000 Ticks, siehe Ergebnis):
Hard gegen Passiv nur 15–60 % auf den Karten mit langem Anmarsch, fast alles Zeitablauf;
Basismodus auf `test_large` 0 % (Hard/Normal gegen Passiv), KI gegen KI auf 1-km-Karten
100 % Zeitablauf.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | **Kommandeur nur für Normal und Hard**; Easy bleibt bei der Gruppenlogik. Neue Stufe `hard-old` (nicht im Menü) = Hard ohne Kommandeur. | Easy soll leicht bleiben; `hard-old` erlaubt den geforderten Vergleich alt gegen neu im Turnier. |
| 2 | **Lagebild aus dem `PlayerView`**: Sichtungen mit Typ, Trefferpunkten und Wannenrichtung; Wissen verblasst ab 30 s und ist nach 4 min weg (Gebäude bleiben). Gefahr = Wert eigener Verluste im Umkreis von 120 m (verblassend) + Wert frisch gesehener Waffen, die den Punkt erreichen. Fronten = Sichtungen im Abstand ≤ 160 m. **[vorläufig]** | Die KI sieht weiter nur, was ihre Einheiten sahen; Verluste kennt jeder Spieler. Werte am Turnier abgestimmt. |
| 3 | **Bereitstellen, dann Angriff in Linie**: Sammeln 70 m außerhalb der Reichweite der Front, auf der Achse, die die Flanke der gesehenen Wannen zeigt, wenig Gefahr hat und nicht schon scheiterte (Hard ±90°, Normal ±45°); Weg in 45°-Schritten um die Front herum. Überlegene Reichweite: Kampf aus 85 % der eigenen. **[vorläufig]** | Gegen stehende Verteidiger verlor die alte KI, weil sie einzeln und frontal in deren längere Reichweite fuhr. Flanke + gleichzeitiges Eintreffen bringt lokale Überlegenheit (Seitenpanzerung, eigene Fahrzeuge verdecken). |
| 4 | **Nahangriff** (Panzer schließen auf 45 % der Reichweite auf, gegen Infanterie ohne Pak auf 60 m), **Kampfbilanz alle 15 s**: schlechter Tausch → Ausweichen auf eine andere Achse; klar unterlegen (30 % verloren, < 60 % / Normal 40 % der Front) → **Rückzug** 150 m hinter die Reichweite und neuer Angriff über eine andere Achse. **[vorläufig]** | „Rückzug bei klarer Unterlegenheit, neuer Angriff über eine andere Achse“ aus dem Prompt; Streuung wächst mit der Entfernung. |
| 5 | **Halten**, wenn die Front näher rückt: Linie bilden und kommen lassen. | Verteidiger im Stand gewinnen in dieser Simulation (Turnier: Passiv schlägt Angreifer); gegen angreifende KIs. |
| 6 | **Aufklärung** mit Spähwagen (eigene Rolle): fahren zur am längsten nicht gesehenen 200-m-Zelle ohne bekannte Gefahr, drehen bei Feind in 350 m ab; Basismodus baut einen, wenn keiner mehr lebt. Luftaufklärung wie 033. **[vorläufig]** | Altern des Wissens braucht jemanden, der es erneuert; billig, kein Kampf. |
| 7 | **Konterproduktion** nach gesehener Armee: viel Panzer → Pak-Trupps, Jagdpanzer; viel Infanterie → MG, Mörser, Kampfpanzer; Flugzeuge → Flak (033). Fehlt 30 s lang ein Rohstoff, baut die KI ersatzweise, was bezahlbar ist. **[vorläufig]** | Im Basismodus war Munition der Engpass, Personal türmte sich – die KI stand still. |
| 8 | **Versorgung**: Wellen nehmen die Versorgungspunkte nahe der Basis (wie bisher); zweites Depot und Bunker werden 100 m in Richtung Kartenmitte gesucht (`PlayerView::forward_sites`, Platzierungsprüfung der Sim); ab dem vierten Fahrzeug ein Versorgungs-LKW als „Depot auf Rädern“, der mit der Infanterie nach vorn fährt; Panzer beschießen ohne Feindpanzer in der Nähe zuerst feindliche LKW/Schützenpanzerwagen (Überfall). **[vorläufig]** | Gebäude brauchen ein eigenes Gebäude in 60–120 m, echte Frontdepots gehen nicht. Eigene LKW werden nur hinter der Armee gehalten (Schutz offen). |
| 9 | **Alles-oder-nichts**: Basismodus greift ohne Wellen an, wenn die eigenen Kampfeinheiten das Doppelte aller bekannten Feinde wert sind; ein Angriff ohne Kontakt gibt nach 1 min für 2 min an die Gruppenlogik ab (Vorstoß zu Sichtungen und Gebäuden). | Basispartien blieben unentschieden, weil die Armee in der Bereitstellung wartete. |
| 10 | **Aufgeben** (Easy, Normal, Hard; nicht `hard-old`): 60 s aussichtslos ⇒ `Command::Surrender`. Aussichtslos: keine Produktion (oder Produktion ohne Hauptquartier) und entweder keine eigene Waffe mit Munition kann einem bekannten Feind schaden, oder die eigenen Kampfeinheiten sind < 15 % (ohne HQ < 10 %) der bekannten Feinde wert. **[vorläufig]** | Verlorene Partien enden statt auf Zeit zu laufen; nur gesehene Feinde zählen, nie unsichtbare. 15 % ≈ ein Panzer gegen sieben. |
| 11 | **Keine Sim-Änderung.** Nur `crates/ai` (+ `--trace` und Überlebende im Turnier-CLI). | Parallel laufende Prompts; Golden-Hash unverändert. |
| 12 | Schlachtmodus im Turnier mit `--max-ticks 12000` (10 min) gemessen. | Auf 1–2-km-Karten dauert schon der Anmarsch mehrere Minuten; 5 min maß nur den Zeitablauf. |
