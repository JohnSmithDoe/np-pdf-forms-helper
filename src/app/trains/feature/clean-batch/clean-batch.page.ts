// ─── why ────────────────────────────────────────────────────────
// The end of the cleaning cycle: what became of every file of the batch, in
// one list. It is a complete result on its own — a user who only cleans stops
// here with every file filed — and the import is offered from it rather than
// started by it, because the import is a separate act the user may take now,
// later, or from the document list.
//
// Each filed file offers its folder, its two copies and its own „Importieren“; „Alle
// importieren“ walks the filed-but-not-imported ones one document at a time.
// Both are behind `importEnabled` (`npdh.import`), like the document list's.
// A file the app already owned before this batch is listed with that document,
// so a re-dropped folder still leads to what was made of it.
//
// „Fertig“ empties the batch: its list has served its purpose, and the
// documents themselves outlive it in the ledger.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import {
  IonButton,
  IonChip,
  IonItem,
  IonLabel,
  IonList,
  IonNote,
} from '@ionic/angular/standalone';
import { SettingsService } from '../../../@shared/data/settings/settings.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { WizardShellComponent } from '../../../@shared/ui/wizard-shell/wizard-shell.component';
import { ImportWalkFacade, IntakeFacade, type SummaryRow } from '../../data';

interface Chip {
  label: string;
  color: string;
}

const OUTCOME: Record<SummaryRow['outcome'], Chip> = {
  bereinigt: { label: 'bereinigt', color: 'success' },
  verworfen: { label: 'verworfen', color: 'medium' },
  fehlgeschlagen: { label: 'fehlgeschlagen', color: 'danger' },
  vorhanden: { label: 'vorhanden', color: 'medium' },
  uebersprungen: { label: 'übersprungen', color: 'medium' },
  offen: { label: 'offen', color: 'warning' },
};

@Component({
  selector: 'app-page-clean-batch',
  templateUrl: 'clean-batch.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    IonButton,
    IonChip,
    IonItem,
    IonLabel,
    IonList,
    IonNote,
    WizardShellComponent,
  ],
})
export class CleanBatchPage {
  protected readonly facade = inject(IntakeFacade);
  protected readonly importEnabled = inject(SettingsService).importEnabled;
  readonly #walk = inject(ImportWalkFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);

  protected chip(row: SummaryRow): Chip {
    return OUTCOME[row.outcome];
  }

  protected onOpen(path: string): void {
    void this.#reports.run(() => this.facade.openFile(path));
  }

  protected onOpenFolder(folder: string): void {
    void this.#reports.run(() => this.facade.openFolder(folder));
  }

  protected async onImport(id: string): Promise<void> {
    const ok = await this.#reports.run(() => this.#walk.start(id));
    if (ok) await this.#router.navigate(['/trains/import/partners']);
  }

  protected async onImportAll(): Promise<void> {
    const ids = this.facade.importable().map((dokument) => dokument.id);
    const ok = await this.#reports.run(() => this.#walk.startQueue(ids));
    if (ok) await this.#router.navigate(['/trains/import/partners']);
  }

  protected async onBack(): Promise<void> {
    await this.#router.navigate(['/trains/clean']);
  }

  protected async onDone(): Promise<void> {
    this.facade.clear();
    await this.#router.navigate(['/trains']);
  }
}
