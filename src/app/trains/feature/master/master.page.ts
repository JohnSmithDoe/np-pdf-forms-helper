// ─── why ────────────────────────────────────────────────────────
// The customer's master workbook: which file it is, which of its sheets each
// template refreshes, and the button that writes the refreshed COPY. The
// original is never written — the page says so, because that is the promise
// that makes pressing the button safe.
//
// Every change sends the WHOLE `MasterSettings` back, like `restage_import`
// takes the whole plan: the backend answers with a fresh `MasterView`, and the
// sheet headers the key and alias selects offer come from that answer, so a
// newly bound sheet's columns appear as soon as it is chosen.
//
// The selects offer only what exists — the workbook's sheets, the bound sheet's
// header row, the template's columns — so a binding cannot name a column by a
// typo. A binding whose names went stale (a renamed sheet) still shows, with
// the stale value, and the refresh reports it rather than this page hiding it.
//
// `snapshot` and `feed` are spelled for the user as what they do to the sheet:
// „Stand ersetzen“ and „Fortlaufend ergänzen“ (docs/decisions.md).
//
// The IMPORT runs the other way: every binding with an „Inhalt“ (its sheet
// kind) is read into the Schattensystem, sheet by sheet through the import
// walk, after the facts are emptied — the Schattensystem mirrors the master.
// That empties data, so it asks first and says what stays. A run that was
// left before its last sheet is INCOMPLETE and says so, with „Fortsetzen“
// walking only the sheets still open; checking a partial mirror against the
// customer as if it were whole is the mistake the banner exists to prevent.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
} from '@angular/core';
import {
  IonBackButton,
  IonButton,
  IonButtons,
  IonContent,
  IonHeader,
  IonIcon,
  IonItem,
  IonLabel,
  IonList,
  IonListHeader,
  IonNote,
  IonSelect,
  IonSelectOption,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { Router, RouterLink } from '@angular/router';
import { addIcons } from 'ionicons';
import {
  addOutline,
  cloudDownloadOutline,
  eyeOutline,
  folderOpenOutline,
  refreshOutline,
  trashOutline,
  warningOutline,
} from 'ionicons/icons';
import { OverlayService } from '../../../@shared/data/overlays/overlay.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import type { ClientReport } from '../../../@shared/model/client.types';
import { ImportWalkFacade, TrainsFacade } from '../../data';
import type {
  MasterBinding,
  MasterMode,
  MasterSettings,
  SheetKind,
} from '../../model/trains.types';

interface ModeOption {
  value: MasterMode;
  label: string;
}

const MODES: readonly ModeOption[] = [
  { value: 'snapshot', label: 'Stand ersetzen' },
  { value: 'feed', label: 'Fortlaufend ergänzen' },
];

const NO_KEY = '';
const NO_KIND = '';

interface KindOption {
  value: SheetKind;
  label: string;
}

const KINDS: readonly KindOption[] = [
  { value: 'wagenliste', label: 'Wagenliste (eine Zeile je Wagen)' },
  { value: 'radsatzEinbau', label: 'Radsätze mit Einbauposition' },
  { value: 'radsatzBestand', label: 'Radsätze ohne Einbauposition' },
];

@Component({
  selector: 'app-page-trains-master',
  templateUrl: 'master.page.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'ion-page' },
  imports: [
    BusyOverlayComponent,
    IonBackButton,
    IonButton,
    IonButtons,
    IonContent,
    IonHeader,
    IonIcon,
    IonItem,
    IonLabel,
    IonList,
    IonListHeader,
    IonNote,
    IonSelect,
    IonSelectOption,
    IonTitle,
    IonToolbar,
    RouterLink,
  ],
})
export class TrainsMasterPage {
  protected readonly facade = inject(TrainsFacade);
  readonly #walk = inject(ImportWalkFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #overlays = inject(OverlayService);
  readonly #router = inject(Router);

  protected readonly modes = MODES;
  protected readonly kinds = KINDS;
  protected readonly noKey = NO_KEY;
  protected readonly noKind = NO_KIND;

  protected readonly view = this.facade.master;
  protected readonly settings = computed<MasterSettings>(
    () => this.view()?.settings ?? { bindings: [] }
  );
  protected readonly templates = computed(() => this.facade.templates() ?? []);
  protected readonly ready = computed(
    () =>
      !!this.settings().file &&
      this.settings().bindings.some((binding) => binding.templateId) &&
      !this.view()?.problem
  );
  protected readonly importReady = computed(
    () =>
      !!this.settings().file &&
      this.settings().bindings.some((binding) => binding.kind) &&
      !this.view()?.problem
  );
  protected readonly open = computed(() => {
    const run = this.settings().importRun;
    return run ? run.sheets.filter((sheet) => !run.done.includes(sheet)) : [];
  });

  constructor() {
    addIcons({
      addOutline,
      cloudDownloadOutline,
      eyeOutline,
      folderOpenOutline,
      refreshOutline,
      trashOutline,
      warningOutline,
    });
    void this.#reports.run(() => this.facade.loadMaster());
  }

  protected headersOf(sheet: string): string[] {
    return (
      this.view()?.headers.find((candidate) => candidate.name === sheet)
        ?.headers ?? []
    );
  }

  protected sourceHeadersOf(templateId: string): string[] {
    return (
      this.templates()
        .find((template) => template.id === templateId)
        ?.plan.columns.map((column) => column.header)
        .filter((header) => header.trim() !== '') ?? []
    );
  }

  protected onPick(): void {
    void this.#reports.run(() => this.facade.pickMasterFile());
  }

  protected onAdd(): void {
    const bound = new Set(this.settings().bindings.map((b) => b.sheet));
    const binding: MasterBinding = {
      sheet: this.view()?.sheets.find((sheet) => !bound.has(sheet)) ?? '',
      templateId: this.templates()[0]?.id ?? '',
      mode: 'snapshot',
      aliases: [],
    };
    this.#save([...this.settings().bindings, binding]);
  }

  protected onRemove(index: number): void {
    this.#save(this.settings().bindings.filter((_, at) => at !== index));
  }

  protected onChange(index: number, patch: Partial<MasterBinding>): void {
    this.#update(index, (binding) => ({ ...binding, ...patch }));
  }

  protected onSheet(index: number, event: Event): void {
    const sheet = this.#value(event);
    if (sheet === undefined) return;
    this.#update(index, (binding) => ({
      ...binding,
      sheet,
      key: undefined,
      aliases: [],
    }));
  }

  protected onKind(index: number, event: Event): void {
    const kind = this.#value(event);
    if (kind === undefined) return;
    this.onChange(index, {
      kind: kind === NO_KIND ? undefined : (kind as SheetKind),
    });
  }

  protected async onImport(): Promise<void> {
    const confirmed = await this.#overlays.confirm(
      'Master importieren? Wagen, Radsätze, Einbauten und Instandhaltungen im Schattensystem werden geleert und Blatt für Blatt aus der Master-Datei neu aufgebaut. Partner, Vorlagen und Dokumente bleiben.'
    );
    if (!confirmed) return;
    let sheets: string[] = [];
    const ok = await this.#reports.run(async () => {
      sheets = await this.facade.startMasterImport();
      await this.#walk.startMaster(sheets);
    });
    if (ok && sheets.length) {
      await this.#router.navigate(['/trains/import/partners']);
    }
  }

  protected async onContinue(): Promise<void> {
    const sheets = this.open();
    const ok = await this.#reports.run(() => this.#walk.startMaster(sheets));
    if (ok && sheets.length) {
      await this.#router.navigate(['/trains/import/partners']);
    }
  }

  protected onTemplate(index: number, event: Event): void {
    const templateId = this.#value(event);
    if (templateId !== undefined) this.onChange(index, { templateId });
  }

  protected onMode(index: number, event: Event): void {
    const mode = this.#value(event) as MasterMode | undefined;
    if (mode) this.onChange(index, { mode });
  }

  protected onKey(index: number, event: Event): void {
    const key = this.#value(event);
    if (key === undefined) return;
    this.onChange(index, { key: key === NO_KEY ? undefined : key });
  }

  protected onAddAlias(index: number): void {
    this.#update(index, (binding) => ({
      ...binding,
      aliases: [...binding.aliases, { master: '', source: '' }],
    }));
  }

  protected onAlias(
    index: number,
    at: number,
    side: 'master' | 'source',
    event: Event
  ): void {
    const value = this.#value(event);
    if (value === undefined) return;
    this.#update(index, (binding) => ({
      ...binding,
      aliases: binding.aliases.map((alias, position) =>
        position === at ? { ...alias, [side]: value } : alias
      ),
    }));
  }

  protected onRemoveAlias(index: number, at: number): void {
    this.#update(index, (binding) => ({
      ...binding,
      aliases: binding.aliases.filter((_, position) => position !== at),
    }));
  }

  protected async onRefresh(): Promise<void> {
    let report: ClientReport | undefined;
    const ok = await this.#reports.run(async () => {
      report = await this.facade.refreshMaster();
    });
    if (!ok || !report) return;
    const folder = await this.#reports.show(report);
    if (folder) await this.#reports.run(() => this.facade.openFolder(folder));
  }

  #update(index: number, change: (binding: MasterBinding) => MasterBinding) {
    this.#save(
      this.settings().bindings.map((binding, at) =>
        at === index ? change(binding) : binding
      )
    );
  }

  #save(bindings: MasterBinding[]): void {
    void this.#reports.run(() =>
      this.facade.saveMaster({ ...this.settings(), bindings })
    );
  }

  #value(event: Event): string | undefined {
    return (event as CustomEvent<{ value?: string }>).detail.value;
  }
}
