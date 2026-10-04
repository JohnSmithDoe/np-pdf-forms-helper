// ─── why ────────────────────────────────────────────────────────
// The last step: the new version of the client master that was written, and per
// sheet which cells changed — the summary of the update. Read
// from the written run rather than the preview, because the write ran the
// paste again and this is what it did.
//
// „Fertig“ leads back to the document list, where the next document starts the
// update; that one builds on this version, because it is now the current one.
// ────────────────────────────────────────────────────────────────

import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { Router } from '@angular/router';
import {
  IonButton,
  IonCard,
  IonCardContent,
  IonCardHeader,
  IonCardSubtitle,
  IonCardTitle,
  IonIcon,
  IonNote,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { documentOutline, folderOpenOutline } from 'ionicons/icons';
import { ReportPresenterService } from '../../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../../@shared/ui/busy-overlay/busy-overlay.component';
import { WizardShellComponent } from '../../../../@shared/ui/wizard-shell/wizard-shell.component';
import { MasterExportFacade } from '../../../data';
import { EXPORT_PHASE, EXPORT_STEPS } from '../../../model/master-export';
import { CellChangesComponent } from '../../ui/cell-changes/cell-changes.component';

@Component({
  selector: 'app-page-export-result',
  templateUrl: 'export-result.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    CellChangesComponent,
    IonButton,
    IonCard,
    IonCardContent,
    IonCardHeader,
    IonCardSubtitle,
    IonCardTitle,
    IonIcon,
    IonNote,
    WizardShellComponent,
  ],
})
export class ExportResultPage {
  protected readonly facade = inject(MasterExportFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);

  protected readonly phase = EXPORT_PHASE;
  protected readonly steps = EXPORT_STEPS;

  constructor() {
    addIcons({ documentOutline, folderOpenOutline });
  }

  protected name(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  protected onOpenFile(path: string): void {
    void this.#reports.run(() => this.facade.openFile(path));
  }

  protected onOpenFolder(folder: string): void {
    void this.#reports.run(() => this.facade.openFolder(folder));
  }

  protected async onDone(): Promise<void> {
    this.facade.finish();
    await this.#router.navigate(['/trains/documents']);
  }
}
