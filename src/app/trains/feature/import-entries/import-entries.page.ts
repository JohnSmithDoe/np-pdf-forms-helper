// ─── why ────────────────────────────────────────────────────────
// Step four of the import walk: the rows themselves, as the Einbauten and
// Instandhaltungen they will write. The entities were decided on the steps
// before; what is left to decide here is only WHICH rows are taken.
//
// A row is shown even when it will not be written — a declined Wagen took it,
// it was rejected while staging, or it repeats one already in the store — with
// the reason, because a row that silently vanished would look like the import
// lost it. Only a duplicate can be ticked back in: the user may legitimately
// have two identical repairs. A rejected row has no clean Wagennummer and a
// row without its Wagen has nothing to hang off.
//
// „Alle auswählen“ / „Alle abwählen“ act on every row that CAN be ticked —
// never on a rejected row or one whose Wagen was declined.
//
// What a row WRITES is read off its cells for display only. Whether the
// commit records an Einbau or an Instandhaltung is decided again in Rust, so
// this line describes and cannot authorise.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import {
  IonButton,
  IonCheckbox,
  IonChip,
  IonItem,
  IonLabel,
  IonList,
  IonNote,
} from '@ionic/angular/standalone';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { WizardShellComponent } from '../../../@shared/ui/wizard-shell/wizard-shell.component';
import { ImportWalkFacade, type EntryView } from '../../data';
import { IMPORT_PHASE, IMPORT_STEPS } from '../../model/import-walk';

@Component({
  selector: 'app-page-import-entries',
  templateUrl: 'import-entries.page.html',
  styleUrls: ['import-entries.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    IonButton,
    IonCheckbox,
    IonChip,
    IonItem,
    IonLabel,
    IonList,
    IonNote,
    WizardShellComponent,
  ],
})
export class ImportEntriesPage {
  protected readonly facade = inject(ImportWalkFacade);
  readonly #router = inject(Router);

  protected readonly phase = IMPORT_PHASE;
  protected readonly steps = IMPORT_STEPS;

  protected line(entry: EntryView): string {
    return entry.row.cells
      .filter((cell) => cell.field !== 'ignorieren' && cell.raw.trim() !== '')
      .map((cell) => (cell.ok && cell.parsed ? cell.parsed : cell.raw))
      .join(' · ');
  }

  protected reason(entry: EntryView): string | undefined {
    if (entry.row.status === 'rejected') {
      return 'Abgelehnt: die Zeile ließ sich nicht lesen.';
    }
    if (entry.gone) return 'Entfällt: der Wagen wird nicht übernommen.';
    if (entry.row.status === 'duplicate') {
      return 'Doppelt: diese Zeile ist schon im Schattensystem.';
    }
    if (entry.writes.length === 0) {
      return 'Schreibt keinen Eintrag, nur die Zuordnungen.';
    }
    return undefined;
  }

  protected locked(entry: EntryView): boolean {
    return entry.gone || entry.row.status === 'rejected';
  }

  protected onToggle(entry: EntryView, event: Event): void {
    const checked = (event as CustomEvent<{ checked: boolean }>).detail.checked;
    this.facade.include(entry.row.row, checked);
  }

  protected onAll(include: boolean): void {
    this.facade.includeAll(include);
  }

  protected async onBack(): Promise<void> {
    await this.#router.navigate(['/trains/import/wheelsets']);
  }

  protected async onNext(): Promise<void> {
    await this.#router.navigate(['/trains/import/summary']);
  }
}
