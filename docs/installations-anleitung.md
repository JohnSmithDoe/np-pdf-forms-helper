## Installationsanleitung

npDokumentenhilfe ist **eine einzelne Programmdatei**. Sie braucht keine Installation, keine
Administratorrechte und keine Netzwerkverbindung. Alle Daten liegen im Ordner `data` neben dem
Programm.

### Schritt 1: Herunterladen

Auf der [Release-Seite](https://github.com/JohnSmithDoe/np-pdf-forms-helper/releases/latest) die
Datei `npDokumentenhilfe_<Version>_x64-portable.exe` herunterladen.

### Schritt 2: Ablegen

1. Auf dem **eigenen Laufwerk** (meist das Home-Laufwerk, z. B. `H:`) einen Ordner
   `npDokumentenhilfe` anlegen.
2. Die heruntergeladene Datei dorthin verschieben und in `npDokumentenhilfe.exe` umbenennen.
   Dann bleibt eine Verknüpfung auch nach einem Update gültig.

> **Wichtig unter Citrix:** Nicht auf dem Desktop, in „Downloads“ oder auf `C:` ablegen. Diese
> Orte werden je nach Einrichtung beim Abmelden gelöscht — und mit ihnen alle Daten.

> **Jede Person braucht ihre eigene Kopie.** Starten mehrere Personen dieselbe Datei aus einem
> gemeinsamen Ordner, schreiben sie in dieselbe Datenbank, und Änderungen gehen ohne Fehlermeldung
> verloren.

### Schritt 3: Starten

Die Datei per **Doppelklick im Explorer** starten. Beim ersten Start legt das Programm daneben den
Ordner `data` an.

Für eine Verknüpfung: Rechtsklick auf die Datei → „Verknüpfung erstellen“. In den Eigenschaften der
Verknüpfung muss **„Ausführen in“** der Ordner sein, in dem die Datei liegt — das Programm legt
`data` dort an, wo es gestartet wird.

Unter **Info** zeigt das Programm, welche Ordner es tatsächlich verwendet. Ein Blick dorthin nach
dem ersten Start bestätigt, dass die Daten auf dem eigenen Laufwerk liegen.

Zeigt Windows beim ersten Start „Der Computer wurde durch Windows geschützt“, über „Weitere
Informationen“ → „Trotzdem ausführen“ starten. Startet das Programm gar nicht, blockiert
vermutlich eine Richtlinie Programme auf diesem Laufwerk — dann bitte die IT ansprechen.

### Update

Die neue Datei herunterladen, in `npDokumentenhilfe.exe` umbenennen und die alte damit
**ersetzen**. Den Ordner `data` nicht anfassen — er enthält alle Dokumente, Profile und das
Schattensystem.

### Sicherung

Gesichert werden muss nur der Ordner `data`. Wer ihn kopiert, hat alles.

Die Master-Datei sichert das Programm selbst. Vor jedem „In Master übertragen“ legt es eine Kopie
des bisherigen Stands im Ordner `Sicherungen` neben der Master-Datei an, mit Datum und Uhrzeit im
Namen. Wenn diese Kopie nicht angelegt werden kann, wird nichts geschrieben. Die Master-Datei muss
dafür in Excel geschlossen sein. Alte Sicherungen löscht das Programm nicht.

### Optional: Datenordner woanders ablegen

Liegt im Programmordner eine Datei `.npconfig`, gelten die Pfade daraus. Jeder Eintrag ist
optional; was fehlt, liegt wie gewohnt unter `./data`. Relative Pfade gelten ab dem Ordner, aus dem
das Programm gestartet wird.

```json
{
  "DATA_PATH": "H:/npDokumentenhilfe-Daten",
  "OUTPUT_PATH": "H:/Ausgefüllte Formulare"
}
```

| Eintrag        | Standard                         | Inhalt                                                       |
| -------------- | -------------------------------- | ------------------------------------------------------------ |
| `DATA_PATH`    | `./data`                         | alle Daten, auch das Schattensystem                          |
| `OUTPUT_PATH`  | `<DATA_PATH>/out`                | die ausgefüllten Dokumente                                   |
| `CACHE_PATH`   | `<DATA_PATH>/cache`              | Vorschauen                                                   |
| `DB_FILE`      | `<DATA_PATH>/data.db`            | verknüpfte Dokumente und Feldnamen                           |
| `PROFILE_FILE` | `<DATA_PATH>/profiles.db`        | Profile                                                      |

Statt der Datei gehen auch Umgebungsvariablen: `APP_DATA`, `APP_OUTPUT`, `APP_CACHE`,
`APP_DB_FILE`, `APP_PROFILE_FILE`. Ein Eintrag in `.npconfig` hat Vorrang.

### Für die IT

- Keine Installation, keine Administratorrechte, keine Netzwerkzugriffe, keine weiteren Programme.
- Benötigt die **WebView2-Laufzeit**, die in Windows 11 enthalten ist.
- Schreibt nur in seinen Datenordner (siehe oben). Zusätzlich legt WebView2 unter
  `%LOCALAPPDATA%\de.npdokumentenhilfe.desktop` einen Browser-Cache an; dort liegen nur
  Ansichtseinstellungen. Geht er beim Abmelden verloren, ist nur die zuletzt gewählte Ansicht
  vergessen.
- Für die Ausführung von einem Home-Laufwerk muss eine Anwendungs-Whitelist (AppLocker/WDAC) die
  Datei zulassen.
