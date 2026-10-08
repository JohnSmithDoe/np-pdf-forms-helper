# Fachdomäne — Güterwagen, Radsätze, Instandhaltung

Das Fachwissen hinter dem `trains`-Modul, und der **Schlüssel zwischen Fachbegriff und
Code-Bezeichner**. Wer eine Anforderung aus einer Mail übersetzt oder einen Supportanruf einordnet,
liest hier nach.

Entschiedenes steht in [decisions.md](./decisions.md), empirische Fallen in
[footguns.md](./footguns.md), Blockiertes in [state.md](./state.md).

**Zum Wahrheitsgehalt.** Fachbegriffe, Rollen und Normnummern sind belastbar. Was ausdrücklich
`⚠ prüfen` trägt, ist **nicht** verifiziert — Anhangs- und Teilnummern von Regelwerken wandern
zwischen Ausgaben, und konkrete Grenzmaße stehen ohnehin in der jeweiligen Instandhaltungsanweisung
und nicht hier. Solche Angaben sind Orientierung, keine Grundlage für eine Entscheidung im Code.

---

## 1. Wozu dieses Dokument

Die Anwender sprechen Fachdeutsch. Jede Absenderdatei, jeder Supportanruf und jede Anforderung
kommt in dieser Sprache an. Steht im Code eine erfundene englische Übersetzung, kostet jede dieser
Nachrichten einen Übersetzungsschritt, für den es keinen niedergeschriebenen Schlüssel gibt — und
der zweimal verschieden ausfallen kann.

Deshalb gilt: **Fachbegriffe stehen als solche im Code.** Nicht als Kommentar daneben, sondern als
Typ- und Feldname. Der Satz „der Halter des Wagens hat den Radsatz ausbauen lassen" soll sich ohne
Zwischenschritt auf Bezeichner abbilden lassen.

### Wo das Deutsch aufhört

**Fachliche Substantive sind deutsch. Anwendungsmechanik bleibt englisch.**

Ein Mitarbeiter beim Halter hat ein Wort für einen Wagen, einen Radsatz, einen Einbau, eine
Instandhaltung, einen Halter, eine Werkstatt. Niemand dort hat ein Wort für eine `Resolution`, ein
`ColumnBinding`, einen `LayoutHint`, eine `Provenance`, einen `dedupeKey` oder einen
Staging-Lauf. Diese Begriffe zu übersetzen bringt nichts und kostet Lesbarkeit — sie bleiben
englisch. Die Grenze verläuft an der Frage: **hat der Anwender dafür ein Wort?**

### Schreibweise

**Bezeichner und JSON-Schlüssel sind ASCII.** Das kostet fast nichts, weil die Fachbegriffe zufällig
umlautfrei sind — es heißt `Wagen`, nicht `Güterwagen`, und `Halter`, nicht `Eigentümer`. Wo ein
Umlaut unvermeidlich ist, wird transliteriert: `ae`, `oe`, `ue`, `ss` (`eigentuemerId`,
`radsaetze`). **Die richtige Schreibweise mit Umlauten ist den Beschriftungen in der Oberfläche
vorbehalten** — dort steht `Radsätze` und `Eigentümer`.

---

## 2. Wer die Anwender sind

**Halter und Vermieter von Güterwagen.** Sie besitzen keine Lokomotiven und fahren keine Züge; sie
stellen Wagen bereit und verantworten deren Instandhaltung.

Zielanwender ist ein **mitteleuropäischer Wagenhalter** mit einer Flotte in der Größenordnung
einiger tausend Wagen — im Repo durchgehend **Wagenmut AG** genannt. Der Name ist erfunden, so wie
jeder andere Firmenname in diesem Projekt: **hier steht kein echtes Unternehmen**, weder der
Anwender noch ein Absender. Wo ein zweiter Halter gebraucht wird, heißt er **Achsentreu SA**, die
Leasinggesellschaft **Nietenzähler Rail Leasing GmbH** und die Werkstätten **Schienenbein
Waggonwerk GmbH**, **Dreh & Gestell Technik** und **Rundlauf Radsatztechnik**.

Der übrige Vermietmarkt gehört zum Umfeld, nicht zum Zielbild: eine Handvoll großer privater
Vermieter teilt ihn unter sich auf, und die Konsolidierung der letzten Jahre hat mehrere Flotten
zusammengelegt. **Lokomotivvermieter sind eine andere Anlagenklasse** und haben mit Radsatzpools in
diesem Sinn nichts zu tun.

Radsätze kommen von einer kleinen Zahl europäischer Hersteller; **aufgearbeitet wird meist regional,
nicht beim Hersteller** — und das ist der Grund, warum die Werkstatt in einer Absenderdatei fast nie
die ist, die den Radsatz gebaut hat.

Nordamerika folgt anderen Regelwerken (AAR statt UIC/EVN) und ist hier kein Maßstab.

---

## 3. Was den Daten zu trauen ist

**Fast nichts.** Die eintreffenden Dateien sind Werkstattaufträge und Rechnungen, in Excel von Hand
nachbearbeitet, von wechselnden Absendern, ohne verabredetes Format. Sie sind eine **Quelle von
Indizien, keine Stammdatenquelle.** Wer das Modul erweitert, geht von dieser Annahme aus und nicht
von der Gegenrichtung.

Belastbar sind genau zwei Angaben:

| Angabe | Warum belastbar |
| --- | --- |
| **Wagennummer** | Zwölfstellig **mit Prüfziffer**. Ein Zahlendreher fällt auf, bevor er etwas anlegt. Das Einzige, was ohne Rückfrage entscheiden darf |
| **Radsatznummer — erst nach Bestätigung** | Ohne Prüfziffer und ohne eindeutiges Format, also **nicht** aus sich heraus belastbar. Erst wenn der Anwender sie einem Radsatz zugeordnet hat, ist sie für **diesen Absender** verlässlich — und genau das ist es, was die Alias-Tabelle festhält |
| **Bestellnummer — nur zusammen mit der Wagennummer** | Vom Bestellsystem in einem Schema vergeben (`12345-26`, `12345-26/01`), also je Wagen eindeutig. Allein entscheidet sie nie: dieselbe Nummer an zwei Wagen ist eine Sammelbestellung und damit eine Frage, keine Zusammenführung |

**Die Master-Datei des Kunden ist die Ausnahme von „Indiz“.** Sie ist kein Absenderdokument, sondern
der Stand, den der Kunde selbst pflegt; das Schattensystem **spiegelt** sie bei jedem Import neu
(siehe [decisions.md](./decisions.md), „Der Master wird gespiegelt“). Widersprechen sich zwei ihrer
Blätter, entscheidet der Anwender — das Programm wählt keine Quelle.

Alles Übrige ist Indiz und wird als solches behandelt:

- **Partnernamen** — Schreibweisen, Rechtsformen, Abkürzungen; dieselbe Werkstatt in vier Fassungen.
- **Datumsangaben** — Tag/Monat vertauscht, Text statt Datum, 1904-Basis.
- **Beträge** — deutsches oder englisches Dezimaltrennzeichen, brutto oder netto, mit oder ohne
  Währung.
- **Leistungstexte** — Freitext ohne Wertevorrat.
- **Einbaupositionen** — Hauskonvention (siehe Abschnitt 7).
- **Halter, Eigentümer, Kunde, Mieter** — vier Spaltenüberschriften für Parteien, die sich
  überschneiden, aber nicht dasselbe sind (Abschnitt 5).

### Was daraus im Code folgt

Das ist kein Beiwerk — es ist die Begründung für fast jede Entwurfsentscheidung im Modul:

- **Nur die Wagennummer ist Pflicht.** Kein anderes Feld darf einen Import scheitern lassen.
- **`sanitise` behauptet nicht, es meldet.** Ein `Err` ist ein Fehler, ein `Ok` mit Warnung eine
  Warnung — die Schwere ist die Form des Ergebnisses, nicht ein Feld daneben.
- **Dezimaltrennzeichen und Datumsreihenfolge werden über die ganze Spalte entschieden**, nie je
  Zelle: die Spalte hat Belege, die die einzelne Zelle nicht hat.
- **`Resolution` statt stiller Zuordnung.** Unbekannt heißt Rückfrage, nicht Neuanlegen.
- **Die Alias-Tabelle lernt, statt zu raten.** Was der Anwender einmal bestätigt hat, gilt beim
  nächsten Mal ohne Rückfrage — das ist der einzige Weg, wie das Verfahren mit der Zeit leichter
  wird statt lästiger.

---

## 4. Glossar: Fachbegriff → Code-Bezeichner

**Die operative Tabelle.** Sie ist der Teil dieses Dokuments, der zuerst veraltet — wer im
`trains`-Modul umbenennt, pflegt sie mit.

### Entitäten

| Fachbegriff | Typ | Felder |
| --- | --- | --- |
| Wagen (Güterwagen) | `Wagen` | `nummer`, `halterId`, `eigentuemerId`, `bauart`, `bemerkung` |
| Radsatz | `Radsatz` | `nummer`, `matchKey`, `aliases`, `wellennummer`, `systemId`, `bauart`, `bemerkung` |
| Einbau (eines Radsatzes in einen Wagen) | `Einbau` | `radsatzId`, `wagenId`, `position`, `eingebautAm`, `ausgebautAm` |
| Instandhaltung | `Instandhaltung` | `wagenId`, `werkstattId`, `radsatzId`, `datum`, `leistung`, `betragCent`, `bemerkung` |
| Partner (Halter, Eigentümer, Werkstatt) | `Partner` | `rollen`, `name`, `bemerkung` |
| Telematik-Gerät | `TelematikGeraet` | `kennung` (Pointer-ID des Absenders), `wagenId`, `angebautAm` |
| Telematik-Meldung (nur die neueste je Wagen) | `TelematikMeldung` | `wagenId`, `geraetId`, `zeitpunkt` (mit Uhrzeit), `stadt`, `land`, `standort`, `laufleistungKm`, `energieProzent`, `bewegung` |
| Schadensmeldung | `Schadensmeldung` | `wagenId`, `gemeldetAm`, `gemeldetVon`, `schadcode`, `notiz`, `ausgesetzt`, `beladen`, `ausfuehrender`, `geplantAm`, `aktion`, `erledigtAm` |
| Werkstattauftrag (Bestellung) | `Werkstattauftrag` | `wagenId`, `bestellnummer`, `werkstattId`, `status`, `erfasstAm`, `eingangAm`, `ausgangAm`, `versendetAm`, `bemerkung` |
| Prüfung (P8, Revision G4.x, …) | `Pruefung` | `wagenId`, `art`, `faelligAm`, `geplantAm`, `durchgefuehrtAm`, `status`, `bestellnummer` |

### Rollen

| Fachbegriff | Wert |
| --- | --- |
| Halter | `halter` |
| Eigentümer | `eigentuemer` |
| Werkstatt | `werkstatt` |

### Spaltenarten beim Import (`FieldKind`)

| Fachbegriff | Wert |
| --- | --- |
| Wagennummer | `wagennummer` |
| Datum | `datum` |
| Werkstatt | `werkstatt` |
| Halter | `halter` |
| Eigentümer | `eigentuemer` |
| Leistung | `leistung` |
| Betrag | `betrag` |
| Bemerkung | `bemerkung` |
| Radsatznummer | `radsatznummer` |
| Radsatzwellennummer | `wellennummer` |
| Radsatz-ID | `radsatzSystemId` |
| Einbauposition | `einbauposition` |
| Eingebaut am | `eingebautAm` |
| Ausgebaut am | `ausgebautAm` |
| Telematik-Gerät, … angebaut am, … Zeitpunkt | `telematikGeraet`, `telematikAngebautAm`, `telematikZeitpunkt` |
| Stadt, Land, Standort | `telematikStadt`, `telematikLand`, `telematikStandort` |
| Laufleistung (km), Energie-Reserve (%), Bewegung | `telematikLaufleistung`, `telematikEnergie`, `telematikBewegung` |
| Schaden gemeldet am / von, Schadcode, Schadensnotiz | `schadenGemeldetAm`, `schadenGemeldetVon`, `schadcode`, `schadenNotiz` |
| Ausgesetzt, Beladen (ja/nein) | `ausgesetzt`, `beladen` |
| Ausführende Werkstatt/EVU, Schaden geplant am, Notwendige Aktion, Schaden erledigt am | `schadenAusfuehrender`, `schadenGeplantAm`, `schadenAktion`, `schadenErledigtAm` |
| Bestellnummer | `bestellnummer` |
| Auftragsstatus, Auftrag erfasst am, Werkstatteingang, Werkstattausgang, Auftrag versendet am, Auftragsbemerkung | `auftragStatus`, `auftragErfasstAm`, `auftragEingangAm`, `auftragAusgangAm`, `auftragVersendetAm`, `auftragBemerkung` |
| Prüfart, Prüfung fällig / geplant / durchgeführt am, Prüfungsstatus | `pruefart`, `pruefungFaelligAm`, `pruefungGeplantAm`, `pruefungDurchgefuehrtAm`, `pruefungStatus` |
| Nicht importieren | `ignorieren` |

**Bewusst englisch geblieben**, weil Mechanik und kein Fachbegriff: `id`, `createdAt`, `source`,
`matchKey`, `aliases`, `dedupeKey`, `Provenance`, `Resolution`, `ColumnBinding`,
`LayoutHint`, `ImportPlan`, `ImportTemplate`, `StagedRow`, `RowStatus`.

---

## 5. Die Rollen, und warum sie nicht dasselbe sind

Das deutsche Eisenbahnrecht trennt Parteien, die im Alltagsgespräch zusammenfallen. **Sie zu
verwechseln ist der klassische Fehler dieses Moduls**, weil in Absenderdateien Spalten wie
„Halter", „Kunde", „Mieter" und „Eigentümer" nebeneinander stehen und Verschiedenes meinen.

**Halter.** Die im nationalen Fahrzeugeinstellungsregister (NVR) eingetragene Partei, gekennzeichnet
durch das Halterkürzel (VKM) am Langträger. **Das ist, was der Anwender selbst ist.** Wenn eine
Absenderdatei „Halter" schreibt, ist im Regelfall das gemeint.

**Eigentümer.** Der rechtliche Eigentümer, häufig eine Leasinggesellschaft oder Zweckgesellschaft,
die im Werkstattalltag nie vorkommt. Nicht identisch mit dem Halter, und der Grund, warum das
Modell beide Felder führt statt eines.

**EVU / Mieter.** Das Eisenbahnverkehrsunternehmen, das den Wagen tatsächlich einsetzt — meist der
Mieter. Spalten mit „Kunde" oder „Mieter" meinen oft dies und **nicht** den Halter.

**ECM — Entity in Charge of Maintenance / für die Instandhaltung zuständige Stelle.** Eine
zertifizierte Rolle nach **VO (EU) 2019/779** (ersetzte VO 445/2011) mit vier Funktionen: Leitung,
Instandhaltungsentwicklung, Fahrzeuginstandhaltungsmanagement und Erbringung der Instandhaltung.
Der Halter ist üblicherweise ECM; Werkstätten sind meist nur Erbringer. Das Modul bildet die ECM
**nicht** ab (siehe Abschnitt 9), muss den Begriff aber kennen, weil Nachvollziehbarkeit von
Radsätzen genau daher als Anforderung kommt.

**Werkstatt.** Der ausführende Betrieb. Im Modell dieselbe Entität wie Halter und Eigentümer, mit
einer anderen Rolle — eine Werkstatt, die selbst Wagen hält, ist real, und zwei getrennte Typen
würden die Alias-Mechanik doppeln.

---

## 6. Der Wagen

**Die Wagennummer ist die zwölfstellige europäische Fahrzeugnummer (EVN)** und die einzige
Eigenschaft, auf die im Modul gematcht wird. Sie trägt eine **Prüfziffer** an letzter Stelle, ist
also validierbar — `util/uic.util.ts` tut das. Diese Eigenschaft ist der Grund, warum ein Wagen
über einen exakten Index aufgelöst werden darf und ein Radsatz nicht (Abschnitt 6).

Daneben stehen das **Halterkürzel (VKM)** am Fahrzeug und der Eintrag im **NVR**.

**Gattung und Bauart sind zweierlei.** Das Gattungszeichen ist der genormte Buchstabencode aus
Gattungsbuchstabe und Kennbuchstaben (`Zans`, `Sgns`, `Habbins`, `Falns`) und beschreibt Bauart und
Ausstattung in einer Kurzform. Die Bauart im weiteren Sinn ist die Herstellerbezeichnung der
Fahrzeugserie. Das Feld `bauart` nimmt auf, was in der Quelldatei steht — beides kommt vor.

---

## 7. Der Radsatz

Ein **Radsatz** besteht aus der **Radsatzwelle**, zwei fest aufgepressten **Rädern** und den
**Radsatzlagern**; je nach Bauart kommen Bremsscheiben hinzu. Ein Regelgüterwagen läuft auf vier
Radsätzen in zwei Drehgestellen.

**Ein Radsatz ist ein Wechselbauteil.** Er wird ausgebaut, geht in einen Pool, wird aufgearbeitet
und kommt unter einem **anderen** Wagen wieder ein — unter Umständen bei einem anderen Halter.
Deshalb ist die Zuordnung Radsatz ↔ Wagen kein Feld auf einer der beiden Seiten, sondern eine Liste
von `Einbau`-Sätzen. Ein **offener** Einbau — ohne `ausgebautAm` — bedeutet „aktuell eingebaut", und
ein Radsatz hat höchstens einen davon.

### Die Radsatznummer ist kein Schlüssel

**Der wichtigste Satz dieses Dokuments.** Anders als die Wagennummer hat die Radsatznummer

- **keine Prüfziffer** — ein Zahlendreher ist nicht erkennbar;
- **kein europaweit einheitliches Format** — sie wird vom Halter oder von der Werkstatt vergeben;
- **keine garantierte Eindeutigkeit** — zwei Werkstätten können dieselbe Zeichenfolge völlig
  berechtigt für verschiedene Radsätze führen.

Genormt ist die Kennzeichnung der **Welle**: nach **EN 13261** trägt sie Herstellerzeichen,
Stahlsorte, Schmelze/Charge und Seriennummer. Die Radsatznummer darüber ist Hauskonvention.

Daraus folgt für das Modul: **die Alias-Tabelle ist nach Absender getrennt.** Ein Alias bedeutet
nicht „dieser Radsatz heißt auch X", sondern **„DIESER ABSENDER nennt diesen Radsatz X"**:

| Beleg | Ergebnis |
| --- | --- |
| Alias für **diesen** Absender bestätigt | `Known` — ohne Rückfrage |
| gleiche Nummer, aber **kein** Alias für diesen Absender | `Ambiguous` — Rückfrage, nie stille Zusammenführung |
| unterscheidet sich nur in **führenden Nullen** | `Likely` — Vorschlag, nie Treffer |
| nichts | `New` |

Wer bestätigt, lehrt den Alias für **seinen** Absender; der erste Absender bleibt unberührt. Wer
stattdessen „neu" wählt, hinterlässt **zwei Radsätze mit demselben `matchKey`** unter verschiedenen
Absendern — genau die reale Lage, und der Grund, warum ein Schlüssel im Index auf **mehrere** Ids
zeigt.

**Eine abweichende Ziffer ist ein anderer Radsatz**, und nichts darf etwas anderes vorschlagen — das
setzte die falsche Historie unter einen Wagen. Die einzige Ausnahme sind **führende Nullen**: das ist
eine Formatierungs-, keine Ziffernfrage (Excel frisst sie von selbst), und deshalb ein Vorschlag.

Der Absender ist die Werkstatt der **Importvorlage**, ersatzweise die im Zeilen-Import **bestätigte**
Werkstatt. Ist er unbekannt, entscheidet eine blanke Nummer nie.

Die **Radsatzwellennummer** wird erfasst, weil sie der einzige annähernd globale Bezeichner ist, den
ein Radsatz hat — sie **entscheidet aber nichts**, solange kein reales Absenderformat vorliegt.

Die **Radsatz-ID** (`systemId`) ist die Nummer, unter der das System **eines Absenders** den Radsatz
führt — in einem realen Radsatzmonitoring eine neunstellige Zahl, eindeutig je Zeile. Eindeutig ist
sie nur **in diesem einen System**; ein zweiter Absender vergibt seine eigene. Deshalb wird sie wie
die Wellennummer **gespeichert, füllt nur eine Lücke und entscheidet nichts**. Im Code heißt sie
`systemId` und nicht `radsatzId`, weil `radsatzId` schon der Verweis von `Einbau` und
`Instandhaltung` auf den Radsatz ist.

### Einbauposition

Wie die vier Positionen benannt werden, ist Hauskonvention: `1`–`4` durchgezählt, `DG1/RS2` nach
Drehgestell und Radsatz, `A1/B2`. **Deshalb Freitext** — eine Aufzählung wäre die falsche
Behauptung, dass es eine gültige Liste gibt.

### Was gemessen wird

Ein Werkstattbericht nennt typischerweise: **Laufkreisdurchmesser**, **Spurkranzdicke (Sd)**,
**Spurkranzhöhe (Sh)**, **qR-Maß**, **Radsatzinnenmaß**, **Hohllaufmaß**; als Befunde
**Flachstellen**, **Ausbröckelungen** und thermische Risse.

**Reprofilieren** auf der Unterflurdrehmaschine stellt das Profil wieder her und kostet dabei
Durchmesser. Ein Radsatz hat damit eine **endliche Zahl von Abdrehungen**, bis das Schrottmaß
erreicht ist. Dazu kommen wiederkehrende zerstörungsfreie Prüfungen der Welle.

⚠ prüfen — **konkrete Grenzmaße stehen hier bewusst nicht.** Verbindlich sind der VPI-EMG und die
jeweilige Instandhaltungsanweisung des Fahrzeugs; sie unterscheiden sich nach Bauart.

Ein Hinweis für die Auswertung: der Umstieg auf **Verbundstoff-Bremssohlen (K- und LL-Sohlen)** im
Zuge der TSI Lärm hat das Verschleiß- und Schadensbild an der Lauffläche verändert. Erfahrungswerte
aus der Zeit der Graugusssohlen lassen sich nicht ungeprüft fortschreiben.

---

## 8. Instandhaltung

**Instandhaltung ist der Oberbegriff.** Nach **DIN 31051** (begrifflich abgestimmt mit
**EN 13306**) umfasst sie vier Grundmaßnahmen:

| Begriff | Bedeutung |
| --- | --- |
| **Wartung** | Verzögerung des Abbaus des Abnutzungsvorrats — Pflege, Schmierung, Nachstellen |
| **Inspektion** | Feststellung und Beurteilung des Ist-Zustands |
| **Instandsetzung** | Rückführung in den funktionsfähigen Zustand — die Reparatur |
| **Verbesserung** | Steigerung der Zuverlässigkeit ohne Änderung der Funktion |

Deshalb heißt die Entität `Instandhaltung` und nicht `Wartung`: **Wartung ist nur eine der vier**,
und ein importierter Datensatz kann jede davon sein.

**Was tatsächlich ankommt, sind Arbeitspositionen.** Die Quelldateien sind in aller Regel
Werkstattaufträge oder Rechnungen, aufgelöst in Zeilen. Eine Zeile trägt eine Leistung, oft einen
Betrag, manchmal ein Datum. Das ist der Grund, warum **nur die Wagennummer Pflicht ist** und
`datum` optional bleibt: viele der eintreffenden Dokumente sind nicht die Dokumentation einer
datierten Reparatur.

Als Wertevorrat für `leistung` bietet sich der **Schadenskatalog des AVV** an — er ist die
gemeinsame Sprache dafür, was gefunden wurde und wer es bezahlt.

**Ein Radsatzwechsel erzeugt drei Datensätze aus einem Werkstattbesuch**: die `Instandhaltung` (die
Rechnungszeile), einen **geschlossenen** `Einbau` für den ausgebauten und einen **offenen** für den
eingebauten Radsatz. Eine Zeile, die einen Radsatz lediglich **nennt**, dokumentiert keinen Wechsel
— erst ein Ein- oder Ausbaudatum ist eine Bewegung.

### Fristen in der Master-Datei des Kunden

Der Kunde **disponiert** seine Wagen selbst: er plant, wann welcher Wagen in welche Werkstatt geht,
und prüft und korrigiert die Rechnungen. Die Master-Datei nennt dafür diese wiederkehrenden Arbeiten
(Stand 2026-10-04, gemessen in der echten Datei):

| Begriff | Bedeutung | Bezug |
| --- | --- | --- |
| **Revision G4.x** | Hauptuntersuchung des Wagens; Zyklus in der Datei 72 Monate (`ZYKLUS_REV`), nächste Stufe z. B. „G 4.0“ | Wagen |
| **P8** | **Jährliche Inspektion** je Wagen (Martin, 2026-10-07), mit festem Leistungsumfang und vereinbarten Preisen; erste P8 ein Jahr nach der Revision. Im Modell eine `Pruefung` mit `art` „P8“ | Wagen |
| **KP-P** | Kesselprüfung; in der Rechnungsaufteilung zusammen mit P8 abgerechnet | Wagen |
| **RID-Frist** | Prüffrist für Gefahrgutwagen (RID); im Bestand des ersten Kunden leer, das Modell muss sie trotzdem tragen | Wagen |
| **AL-RS 2 Jahre** | Ein Radsatz mit „AL“-Nummer braucht innerhalb von zwei Jahren nach Einbau eine IS2/3 | Radsatz |
| **IS-13-Jahre-Limit** | Zeitgrenze für die Radsatz-Instandsetzung | Radsatz |
| **Status G / A** | Bestellstand einer Frist im Portal-Export: **G = geplant** (noch keine Bestellung), **A = aktuell** (bestellt, Bestellnummer und Erfassungsdatum vorhanden) | Frist |

Die Kette, über die Bestellnummer verbunden: Frist (G) → Bestellung (A) → Werkstatteingang → Rechnung.
Fälligkeiten werden gespiegelt, **nicht berechnet** — die Zyklusregeln je Fristart gehören erst dem
Schattensystem als führendem System (v3).

**Seit 2026-10-07 ist jede dieser Fristen eine `Pruefung`** (KISS: eine Entität für alle Arten),
importiert aus einer Datei wie jede andere. Die Art steht in einer Spalte oder, wenn die ganze Datei
eine Art ist wie die P8-Liste, fest an der Vorlage (`ImportPlan.pruefart`). Die Radsatz-Fristen
(AL-RS, IS 13) gehören nicht dazu — sie hängen am Radsatz, nicht am Wagen.

---

## 9. Normen und Regelwerke

| Regelwerk | Inhalt |
| --- | --- |
| **EN 15313** | **Radsätze im Betrieb** — Anforderungen an Betrieb und Instandhaltung. Das für dieses Modul einschlägige |
| EN 13260 | Radsätze — Produktanforderungen |
| EN 13261 / EN 13262 | Radsatzwellen / Räder, inkl. Kennzeichnung der Welle |
| EN 13715 | Radprofile (S1002, EPS) |
| EN 13979-1 | Vollräder, technische Zulassung |
| EN 12080 / 12081 / 12082 | Radsatzlager, Schmierfette, Lagergehäuse |
| EN 15437 | Zustandsüberwachung der Radsatzlager (Heißläuferortung) |
| UIC 510-2, UIC 812-3 | Räder und Radsätze für Wagen; Radsatzwellen |
| ISO 1005-3 / -6 / -7 | Wellen / Radsätze / Räder — außerhalb Europas noch in Bezug |
| ISO/TS 22163 (IRIS) | Qualitätsmanagement der Bahnindustrie — worauf ein Lieferant zertifiziert ist |
| **AVV / GCU** | Allgemeiner Vertrag über die Verwendung von Güterwagen, inkl. **Schadenskatalog** |
| **VPI-EMG** | VPI European Maintenance Guide — das praktische Instandhaltungsregelwerk für Privatgüterwagen, mit eigenem Teil zu Radsätzen |
| VO (EU) 2019/779 | ECM-Zertifizierung, vier Funktionen |
| TSI WAG, TSI Lärm | Interoperabilität Güterwagen; Lärm (Bremssohlen) |

⚠ prüfen — **Anlagennummern des AVV und Teilnummern des VPI-EMG sind hier bewusst nicht genannt.**
Sie haben sich zwischen Ausgaben verschoben; gegen die jeweils aktuelle Fassung prüfen, bevor eine
davon in Code oder Oberfläche auftaucht.

---

## 10. Was bewusst nicht modelliert ist

Kein Versehen, sondern Zuschnitt. Wenn eines davon gebraucht wird, ist es eine Entscheidung und
gehört nach [decisions.md](./decisions.md). **Seit 2026-10-04 ist der Master-Spiegel diese
Entscheidung für mehrere Punkte**: was der Kunde in seiner Master-Datei pflegt, braucht er, und wird
in Phasen gespiegelt — die Punkte unten sagen, welche.

- **Zustands- und Verschleißdaten** — Durchmesser, Sd/Sh/qR, Zahl der Abdrehungen, Schrottmaß,
  letzte Ultraschallprüfung. Erfordert Einheiten, Plausibilitätsbereiche und Spalten, die Absender
  selten einheitlich füllen. *Geplant aus dem Master (Phase 5): LKD, Laufleistung, Restlauftage je
  Radsatz mit Stichtag.*
- **Fristen und Instandhaltungsstufen** — *seit 2026-10-07 als `Pruefung` modelliert* (P8,
  Revision G4.x, …), gespiegelt und nicht berechnet; siehe §8. Die Zyklusregeln bleiben draußen.
- **Miet- und Vertragsdaten** — wer welchen Wagen wie lange gemietet hat.
- **Schadensregulierung nach AVV** — Schadensursache, Kostenzuordnung, Haftung.
- **ECM-Nachweisführung** — das Modul verwaltet Daten, es ist kein zertifiziertes
  Instandhaltungssystem.
- **Werkstattaufträge (Bestellungen)** — *seit 2026-10-07 als `Werkstattauftrag` modelliert*,
  Schlüssel Wagen + Bestellnummer; der Rest dieses Punkts ist die Vorgeschichte. Bestellnummer, Status (`erfasst → zugestellt →
  ausgeführt`), Eingang und Ausgang. Ein Auftrag ändert sich, eine Instandhaltung nicht. Bis auf
  Weiteres wird eine Auftragsliste nur als Quelle **abgeschlossener** Instandhaltungen gelesen
  (Datum = Werkstattausgang); ein eigenes `Auftrag` mit der Bestellnummer als Schlüssel ist
  zurückgestellt, bis eine echte Rechnungsdatei zeigt, dass Rechnung und Auftrag über sie
  zusammenfinden. Siehe [decisions.md](./decisions.md). *Geplant aus dem Master (Phase 3) als
  `Werkstattauftrag`; die Rechnung findet über Bestellnummer + Wagennummer zum Auftrag (§3).*

Der Master-Spiegel nach Phasen, entlang der drei Aufgaben des Kunden:

- **Phase 1, umgesetzt:** Wagen, Halter, Radsätze und Einbauten (mit und ohne Position).
- **Phase 2 — wo ist der Wagen, wie ist sein Zustand:** Wagenmeldung (die Handspalten und
  Handfarben des Dashboards, Meldelisten, Checklisten), Telematik-Gerät, Werkstatteingang.
  *Modelliert 2026-10-07 als Wagen-Zustand* (`TelematikGeraet`, `TelematikMeldung`,
  `Schadensmeldung`, `Werkstattauftrag`, `Pruefung`), importiert aus Absenderdateien — noch nicht
  aus dem Master selbst.
- **Phase 3 — stimmt die Rechnung:** Werkstattauftrag, Rechnungsaufteilung (je Blatt ein
  Reparaturfall), Leistungskatalog mit Sollpreisen, Rechnungsrücksendungen.
- **Phase 4 — wer muss wann in die Werkstatt:** Frist, Werkstattbedarf, Flottenbedarf, Standorte
  mit Ansprechpartnern.
- **Phase 5:** Bauteile mit Seriennummer (Bremseinheiten, Federlenker mit Gewährleistung),
  Radsatz-Messwerte, Wagen-Stammdaten (Gattung, Baujahr, ECM, Vertrag).

Bewusst weiter **nicht**: eine Achszahl je Wagen, ob gespeichert oder aus offenen Einbauten
abgeleitet — siehe [decisions.md](./decisions.md).
