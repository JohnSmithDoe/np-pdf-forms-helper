// ─── why ────────────────────────────────────────────────────────
// Step two: the column check. Only STRUCTURE asks here — a document column the
// sheet has no column for, a remembered key or alias the sheet lost. A value
// that differs from the master's is the update itself and is shown on the next
// step, never asked about.
//
// Each document column without a home gets one select: a hand-kept column of
// the sheet to write it into, or „nicht übertragen“. An answered column stays
// listed with its answer so it can be changed back. Weiter stays dead while a
// column is open or a sheet cannot be written at all — leaving it would mean
// the preview shows a sheet the export then refuses.
//
// The key select is offered on every sheet: a feed cannot run without one, and
// on a snapshot it is what lets hand-kept cells move with their row.
//
// Every select re-runs the dry run (the facade), so step three always shows
// what these answers write.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import {
  IonCard,
  IonCardContent,
  IonCardHeader,
  IonCardSubtitle,
  IonCardTitle,
  IonIcon,
  IonItem,
  IonLabel,
  IonList,
  IonListHeader,
  IonSelect,
  IonSelectOption,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { alertCircleOutline, warningOutline } from 'ionicons/icons';
import { ReportPresenterService } from '../../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../../@shared/ui/busy-overlay/busy-overlay.component';
import { WizardShellComponent } from '../../../../@shared/ui/wizard-shell/wizard-shell.component';
import { MasterExportFacade } from '../../../data';
import {
  EXPORT_PHASE,
  EXPORT_STEPS,
  MODE_LABELS,
} from '../../../model/master-export';

const IGNORE = '\u0000nicht-uebertragen';
const NO_KEY = '\u0000kein-schluessel';

@Component({
  selector: 'app-page-export-structure',
  templateUrl: 'export-structure.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    IonCard,
    IonCardContent,
    IonCardHeader,
    IonCardSubtitle,
    IonCardTitle,
    IonIcon,
    IonItem,
    IonLabel,
    IonList,
    IonListHeader,
    IonSelect,
    IonSelectOption,
    WizardShellComponent,
  ],
})
export class ExportStructurePage {
  protected readonly facade = inject(MasterExportFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);

  protected readonly phase = EXPORT_PHASE;
  protected readonly steps = EXPORT_STEPS;
  protected readonly modes = MODE_LABELS;
  protected readonly ignore = IGNORE;
  protected readonly noKey = NO_KEY;

  constructor() {
    addIcons({ alertCircleOutline, warningOutline });
  }

  protected onAnswer(sheet: string, column: string, event: Event): void {
    const value = this.#value(event);
    if (value === undefined) return;
    void this.#reports.run(() =>
      this.facade.answer(sheet, column, value === IGNORE ? undefined : value)
    );
  }

  protected onKey(sheet: string, event: Event): void {
    const value = this.#value(event);
    void this.#reports.run(() =>
      this.facade.setKey(sheet, value === NO_KEY ? undefined : value)
    );
  }

  protected async onBack(): Promise<void> {
    await this.#router.navigate(['/trains/master/export/sheets']);
  }

  protected async onNext(): Promise<void> {
    await this.#router.navigate(['/trains/master/export/preview']);
  }

  #value(event: Event): string | undefined {
    return (event as CustomEvent<{ value?: string }>).detail.value;
  }
}
