# Testdateien für den Zug-Import

Vier erzeugte Arbeitsmappen für den ersten `pnpm run tauri:dev`-Durchgang. Sie sind **keine echten
Absenderdateien** — echte fehlen noch (siehe [../state.md](../state.md)) — aber jede Zeile ist auf
einen Fall gebaut, der schiefgehen kann. Alle Wagennummern tragen eine **gültige Prüfziffer**, außer
der einen, die es ausdrücklich nicht tut.

Erzeugt mit openpyxl, also mit **Inline-Strings** statt `sharedStrings` — genau die Variante, in der
der Lesefehler bei `&` auftritt (siehe [../footguns.md](../footguns.md)).

| Datei | Wofür |
| --- | --- |
| `werkstattrechnung_mueller_2026-01.xlsx` | Der saubere Fall. Kopfzeile in Zeile 1, echte Datums- und Zahlenzellen. Legt Wagen, Partner und Radsätze an; enthält einen Ein- und einen Ausbau |
| `werkstattrechnung_mueller_2026-02.xlsx` | **Derselbe Absender**, gleiche Struktur (Vorlage sollte greifen). `RS 4711` statt `RS-4711` → muss `Known` sein. `12345` gegen bekanntes `0012345` → `Likely`. Ein neuer Radsatz |
| `werkstattrechnung_bremsklotz_2026-02.xlsx` | **Anderer Absender mit derselben Radsatznummer** `RS-4711`. Muss `Ambiguous` werden, nicht stillschweigend zusammengeführt — der Kern von `resolve/radsatz.rs` |
| `problemfaelle_2026-03.xlsx` | Zwei Titelzeilen über der Kopfzeile (Kopf in Zeile 4), Beträge als Text in deutscher Schreibweise, Datum als Text, falsche Prüfziffer, Leerzeile, Zeile ohne Datum, unlesbarer Betrag, doppelte Zeile, Werkstattname in zwei Schreibweisen, und ein `&` im Firmennamen |

## Reihenfolge

Der Sinn steckt in der Abfolge — einzeln importiert zeigen die Dateien die Hälfte nicht:

1. `mueller_2026-01` importieren und übernehmen, dabei die Vorlage speichern.
2. `mueller_2026-02` importieren: Radsätze sollten ohne Rückfrage erkannt werden.
3. `bremsklotz_2026-02` importieren: `RS-4711` muss **nachfragen**.
4. `problemfaelle_2026-03` importieren: nichts darf den Lauf abbrechen; alles wird zur Meldung.

## Neu erzeugen

Das Skript liegt nicht im Repo. Die Dateien sind Fixtures und werden von Hand ersetzt, sobald echte
Absenderdateien vorliegen.
