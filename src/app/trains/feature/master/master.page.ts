// ─── why ────────────────────────────────────────────────────────
// The customer's master workbook: which version is current, and how each of
// its sheets is bound — what it IS, and which template's documents the export wizard
// suggests for it, with the key and aliases that export remembered. Writing
// happens in that wizard, started per document from the document list; each
// write is a new version of the client master, and the page says so.
//
// The file is NOT chosen here. The client master (`/trains/master-file`) is the
// only master, and Rust points everything on this page at its current version
// (`bindings::follow`); the row links to its versions instead of a picker.
//
// Every change sends the WHOLE `MasterSettings` back, like `restage_import`
// takes the whole plan: the backend answers with a fresh `MasterView`, and the
// sheet headers the key and alias selects offer come from that answer, so a
// newly bound sheet's columns appear as soon as it is chosen.
//
// The selects offer only what exists — the workbook's sheets, the bound sheet's
// header row, the template's columns — so a binding cannot name a column by a
// typo. A binding whose names went stale (a renamed sheet) still shows, with
// the stale value, and the export reports it rather than this page hiding it.
//
// There is no mode: the master update is always incremental, matched by the
// key column set here (docs/decisions.md, „Die Kunden-Master ist die einzige
// Master“).
//
// The IMPORT runs the other way: every binding with an „Inhalt“ (its sheet
// kind) is read into the Schattensystem, sheet by sheet through the import
// walk, after the facts are emptied — the Schattensystem mirrors the master.
// That empties data, so it asks first and says what stays. „Alles
// importieren“ is the same run in one backend call (`import_master_all`): it
// still asks that one question, because it still empties, and then nothing
// else — Rust answers every group the way „alle neuen anlegen“ would and the
// page shows its report afterwards. A run that was
// left before its last sheet is INCOMPLETE and says so, with „Fortsetzen“
// walking only the sheets still open; checking a partial mirror against the
// customer as if it were whole is the mistake the banner exists to prevent.
//
// The real master has 28 sheets, so a binding is ONE ROW — sheet, what it
// imports, which template it is exported from — and its selects open on demand. Five selects on
// each of 28 sheets would make the list unreadable, and most bindings are
// never edited by hand.
//
// Nobody has to bind anything: taking a version over binds EVERY sheet by its
// header row (Rust, `kinds::recognise`), and „Standardzuordnung“ restores
// those defaults. A binding the user changes is marked `auto: false` here —
// the page is the only place a hand edit happens — and its row says
// „von Hand angepasst“, because that is exactly what a reset would throw away.
// Reading the file costs seconds on the real master, which is why only taking a
// version over, opening this page on a new one, the reset and the import touch
// it; the busy overlay covers them.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  signal,
} from '@angular/core';
import {
  IonBackButton,
  IonButton,
  IonButtons,
  IonContent,
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
} from '@ionic/angular/standalone';
import { Router, RouterLink } from '@angular/router';
import { addIcons } from 'ionicons';
import {
  addOutline,
  chevronUpOutline,
  cloudDownloadOutline,
  createOutline,
  flashOutline,
  eyeOutline,
  folderOpenOutline,
  refreshOutline,
  trashOutline,
  warningOutline,
} from 'ionicons/icons';
import { OverlayService } from '../../../@shared/data/overlays/overlay.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import type { ClientReport } from '../../../@shared/model/client.types';
import { BusyOverlayComponent } from '../../../@shared/ui/busy-overlay/busy-overlay.component';
import { ImportWalkFacade, MasterFileFacade, TrainsFacade } from '../../data';
import type {
  MasterBinding,
  MasterSettings,
  SheetKind,
} from '../../model/trains.types';

const NO_KEY = '';
const NO_KIND = '';

interface KindOption {
  value: SheetKind;
  label: string;
  short: string;
}

const KINDS: readonly KindOption[] = [
  {
    value: 'wagenliste',
    label: 'Wagenliste (eine Zeile je Wagen)',
    short: 'Wagen',
  },
  {
    value: 'radsatzEinbau',
    label: 'Radsätze mit Einbauposition',
    short: 'Radsätze mit Position',
  },
  {
    value: 'radsatzBestand',
    label: 'Radsätze ohne Einbauposition',
    short: 'Radsätze ohne Position',
  },
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
    IonItemGroup,
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
  protected readonly masterFile = inject(MasterFileFacade);
  readonly #walk = inject(ImportWalkFacade);
  readonly #reports = inject(ReportPresenterService);
  readonly #overlays = inject(OverlayService);
  readonly #router = inject(Router);

  protected readonly kinds = KINDS;
  protected readonly noKey = NO_KEY;
  protected readonly noKind = NO_KIND;

  protected readonly view = this.facade.master;
  readonly #expanded = signal<number[]>([]);
  protected readonly settings = computed<MasterSettings>(
    () => this.view()?.settings ?? { bindings: [] }
  );
  protected readonly templates = computed(() => this.facade.templates() ?? []);
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
      chevronUpOutline,
      cloudDownloadOutline,
      createOutline,
      eyeOutline,
      flashOutline,
      folderOpenOutline,
      refreshOutline,
      trashOutline,
      warningOutline,
    });
    void this.#reports.run(() => this.facade.loadMaster());
  }

  protected isExpanded(index: number): boolean {
    return this.#expanded().includes(index);
  }

  protected toggle(index: number): void {
    this.#expanded.update((open) =>
      open.includes(index)
        ? open.filter((entry) => entry !== index)
        : [...open, index]
    );
  }

  protected summary(binding: MasterBinding): string {
    const kind = KINDS.find((option) => option.value === binding.kind);
    const template = this.templates().find(
      (candidate) => candidate.id === binding.templateId
    );
    return [
      kind ? `importiert: ${kind.short}` : 'nur Ansicht',
      template ? `aufgefrischt aus „${template.name}“` : undefined,
      binding.auto ? undefined : 'von Hand angepasst',
    ]
      .filter(Boolean)
      .join(' · ');
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

  protected async onReset(): Promise<void> {
    const edited = this.settings().bindings.some((binding) => !binding.auto);
    if (edited) {
      const confirmed = await this.#overlays.confirm(
        'Standardzuordnung wiederherstellen? Von Hand angepasste Zuordnungen gehen verloren; jedes Blatt wird wieder nach seiner Kopfzeile zugeordnet.'
      );
      if (!confirmed) return;
    }
    this.#expanded.set([]);
    await this.#reports.run(() => this.facade.resetMasterBindings());
  }

  protected onAdd(): void {
    const bound = new Set(this.settings().bindings.map((b) => b.sheet));
    const binding: MasterBinding = {
      sheet: this.view()?.sheets.find((sheet) => !bound.has(sheet)) ?? '',
      templateId: this.templates()[0]?.id ?? '',
      aliases: [],
      auto: false,
    };
    this.#expanded.set([this.settings().bindings.length]);
    this.#save([...this.settings().bindings, binding]);
  }

  protected onRemove(index: number): void {
    this.#expanded.set([]);
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
      'Master importieren? Die aktuellen Daten im Schattensystem — Wagen, Radsätze, Einbauten, Instandhaltungen und der Wagen-Zustand — werden geleert und Blatt für Blatt aus der Master-Datei neu aufgebaut. Partner, Vorlagen und Dokumente bleiben.'
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

  protected async onImportAll(): Promise<void> {
    const confirmed = await this.#overlays.confirm(
      'Ganze Master-Datei ohne Rückfragen importieren? Die aktuellen Daten im Schattensystem — Wagen, Radsätze, Einbauten, Instandhaltungen und der Wagen-Zustand — werden geleert und aus allen Blättern neu aufgebaut. Neues wird angelegt; was nur ähnlich oder mehrdeutig ist, bleibt unverknüpft. Partner, Vorlagen und Dokumente bleiben.'
    );
    if (!confirmed) return;
    let report: ClientReport | undefined;
    const ok = await this.#reports.run(async () => {
      report = await this.facade.importMasterAll();
    });
    if (ok && report) await this.#reports.show(report);
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

  #update(index: number, change: (binding: MasterBinding) => MasterBinding) {
    this.#save(
      this.settings().bindings.map((binding, at) =>
        at === index ? { ...change(binding), auto: false } : binding
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
