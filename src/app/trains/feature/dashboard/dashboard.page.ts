// ─── why ────────────────────────────────────────────────────────
// The trains hub. One menu entry — „Schattensystem“ — leads here, and every
// entity list plus the import is reached from this page rather than from the
// menu. Eight sibling entries said nothing about what belongs to what; a hub
// says it once, and the menu is back to the three things the app is made of.
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
// `undefined` means "not loaded yet" and renders no number at all, rather than a
// 0 that would read as an empty store. The route sits under the domain's
// resolver, so in practice the data is there before the page is.
//
// Import gets no count and comes first: it is the only tile that is an ACTION,
// and everything the others show arrives through it.
// ────────────────────────────────────────────────────────────────

import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
} from '@angular/core';
import { Router } from '@angular/router';
import {
  IonButtons,
  IonCard,
  IonCardContent,
  IonCardHeader,
  IonCardSubtitle,
  IonCardTitle,
  IonContent,
  IonHeader,
  IonIcon,
  IonMenuButton,
  IonNote,
  IonTitle,
  IonToolbar,
} from '@ionic/angular/standalone';
import { addIcons } from 'ionicons';
import {
  albumsOutline,
  buildOutline,
  businessOutline,
  cloudUploadOutline,
  ellipseOutline,
  peopleOutline,
  ribbonOutline,
  trainOutline,
} from 'ionicons/icons';
import { TrainsFacade } from '../../data';
import type { PartnerRolle } from '../../model/trains.types';

interface DashboardTile {
  route: string;
  label: string;
  icon: string;
  description: string;
  count?: number;
}

@Component({
  selector: 'app-page-trains-dashboard',
  templateUrl: 'dashboard.page.html',
  styleUrls: ['dashboard.page.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { class: 'ion-page' },
  imports: [
    IonButtons,
    IonCard,
    IonCardContent,
    IonCardHeader,
    IonCardSubtitle,
    IonCardTitle,
    IonContent,
    IonHeader,
    IonIcon,
    IonMenuButton,
    IonNote,
    IonTitle,
    IonToolbar,
  ],
})
export class TrainsDashboardPage {
  readonly #facade = inject(TrainsFacade);
  readonly #router = inject(Router);

  protected readonly loaded = this.#facade.loaded;

  protected readonly importTile: DashboardTile = {
    route: '/trains/import',
    label: 'Import',
    icon: 'cloud-upload-outline',
    description:
      'Einen Ordner oder Dateien einlesen, erkennen lassen, bereinigen, prüfen und übernehmen.',
  };

  protected readonly tiles = computed<DashboardTile[]>(() => {
    const counts = this.#facade.counts();
    const loaded = this.#facade.loaded();
    return [
      {
        route: '/trains/wagen',
        label: 'Wagen',
        icon: 'train-outline',
        description: 'Die Güterwagen, erkannt an ihrer Wagennummer.',
        count: loaded ? counts.wagen : undefined,
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
    ];
  });

  constructor() {
    addIcons({
      albumsOutline,
      buildOutline,
      businessOutline,
      cloudUploadOutline,
      ellipseOutline,
      peopleOutline,
      ribbonOutline,
      trainOutline,
    });
  }

  protected entries(count: number | undefined): string {
    if (count === undefined) return 'wird geladen …';
    return count === 1 ? '1 Eintrag' : `${count} Einträge`;
  }

  protected open(tile: DashboardTile): void {
    void this.#router.navigate([tile.route]);
  }

  #partnerCount(role: PartnerRolle): number | undefined {
    return this.#facade
      .partners()
      ?.filter((partner) => partner.rollen.includes(role)).length;
  }
}
