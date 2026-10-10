// ─── why ────────────────────────────────────────────────────────
// Step one of the master update, and the only one before the preview: the
// sheet the document goes into and what is wrong with it. The client updates
// ONE sheet per document, so only the sheet bound to the document's template
// is shown — Rust's `suggested`, the only pre-ticked one — and nothing is
// chosen. With no bound sheet the step names where to bind one.
//
// The column mapping is the template's and is not edited here: the step states
// the KEY pair read-only, because the update matches rows by it, and lists
// what the dry run found wrong — a sheet that cannot be written (danger, Weiter
// dead), a remembered alias or key the sheet lost, document columns the sheet
// has no column for (not transferred). The dry run already ran in `begin`.
//
// „Neue Zeilen anhängen“: the update is incremental either way, the toggle only
// decides whether a key the sheet lacks becomes a row. „Fehlende Zeilen leeren“
// is the opposite direction — a sheet row whose key the document lacks is
// emptied, never moved — and starts where it was last left for this template:
// whether a document is complete is the user's knowledge, not the code's. Each
// toggle re-runs the dry run.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
} from '@angular/core';
import { Router } from '@angular/router';
import {
  IonIcon,
  IonItem,
  IonLabel,
  IonList,
  IonListHeader,
  IonNote,
  IonToggle,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { alertCircleOutline, warningOutline } from 'ionicons/icons';
import { ReportPresenterService } from '../../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../../@shared/ui/busy-overlay/busy-overlay.component';
import { WizardShellComponent } from '../../../../@shared/ui/wizard-shell/wizard-shell.component';
import { MasterExportFacade } from '../../../data';
import { EXPORT_PHASE, EXPORT_STEPS } from '../../../model/master-export';

@Component({
  selector: 'app-page-export-sheets',
  templateUrl: 'export-sheets.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    IonIcon,
    IonItem,
    IonLabel,
    IonList,
    IonListHeader,
    IonNote,
    IonToggle,
    WizardShellComponent,
  ],
})
export class ExportSheetsPage {
  protected readonly facade = inject(MasterExportFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);

  protected readonly phase = EXPORT_PHASE;
  protected readonly steps = EXPORT_STEPS;

  protected readonly sheets = computed(() =>
    this.facade.sheets().filter((view) => view.sheet.suggested)
  );

  constructor() {
    addIcons({ alertCircleOutline, warningOutline });
  }

  protected onRemove(sheet: string, event: Event): void {
    const on = (event as CustomEvent<{ checked: boolean }>).detail.checked;
    void this.#reports.run(() => this.facade.setRemove(sheet, on));
  }

  protected onAppend(sheet: string, event: Event): void {
    const on = (event as CustomEvent<{ checked: boolean }>).detail.checked;
    void this.#reports.run(() => this.facade.setAppend(sheet, on));
  }

  protected async onBack(): Promise<void> {
    this.facade.finish();
    await this.#router.navigate(['/trains/documents']);
  }

  protected async onNext(): Promise<void> {
    await this.#router.navigate(['/trains/master/export/preview']);
  }
}
