// ─── why ────────────────────────────────────────────────────────
// What the cleaning of the master changed, sheet by sheet — so the user can
// judge it before taking the version over. Only sheets with something to say
// are listed; the rest are one count, because on the real master most of 28
// sheets are byte-identical copies and a list of „nichts geändert“ would bury
// the dozen that are not.
//
// Notes come before examples: a note is something the cleaning DID NOT do and
// the user may want to fix in Excel; an example is something it did.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
} from '@angular/core';
import {
  IonChip,
  IonItem,
  IonLabel,
  IonList,
  IonListHeader,
  IonNote,
} from '@ionic/angular/standalone';
import {
  RULE_LABELS,
  type MasterFileReport,
  type MasterFileSheet,
} from '../../model/master-file';

@Component({
  selector: 'app-master-file-report',
  templateUrl: 'master-file-report.component.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [IonChip, IonItem, IonLabel, IonList, IonListHeader, IonNote],
})
export class MasterFileReportComponent {
  readonly report = input.required<MasterFileReport>();

  protected readonly labels = RULE_LABELS;

  protected readonly changed = computed(() =>
    this.report().sheets.filter((sheet) => touched(sheet))
  );

  protected readonly unchanged = computed(
    () => this.report().sheets.length - this.changed().length
  );
}

function touched(sheet: MasterFileSheet): boolean {
  return (
    sheet.rowsCut > 0 ||
    sheet.tailRowsCut > 0 ||
    sheet.trimmed > 0 ||
    sheet.numbers > 0 ||
    sheet.dates > 0 ||
    sheet.noteCount > 0 ||
    sheet.formulaTail !== undefined
  );
}
