// ─── why ────────────────────────────────────────────────────────
// Step one of the master update: the sheet the document goes into. The client
// updates ONE sheet per document, so only the sheet bound to the document's
// template is shown — Rust's `suggested`, already the only pre-ticked one — and
// nothing is chosen here. The other sheets stay in the start data and are simply
// not offered; with no bound sheet the step names where to bind one and Weiter
// stays dead. The base is always the client master's current version, so it is
// named, not offered.
//
// „Neue Zeilen anhängen“: the update is incremental either way, the toggle only
// decides whether a key the sheet lacks becomes a row. „Fehlende Zeilen leeren“
// is the opposite direction — a sheet row whose key the document lacks is
// emptied, never moved — and starts where it was last left for this template:
// whether a document is complete is the user's knowledge, not the code's.
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
import { warningOutline } from 'ionicons/icons';
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
    addIcons({ warningOutline });
  }

  protected onRemove(sheet: string, event: Event): void {
    this.facade.setRemove(
      sheet,
      (event as CustomEvent<{ checked: boolean }>).detail.checked
    );
  }

  protected onAppend(sheet: string, event: Event): void {
    this.facade.setAppend(
      sheet,
      (event as CustomEvent<{ checked: boolean }>).detail.checked
    );
  }

  protected async onBack(): Promise<void> {
    this.facade.finish();
    await this.#router.navigate(['/trains/documents']);
  }

  protected async onNext(): Promise<void> {
    const ok = await this.#reports.run(() => this.facade.run());
    if (ok) await this.#router.navigate(['/trains/master/export/structure']);
  }
}
