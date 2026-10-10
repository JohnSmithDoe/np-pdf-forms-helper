// ─── why ────────────────────────────────────────────────────────
// The Schattensystem hub, reached from the menu's „Schattensystem“ entry. Every
// entity list, the import and the master import are reached from this page
// rather than from the menu. Cleaning and the master file live on the other
// hub, „Dokumente“ (`feature/start`); the Dokumente tile stays here too because
// an import into the Schattensystem starts from a filed document.
//
// The tiles carry COUNTS, which is what makes this a dashboard rather than a
// second menu: „Wagen 128“ answers the question the list would have to be opened
// to answer, and a zero is the tell that an import has not happened yet.
//
// Where a count comes from is not uniform, and that is deliberate. `counts` is
// the backend's own tally over the whole store and is what `wagen`, `radsaetze`
// and `instandhaltungen` show — the events list is PAGED, so its loaded length
// would say 100. The three partner roles are not counted server-side at all
// (`counts.partners` is every partner, whatever their roles), so they are
// derived from the loaded list the same way `PartnerListFacade` filters it, and a
// partner in two roles is counted in both — which is what the lists show too.
//
// Einstellungen is the one tile with no count — it is not a list — and it comes
// last for the same reason.
//
// `undefined` means "not loaded yet" and renders no number at all, rather than a
// 0 that would read as an empty store. The route sits under the domain's
// resolver, so in practice the data is there before the page is.
//
// Dokumente comes first: the files the app owns, and the place an import
// starts from.
//
// „Export erstellen“ sits in the toolbar here because the dashboard is the
// domain's one way in. It runs `silent` and presents its own report, since no
// page in trains listens on `report$` any more.
//
// A master import left before its last sheet leaves the Schattensystem HALF a
// mirror, and every count on this page would read as the whole. So the open run
// is a banner above the tiles, from `masterImportRun` — which rides on the list
// answers, so the dashboard learns it without opening the workbook. It links to
// the master page rather than continuing here: that page owns the run.
//
// The entity tiles — Wagen, Radsätze, Instandhaltungen and the three partner
// roles — are always there: the Schattensystem is the information hub. So is
// „Master-Import“ (`/trains/master`), which EMPTIES the current data and
// rebuilds it from the master's sheets — named apart from „Master-Datei“ on the
// other hub, which only chooses the customer's workbook.
// „Telematik“ is its own tile beside Wagen: someone after a silent device
// starts from the devices. Its count and „N stumm“ come from the same view Rust
// builds for the list (`get_telematik`), reloaded whenever the Wagen-Zustand
// changes, so the tile and the list cannot disagree about who is silent. A
// failed load leaves the tile at „wird geladen …“ rather than raising a toast
// on every visit to the hub.
// Without `fullEnabled` only Vorlagen, Einstellungen and „Export erstellen“ are
// hidden, not removed.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  untracked,
} from '@angular/core';
import { Router } from '@angular/router';
import {
  IonButton,
  IonButtons,
  IonCard,
  IonCardContent,
  IonCardHeader,
  IonCardSubtitle,
  IonCardTitle,
  IonContent,
  IonHeader,
  IonIcon,
  IonItem,
  IonLabel,
  IonList,
  IonMenuButton,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import {
  albumsOutline,
  buildOutline,
  businessOutline,
  cloudUploadOutline,
  documentsOutline,
  ellipseOutline,
  gridOutline,
  peopleOutline,
  radioOutline,
  ribbonOutline,
  settingsOutline,
  trainOutline,
  warningOutline,
} from 'ionicons/icons';
import { SettingsService } from '../../../@shared/data/settings/settings.service';
import { ReportPresenterService } from '../../../@shared/feature/report/report-presenter.service';
import { TelematikListFacade, TrainsFacade } from '../../data';
import type { ClientReport } from '../../../@shared/model/client.types';
import type { PartnerRolle } from '../../model/trains.types';

interface DashboardTile {
  route: string;
  label: string;
  icon: string;
  description: string;
  count?: number;
  subtitle?: string;
}

const FULL_ONLY = new Set(['/trains/templates', '/trains/settings']);

@Component({
  selector: 'app-page-trains-dashboard',
  templateUrl: 'dashboard.page.html',
  styleUrls: ['dashboard.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'ion-page' },
  imports: [
    IonButton,
    IonButtons,
    IonItem,
    IonLabel,
    IonList,
    IonCard,
    IonCardContent,
    IonCardHeader,
    IonCardSubtitle,
    IonCardTitle,
    IonContent,
    IonHeader,
    IonIcon,
    IonMenuButton,
    IonTitle,
    IonToolbar,
  ],
})
export class TrainsDashboardPage {
  readonly #facade = inject(TrainsFacade);
  readonly #telematik = inject(TelematikListFacade);
  readonly #router = inject(Router);
  readonly #reports = inject(ReportPresenterService);
  protected readonly fullEnabled = inject(SettingsService).fullEnabled;

  protected readonly tiles = computed<DashboardTile[]>(() =>
    this.fullEnabled
      ? this.#allTiles()
      : this.#allTiles().filter((tile) => !FULL_ONLY.has(tile.route))
  );

  readonly #allTiles = computed<DashboardTile[]>(() => {
    const counts = this.#facade.counts();
    const loaded = this.#facade.loaded();
    return [
      {
        route: '/trains/documents',
        label: 'Dokumente',
        icon: 'documents-outline',
        description:
          'Die bereinigten Dateien — von hier aus ins Schattensystem importieren.',
        count: loaded ? counts.dokumente : undefined,
      },
      {
        route: '/trains/wagen',
        label: 'Wagen',
        icon: 'train-outline',
        description: 'Die Güterwagen, erkannt an ihrer Wagennummer.',
        count: loaded ? counts.wagen : undefined,
      },
      {
        route: '/trains/telematik',
        label: 'Telematik',
        icon: 'radio-outline',
        description:
          'Wo die Wagen stehen und welche sich lange nicht gemeldet haben.',
        subtitle: this.#telematikSubtitle(),
      },
      {
        route: '/trains/radsaetze',
        label: 'Radsätze',
        icon: 'ellipse-outline',
        description: 'Radsätze und in welchem Wagen sie gerade laufen.',
        count: loaded ? counts.radsaetze : undefined,
      },
      {
        route: '/trains/instandhaltungen',
        label: 'Instandhaltungen',
        icon: 'build-outline',
        description: 'Wartung, Inspektion, Instandsetzung und Verbesserung.',
        count: loaded ? counts.events : undefined,
      },
      {
        route: '/trains/werkstaetten',
        label: 'Werkstätten',
        icon: 'business-outline',
        description: 'Wer die Arbeit ausgeführt und die Datei geschickt hat.',
        count: this.#partnerCount('werkstatt'),
      },
      {
        route: '/trains/halter',
        label: 'Halter',
        icon: 'people-outline',
        description: 'Der im NVR eingetragene Halter des Wagens.',
        count: this.#partnerCount('halter'),
      },
      {
        route: '/trains/eigentuemer',
        label: 'Eigentümer',
        icon: 'ribbon-outline',
        description: 'Wem der Wagen gehört — meist eine Leasinggesellschaft.',
        count: this.#partnerCount('eigentuemer'),
      },
      {
        route: '/trains/templates',
        label: 'Vorlagen',
        icon: 'albums-outline',
        description: 'Gespeicherte Spaltenzuordnungen, eine je Dateiform.',
        count: this.#facade.templates()?.length,
      },
      {
        route: '/trains/master',
        label: 'Master-Import',
        icon: 'grid-outline',
        description:
          'Die Master-Datei ins Schattensystem importieren — die aktuellen Daten werden dabei geleert — und Blatt für Blatt ansehen.',
        subtitle: 'Import und Ansicht',
      },
      {
        route: '/trains/settings',
        label: 'Einstellungen',
        icon: 'settings-outline',
        description: 'Wie das Schattensystem Wagennummern schreibt und zeigt.',
        subtitle: 'Für alle Listen und Dateien',
      },
    ];
  });

  protected readonly openSheets = computed(() => {
    const run = this.#facade.masterImportRun();
    return run ? run.sheets.filter((sheet) => !run.done.includes(sheet)) : [];
  });

  constructor() {
    effect(() => {
      this.#facade.zustand();
      untracked(() => void this.#telematik.load().catch(() => {}));
    });
    addIcons({
      albumsOutline,
      buildOutline,
      businessOutline,
      cloudUploadOutline,
      documentsOutline,
      ellipseOutline,
      gridOutline,
      peopleOutline,
      radioOutline,
      ribbonOutline,
      settingsOutline,
      trainOutline,
      warningOutline,
    });
  }

  protected entries(count: number | undefined): string {
    if (count === undefined) return 'wird geladen …';
    return count === 1 ? '1 Eintrag' : `${count} Einträge`;
  }

  protected open(tile: DashboardTile): void {
    void this.#router.navigate([tile.route]);
  }

  protected onMaster(): void {
    void this.#router.navigate(['/trains/master']);
  }

  protected async onExport(): Promise<void> {
    let report: ClientReport | undefined;
    const ok = await this.#reports.run(async () => {
      report = await this.#facade.createExport();
    });
    if (!ok || !report) return;
    const folder = await this.#reports.show(report);
    if (folder) await this.#reports.run(() => this.#facade.openFolder(folder));
  }

  #telematikSubtitle(): string | undefined {
    const count = this.#telematik.count();
    if (count === undefined) return undefined;
    const stumm = this.#telematik.stumm() ?? 0;
    const wagen = count === 1 ? '1 Wagen' : `${count} Wagen`;
    return stumm ? `${wagen} · ${stumm} stumm` : wagen;
  }

  #partnerCount(role: PartnerRolle): number | undefined {
    return this.#facade
      .partners()
      ?.filter((partner) => partner.rollen.includes(role)).length;
  }
}
