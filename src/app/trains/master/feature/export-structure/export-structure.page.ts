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
// The key is offered on every sheet as a PAIR — the document's identifier
// column and the sheet's — because the update is incremental and matches rows
// by it, so a sheet without one — or with a key the document names twice —
// cannot be written. Choosing both sides pairs them and makes them the key in
// one go; a half-chosen key is the page's draft, like a half-chosen pair. Only a sheet that takes new rows asks about
// columns; a sheet that is only updated takes what it shares.
//
// „Zuordnung“ lists every pair of a ticked sheet — document column → master
// column, by name or set by hand — and each side can be changed, a pair
// removed („nicht übertragen“) or added from the columns still free. That is
// how a column whose header is spelled differently in the master
// (`RadsatzID` → `Radsatz ID`) is fed, on any sheet and not only the one that
// asks. A new pair is sent once both sides are chosen; until then it is the
// page's own draft, not the store's.
//
// Every select re-runs the dry run (the facade), so step three always shows
// what these answers write.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  inject,
  signal,
} from '@angular/core';
import { Router } from '@angular/router';
import {
  IonButton,
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
  IonNote,
  IonSelect,
  IonSelectOption,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import {
  alertCircleOutline,
  arrowForwardOutline,
  closeOutline,
  warningOutline,
} from 'ionicons/icons';
import { ReportPresenterService } from '../../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../../@shared/ui/busy-overlay/busy-overlay.component';
import { WizardShellComponent } from '../../../../@shared/ui/wizard-shell/wizard-shell.component';
import {
  MasterExportFacade,
  type PairView,
  type StructureView,
} from '../../../data';
import {
  EXPORT_PHASE,
  EXPORT_STEPS,
  APPEND_LABELS,
} from '../../../model/master-export';

const IGNORE = '\u0000nicht-uebertragen';
const NO_KEY = '\u0000kein-schluessel';

interface Draft {
  source?: string;
  master?: string;
}

@Component({
  selector: 'app-page-export-structure',
  templateUrl: 'export-structure.page.html',
  styleUrls: ['export-structure.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    BusyOverlayComponent,
    IonButton,
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
    IonNote,
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
  protected readonly appendLabels = APPEND_LABELS;
  protected readonly ignore = IGNORE;
  protected readonly noKey = NO_KEY;
  protected readonly draft = signal<Record<string, Draft | undefined>>({});
  protected readonly keyDraft = signal<Record<string, Draft | undefined>>({});

  constructor() {
    addIcons({
      alertCircleOutline,
      arrowForwardOutline,
      closeOutline,
      warningOutline,
    });
  }

  protected onPairSource(sheet: string, pair: PairView, event: Event): void {
    const source = this.#value(event);
    if (!source || source === pair.source) return;
    void this.#reports.run(() => this.facade.pair(sheet, source, pair.master));
  }

  protected onPairMaster(sheet: string, pair: PairView, event: Event): void {
    const master = this.#value(event);
    if (!master || master === pair.master) return;
    void this.#reports.run(() => this.facade.pair(sheet, pair.source, master));
  }

  protected onUnpair(sheet: string, source: string): void {
    void this.#reports.run(() => this.facade.unpair(sheet, source));
  }

  protected onDraft(sheet: string, side: keyof Draft, event: Event): void {
    const draft = { ...this.draft()[sheet], [side]: this.#value(event) };
    const { source, master } = draft;
    if (source && master) {
      this.draft.update((drafts) => ({ ...drafts, [sheet]: undefined }));
      void this.#reports.run(() => this.facade.pair(sheet, source, master));
    } else {
      this.draft.update((drafts) => ({ ...drafts, [sheet]: draft }));
    }
  }

  protected onAnswer(sheet: string, column: string, event: Event): void {
    const value = this.#value(event);
    if (value === undefined) return;
    void this.#reports.run(() =>
      this.facade.answer(sheet, column, value === IGNORE ? undefined : value)
    );
  }

  protected onKey(view: StructureView, side: keyof Draft, event: Event): void {
    const sheet = view.run.sheet;
    const value = this.#value(event);
    if (value === NO_KEY) {
      this.keyDraft.update((drafts) => ({ ...drafts, [sheet]: undefined }));
      void this.#reports.run(() => this.facade.setKey(sheet, undefined));
      return;
    }
    const draft = {
      source: view.keySource,
      master: view.run.key,
      ...this.keyDraft()[sheet],
      [side]: value,
    };
    const { source, master } = draft;
    if (source && master) {
      if (source === view.keySource && master === view.run.key) return;
      this.keyDraft.update((drafts) => ({ ...drafts, [sheet]: undefined }));
      void this.#reports.run(() =>
        this.facade.setIdentifier(sheet, source, master)
      );
    } else {
      this.keyDraft.update((drafts) => ({ ...drafts, [sheet]: draft }));
    }
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
