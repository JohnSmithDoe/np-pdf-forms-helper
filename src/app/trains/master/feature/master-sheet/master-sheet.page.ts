// ─── why ────────────────────────────────────────────────────────
// One bound master sheet, read-only, as the Schattensystem holds it — to be
// put beside the workbook and compared by eye. Rust builds the whole view
// (backend for frontend); this page picks the sheet, asks, and renders.
//
// The sheet is the ROUTE PARAM, the customer's own sheet name, so a view can
// be reached and reloaded directly and the segment above it only navigates.
// The segment lists the BOUND sheets in binding order: an unbound sheet has no
// kind and no template, so there is nothing to project onto it.
//
// The view is held by the page and not by the store: up to 1,700 rows of
// display strings that nothing else reads. An answer for a sheet the user has
// already left is dropped, or a slow big sheet would overwrite a small one
// picked after it.
//
// The search filters on the row KEY only — the formatted Wagennummer (plus the
// Radsatznummer on radsatz rows), which is what one looks up in Excel too.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  signal,
  untracked,
} from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, Router } from '@angular/router';
import {
  IonBackButton,
  IonButtons,
  IonChip,
  IonContent,
  IonHeader,
  IonIcon,
  IonItem,
  IonLabel,
  IonNote,
  IonSearchbar,
  IonSegment,
  IonSegmentButton,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import { warningOutline } from 'ionicons/icons';
import { ReportPresenterService } from '../../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../../@shared/ui/busy-overlay/busy-overlay.component';
import { TrainsFacade } from '../../../data';
import type { MasterSheetView } from '../../../model/trains.types';
import { SheetTableComponent } from '../../ui/sheet-table/sheet-table.component';

@Component({
  selector: 'app-page-trains-master-sheet',
  templateUrl: 'master-sheet.page.html',
  styleUrls: ['master-sheet.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'ion-page' },
  imports: [
    BusyOverlayComponent,
    IonBackButton,
    IonButtons,
    IonChip,
    IonContent,
    IonHeader,
    IonIcon,
    IonItem,
    IonLabel,
    IonNote,
    IonSearchbar,
    IonSegment,
    IonSegmentButton,
    IonTitle,
    IonToolbar,
    SheetTableComponent,
  ],
})
export class MasterSheetPage {
  protected readonly facade = inject(TrainsFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #router = inject(Router);
  readonly #params = toSignal(inject(ActivatedRoute).paramMap);

  protected readonly sheet = computed(() => this.#params()?.get('sheet') ?? '');
  protected readonly sheets = computed(() =>
    (this.facade.master()?.settings.bindings ?? [])
      .map((binding) => binding.sheet)
      .filter((sheet) => sheet !== '')
  );

  protected readonly view = signal<MasterSheetView | undefined>(undefined);
  readonly #term = signal('');

  protected readonly rows = computed(() => {
    const rows = this.view()?.rows ?? [];
    const term = this.#term().trim().toLowerCase();
    return term
      ? rows.filter((row) => row.key.toLowerCase().includes(term))
      : rows;
  });

  constructor() {
    addIcons({ warningOutline });
    if (!this.facade.master()) {
      void this.#reports.run(() => this.facade.loadMaster());
    }
    effect(() => {
      const sheet = this.sheet();
      untracked(() => void this.#load(sheet));
    });
  }

  protected onSheet(event: Event): void {
    const sheet = (event as CustomEvent<{ value?: string }>).detail.value;
    if (!sheet || sheet === this.sheet()) return;
    void this.#router.navigate(['/trains/master/sheets', sheet], {
      replaceUrl: true,
    });
  }

  protected onSearch(event: Event): void {
    this.#term.set(
      (event as CustomEvent<{ value?: string | null }>).detail.value ?? ''
    );
  }

  async #load(sheet: string): Promise<void> {
    this.view.set(undefined);
    if (!sheet) return;
    await this.#reports.run(async () => {
      const view = await this.facade.masterSheet(sheet);
      if (sheet === this.sheet()) this.view.set(view);
    });
  }
}
