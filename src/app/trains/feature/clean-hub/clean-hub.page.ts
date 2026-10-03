// ─── why ────────────────────────────────────────────────────────
// The cleaning's front door: drop a folder (or click and pick one, or pick
// files), see what each file was recognised as, correct that where needed,
// start. This is phase one of two — it ends in a batch summary and imports
// nothing — which is why the header carries the „Bereinigen“ chip.
//
// TWO PICKERS, NOT ONE. Windows' native dialog picks files OR a folder, never
// both, so the drop zone opens the folder dialog and „Dateien wählen“ the file
// dialog. A drop accepts either, which is why the zone is the big target.
//
// The drop listener lives as long as the page does, but a drop only COUNTS
// while this page is the one shown. Ionic keeps a page alive in its stack after
// navigating forward, so without the `ionViewDidEnter`/`ionViewWillLeave` gate a
// folder dropped onto the review screen would silently replace the list the
// walk is running over.
//
// The list shows what the backend recognised and asks only where it must: a
// single match is preselected, several leave the select EMPTY and the file
// cannot start until the user picks — the first of two matches would be a
// guess. „Vorlage anlegen“ is always on offer, so a wrong recognition is never
// a dead end, and neither is a file the user does not want at all: „Nicht
// bereinigen“ answers the row. A file the app already owns is listed as
// `vorhanden` with its dates and cannot be picked; unsupported and unreadable
// files are listed with their reason, so the user sees they were looked at.
//
// „Starten“ walks the list one file at a time because the backend holds one
// cleaning. A file that needs a template is handed to the mapper; coming back
// here (`ionViewDidEnter`) rescans exactly that file, which the template just
// saved now recognises, and the button reads „Fortsetzen“. When nothing is
// left, the footer leads to the batch summary instead.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  inject,
  signal,
} from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { Router } from '@angular/router';
import {
  IonBackButton,
  IonButton,
  IonButtons,
  IonCard,
  IonCardContent,
  IonCardHeader,
  IonCardSubtitle,
  IonCardTitle,
  IonChip,
  IonContent,
  IonFooter,
  IonHeader,
  IonIcon,
  IonItem,
  IonItemGroup,
  IonLabel,
  IonList,
  IonListHeader,
  IonNote,
  IonSelect,
  IonSelectOption,
  IonTitle,
  IonToolbar,
  type ViewDidEnter,
  type ViewWillLeave,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import {
  createOutline,
  documentOutline,
  documentsOutline,
  folderOpenOutline,
} from 'ionicons/icons';
import type { FileDrop } from '../../../@shared/data/backend/backend.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import {
  IntakeFacade,
  type FileOutcome,
  type IntakeRow,
  type IntakeStep,
} from '../../data';

const STEP_ROUTES: Record<IntakeStep, string> = {
  clean: '/trains/clean/file',
  template: '/trains/clean/template',
  summary: '/trains/clean/summary',
};
import type { ScanStatus } from '../../model/trains.types';

interface Chip {
  label: string;
  color: string;
}

const STATUS: Record<ScanStatus, Chip> = {
  erkannt: { label: 'erkannt', color: 'success' },
  mehrdeutig: { label: 'mehrdeutig', color: 'warning' },
  unbekannt: { label: 'unbekannt', color: 'medium' },
  nichtUnterstuetzt: { label: 'nicht unterstützt', color: 'medium' },
  unlesbar: { label: 'unlesbar', color: 'danger' },
  vorhanden: { label: 'vorhanden', color: 'medium' },
};

const OUTCOME: Record<FileOutcome, Chip> = {
  bereinigt: { label: 'bereinigt', color: 'success' },
  verworfen: { label: 'verworfen', color: 'medium' },
  fehlgeschlagen: { label: 'fehlgeschlagen', color: 'danger' },
};

const SKIPPED: Chip = { label: 'übersprungen', color: 'medium' };

@Component({
  selector: 'app-page-clean-hub',
  templateUrl: 'clean-hub.page.html',
  styleUrls: ['clean-hub.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'ion-page' },
  imports: [
    BusyOverlayComponent,
    IonBackButton,
    IonButton,
    IonButtons,
    IonCard,
    IonCardContent,
    IonCardHeader,
    IonCardSubtitle,
    IonCardTitle,
    IonChip,
    IonContent,
    IonFooter,
    IonHeader,
    IonIcon,
    IonItem,
    IonItemGroup,
    IonLabel,
    IonList,
    IonListHeader,
    IonNote,
    IonSelect,
    IonSelectOption,
    IonTitle,
    IonToolbar,
  ],
})
export class CleanHubPage implements ViewDidEnter, ViewWillLeave {
  protected readonly facade = inject(IntakeFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);

  readonly #active = signal(false);
  protected readonly hovering = signal(false);

  constructor() {
    addIcons({
      createOutline,
      documentOutline,
      documentsOutline,
      folderOpenOutline,
    });
    this.facade.fileDrops$
      .pipe(takeUntilDestroyed(inject(DestroyRef)))
      .subscribe((drop) => void this.#onDrop(drop));
  }

  ionViewDidEnter(): void {
    this.#active.set(true);
    void this.#reports.run(() => this.facade.resume());
  }

  ionViewWillLeave(): void {
    this.#active.set(false);
    this.hovering.set(false);
  }

  protected chip(row: IntakeRow): Chip {
    if (row.result) return OUTCOME[row.result.outcome];
    return row.skipped ? SKIPPED : STATUS[row.file.status];
  }

  protected editable(row: IntakeRow): boolean {
    return (
      row.selectable &&
      (row.result === undefined || row.result.outcome === 'fehlgeschlagen')
    );
  }

  protected async onPickFolder(): Promise<void> {
    await this.#reports.run(() => this.facade.pickFolder());
  }

  protected async onPickFiles(): Promise<void> {
    await this.#reports.run(() => this.facade.pickFiles());
  }

  protected onChoose(row: IntakeRow, event: Event): void {
    const value = (event as CustomEvent<{ value?: string }>).detail.value;
    this.facade.choose(row.file.path, value);
  }

  protected onClear(): void {
    this.facade.clear();
  }

  protected onTemplate(): void {
    void this.#router.navigate(['/trains/clean/template']);
  }

  protected onSummary(): void {
    void this.#router.navigate(['/trains/clean/summary']);
  }

  protected async onStart(): Promise<void> {
    let step = 'summary' as IntakeStep;
    const ok = await this.#reports.run(async () => {
      step = await this.facade.startNext();
    });
    if (ok) await this.#router.navigate([STEP_ROUTES[step]]);
  }

  async #onDrop(drop: FileDrop): Promise<void> {
    if (!this.#active()) return;
    if (drop.type !== 'drop') {
      this.hovering.set(drop.type === 'enter');
      return;
    }
    this.hovering.set(false);
    await this.#reports.run(() => this.facade.scanPaths(drop.paths));
  }
}
